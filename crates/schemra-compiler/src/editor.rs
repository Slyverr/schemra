//! Source token metadata used by editor syntax coloring.

use std::collections::HashSet;

use schemra_syntax::{SourceSpan, Statement, StatementKind, TokenKind, parse_syntax};
use serde::Serialize;

use crate::{BodySpec, PropertyKind, Registry, Scope};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SyntaxTokenKind {
    Keyword,
    Property,
    Value,
    Identifier,
    Number,
    String,
    Comment,
    Operator,
    Punctuation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SyntaxToken {
    pub kind: SyntaxTokenKind,
    pub span: SourceSpan,
}

/// Classifies source tokens using the same construct and property registry as compilation.
pub fn highlight_tokens(source: &str) -> Vec<SyntaxToken> {
    let syntax = parse_syntax(source);
    let Ok(registry) = Registry::builtin_shared() else {
        return Vec::new();
    };

    let mut property_offsets = HashSet::new();
    let mut value_offsets = HashSet::new();
    collect_property_offsets(
        &syntax.statements,
        Scope::File,
        registry,
        &mut property_offsets,
        &mut value_offsets,
    );

    syntax
        .tokens
        .iter()
        .filter_map(|token| {
            let kind = match &token.kind {
                TokenKind::Comment(_) => SyntaxTokenKind::Comment,
                TokenKind::String(_) => SyntaxTokenKind::String,
                TokenKind::Integer(_) | TokenKind::Duration(_) => SyntaxTokenKind::Number,
                TokenKind::Identifier | TokenKind::BareValue(_)
                    if property_offsets.contains(&token.span.start.offset) =>
                {
                    SyntaxTokenKind::Property
                }
                TokenKind::Identifier | TokenKind::BareValue(_)
                    if value_offsets.contains(&token.span.start.offset) =>
                {
                    SyntaxTokenKind::Value
                }
                TokenKind::Identifier | TokenKind::BareValue(_) => {
                    if registry.is_highlighted_word(&token.text) {
                        SyntaxTokenKind::Keyword
                    } else {
                        SyntaxTokenKind::Identifier
                    }
                }
                TokenKind::Newline => return None,
                TokenKind::Arrow | TokenKind::Undirected => SyntaxTokenKind::Operator,
                TokenKind::Colon
                | TokenKind::LeftBrace
                | TokenKind::RightBrace
                | TokenKind::Semicolon
                | TokenKind::Error(_) => SyntaxTokenKind::Punctuation,
            };
            Some(SyntaxToken {
                kind,
                span: token.span,
            })
        })
        .collect()
}

fn collect_property_offsets(
    statements: &[Statement],
    scope: Scope,
    registry: &Registry,
    property_offsets: &mut HashSet<usize>,
    value_offsets: &mut HashSet<usize>,
) {
    for statement in statements {
        match &statement.kind {
            StatementKind::Property {
                name,
                value,
                additional_values,
            } => {
                let Some(spec) = registry.property_spec(scope, &name.text) else {
                    continue;
                };
                property_offsets.insert(name.span.start.offset);
                match spec.kind {
                    PropertyKind::Value(values) => {
                        if values.contains(&value.text.as_str()) {
                            value_offsets.insert(value.span.start.offset);
                        }
                        for argument in additional_values {
                            if values.contains(&argument.text.as_str()) {
                                value_offsets.insert(argument.span.start.offset);
                            }
                        }
                    }
                    PropertyKind::Highlight => {
                        for argument in std::iter::once(value).chain(additional_values) {
                            if registry.is_registered_property_value(&name.text, &argument.text) {
                                value_offsets.insert(argument.span.start.offset);
                            }
                        }
                    }
                    _ => {}
                }
            }
            StatementKind::Invocation {
                name,
                arguments,
                body,
            } => {
                if name.text == "highlight" {
                    for argument in arguments.iter().skip(2) {
                        if matches!(
                            argument.text.as_str(),
                            "trace" | "pulse" | "left" | "right" | "top" | "bottom"
                        ) {
                            value_offsets.insert(argument.span.start.offset);
                        }
                    }
                }
                if let (Some(body), Some(spec)) = (body, registry.spec(&name.text)) {
                    let child_scope = match spec.body {
                        BodySpec::Properties(scope) | BodySpec::Scope(scope) => scope,
                        BodySpec::None => continue,
                    };
                    collect_property_offsets(
                        body,
                        child_scope,
                        registry,
                        property_offsets,
                        value_offsets,
                    );
                }
            }
            StatementKind::Edge {
                properties: Some(properties),
                ..
            } => collect_property_offsets(
                properties,
                Scope::Edge,
                registry,
                property_offsets,
                value_offsets,
            ),
            StatementKind::Error | StatementKind::Edge { .. } => {}
        }
    }
}
