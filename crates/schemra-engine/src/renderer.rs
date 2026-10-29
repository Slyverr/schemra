//! Small deterministic SVG output for static scenes.

use std::{error::Error, fmt, fmt::Write as _};

use schemra_scene::{Rect, Scene, SceneFont, SceneNode, SceneNodeShape, ScenePalette};

/// Errors returned when scene geometry cannot be represented as SVG.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SvgRenderError {
    InvalidCanvasBounds,
    InvalidNodeBounds,
    InvalidNodeStyle,
}

impl fmt::Display for SvgRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCanvasBounds => {
                formatter.write_str("scene canvas bounds must be finite and positive")
            }
            Self::InvalidNodeBounds => {
                formatter.write_str("node bounds must be finite and positive")
            }
            Self::InvalidNodeStyle => formatter.write_str(
                "node opacity and radius must be finite, with opacity between zero and one",
            ),
        }
    }
}

impl Error for SvgRenderError {}

/// Serializes the canvas and static node layer of a scene to an SVG document.
///
/// Scene text and attributes are XML-escaped. Edges and other scene layers are
/// added by their corresponding renderer slices.
pub fn render_svg(scene: &Scene) -> Result<String, SvgRenderError> {
    if !scene.width.is_finite()
        || !scene.height.is_finite()
        || scene.width <= 0.0
        || scene.height <= 0.0
    {
        return Err(SvgRenderError::InvalidCanvasBounds);
    }
    if scene.nodes.iter().any(|node| !valid_bounds(node.bounds)) {
        return Err(SvgRenderError::InvalidNodeBounds);
    }
    if scene.nodes.iter().any(|node| {
        node.style
            .opacity
            .is_some_and(|opacity| !opacity.is_finite() || !(0.0..=1.0).contains(&opacity))
            || node.style.radius.is_some_and(|radius| !radius.is_finite())
    }) {
        return Err(SvgRenderError::InvalidNodeStyle);
    }

    let (default_canvas, default_card, default_border, default_text, default_muted) =
        palette(scene.theme.palette);
    let canvas = safe_color(scene.theme.colors.canvas.as_deref(), default_canvas);
    let card = safe_color(scene.theme.colors.card.as_deref(), default_card);
    let border = safe_color(scene.theme.colors.border.as_deref(), default_border);
    let text = safe_color(scene.theme.colors.text.as_deref(), default_text);
    let muted = safe_color(scene.theme.colors.muted.as_deref(), default_muted);
    let mut svg = String::with_capacity(1024 + scene.nodes.len() * 256);
    write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-labelledby=\"schemra-title schemra-description\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        number(scene.width),
        number(scene.height),
        number(scene.width),
        number(scene.height),
    )
    .expect("writing to a String cannot fail");
    svg.push_str("<title id=\"schemra-title\">");
    write_xml(&mut svg, &scene.title);
    svg.push_str("</title><desc id=\"schemra-description\">Schemra architecture visual</desc>");
    write!(
        svg,
        "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        number(scene.width),
        number(scene.height),
        canvas,
    )
    .expect("writing to a String cannot fail");

    for node in &scene.nodes {
        write_node(&mut svg, node, card, border, text, muted, scene.theme.font);
    }
    svg.push_str("</svg>");
    Ok(svg)
}

fn write_node(
    svg: &mut String,
    node: &SceneNode,
    card: &str,
    border: &str,
    text: &str,
    muted: &str,
    theme_font: SceneFont,
) {
    let bounds = node.bounds;
    write!(
        svg,
        "<g data-node-id=\"{}\" data-node-label=\"{}\" data-node-shape=\"{}\"",
        escape_attribute(&node.id),
        escape_attribute(&node.label),
        match node.shape {
            SceneNodeShape::Round => "round",
            SceneNodeShape::Rectangle => "rectangle",
            SceneNodeShape::Cylinder => "cylinder",
        },
    )
    .expect("writing to a String cannot fail");
    if let Some(opacity) = node.style.opacity {
        write!(svg, " opacity=\"{}\"", number(opacity)).expect("writing to a String cannot fail");
    }
    svg.push('>');
    write_node_shape(
        svg,
        bounds,
        node,
        safe_color(node.style.fill.as_deref(), card),
        safe_color(node.style.stroke.as_deref(), border),
    );

    let text_x = bounds.x + if node.icon.is_some() { 56.0 } else { 16.0 };
    let text_anchor = match node.text_align {
        schemra_scene::SceneTextAlign::Left => "start",
        schemra_scene::SceneTextAlign::Center => "middle",
    };
    let text_x = if text_anchor == "middle" {
        bounds.x + bounds.width / 2.0
    } else {
        text_x
    };
    let (label_y, detail_y) = if node.detail.is_some() {
        (
            bounds.y + bounds.height / 2.0 - 3.0,
            bounds.y + bounds.height / 2.0 + 16.0,
        )
    } else {
        (bounds.y + bounds.height / 2.0 + 5.0, 0.0)
    };
    write!(
        svg,
        "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-family=\"{}\" font-size=\"16\" font-weight=\"{}\" fill=\"{}\">",
        number(text_x),
        number(label_y),
        text_anchor,
        font_family(node.style.font.unwrap_or(theme_font)),
        node.style.weight.unwrap_or(600),
        safe_color(node.style.text_color.as_deref(), text),
    )
    .expect("writing to a String cannot fail");
    write_xml(svg, &node.label);
    svg.push_str("</text>");
    if let Some(detail) = &node.detail {
        write!(
            svg,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-family=\"{}\" font-size=\"13\" font-weight=\"{}\" fill=\"{}\">",
            number(text_x),
            number(detail_y),
            text_anchor,
            font_family(node.style.font.unwrap_or(theme_font)),
            node.style.weight.unwrap_or(400),
            safe_color(node.style.text_color.as_deref(), muted),
        )
        .expect("writing to a String cannot fail");
        write_xml(svg, detail);
        svg.push_str("</text>");
    }
    svg.push_str("</g>");
}

