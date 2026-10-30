//! Typed, renderer-neutral models for Schemra visuals.

mod ids;
mod module;
mod scenario;
mod timeline;
mod visual;

pub use ids::{GroupId, NodeId, ScenarioId, TokenId, VisualId};
pub use module::ModuleIr;
pub use scenario::{
    ActionIr, HighlightEffect, HighlightOrigin, HighlightSide, ScenarioIr, TokenIr,
};
pub use schemra_syntax::{SourceLocation, SourceSpan};
pub use timeline::{
    HighlightFrame, SceneEdgeProgress, SceneHighlight, SceneTimelineFrame, SceneTokenPosition,
    Timeline, TimelineError, TimelineEvent, TimelineEventKind, TimelineFrame, TokenFrame,
    TokenPosition,
};
pub use visual::{Direction, EdgeDirectionality, EdgeIr, GroupIr, GroupLayout, NodeIr, VisualIr};
