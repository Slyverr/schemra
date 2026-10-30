use std::collections::HashSet;

use schemra_syntax::{
    Diagnostic, SourceSpan, Statement, StatementKind, SyntaxFile, Token, TokenKind,
};

use crate::{Registry, Scope};

const MAX_REPEAT_ITERATIONS: usize = 1_000;
const MAX_REPEAT_EXPANDED_STATEMENTS: usize = 10_000;

pub(super) fn expand_visual_repeats(file: &mut SyntaxFile, registry: &Registry) -> Vec<Diagnostic> {
    let mut expander = Expander {
        registry,
        diagnostics: Vec::new(),
        expanded_statements: 0,
    };
    let statements = std::mem::take(&mut file.statements);
    file.statements = expander.expand_file(&statements);
    check_unexpanded_indices(&file.statements, &mut expander.diagnostics);
    expander.diagnostics
}

struct Expander<'a> {
    registry: &'a Registry,
    diagnostics: Vec<Diagnostic>,
    expanded_statements: usize,
}

impl Expander<'_> {
    fn expand_file(&mut self, statements: &[Statement]) -> Vec<Statement> {
        statements
            .iter()
            .map(|statement| {
                let mut expanded = statement.clone();
                if let StatementKind::Invocation { name, body, .. } = &mut expanded.kind
                    && name.text == "visual"
                    && let Some(body) = body
                {
                    *body = self.expand_visual_scope(body, Scope::Visual);
                }
                expanded
            })
            .collect()
    }

    fn expand_visual_scope(&mut self, statements: &[Statement], scope: Scope) -> Vec<Statement> {
        let mut expanded = Vec::new();
        for statement in statements {
            match &statement.kind {
                StatementKind::Invocation { name, .. } if name.text == "repeat" => {
                    expanded.extend(self.expand_repeat(statement, scope));
                }
                StatementKind::Invocation { name, .. }
                    if scope == Scope::Visual && name.text == "group" =>
                {
                    let mut group = statement.clone();
                    if let StatementKind::Invocation { body, .. } = &mut group.kind
                        && let Some(body) = body
                    {
                        *body = self.expand_visual_scope(body, Scope::Group);
                    }
                    expanded.push(group);
                }
                _ => expanded.push(statement.clone()),
            }
        }
        expanded
    }

    fn expand_repeat(&mut self, statement: &Statement, scope: Scope) -> Vec<Statement> {
        let StatementKind::Invocation {
            name,
            arguments,
            body: Some(body),
        } = &statement.kind
        else {
            self.diagnostics.push(error(
                "E_REPEAT_BODY",
                "A visual repeat needs a body containing nodes or links.",
                statement.span,
                "Add a block after the repeat bounds.",
            ));
            return Vec::new();
        };

        let repeat = match parse_bounds(arguments, name.span) {
            Ok(repeat) => repeat,
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                return Vec::new();
            }
        };

        if contains_repeat(body) {
            self.diagnostics.push(error(
                "E_NESTED_VISUAL_REPEAT",
                "Visual repeat blocks cannot be nested.",
                name.span,
                "Use one repeat block for each generated set of nodes and links.",
            ));
            return Vec::new();
        }

        if let Some(invalid) = body.iter().find(|statement| {
            !is_structural_statement(statement) && !matches!(statement.kind, StatementKind::Error)
        }) {
            self.diagnostics.push(error(
                "E_REPEAT_BODY_STATEMENT",
                "A visual repeat can contain only node declarations and links.",
                invalid.span,
                "Declare groups outside the repeat block, then repeat their nodes or links.",
            ));
            return Vec::new();
        }

        let template_statements = body
            .iter()
            .filter(|statement| is_structural_statement(statement))
            .count();
        if template_statements == 0 {
            self.diagnostics.push(error(
                "E_EMPTY_VISUAL_REPEAT",
                "A visual repeat needs at least one node or link.",
                name.span,
                "Add a node declaration or a link to the repeat block.",
            ));
            return Vec::new();
        }

        let expanded_count = template_statements.saturating_mul(repeat.indices.len());
        if self.expanded_statements.saturating_add(expanded_count) > MAX_REPEAT_EXPANDED_STATEMENTS
        {
            self.diagnostics.push(error(
                "E_VISUAL_REPEAT_EXPANSION_LIMIT",
                format!(
                    "Visual repeat blocks can expand to at most {MAX_REPEAT_EXPANDED_STATEMENTS} nodes and links in one source file."
                ),
                name.span,
                "Reduce the repeat count or split the graph into smaller source files.",
            ));
            return Vec::new();
        }
        self.expanded_statements += expanded_count;

        let implicit_ids = body
            .iter()
            .filter_map(|statement| match &statement.kind {
                StatementKind::Invocation {
                    name, arguments, ..
                } if name.text == "node" => arguments.first(),
                _ => None,
            })
            .filter(|id| indexed_identifier(&id.text).ok().flatten().is_none())
            .map(|id| id.text.clone())
            .collect::<HashSet<_>>();

        if repeat.indices.is_empty() {
            let probe_index = repeat.start.unwrap_or(1);
            let Some(probe) = instantiate(
                body,
                &implicit_ids,
                repeat.binding.as_deref(),
                probe_index,
                self,
            ) else {
                return Vec::new();
            };
            self.diagnostics
                .extend(self.registry.validate_statements_in_scope(&probe, scope));
            return Vec::new();
        }

        let mut output = Vec::with_capacity(expanded_count);
        for index in repeat.indices {
            let Some(mut instance) =
                instantiate(body, &implicit_ids, repeat.binding.as_deref(), index, self)
            else {
                return Vec::new();
            };
            output.append(&mut instance);
        }
        output
    }
}