fn valid_bounds(bounds: Rect) -> bool {
    bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.width > 0.0
        && bounds.height > 0.0
        && (bounds.x + bounds.width).is_finite()
        && (bounds.y + bounds.height).is_finite()
}

fn write_node_shape(svg: &mut String, bounds: Rect, node: &SceneNode, fill: &str, stroke: &str) {
    match node.shape {
        SceneNodeShape::Cylinder => {
            let arc = (bounds.height * 0.12).min(12.0).min(bounds.height / 2.0);
            write!(
                svg,
                "<path d=\"M {} {} C {} {} {} {} {} {} V {} C {} {} {} {} {} {} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                number(bounds.x),
                number(bounds.y + arc),
                number(bounds.x),
                number(bounds.y),
                number(bounds.x + bounds.width),
                number(bounds.y),
                number(bounds.x + bounds.width),
                number(bounds.y + arc),
                number(bounds.y + bounds.height - arc),
                number(bounds.x + bounds.width),
                number(bounds.y + bounds.height),
                number(bounds.x),
                number(bounds.y + bounds.height),
                number(bounds.x),
                number(bounds.y + bounds.height - arc),
                fill,
                stroke,
            )
            .expect("writing to a String cannot fail");
        }
        SceneNodeShape::Round | SceneNodeShape::Rectangle => {
            let radius = if node.shape == SceneNodeShape::Round {
                node.style
                    .radius
                    .unwrap_or(12.0)
                    .max(0.0)
                    .min(bounds.width.min(bounds.height) / 2.0)
            } else {
                0.0
            };
            write!(
                svg,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                number(bounds.x),
                number(bounds.y),
                number(bounds.width),
                number(bounds.height),
                number(radius),
                fill,
                stroke,
            )
            .expect("writing to a String cannot fail");
        }
    }
}

fn palette(
    palette: ScenePalette,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match palette {
        ScenePalette::Paper => ("#ffffff", "#ffffff", "#e4e4e7", "#18181b", "#52525b"),
        ScenePalette::Midnight => ("#0f172a", "#1e293b", "#475569", "#f8fafc", "#cbd5e1"),
        ScenePalette::Lagoon => ("#f0fdfa", "#ffffff", "#99f6e4", "#134e4a", "#0f766e"),
        ScenePalette::Sand => ("#fffcf5", "#ffffff", "#f5e6c8", "#422006", "#854d0e"),
    }
}

fn safe_color<'a>(value: Option<&'a str>, fallback: &'a str) -> &'a str {
    value
        .filter(|value| {
            if let Some(hex) = value.strip_prefix('#') {
                return matches!(hex.len(), 3 | 4 | 6 | 8)
                    && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
            }
            !value.is_empty()
                && value.len() <= 32
                && value.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
        .unwrap_or(fallback)
}

fn font_family(font: SceneFont) -> &'static str {
    match font {
        SceneFont::Sans => "sans-serif",
        SceneFont::Serif => "serif",
        SceneFont::Mono => "monospace",
    }
}

fn number(value: f32) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let mut text = format!("{rounded:.3}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" { "0".into() } else { text }
}

fn write_xml(svg: &mut String, text: &str) {
    for character in text.chars() {
        match character {
            '&' => svg.push_str("&amp;"),
            '<' => svg.push_str("&lt;"),
            '>' => svg.push_str("&gt;"),
            '"' => svg.push_str("&quot;"),
            '\'' => svg.push_str("&apos;"),
            _ if is_xml_character(character) => svg.push(character),
            _ => svg.push('\u{fffd}'),
        }
    }
}

fn is_xml_character(character: char) -> bool {
    matches!(character as u32, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
}

fn escape_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    write_xml(&mut escaped, value);
    escaped
}
