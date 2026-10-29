use serde::{Deserialize, Serialize};

use crate::{HighlightEffect, HighlightOrigin, NodeId, SourceSpan, TokenId};

/// A finite, deterministic schedule lowered from a scenario's action tree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub duration_ms: u64,
    pub events: Vec<TimelineEvent>,
}

/// Deterministic marker movement instances and highlights at a timestamp.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimelineFrame {
    pub time_ms: u64,
    pub tokens: Vec<TokenFrame>,
    pub highlighted_nodes: Vec<NodeId>,
}

/// Scene coordinates and active links sampled from a timeline at one timestamp.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SceneTimelineFrame {
    pub time_ms: u64,
    pub tokens: Vec<SceneTokenPosition>,
    pub active_edges: Vec<SceneEdgeProgress>,
    pub highlights: Vec<SceneHighlight>,
}

/// Position of one visible movement marker instance in scene coordinates.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SceneTokenPosition {
    /// Stable identity of this movement event, distinct from the reusable token style.
    pub instance_id: usize,
    pub token: String,
    pub x: f32,
    pub y: f32,
    pub traveling: bool,
}

/// Progress along a scene edge for the active movement.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SceneEdgeProgress {
    /// Stable identity of this movement event, distinct from the reusable token style.
    pub instance_id: usize,
    pub token: String,
    pub edge_index: usize,
    pub progress_micros: u32,
    pub traversal_direction: i8,
}

/// The rendered pulse strength for an active node highlight.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SceneHighlight {
    pub instance_id: usize,
    pub node_id: String,
    pub effect: HighlightEffect,
    pub progress_micros: u32,
    pub pulse_opacity: f32,
    pub origin: Option<[f32; 2]>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenFrame {
    /// Stable identity of this movement event, distinct from the reusable token style.
    pub instance_id: usize,
    pub token: TokenId,
    pub position: TokenPosition,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TokenPosition {
    Moving {
        from: NodeId,
        to: NodeId,
        /// Progress in millionths, avoiding floating-point drift across targets.
        progress_micros: u32,
    },
}

/// One time-bounded action in a compiled timeline.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub start_ms: u64,
    pub end_ms: u64,
    pub kind: TimelineEventKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TimelineEventKind {
    Move {
        token: TokenId,
        from: NodeId,
        to: NodeId,
    },
    Highlight {
        node: NodeId,
        effect: HighlightEffect,
        origin: HighlightOrigin,
    },
    Wait,
}

/// A source-located scheduling failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineError {
    pub code: &'static str,
    pub message: &'static str,
    pub span: SourceSpan,
}
