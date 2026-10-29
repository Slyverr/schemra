use std::fmt::Write as _;

use schemra_scene::{
    Scene, SceneImage, SceneImageFit, SceneText, SceneTextAlign, is_safe_image_source,
};

use super::{
    font_family, icons, nodes,
    text::wrap_text_runs,
    theme::{self, PaletteColors},
};
use crate::renderer::{number, write_xml};

pub(super) fn write_texts(svg: &mut String, scene: &Scene, colors: &PaletteColors<'_>) {
    for annotation in &scene.texts {
        write_text(svg, annotation, colors, scene.theme.font, &scene.theme);
    }
}

pub(super) fn write_images(svg: &mut String, scene: &Scene) {
    for image in &scene.images {
        write_image(svg, image);
    }
}

pub(super) fn write_icons(svg: &mut String, scene: &Scene, colors: &PaletteColors<'_>) {
    for icon in &scene.icons {
        icons::write_annotation_icon(
            svg,
            &icon.id,
            &icon.icon,
            icon.bounds,
            colors,
            scene.theme.icon_pack,
        );
    }
}

fn write_text(
    svg: &mut String,
    annotation: &SceneText,
    colors: &PaletteColors<'_>,
    theme_font: schemra_scene::SceneFont,
    theme_data: &schemra_scene::SceneTheme,
) {
    let font_profile = annotation.style.font.unwrap_or(theme_font);
    let font = font_family(font_profile);
    let weight = annotation.style.weight.unwrap_or(450);
    let lines = wrap_text_runs(
        font_profile,
        &annotation.text,
        annotation.bounds.width,
        14.0,
        weight,
    );
    let color = theme::resolve_color(
        annotation.style.text_color.as_deref(),
        colors.text,
        theme_data,
    );
    let (x, anchor) = match annotation.align {
        SceneTextAlign::Left => (annotation.bounds.x, "start"),
        SceneTextAlign::Center => (
            annotation.bounds.x + annotation.bounds.width / 2.0,
            "middle",
        ),
    };

    svg.push_str("<g data-text-id=\"");
    write_xml(svg, &annotation.id);
    svg.push_str("\" aria-label=\"");
    write_xml(svg, &annotation.text);
    svg.push('"');
    if let Some(opacity) = annotation.style.opacity {
        write!(svg, " opacity=\"{}\"", number(opacity)).expect("writing to a String cannot fail");
    }
    svg.push('>');

    for (index, line) in lines.iter().enumerate() {
        write!(
            svg,
            "<text data-text-line=\"{index}\" x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\" font-family=\"{font}\" font-size=\"14\" font-weight=\"{weight}\" fill=\"{color}\">",
            number(x),
            number(annotation.bounds.y + 16.0 + index as f32 * 20.0),
        )
        .expect("writing to a String cannot fail");
        nodes::write_text_runs(svg, line, font_profile);
        svg.push_str("</text>");
    }

    svg.push_str("</g>");
}

fn write_image(svg: &mut String, image: &SceneImage) {
    if !is_safe_image_source(&image.src) {
        return;
    }
    let bounds = image.bounds;
    svg.push_str("<image data-image-id=\"");
    write_xml(svg, &image.id);
    write!(
        svg,
        "\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"",
        number(bounds.x),
        number(bounds.y),
        number(bounds.width),
        number(bounds.height),
    )
    .expect("writing to a String cannot fail");
    write_xml(svg, &image.src);
    svg.push_str("\" preserveAspectRatio=\"");
    svg.push_str(match image.fit {
        SceneImageFit::Contain => "xMidYMid meet",
        SceneImageFit::Cover => "xMidYMid slice",
        SceneImageFit::Fill => "none",
    });
    svg.push('"');
    if let Some(opacity) = image.style.opacity {
        write!(svg, " opacity=\"{}\"", number(opacity)).expect("writing to a String cannot fail");
    }
    if let Some(alt) = image.alt.as_deref().filter(|alt| !alt.is_empty()) {
        svg.push_str(" role=\"img\" aria-label=\"");
        write_xml(svg, alt);
        svg.push('"');
    } else {
        svg.push_str(" aria-hidden=\"true\"");
    }
    svg.push_str("/>");
}
