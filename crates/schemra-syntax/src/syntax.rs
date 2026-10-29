//! Generic, source-preserving syntax for the v0 construct language.
//!
//! This layer recognizes only invocations, properties, and edges. It does not
//! assign meaning to identifiers such as `visual`, `node`, or `duration`.

use crate::{Diagnostic, SourceLocation, SourceSpan};

const MAX_BLOCK_NESTING: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Identifier,
    String(String),
    Integer(String),
    Duration(String),
    BareValue(String),
    Arrow,
    Undirected,
    Colon,
    LeftBrace,
    RightBrace,
    Newline,
    Semicolon,
    Comment(String),
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxFile {
    /// Original lexical stream, including separators and comments.
    pub tokens: Vec<Token>,
    pub statements: Vec<Statement>,
    pub comments: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementKind {
    Invocation {
        name: Token,
        arguments: Vec<Token>,
        body: Option<Vec<Statement>>,
    },
    Property {
        name: Token,
        value: Token,
        additional_values: Vec<Token>,
    },
    Edge {
        from: Token,
        operator: Token,
        to: Token,
        properties: Option<Vec<Statement>>,
    },
    Error,
}

/// Parse generic v0 syntax while retaining comments, source text, and spans.
///
/// Malformed statements are represented as `Error` nodes and diagnosed; the
/// parser synchronizes at a newline, semicolon, or closing brace.
pub fn parse_syntax(source: &str) -> SyntaxFile {
    let (tokens, mut diagnostics) = Lexer::new(source).lex();
    let comments = tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Comment(_)))
        .cloned()
        .collect();
    let parser_tokens = tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Comment(_)))
        .cloned()
        .collect();
    let (statements, parse_diagnostics) = Parser::new(parser_tokens).parse_file();
    diagnostics.extend(parse_diagnostics);
    SyntaxFile {
        tokens,
        statements,
        comments,
        diagnostics,
    }
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn lex(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        let mut diagnostics = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                let start = self.location();
                self.advance();
                tokens.push(self.token(TokenKind::Newline, start));
            } else if ch.is_ascii_whitespace() {
                self.advance();
            } else if ch == '/' && self.peek_after(1) == Some('/') {
                let start = self.location();
                self.advance();
                self.advance();
                let content_start = self.offset;
                while self.peek().is_some_and(|value| value != '\n') {
                    self.advance();
                }
                let content = self.source[content_start..self.offset].to_owned();
                tokens.push(self.token(TokenKind::Comment(content), start));
            } else {
                let start = self.location();
                let token = match ch {
                    '{' => self.single(TokenKind::LeftBrace, start),
                    '}' => self.single(TokenKind::RightBrace, start),
                    ':' => self.single(TokenKind::Colon, start),
                    ';' => self.single(TokenKind::Semicolon, start),
                    '-' if self.peek_after(1) == Some('>') => {
                        self.advance();
                        self.advance();
                        self.token(TokenKind::Arrow, start)
                    }
                    '-' if self.peek_after(1) == Some('-') => {
                        self.advance();
                        self.advance();
                        self.token(TokenKind::Undirected, start)
                    }
                    '"' => match self.string(start) {
                        Ok(token) => token,
                        Err(message) => {
                            let token = self.token(TokenKind::Error(message.clone()), start);
                            diagnostics.push(Diagnostic::error(
                                "E_SYNTAX_STRING",
                                message,
                                token.span,
                                None::<String>,
                            ));
                            tokens.push(token);
                            continue;
                        }
                    },
                    ch if ch.is_ascii_digit() => self.number_or_duration(start),
                    ch if is_identifier_start(ch) => self.word(start),
                    _ => {
                        self.advance();
                        let token = self.token(
                            TokenKind::Error(format!("unexpected character {ch:?}")),
                            start,
                        );
                        diagnostics.push(Diagnostic::error(
                            "E_SYNTAX_CHARACTER",
                            format!("Unexpected character {ch:?}."),
                            token.span,
                            None::<String>,
                        ));
                        token
                    }
                };
                tokens.push(token);
            }
        }
        (tokens, diagnostics)
    }

    fn string(&mut self, start: SourceLocation) -> Result<Token, String> {
        self.advance();
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.advance();
                    return Ok(self.token(TokenKind::String(value), start));
                }
                '\n' => break,
                '\\' => {
                    self.advance();
                    match self.peek() {
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('n') => value.push('\n'),
                        Some('r') => value.push('\r'),
                        Some('t') => value.push('\t'),
                        Some(other) => {
                            self.advance();
                            return Err(format!("Unsupported string escape \\{other}."));
                        }
                        None => break,
                    }
                    self.advance();
                }
                other => {
                    value.push(other);
                    self.advance();
                }
            }
        }
        Err("Unterminated string literal.".into())
    }

    fn number_or_duration(&mut self, start: SourceLocation) -> Token {
        let begin = self.offset;
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance();
        }
        let digits_end = self.offset;
        while self.peek().is_some_and(|ch| ch.is_ascii_alphabetic()) {
            self.advance();
        }
        let raw = self.source[begin..self.offset].to_owned();
        let kind = if digits_end != self.offset {
            TokenKind::Duration(raw.clone())
        } else {
            TokenKind::Integer(raw.clone())
        };
        self.token_with_text(kind, raw, start)
    }

    fn word(&mut self, start: SourceLocation) -> Token {
        let begin = self.offset;
        while self.peek().is_some_and(is_identifier_continue) {
            self.advance();
        }
        // Hyphens are permitted in bare enum values, but not in identifiers.
        while self.peek() == Some('-') && self.peek_after(1).is_some_and(is_identifier_start) {
            self.advance();
            while self.peek().is_some_and(is_identifier_continue) {
                self.advance();
            }
        }
        let raw = self.source[begin..self.offset].to_owned();
        let kind = if raw.contains('-') {
            TokenKind::BareValue(raw.clone())
        } else {
            TokenKind::Identifier
        };
        self.token_with_text(kind, raw, start)
    }

    fn single(&mut self, kind: TokenKind, start: SourceLocation) -> Token {
        self.advance();
        self.token(kind, start)
    }

    fn token(&self, kind: TokenKind, start: SourceLocation) -> Token {
        self.token_with_text(
            kind,
            self.source[start.offset..self.offset].to_owned(),
            start,
        )
    }

    fn token_with_text(&self, kind: TokenKind, text: String, start: SourceLocation) -> Token {
        Token {
            kind,
            text,
            span: SourceSpan {
                start,
                end: self.location(),
            },
        }
    }

    fn location(&self) -> SourceLocation {
        SourceLocation {
            offset: self.offset,
            line: self.line,
            column: self.column,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn peek_after(&self, chars: usize) -> Option<char> {
        self.source[self.offset..].chars().nth(chars)
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            index: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_file(mut self) -> (Vec<Statement>, Vec<Diagnostic>) {
        let statements = self.parse_scope(0);
        (statements, self.diagnostics)
    }

    fn parse_scope(&mut self, depth: usize) -> Vec<Statement> {
        let mut statements = Vec::new();
        let mut closed = false;
        loop {
            self.skip_separators();
            let Some(token) = self.peek() else { break };
            if matches!(token.kind, TokenKind::RightBrace) {
                if depth > 0 {
                    self.bump();
                    closed = true;
                } else {
                    self.report("E_SYNTAX_BRACE", "Unexpected closing brace.", token.span);
                    self.bump();
                }
                break;
            }
            let before = self.index;
            statements.push(self.parse_statement(depth));
            if let Some(token) = self.peek()
                && !matches!(
                    token.kind,
                    TokenKind::Newline | TokenKind::Semicolon | TokenKind::RightBrace
                )
            {
                self.report(
                    "E_SYNTAX_SEPARATOR",
                    "Separate statements with a newline or semicolon.",
                    token.span,
                );
                self.recover_line();
            }
            if self.index == before {
                self.bump();
            }
        }
        if depth > 0 && !closed {
            let end = self.tokens.last().map_or_else(
                || {
                    SourceSpan::point(SourceLocation {
                        offset: 0,
                        line: 1,
                        column: 1,
                    })
                },
                |token| SourceSpan::point(token.span.end),
            );
            self.report("E_SYNTAX_BRACE", "Unclosed block.", end);
        }
        statements
    }

    fn parse_statement(&mut self, depth: usize) -> Statement {
        let Some(first) = self.bump() else {
            return Statement {
                kind: StatementKind::Error,
                span: SourceSpan::point(SourceLocation {
                    offset: 0,
                    line: 1,
                    column: 1,
                }),
            };
        };
        if !matches!(first.kind, TokenKind::Identifier) {
            self.report(
                "E_SYNTAX_STATEMENT",
                "Expected a statement name or edge source.",
                first.span,
            );
            self.recover_line();
            return Statement {
                kind: StatementKind::Error,
                span: first.span,
            };
        }
        if self
            .consume_if(|kind| matches!(kind, TokenKind::Colon))
            .is_some()
        {
            let Some(value) = self.bump().filter(|token| is_literal(&token.kind)) else {
                self.report(
                    "E_SYNTAX_PROPERTY",
                    "Expected a property value after `:`.",
                    first.span,
                );
                self.recover_line();
                return Statement {
                    kind: StatementKind::Error,
                    span: first.span,
                };
            };
            let mut additional_values = Vec::new();
            while self.peek().is_some_and(|token| is_literal(&token.kind)) {
                if let Some(value) = self.bump() {
                    additional_values.push(value);
                }
            }
            let end = additional_values
                .last()
                .map_or(value.span, |token| token.span);
            let span = join_span(first.span, end);
            return Statement {
                kind: StatementKind::Property {
                    name: first,
                    value,
                    additional_values,
                },
                span,
            };
        }
        if let Some(operator) =
            self.consume_if(|kind| matches!(kind, TokenKind::Arrow | TokenKind::Undirected))
        {
            let Some(to) = self
                .bump()
                .filter(|token| matches!(token.kind, TokenKind::Identifier))
            else {
                self.report(
                    "E_SYNTAX_EDGE",
                    "Expected an identifier after the edge operator.",
                    operator.span,
                );
                self.recover_line();
                return Statement {
                    kind: StatementKind::Error,
                    span: join_span(first.span, operator.span),
                };
            };
            let properties = self.parse_optional_body(depth);
            let end = self.previous().map_or(to.span, |token| token.span);
            let span = join_span(first.span, end);
            return Statement {
                kind: StatementKind::Edge {
                    from: first,
                    operator,
                    to,
                    properties,
                },
                span,
            };
        }
        let mut arguments = Vec::new();
        while self
            .peek()
            .is_some_and(|token| is_header_argument(&token.kind))
        {
            if let Some(argument) = self.bump() {
                arguments.push(argument);
            }
        }
        let body = self.parse_optional_body(depth);
        let end = self.previous().map_or(first.span, |token| token.span);
        let span = join_span(first.span, end);
        Statement {
            kind: StatementKind::Invocation {
                name: first,
                arguments,
                body,
            },
            span,
        }
    }

    fn parse_optional_body(&mut self, depth: usize) -> Option<Vec<Statement>> {
        let opening = self.consume_if(|kind| matches!(kind, TokenKind::LeftBrace))?;
        if depth >= MAX_BLOCK_NESTING {
            let span = self.skip_balanced_body(opening.span);
            self.report(
                "E_SYNTAX_DEPTH",
                "Block nesting exceeds the supported syntax depth.",
                opening.span,
            );
            return Some(vec![Statement {
                kind: StatementKind::Error,
                span,
            }]);
        }
        Some(self.parse_scope(depth + 1))
    }

    fn skip_balanced_body(&mut self, opening_span: SourceSpan) -> SourceSpan {
        let mut depth = 1usize;
        let mut end = opening_span;
        while let Some(token) = self.bump() {
            end = token.span;
            match token.kind {
                TokenKind::LeftBrace => depth = depth.saturating_add(1),
                TokenKind::RightBrace => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        join_span(opening_span, end)
    }

    fn skip_separators(&mut self) {
        while self
            .consume_if(|kind| matches!(kind, TokenKind::Newline | TokenKind::Semicolon))
            .is_some()
        {}
    }

    fn recover_line(&mut self) {
        while self.peek().is_some_and(|token| {
            !matches!(
                token.kind,
                TokenKind::Newline | TokenKind::Semicolon | TokenKind::RightBrace
            )
        }) {
            self.bump();
        }
    }

    fn report(&mut self, code: &str, message: &str, span: SourceSpan) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span, None::<String>));
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }
    fn previous(&self) -> Option<&Token> {
        self.index.checked_sub(1).and_then(|i| self.tokens.get(i))
    }
    fn bump(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index)?.clone();
        self.index += 1;
        Some(token)
    }

    fn consume_if(&mut self, predicate: impl FnOnce(&TokenKind) -> bool) -> Option<Token> {
        if predicate(&self.peek()?.kind) {
            self.bump()
        } else {
            None
        }
    }
}

fn is_header_argument(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::String(_)
            | TokenKind::Integer(_)
            | TokenKind::Duration(_)
            | TokenKind::BareValue(_)
            | TokenKind::Arrow
            | TokenKind::Undirected
    )
}

fn is_literal(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::String(_)
            | TokenKind::Integer(_)
            | TokenKind::Duration(_)
            | TokenKind::BareValue(_)
    )
}

fn join_span(start: SourceSpan, end: SourceSpan) -> SourceSpan {
    SourceSpan {
        start: start.start,
        end: end.end,
    }
}
