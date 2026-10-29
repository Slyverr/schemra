//! Context-aware completion suggestions derived from registered language schemas.

use schemra_syntax::{Token, TokenKind, parse_syntax};
use serde::Serialize;

use crate::{BodySpec, HeaderKind, PropertyKind, Registry, Scope};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompletionKind {
    Keyword,
    Value,
    Identifier,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompletionItem {
    pub label: String,
    pub insert_text: String,
    pub detail: String,
    pub kind: CompletionKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompletionList {
    pub start: usize,
    pub end: usize,
    pub items: Vec<CompletionItem>,
}

/// Suggests registered language constructs and values at a UTF-8 byte offset.
pub fn complete_at(source: &str, cursor_offset: usize, explicit: bool) -> Option<CompletionList> {
    complete_at_with_nodes(source, cursor_offset, explicit, &[])
}

/// Suggests completions and includes project-provided node IDs where relevant.
pub fn complete_at_with_nodes(
    source: &str,
    cursor_offset: usize,
    explicit: bool,
    node_ids: &[String],
) -> Option<CompletionList> {
    let cursor = clamp_boundary(source, cursor_offset);
    let before_cursor = source.get(..cursor)?;
    let prefix_start = word_start(before_cursor);
    let prefix = before_cursor.get(prefix_start..)?;
    let syntax = parse_syntax(before_cursor);
    let registry = Registry::builtin_shared().ok()?;
    let scope = active_scope(&syntax.tokens, registry, cursor);
    if prefix.is_empty() && !explicit && !matches!(scope, Scope::Scenario | Scope::Action) {
        return None;
    }

    let line_start = before_cursor
        .rfind(['\n', ';'])
        .map_or(0, |position| position + 1);
    let line = before_cursor.get(line_start..prefix_start)?;
    let line_tokens: Vec<&Token> = syntax
        .tokens
        .iter()
        .filter(|token| {
            token.span.start.offset >= line_start && token.span.end.offset <= prefix_start
        })
        .filter(|token| !matches!(token.kind, TokenKind::Newline | TokenKind::Comment(_)))
        .collect();

    let candidates = if let Some((property, _)) = line.split_once(':') {
        let property = property.trim();
        if property == "highlight" {
            let value_text = line
                .split_once(':')
                .map_or("", |(_, values)| values.trim_end());
            let last_value = value_text.split_whitespace().next_back();
            if last_value == Some("from") {
                highlight_side_values()
            } else if matches!(last_value, Some("trace" | "pulse")) {
                vec![item(
                    "from",
                    "Choose where the trace starts",
                    CompletionKind::Keyword,
                )]
            } else {
                registry
                    .property_spec(scope, property)
                    .map(|spec| property_values(spec.kind))
                    .unwrap_or_default()
            }
        } else {
            registry
                .property_spec(scope, property)
                .map(|spec| property_values(spec.kind))
                .unwrap_or_default()
        }
    } else if let Some(candidates) = header_values(&line_tokens, node_ids, &syntax.tokens, registry)
    {
        candidates
    } else if let Some(candidates) =
        edge_values(&line_tokens, scope, node_ids, registry, &syntax.tokens)
    {
        candidates
    } else if !line_tokens.is_empty() {
        Vec::new()
    } else {
        let mut candidates = registry
            .constructs_in_scope(scope)
            .map(|spec| CompletionItem {
                label: spec.name.to_owned(),
                insert_text: spec.name.to_owned(),
                detail: format!("{} construct", spec.name),
                kind: CompletionKind::Keyword,
            })
            .collect::<Vec<_>>();
        candidates.extend(
            registry
                .properties_for_scope(scope)
                .map(|property| CompletionItem {
                    label: property.name.to_owned(),
                    insert_text: format!("{}: ", property.name),
                    detail: "Property".to_owned(),
                    kind: CompletionKind::Keyword,
                }),
        );
        candidates
    };

    let prefix = prefix.to_ascii_lowercase();
    let items: Vec<_> = candidates
        .into_iter()
        .filter(|item| item.label.to_ascii_lowercase().starts_with(&prefix))
        .collect();
    (!items.is_empty()).then_some(CompletionList {
        start: prefix_start,
        end: cursor,
        items,
    })
}

fn clamp_boundary(source: &str, offset: usize) -> usize {
    let mut offset = offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn word_start(source: &str) -> usize {
    source
        .char_indices()
        .rev()
        .find(|(_, character)| !is_word_character(*character))
        .map_or(0, |(offset, character)| offset + character.len_utf8())
}

fn is_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
}

fn active_scope(tokens: &[Token], registry: &Registry, cursor: usize) -> Scope {
    let mut scopes = vec![Scope::File];
    let mut statement: Vec<&Token> = Vec::new();
    for token in tokens
        .iter()
        .filter(|token| token.span.end.offset <= cursor)
    {
        match token.kind {
            TokenKind::Newline | TokenKind::Semicolon => statement.clear(),
            TokenKind::LeftBrace => {
                let scope = if scopes.last() == Some(&Scope::Visual)
                    && statement
                        .iter()
                        .any(|token| matches!(token.kind, TokenKind::Arrow | TokenKind::Undirected))
                {
                    Some(Scope::Edge)
                } else {
                    statement
                        .first()
                        .and_then(|name| registry.spec(&name.text))
                        .and_then(|spec| match spec.body {
                            BodySpec::Properties(scope) | BodySpec::Scope(scope) => Some(scope),
                            BodySpec::None => None,
                        })
                };
                if let Some(scope) = scope {
                    scopes.push(scope);
                }
                statement.clear();
            }
            TokenKind::RightBrace => {
                if scopes.len() > 1 {
                    scopes.pop();
                }
                statement.clear();
            }
            TokenKind::Comment(_) => {}
            _ => statement.push(token),
        }
    }
    scopes.last().copied().unwrap_or(Scope::File)
}

fn property_values(kind: PropertyKind) -> Vec<CompletionItem> {
    match kind {
        PropertyKind::Value(values) => values
            .iter()
            .map(|value| item(value, "Accepted value", CompletionKind::Value))
            .collect(),
        PropertyKind::String
        | PropertyKind::Integer
        | PropertyKind::PositiveInteger
        | PropertyKind::Duration
        | PropertyKind::PositiveDuration => Vec::new(),
        PropertyKind::Highlight => vec![
            item("off", "Disable highlighting", CompletionKind::Value),
            item("trace", "Trace the node border", CompletionKind::Value),
            item("pulse", "Pulse the node border", CompletionKind::Value),
        ],
    }
}

fn header_values(
    line: &[&Token],
    node_ids: &[String],
    tokens: &[Token],
    registry: &Registry,
) -> Option<Vec<CompletionItem>> {
    let construct = line.first()?;
    let spec = registry.spec(&construct.text)?;
    if construct.text == "highlight" && line.last().is_some_and(|token| token.text == "from") {
        return Some(highlight_side_values());
    }
    let argument_index = line.len() - 1;
    let parameter = spec.header.get(argument_index)?;
    Some(match parameter.kind {
        HeaderKind::Literal(value) => {
            vec![item(value, "Required keyword", CompletionKind::Keyword)]
        }
        HeaderKind::Arrow => vec![item("->", "Directed connection", CompletionKind::Keyword)],
        HeaderKind::TokenIdentifier => declared_tokens(tokens),
        HeaderKind::NodeIdentifier => node_identifiers(node_ids),
        HeaderKind::Identifier => Vec::new(),
        HeaderKind::String
        | HeaderKind::Integer
        | HeaderKind::PositiveInteger
        | HeaderKind::Duration => Vec::new(),
        HeaderKind::HighlightEffect => vec![
            item("trace", "Trace the node border", CompletionKind::Value),
            item("pulse", "Pulse the node border", CompletionKind::Value),
        ],
        HeaderKind::HighlightSide => highlight_side_values(),
    })
}

fn highlight_side_values() -> Vec<CompletionItem> {
    ["left", "right", "top", "bottom"]
        .into_iter()
        .map(|side| {
            item(
                side,
                "Start the trace on this node side",
                CompletionKind::Value,
            )
        })
        .collect()
}

fn declared_tokens(tokens: &[Token]) -> Vec<CompletionItem> {
    tokens
        .windows(2)
        .filter(|pair| pair[0].text == "token" && matches!(pair[1].kind, TokenKind::Identifier))
        .map(|pair| {
            item(
                &pair[1].text,
                "Token in this scenario",
                CompletionKind::Identifier,
            )
        })
        .collect()
}

fn node_identifiers(node_ids: &[String]) -> Vec<CompletionItem> {
    node_ids
        .iter()
        .map(|id| item(id, "Node in this visual", CompletionKind::Identifier))
        .collect()
}

fn edge_values(
    line: &[&Token],
    scope: Scope,
    node_ids: &[String],
    registry: &Registry,
    tokens: &[Token],
) -> Option<Vec<CompletionItem>> {
    if !matches!(scope, Scope::Visual | Scope::Group) || line.is_empty() {
        return None;
    }
    let is_construct = line
        .first()
        .is_some_and(|token| registry.spec(&token.text).is_some());
    if is_construct {
        return None;
    }
    let has_operator = line
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Arrow | TokenKind::Undirected));
    if has_operator {
        Some(node_identifiers(node_ids))
    } else if line.len() == 1 && is_declared_node(&line[0].text, tokens, node_ids) {
        Some(vec![
            item("->", "Directed connection", CompletionKind::Keyword),
            item("--", "Undirected connection", CompletionKind::Keyword),
        ])
    } else {
        None
    }
}

fn is_declared_node(id: &str, tokens: &[Token], external_ids: &[String]) -> bool {
    external_ids.iter().any(|candidate| candidate == id)
        || tokens.windows(2).any(|pair| {
            pair[0].text == "node"
                && matches!(pair[1].kind, TokenKind::Identifier)
                && pair[1].text == id
        })
}

fn item(label: &str, detail: &str, kind: CompletionKind) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        insert_text: label.to_owned(),
        detail: detail.to_owned(),
        kind,
    }
}
