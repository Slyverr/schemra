//! Shared source positions and diagnostics for Schemra language tooling.

mod diagnostic;
mod span;

pub use diagnostic::{Diagnostic, Severity};
pub use span::{SourceLocation, SourceSpan};
