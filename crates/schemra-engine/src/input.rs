//! Finite resource bounds for visual layout and rendering.

use schemra_ir::VisualIr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisualLimitError {
    pub code: &'static str,
    pub message: String,
    pub span: schemra_ir::SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VisualLimits {
    pub max_visual_elements: usize,
    pub max_visual_links: usize,
}

impl Default for VisualLimits {
    fn default() -> Self {
        Self {
            max_visual_elements: 10_000,
            max_visual_links: 1_000,
        }
    }
}

pub fn check_visual_limits(
    visual: &VisualIr,
    limits: VisualLimits,
) -> Result<(), VisualLimitError> {
    let elements = visual
        .groups
        .len()
        .saturating_add(visual.nodes.len())
        .saturating_add(visual.edges.len());
    let links = visual.edges.len();
    visual_limit_error(elements, links, visual.span, limits).map_or(Ok(()), Err)
}

fn visual_limit_error(
    elements: usize,
    links: usize,
    span: schemra_ir::SourceSpan,
    limits: VisualLimits,
) -> Option<VisualLimitError> {
    if elements > limits.max_visual_elements {
        return Some(VisualLimitError {
            code: "E_VISUAL_ELEMENT_LIMIT",
            message: format!(
                "This visual exceeds the configured limit of {} elements.",
                limits.max_visual_elements
            ),
            span,
        });
    }
    if links > limits.max_visual_links {
        return Some(VisualLimitError {
            code: "E_VISUAL_LINK_LIMIT",
            message: format!(
                "This visual exceeds the configured limit of {} links.",
                limits.max_visual_links
            ),
            span,
        });
    }
    None
}
