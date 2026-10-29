//! Shared source positions and diagnostics for Schemra language tooling.

mod diagnostic;
mod span;
mod syntax;

pub use diagnostic::{Diagnostic, Severity};
pub use span::{SourceLocation, SourceSpan};
pub use syntax::{Statement, StatementKind, SyntaxFile, Token, TokenKind, parse_syntax};
