use std::fmt::Write as _;

use schemra_scene::{Scene, SceneGroup, SceneVariant};

use super::{
    font_family, nodes,
    text::unwrapped_text_line,
    theme::{self, PaletteColors},
};
use crate::renderer::{number, write_xml};

pub(super) fn write_backgrounds(svg: &mut String, scene: &Scene, colors: &PaletteColors<'_>) {
    for group in &scene.groups {
        write_background(svg, group, colors, &scene.theme);
    }
}

pub(super) fn write_labels(svg: &mut String, scene: &Scene, colors: &PaletteColors<'_>) {
    for group in &scene.groups {
        write_label(svg, group, colors, &scene.theme);
    }
}

fn write_background(
    svg: &mut String,
    group: &SceneGroup,
    colors: &PaletteColors<'_>,
    theme_data: &schemra_scene::SceneTheme,
) {
    let semantic = theme::variant_colors(theme_data, group.style.variant);
    let (fill, stroke) = if group.style.variant == SceneVariant::Default {
        (colors.group_fill, colors.group_border)
    } else {
        (semantic.fill, semantic.stroke)
    };
    svg.push_str("<rect data-group-id=\"");
    write_xml(svg, &group.name);
    write!(
		svg,
		"\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"5 5\"",
		number(group.bounds.x),
		number(group.bounds.y),
		number(group.bounds.width),
		number(group.bounds.height),
		number(group.style.radius.unwrap_or(18.0)),
		theme::resolve_color(group.style.fill.as_deref(), fill, theme_data),
		theme::resolve_color(group.style.stroke.as_deref(), stroke, theme_data),
	)
	.expect("writing to a String cannot fail");
    if let Some(opacity) = group.style.opacity {
        write!(svg, " opacity=\"{}\"", number(opacity)).expect("writing to a String cannot fail");
    }
    svg.push_str("/>");
}

fn write_label(
    svg: &mut String,
    group: &SceneGroup,
    colors: &PaletteColors<'_>,
    theme_data: &schemra_scene::SceneTheme,
) {
    let font = group.style.font.unwrap_or(theme_data.font);
    let weight = group.style.weight.unwrap_or(650);
    let line = unwrapped_text_line(font, &group.title, 11.0, weight);
    let semantic = theme::variant_colors(theme_data, group.style.variant);
    let default_color = if group.style.variant == SceneVariant::Default {
        colors.muted
    } else {
        semantic.text
    };
    svg.push_str("<text data-group-id=\"");
    write_xml(svg, &group.name);
    write!(
		svg,
		"\" x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"11\" font-weight=\"{}\" letter-spacing=\"1.1\" fill=\"{}\" stroke=\"{}\" stroke-width=\"4\" stroke-linejoin=\"round\" paint-order=\"stroke\"",
		number(group.bounds.x + 16.0),
		number(group.bounds.y + 24.0),
		font_family(font),
		weight,
		theme::resolve_color(group.style.text_color.as_deref(), default_color, theme_data),
		colors.canvas,
	)
	.expect("writing to a String cannot fail");
    if let Some(opacity) = group.style.opacity {
        write!(svg, " opacity=\"{}\"", number(opacity)).expect("writing to a String cannot fail");
    }
    svg.push('>');
    nodes::write_text_runs(svg, &line, font);
    svg.push_str("</text>");
}
