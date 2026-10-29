use serde::{Deserialize, Serialize};

/// Stable identifier for a declared visual.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisualId(pub String);

/// Stable identifier for a group inside a visual.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroupId(pub String);

/// Stable identifier for a node inside a visual.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeId(pub String);