struct RepeatBounds {
    indices: Vec<usize>,
    start: Option<usize>,
    binding: Option<String>,
}

fn parse_bounds(arguments: &[Token], repeat_span: SourceSpan) -> Result<RepeatBounds, Diagnostic> {
    let first = arguments.first().ok_or_else(|| {
        error(
            "E_VISUAL_REPEAT_COUNT",
            "Add a repeat count or inclusive range.",
            repeat_span,
            "Write `repeat 4` or `repeat 2..=5 as i`.",
        )
    })?;
    let (start, indices, tail) = match (&first.kind, arguments.get(1), arguments.get(2)) {
        (TokenKind::Integer(_), Some(range), Some(last))
            if matches!(range.kind, TokenKind::RangeInclusive) =>
        {
            let start = parse_bound(first, "E_VISUAL_REPEAT_RANGE")?;
            let end = parse_bound(last, "E_VISUAL_REPEAT_RANGE")?;
            if start > end {
                return Err(error(
                    "E_VISUAL_REPEAT_RANGE",
                    "An inclusive visual repeat range cannot descend.",
                    SourceSpan {
                        start: first.span.start,
                        end: last.span.end,
                    },
                    "Put the smaller index first, such as `2..=5`.",
                ));
            }
            if end - start >= MAX_REPEAT_ITERATIONS {
                return Err(error(
                    "E_VISUAL_REPEAT_COUNT",
                    format!("A visual repeat cannot exceed {MAX_REPEAT_ITERATIONS} iterations."),
                    last.span,
                    "Reduce the range to at most 1,000 iterations.",
                ));
            }
            (
                Some(start),
                (start..=end).collect::<Vec<_>>(),
                &arguments[3..],
            )
        }
        (TokenKind::Integer(_), _, _) => {
            let count = parse_bound(first, "E_VISUAL_REPEAT_COUNT")?;
            if count > MAX_REPEAT_ITERATIONS {
                return Err(error(
                    "E_VISUAL_REPEAT_COUNT",
                    format!("A visual repeat cannot exceed {MAX_REPEAT_ITERATIONS} iterations."),
                    first.span,
                    "Choose a count from 0 to 1,000.",
                ));
            }
            (Some(1), (1..=count).collect::<Vec<_>>(), &arguments[1..])
        }
        _ => {
            return Err(error(
                "E_VISUAL_REPEAT_COUNT",
                "Repeat bounds must be whole numbers.",
                first.span,
                "Write `repeat 4` or `repeat 2..=5 as i`.",
            ));
        }
    };

    let binding = match tail {
        [] => None,
        [keyword, binding]
            if keyword.text == "as" && matches!(binding.kind, TokenKind::Identifier) =>
        {
            Some(binding.text.clone())
        }
        [keyword, binding] if keyword.text == "as" => {
            return Err(error(
                "E_VISUAL_REPEAT_BINDING",
                "A repeat binding must be a simple identifier.",
                binding.span,
                "Use a name such as `i` or `index`.",
            ));
        }
        [extra, ..] => {
            return Err(error(
                "E_VISUAL_REPEAT_BINDING",
                "Unexpected value in visual repeat header.",
                extra.span,
                "Use `as name` to bind the repeat index.",
            ));
        }
    };

    Ok(RepeatBounds {
        indices,
        start,
        binding,
    })
}

fn parse_bound(token: &Token, code: &'static str) -> Result<usize, Diagnostic> {
    token.text.parse::<usize>().map_err(|_| {
        error(
            code,
            "Repeat bounds must be whole numbers.",
            token.span,
            "Use a whole number between 0 and 1,000.",
        )
    })
}

fn instantiate(
    body: &[Statement],
    implicit_ids: &HashSet<String>,
    binding: Option<&str>,
    index: usize,
    expander: &mut Expander<'_>,
) -> Option<Vec<Statement>> {
    let mut statements = body.to_vec();
    for statement in &mut statements {
        if !substitute_statement(
            statement,
            implicit_ids,
            binding,
            index,
            &mut expander.diagnostics,
        ) {
            return None;
        }
    }
    Some(statements)
}

