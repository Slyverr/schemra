use std::fmt::Write as _;

use schemra_scene::{
    Rect, SceneFont, SceneIconPack, SceneNode, SceneNodeLayout, SceneNodeShape, SceneTextAlign,
    SceneTheme, SceneVariant,
};

use super::{
    font_family, icons,
    text::{self, TextLine},
    theme::{self, PaletteColors},
};
use crate::renderer::{number, write_xml};

pub(super) fn write_node(
    svg: &mut String,
    node: &SceneNode,
    colors: &PaletteColors<'_>,
    theme_font: SceneFont,
    icon_pack: SceneIconPack,
    theme_data: &SceneTheme,
) {
    let semantic = theme::variant_colors(theme_data, node.style.variant);
    let (default_fill, default_stroke, default_text) =
        if node.style.variant == SceneVariant::Default {
            (colors.card, colors.border, colors.text)
        } else {
            (semantic.fill, semantic.stroke, semantic.text)
        };
    let fill = theme::resolve_color(node.style.fill.as_deref(), default_fill, theme_data);
    let stroke = theme::resolve_color(node.style.stroke.as_deref(), default_stroke, theme_data);
    let text_color =
        theme::resolve_color(node.style.text_color.as_deref(), default_text, theme_data);
    let detail_color = theme::resolve_color(
        node.style.text_color.as_deref(),
        if node.style.variant == SceneVariant::Default {
            colors.muted
        } else {
            semantic.text
        },
        theme_data,
    );
    let profile = node.style.font.unwrap_or(theme_font);

    svg.push_str("<g data-node-id=\"");
    write_xml(svg, &node.id);
    svg.push_str("\" data-node-label=\"");
    write_xml(svg, &node.label);
    svg.push_str("\" data-node-detail=\"");
    write_xml(svg, node.detail.as_deref().unwrap_or_default());
    svg.push_str("\" data-node-shape=\"");
    svg.push_str(shape_name(node.shape));
    svg.push_str("\" data-node-variant=\"");
    svg.push_str(variant_name(node.style.variant));
    svg.push('"');
    if let Some(opacity) = node.style.opacity {
        write!(svg, " opacity=\"{}\"", number(opacity)).expect("writing to a String cannot fail");
    }
    svg.push('>');
    write_node_shape(
        svg,
        node.bounds,
        node.shape,
        fill,
        stroke,
        node.style.radius,
    );

    let has_icon = node.icon.is_some();
    let horizontal = node.layout == SceneNodeLayout::Horizontal;
    let content_gap = node.style.spacing.unwrap_or(12.0);
    let text_width = text::node_text_width(node.bounds.width, node.layout, has_icon, content_gap);
    let title_weight = node.style.weight.unwrap_or(650);
    let detail_weight = node.style.weight.unwrap_or(450);
    let title_lines = text::wrap_text_runs(profile, &node.label, text_width, 17.0, title_weight);
    let detail_lines = node
        .detail
        .as_deref()
        .map(|detail| text::wrap_text_runs(profile, detail, text_width, 14.0, detail_weight))
        .unwrap_or_default();
    let text_height = title_lines.len() as f32 * 22.0
        + if detail_lines.is_empty() {
            0.0
        } else {
            4.0 + detail_lines.len() as f32 * 18.0
        };
    let content_height = if has_icon && !horizontal {
        30.0 + content_gap + text_height
    } else {
        text_height.max(if has_icon { 30.0 } else { 0.0 })
    };
    let content_top = node.bounds.y + (node.bounds.height - content_height) / 2.0;

    let (text_x, anchor, title_y) = if horizontal {
        if let Some(icon) = node.icon.as_deref() {
            icons::write_node_icon(
                svg,
                icon,
                node.bounds.x + 18.0,
                content_top + (content_height - 30.0) / 2.0,
                colors,
                icon_pack,
            );
        }
        let text_start = node.bounds.x + if has_icon { 48.0 + content_gap } else { 18.0 };
        let (x, anchor) = text_anchor(node, text_start, text_width);
        (x, anchor, content_top + 17.0)
    } else {
        if let Some(icon) = node.icon.as_deref() {
            let icon_x = match node.text_align {
                SceneTextAlign::Left => node.bounds.x + 18.0,
                SceneTextAlign::Center => node.bounds.x + (node.bounds.width - 30.0) / 2.0,
            };
            icons::write_node_icon(svg, icon, icon_x, content_top, colors, icon_pack);
        }
        let text_y = content_top + if has_icon { 30.0 + content_gap } else { 0.0 };
        let (x, anchor) = match node.text_align {
            SceneTextAlign::Left => (node.bounds.x + 18.0, "start"),
            SceneTextAlign::Center => (node.bounds.x + node.bounds.width / 2.0, "middle"),
        };
        (x, anchor, text_y + 17.0)
    };

    write_text_lines(
        svg,
        &title_lines,
        TextStyle {
            x: text_x,
            y: title_y,
            line_height: 22.0,
            font_size: 17.0,
            font_weight: title_weight,
            color: text_color,
            anchor,
            role: "label",
            max_width: text_width,
        },
        profile,
    );
    if !detail_lines.is_empty() {
        write_text_lines(
            svg,
            &detail_lines,
            TextStyle {
                x: text_x,
                y: title_y + title_lines.len() as f32 * 22.0 + 4.0,
                line_height: 18.0,
                font_size: 14.0,
                font_weight: detail_weight,
                color: detail_color,
                anchor,
                role: "detail",
                max_width: text_width,
            },
            profile,
        );
    }
    svg.push_str("</g>");
}

