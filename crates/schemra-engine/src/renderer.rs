//! Small deterministic SVG output for static scenes.

use std::{error::Error, fmt, fmt::Write as _};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use schemra_scene::{Rect, Scene, SceneFont};

mod icons;
mod links;
mod nodes;
mod text;
mod theme;

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
    InvalidEdgeGeometry,
    InvalidEdgeStyle,
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
            Self::InvalidEdgeGeometry => {
                formatter.write_str("edge path coordinates must be finite")
            }
            Self::InvalidEdgeStyle => {
                formatter.write_str("edge opacity must be finite and between zero and one")
            }
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
    if scene.edges.iter().any(|edge| {
        edge.path.iter().any(|segment| {
            [
                segment.start.x,
                segment.start.y,
                segment.control1.x,
                segment.control1.y,
                segment.control2.x,
                segment.control2.y,
                segment.end.x,
                segment.end.y,
            ]
            .into_iter()
            .any(|coordinate| !coordinate.is_finite())
        })
    }) {
        return Err(SvgRenderError::InvalidEdgeGeometry);
    }
    if scene.edges.iter().any(|edge| {
        edge.style
            .opacity
            .is_some_and(|opacity| !opacity.is_finite() || !(0.0..=1.0).contains(&opacity))
    }) {
        return Err(SvgRenderError::InvalidEdgeStyle);
    }

    let colors = theme::palette_colors(&scene.theme);
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
        "<svg xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-labelledby=\"schemra-title schemra-description\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" fill=\"none\" data-palette=\"",
        number(scene.width),
        number(scene.height),
        number(scene.width),
        number(scene.height),
    )
    .expect("writing to a String cannot fail");
    write_xml(&mut svg, theme::palette_name(&scene.theme));
    write!(
        svg,
        "\" data-font-profile=\"{}\" data-icon-pack=\"{}\">",
        theme::font_name(scene.theme.font),
        theme::icon_pack_name(scene.theme.icon_pack),
    )
    .expect("writing to a String cannot fail");
    svg.push_str("<title id=\"schemra-title\">");
    write_xml(&mut svg, &scene.title);
    svg.push_str("</title><desc id=\"schemra-description\">Schemra architecture visual</desc>");
    write!(
        svg,
        "<defs><filter id=\"schemra-shadow\" x=\"-20%\" y=\"-20%\" width=\"140%\" height=\"150%\"><feDropShadow dx=\"0\" dy=\"5\" stdDeviation=\"8\" flood-color=\"{}\" flood-opacity=\"0.08\"/></filter>",
        colors.shadow,
    )
    .expect("writing to a String cannot fail");
    svg.push_str("</defs>");
    write_font_faces(&mut svg, &used_fonts);
    write!(
        svg,
        "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        number(scene.width),
        number(scene.height),
        colors.canvas,
    )
    .expect("writing to a String cannot fail");

    links::write_paths(&mut svg, scene, colors.edge);
    links::write_labels(&mut svg, scene, colors.canvas, colors.muted);

    for node in &scene.nodes {
        nodes::write_node(
            &mut svg,
            node,
            &colors,
            scene.theme.font,
            scene.theme.icon_pack,
            &scene.theme,
        );
    }
    svg.push_str("</svg>");
    Ok(svg)
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
    for edge in &scene.edges {
        let Some(label) = edge.label.as_deref() else {
            continue;
        };
        let font = edge.style.font.unwrap_or(scene.theme.font);
        let weight = edge.style.weight.unwrap_or(550);
        if !label.is_empty() {
            mark_font(&mut used, font);
        }
        for (index, selected) in text::font_profiles_for_text(font, label, weight)
            .into_iter()
            .enumerate()
        {
            used[index] |= selected;
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

pub(super) fn font_family(font: SceneFont) -> &'static str {
    match font {
        SceneFont::Sans => "'Inter Variable', sans-serif",
        SceneFont::Serif => "'Noto Serif', serif",
        SceneFont::Mono => "'Noto Sans Mono', monospace",
    }
}

pub(super) fn number(value: f32) -> String {
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

pub(super) fn write_xml(svg: &mut String, text: &str) {
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