fn substitute_statement(
    statement: &mut Statement,
    implicit_ids: &HashSet<String>,
    binding: Option<&str>,
    index: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    match &mut statement.kind {
        StatementKind::Invocation {
            name,
            arguments,
            body,
        } => {
            if name.text == "node"
                && let Some(id) = arguments.first_mut()
                && !substitute_identifier(id, implicit_ids, binding, index, diagnostics)
            {
                return false;
            }
            for argument in arguments {
                substitute_string(argument, binding, index);
            }
            if let Some(body) = body {
                for child in body {
                    if !substitute_statement(child, implicit_ids, binding, index, diagnostics) {
                        return false;
                    }
                }
            }
        }
        StatementKind::Property {
            value,
            additional_values,
            ..
        } => {
            substitute_string(value, binding, index);
            for value in additional_values {
                substitute_string(value, binding, index);
            }
        }
        StatementKind::Edge {
            from,
            to,
            properties,
            ..
        } => {
            if !substitute_identifier(from, implicit_ids, binding, index, diagnostics)
                || !substitute_identifier(to, implicit_ids, binding, index, diagnostics)
            {
                return false;
            }
            if let Some(properties) = properties {
                for property in properties {
                    if !substitute_statement(property, implicit_ids, binding, index, diagnostics) {
                        return false;
                    }
                }
            }
        }
        StatementKind::Error => {}
    }
    true
}

fn substitute_identifier(
    token: &mut Token,
    implicit_ids: &HashSet<String>,
    binding: Option<&str>,
    index: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    match indexed_identifier(&token.text) {
        Ok(Some((base, variable))) => {
            if binding != Some(variable) {
                diagnostics.push(error(
                    "E_VISUAL_REPEAT_INDEX",
                    format!("Repeat index {variable:?} is not in scope."),
                    token.span,
                    "Use the declared repeat index, for example `user[i]` inside `repeat 4 as i`.",
                ));
                return false;
            }
            token.text = format!("{base}_{index}");
        }
        Ok(None) => {
            if implicit_ids.contains(&token.text) {
                token.text = format!("{}_{index}", token.text);
            }
        }
        Err(()) => {
            diagnostics.push(error(
                "E_VISUAL_REPEAT_INDEX",
                "A repeated node identifier has an invalid index expression.",
                token.span,
                "Use a simple repeat index such as `user[i]`.",
            ));
            return false;
        }
    }
    true
}

fn indexed_identifier(value: &str) -> Result<Option<(&str, &str)>, ()> {
    let Some(open) = value.find('[') else {
        return if value.contains(']') {
            Err(())
        } else {
            Ok(None)
        };
    };
    if !value.ends_with(']') {
        return Err(());
    }
    let base = &value[..open];
    let variable = &value[open + 1..value.len() - 1];
    if is_identifier(base) && is_identifier(variable) {
        Ok(Some((base, variable)))
    } else {
        Err(())
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn substitute_string(token: &mut Token, binding: Option<&str>, index: usize) {
    let Some(binding) = binding else { return };
    let needle = format!("{{{binding}}}");
    if let TokenKind::String(value) = &mut token.kind {
        if value.contains(&needle) {
            *value = value.replace(&needle, &index.to_string());
            token.text = token.text.replace(&needle, &index.to_string());
        }
    }
}

fn contains_repeat(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match &statement.kind {
        StatementKind::Invocation { name, body, .. } => {
            name.text == "repeat" || body.as_deref().is_some_and(contains_repeat)
        }
        StatementKind::Edge {
            properties: Some(properties),
            ..
        } => contains_repeat(properties),
        _ => false,
    })
}

fn is_structural_statement(statement: &Statement) -> bool {
    match &statement.kind {
        StatementKind::Edge { .. } => true,
        StatementKind::Invocation { name, .. } => name.text == "node",
        _ => false,
    }
}

fn check_unexpanded_indices(statements: &[Statement], diagnostics: &mut Vec<Diagnostic>) {
    for statement in statements {
        match &statement.kind {
            StatementKind::Invocation {
                name,
                arguments,
                body,
            } => {
                if name.text == "visual" || name.text == "group" {
                    for node in arguments.iter().take(1) {
                        report_unexpanded_index(node, diagnostics);
                    }
                    if let Some(body) = body {
                        check_unexpanded_indices(body, diagnostics);
                    }
                } else if name.text == "node" {
                    if let Some(id) = arguments.first() {
                        report_unexpanded_index(id, diagnostics);
                    }
                    if let Some(body) = body {
                        check_unexpanded_indices(body, diagnostics);
                    }
                }
            }
            StatementKind::Edge {
                from,
                to,
                properties,
                ..
            } => {
                report_unexpanded_index(from, diagnostics);
                report_unexpanded_index(to, diagnostics);
                if let Some(properties) = properties {
                    check_unexpanded_indices(properties, diagnostics);
                }
            }
            _ => {}
        }
    }
}

fn report_unexpanded_index(token: &Token, diagnostics: &mut Vec<Diagnostic>) {
    if indexed_identifier(&token.text).ok().flatten().is_some() {
        diagnostics.push(error(
            "E_VISUAL_REPEAT_INDEX_SCOPE",
            "Indexed node identifiers are only valid inside a visual repeat block.",
            token.span,
            "Declare the node and its links inside a matching `repeat` block.",
        ));
    }
}

fn error(
    code: &'static str,
    message: impl Into<String>,
    span: SourceSpan,
    suggestion: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error(code, message, span, Some(suggestion.into()))
}