fn text_anchor(node: &SceneNode, text_start: f32, text_width: f32) -> (f32, &'static str) {
    match node.text_align {
        SceneTextAlign::Left => (text_start, "start"),
        SceneTextAlign::Center => (text_start + text_width / 2.0, "middle"),
    }
}

fn write_node_shape(
    svg: &mut String,
    bounds: Rect,
    shape: SceneNodeShape,
    fill: &str,
    stroke: &str,
    radius: Option<f32>,
) {
    if shape == SceneNodeShape::Cylinder {
        let ellipse = (bounds.height * 0.12).min(12.0).min(bounds.height / 2.0);
        write!(
            svg,
            "<path data-node-background=\"true\" d=\"M {} {} C {} {} {} {} {} {} V {} C {} {} {} {} {} {} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\" filter=\"url(#schemra-shadow)\"/>",
            number(bounds.x),
            number(bounds.y + ellipse),
            number(bounds.x),
            number(bounds.y),
            number(bounds.x + bounds.width),
            number(bounds.y),
            number(bounds.x + bounds.width),
            number(bounds.y + ellipse),
            number(bounds.y + bounds.height - ellipse),
            number(bounds.x + bounds.width),
            number(bounds.y + bounds.height),
            number(bounds.x),
            number(bounds.y + bounds.height),
            number(bounds.x),
            number(bounds.y + bounds.height - ellipse),
            fill,
            stroke,
        )
        .expect("writing to a String cannot fail");
        write!(
            svg,
            "<path data-node-variant-stroke=\"true\" d=\"M {} {} C {} {} {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.2\"/>",
            number(bounds.x),
            number(bounds.y + bounds.height - ellipse),
            number(bounds.x + bounds.width * 0.22),
            number(bounds.y + bounds.height - ellipse + 5.0),
            number(bounds.x + bounds.width * 0.78),
            number(bounds.y + bounds.height - ellipse + 5.0),
            number(bounds.x + bounds.width),
            number(bounds.y + bounds.height - ellipse),
            stroke,
        )
        .expect("writing to a String cannot fail");
        return;
    }

    let corner_radius = if shape == SceneNodeShape::Round {
        radius
            .unwrap_or(16.0)
            .clamp(0.0, bounds.width.min(bounds.height) / 2.0)
    } else {
        0.0
    };
    write!(
        svg,
        "<rect data-node-background=\"true\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\" filter=\"url(#schemra-shadow)\"/>",
        number(bounds.x),
        number(bounds.y),
        number(bounds.width),
        number(bounds.height),
        number(corner_radius),
        fill,
        stroke,
    )
    .expect("writing to a String cannot fail");
}

fn write_text_lines(
    svg: &mut String,
    lines: &[TextLine],
    style: TextStyle<'_>,
    profile: SceneFont,
) {
    for (index, line) in lines.iter().enumerate() {
        write!(
            svg,
            "<text data-node-text=\"{}\" data-node-text-width=\"{}\" data-node-text-line-height=\"{}\" x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" fill=\"{}\">",
            style.role,
            number(style.max_width),
            number(style.line_height),
            number(style.x),
            number(style.y + index as f32 * style.line_height),
            style.anchor,
            font_family(profile),
            number(style.font_size),
            style.font_weight,
            style.color,
        )
        .expect("writing to a String cannot fail");
        write_text_runs(svg, line, profile);
        svg.push_str("</text>");
    }
}

fn write_text_runs(svg: &mut String, line: &TextLine, base_profile: SceneFont) {
    let Some(runs) = &line.runs else {
        write_xml(svg, &line.text);
        return;
    };
    for run in runs {
        if run.profile == base_profile {
            svg.push_str("<tspan>");
        } else {
            write!(svg, "<tspan font-family=\"{}\">", font_family(run.profile))
                .expect("writing to a String cannot fail");
        }
        write_xml(svg, &run.text);
        svg.push_str("</tspan>");
    }
}

struct TextStyle<'style> {
    x: f32,
    y: f32,
    line_height: f32,
    font_size: f32,
    font_weight: u16,
    color: &'style str,
    anchor: &'style str,
    role: &'style str,
    max_width: f32,
}

fn shape_name(shape: SceneNodeShape) -> &'static str {
    match shape {
        SceneNodeShape::Round => "round",
        SceneNodeShape::Rectangle => "rectangle",
        SceneNodeShape::Cylinder => "cylinder",
    }
}

fn variant_name(variant: SceneVariant) -> &'static str {
    match variant {
        SceneVariant::Default => "default",
        SceneVariant::Muted => "muted",
        SceneVariant::Accent => "accent",
        SceneVariant::Success => "success",
        SceneVariant::Warning => "warning",
        SceneVariant::Danger => "danger",
    }
}
