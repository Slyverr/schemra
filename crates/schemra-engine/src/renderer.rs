//! Small deterministic SVG output for static scenes.

use std::{error::Error, fmt, fmt::Write as _};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use schemra_scene::{Rect, Scene, SceneFont, SceneNode, SceneNodeShape, ScenePalette};

mod text;

pub use text::{node_text_width, wrap_text_lines};

struct BundledFont {
    family: &'static str,
    data: &'static [u8],
}

const BUNDLED_FONTS: [BundledFont; 3] = [
    BundledFont {
        family: "Inter Variable",
        data: include_bytes!("assets/fonts/inter-v4.1-subset.woff2"),
    },
    BundledFont {
        family: "Noto Serif",
        data: include_bytes!("assets/fonts/noto-serif-v2.015-subset.woff2"),
    },
    BundledFont {
        family: "Noto Sans Mono",
        data: include_bytes!("assets/fonts/noto-sans-mono-v2.013-subset.woff2"),
    },
];

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
    let used_fonts = used_font_profiles(scene);
    let embedded_font_capacity = BUNDLED_FONTS
        .iter()
        .zip(used_fonts)
        .filter(|(_, used)| *used)
        .map(|(font, _)| font.data.len() * 4 / 3)
        .sum::<usize>();
    let mut svg = String::with_capacity(1024 + scene.nodes.len() * 256 + embedded_font_capacity);
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
    write_font_faces(&mut svg, &used_fonts);
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

    let font_profile = node.style.font.unwrap_or(theme_font);
    let content_gap = node.style.spacing.unwrap_or(12.0);
    let has_icon = node.icon.is_some();
    let text_width = node_text_width(bounds.width, node.layout, has_icon, content_gap);
    let title_lines = text::wrap_text_runs(
        font_profile,
        &node.label,
        text_width,
        17.0,
        node.style.weight.unwrap_or(650),
    );
    let detail_lines = node
        .detail
        .as_deref()
        .map(|detail| text::wrap_text_runs(font_profile, detail, text_width, 14.0, 450))
        .unwrap_or_default();
    let text_height = title_lines.len() as f32 * 22.0
        + if detail_lines.is_empty() {
            0.0
        } else {
            4.0 + detail_lines.len() as f32 * 18.0
        };
    let content_height = if has_icon && node.layout == schemra_scene::SceneNodeLayout::Vertical {
        30.0 + content_gap + text_height
    } else {
        text_height.max(if has_icon { 30.0 } else { 0.0 })
    };
    let content_top = bounds.y + (bounds.height - content_height) / 2.0;
    let (text_x, title_y, text_anchor) =
        if node.layout == schemra_scene::SceneNodeLayout::Horizontal {
            let text_start = bounds.x + if has_icon { 48.0 + content_gap } else { 18.0 };
            let (x, anchor) = match node.text_align {
                schemra_scene::SceneTextAlign::Left => (text_start, "start"),
                schemra_scene::SceneTextAlign::Center => (text_start + text_width / 2.0, "middle"),
            };
            (x, content_top + 17.0, anchor)
        } else {
            let text_y = content_top + if has_icon { 30.0 + content_gap } else { 0.0 };
            let (x, anchor) = match node.text_align {
                schemra_scene::SceneTextAlign::Left => (bounds.x + 18.0, "start"),
                schemra_scene::SceneTextAlign::Center => (bounds.x + bounds.width / 2.0, "middle"),
            };
            (x, text_y + 17.0, anchor)
        };
    write_text_lines(
        svg,
        &title_lines,
        text_x,
        title_y,
        text_anchor,
        font_profile,
        17.0,
        node.style.weight.unwrap_or(650),
        safe_color(node.style.text_color.as_deref(), text),
    );
    if !detail_lines.is_empty() {
        write_text_lines(
            svg,
            &detail_lines,
            text_x,
            title_y + title_lines.len() as f32 * 22.0 + 4.0,
            text_anchor,
            font_profile,
            14.0,
            450,
            safe_color(node.style.text_color.as_deref(), muted),
        );
    }
    svg.push_str("</g>");
}

#[allow(clippy::too_many_arguments)]
fn write_text_lines(
    svg: &mut String,
    lines: &[text::TextLine],
    x: f32,
    y: f32,
    anchor: &str,
    profile: SceneFont,
    size: f32,
    weight: u16,
    color: &str,
) {
    for (index, line) in lines.iter().enumerate() {
        write!(
            svg,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" fill=\"{}\">",
            number(x),
            number(y + index as f32 * if size > 15.0 { 22.0 } else { 18.0 }),
            anchor,
            font_family(profile),
            number(size),
            weight,
            color,
        )
        .expect("writing to a String cannot fail");
        if let Some(runs) = &line.runs {
            for run in runs {
                if run.profile == profile {
                    write!(svg, "<tspan>").expect("writing to a String cannot fail");
                } else {
                    write!(svg, "<tspan font-family=\"{}\">", font_family(run.profile))
                        .expect("writing to a String cannot fail");
                }
                write_xml(svg, &run.text);
                svg.push_str("</tspan>");
            }
        } else {
            write_xml(svg, &line.text);
        }
        svg.push_str("</text>");
    }
}

fn used_font_profiles(scene: &Scene) -> [bool; 3] {
    let mut used = [false; 3];
    for node in &scene.nodes {
        let font = node.style.font.unwrap_or(scene.theme.font);
        for (text, weight) in
            std::iter::once((node.label.as_str(), node.style.weight.unwrap_or(650))).chain(
                node.detail
                    .iter()
                    .map(|detail| (detail.as_str(), node.style.weight.unwrap_or(450))),
            )
        {
            if !text.is_empty() {
                mark_font(&mut used, font);
            }
            for (index, selected) in text::font_profiles_for_text(font, text, weight)
                .into_iter()
                .enumerate()
            {
                used[index] |= selected;
            }
        }
    }
    used
}

fn write_font_faces(svg: &mut String, used: &[bool; 3]) {
    if !used.iter().any(|used| *used) {
        return;
    }
    let mut css = String::new();
    for (index, font) in BUNDLED_FONTS.iter().enumerate() {
        if used[index] {
            let encoded = STANDARD.encode(font.data);
            write!(
                css,
                "@font-face{{font-family:'{}';font-style:normal;font-weight:100 900;font-stretch:normal;src:url(data:font/woff2;base64,{encoded}) format('woff2')}}",
                font.family,
            )
            .expect("writing to a String cannot fail");
        }
    }
    svg.push_str("<defs><style>");
    write_xml(svg, &css);
    svg.push_str("</style></defs>");
}

fn mark_font(used: &mut [bool; 3], font: SceneFont) {
    used[match font {
        SceneFont::Sans => 0,
        SceneFont::Serif => 1,
        SceneFont::Mono => 2,
    }] = true;
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
        SceneFont::Sans => "'Inter Variable', sans-serif",
        SceneFont::Serif => "'Noto Serif', serif",
        SceneFont::Mono => "'Noto Sans Mono', monospace",
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
