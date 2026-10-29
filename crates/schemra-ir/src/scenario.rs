use crate::{ScenarioId, SourceSpan, TokenId, VisualId};

/// A named set of reusable marker styles associated with a visual.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioIr {
    pub id: ScenarioId,
    pub id_span: SourceSpan,
    pub visual: VisualId,
    pub visual_span: SourceSpan,
    pub tokens: Vec<TokenIr>,
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
