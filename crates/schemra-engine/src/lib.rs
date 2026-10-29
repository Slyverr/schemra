//! Deterministic execution and sampling for compiled Schemra scenarios.

mod input;
mod layout;
mod renderer;
mod routing;
mod timeline;

pub use input::{VisualLimitError, VisualLimits, check_visual_limits};
pub use layout::layout_visual;
pub use renderer::{SvgRenderError, node_text_width, render_svg, wrap_text_lines};
pub use timeline::{TimelineSampling, compile_timeline};
