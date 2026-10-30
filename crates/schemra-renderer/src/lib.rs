//! SVG rendering for Schemra scenes.

mod renderer;

pub use renderer::{SvgRenderError, node_text_width, render_svg, wrap_text_lines};
