//! Deterministic execution and sampling for compiled Schemra scenarios.

mod effects;
mod input;
mod layout;
mod routing;
mod scene_timeline;
mod timeline;

pub use input::{VisualLimitError, VisualLimits, check_visual_limits};
pub use layout::layout_visual;
pub use scene_timeline::{SceneGeometrySampler, SceneTimelineSampler};
pub use schemra_renderer::{SvgRenderError, node_text_width, render_svg, wrap_text_lines};
pub use timeline::{TimelineSampling, compile_timeline};
