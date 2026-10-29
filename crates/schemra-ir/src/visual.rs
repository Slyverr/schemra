use crate::{GroupId, NodeId, SourceSpan, VisualId};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Direction {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GroupLayout {
    #[default]
    Auto,
    Grid {
        columns: Option<u32>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisualIr {
    pub id: VisualId,
    pub id_span: SourceSpan,
    pub title: String,
    pub direction: Direction,
    pub direction_explicit: bool,
    pub groups: Vec<GroupIr>,
    pub nodes: Vec<NodeIr>,
    pub edges: Vec<EdgeIr>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupIr {
    pub id: GroupId,
    pub id_span: SourceSpan,
    pub title: String,
    pub layout: GroupLayout,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIr {
    pub id: NodeId,
    pub id_span: SourceSpan,
    pub label: String,
    pub icon: Option<String>,
    pub detail: Option<String>,
    pub group: Option<GroupId>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgeIr {
    pub from: NodeId,
    pub from_span: SourceSpan,
    pub to: NodeId,
    pub to_span: SourceSpan,
    pub directionality: EdgeDirectionality,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeDirectionality {
    Directed,
    Undirected,
}
