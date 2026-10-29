use crate::{ScenarioId, SourceSpan, TokenId, VisualId};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HighlightEffect {
    #[default]
    Trace,
    Pulse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighlightSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HighlightOrigin {
    Default,
    Arrival {
        from: crate::NodeId,
        to: crate::NodeId,
    },
    Side {
        side: HighlightSide,
    },
}

/// A named set of reusable marker styles associated with a visual.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioIr {
    pub id: ScenarioId,
    pub id_span: SourceSpan,
    pub visual: VisualId,
    pub visual_span: SourceSpan,
    pub tokens: Vec<TokenIr>,
    pub actions: Vec<ActionIr>,
    pub span: SourceSpan,
}

/// A reusable visual marker style declared inside a scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenIr {
    pub id: TokenId,
    pub id_span: SourceSpan,
    pub icon: Option<String>,
    pub span: SourceSpan,
}

/// A typed, source-located operation within a scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionIr {
    Sequence {
        actions: Vec<ActionIr>,
        span: SourceSpan,
    },
    Move {
        token: TokenId,
        token_span: SourceSpan,
        from: crate::NodeId,
        from_span: SourceSpan,
        to: crate::NodeId,
        to_span: SourceSpan,
        duration_ms: Option<u64>,
        span: SourceSpan,
    },
    Wait {
        duration_ms: u64,
        span: SourceSpan,
    },
    Highlight {
        node: crate::NodeId,
        node_span: SourceSpan,
        duration_ms: Option<u64>,
        effect: HighlightEffect,
        origin: HighlightOrigin,
        span: SourceSpan,
    },
    Parallel {
        actions: Vec<ActionIr>,
        span: SourceSpan,
    },
    Repeat {
        times: u32,
        body: Vec<ActionIr>,
        span: SourceSpan,
    },
}
