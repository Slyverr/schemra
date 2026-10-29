use serde::Serialize;

/// A position in source text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SourceLocation {
    /// UTF-8 byte offset from the start of the source.
    pub offset: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based Unicode-scalar column.
    pub column: usize,
}

/// A half-open range in source text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SourceSpan {
    /// First source position in the range.
    pub start: SourceLocation,
    /// First source position after the range.
    pub end: SourceLocation,
}

impl SourceSpan {
    /// Creates a zero-width span at `location`.
    pub const fn point(location: SourceLocation) -> Self {
        Self {
            start: location,
            end: location,
        }
    }
}
