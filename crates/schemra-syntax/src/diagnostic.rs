use serde::Serialize;

use crate::SourceSpan;

/// How a diagnostic affects its source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The source cannot be compiled as written.
    Error,
    /// The source is usable, but has a diagnosed issue.
    Warning,
    /// Advisory feedback that does not invalidate compilation.
    Information,
}

/// A source-linked issue or advisory returned by language tooling.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    /// Stable machine-readable identifier for this diagnostic kind.
    pub code: String,
    /// Whether this diagnostic blocks compilation.
    pub severity: Severity,
    /// Human-readable explanation.
    pub message: String,
    /// Source range associated with the diagnostic.
    pub span: SourceSpan,
    /// Optional suggested correction or next step.
    pub hint: Option<Box<str>>,
}

impl Diagnostic {
    /// Creates an error diagnostic, optionally with a suggested correction.
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        span: SourceSpan,
        hint: Option<impl Into<String>>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            span,
            hint: hint.map(|hint| hint.into().into_boxed_str()),
        }
    }

    /// Creates a non-blocking informational diagnostic.
    pub fn information(
        code: impl Into<String>,
        message: impl Into<String>,
        span: SourceSpan,
        hint: Option<impl Into<String>>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Information,
            message: message.into(),
            span,
            hint: hint.map(|hint| hint.into().into_boxed_str()),
        }
    }
}
