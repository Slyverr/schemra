//! Typed, renderer-neutral models for Schemra visuals.

mod ids;
mod module;
mod visual;

pub use ids::{GroupId, NodeId, VisualId};
pub use module::ModuleIr;
pub use schemra_syntax::{SourceLocation, SourceSpan};
pub use visual::{Direction, EdgeDirectionality, EdgeIr, GroupIr, GroupLayout, NodeIr, VisualIr};
