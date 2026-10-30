use std::fmt::Write as _;

use schemra_scene::{Rect, SceneIconPack, is_safe_image_source};

use super::theme::PaletteColors;
use crate::renderer::{number, write_xml};

pub(super) fn write_node_icon(
    svg: &mut String,
    name: &str,
    x: f32,
    y: f32,
    colors: &PaletteColors<'_>,
    icon_pack: SceneIconPack,
) {
    let (background, foreground) = match icon_pack {
        SceneIconPack::Outline => (colors.icon_background, colors.icon_foreground),
        SceneIconPack::Solid => (colors.solid_icon_background, colors.solid_icon_foreground),
    };
    let name = canonical_name(name);

    svg.push_str("<g data-node-icon=\"");
    write_xml(svg, name);
    svg.push_str("\" aria-hidden=\"true\">");
    write!(
        svg,
        "<rect x=\"{}\" y=\"{}\" width=\"30\" height=\"30\" rx=\"9\" fill=\"{}\"/>",
        number(x),
        number(y),
        background
    )
    .expect("writing to a String cannot fail");

    if name.starts_with("data:image/") && is_safe_image_source(name) {
        write!(
            svg,
            "<image x=\"{}\" y=\"{}\" width=\"24\" height=\"24\" preserveAspectRatio=\"xMidYMid meet\" href=\"",
            number(x + 3.0),
            number(y + 3.0)
        )
        .expect("writing to a String cannot fail");
        write_xml(svg, name);
        svg.push_str("\"/>");
        svg.push_str("</g>");
        return;
    }

    if icon_pack == SceneIconPack::Solid {
        write_solid_icon(svg, name, x + 3.0, y + 3.0, foreground, background);
    } else {
        write_outline_icon(svg, name, x + 3.0, y + 3.0, foreground, background);
    }
    svg.push_str("</g>");
}

pub(super) fn write_annotation_icon(
    svg: &mut String,
    id: &str,
    name: &str,
    bounds: Rect,
    colors: &PaletteColors<'_>,
    icon_pack: SceneIconPack,
) {
    let (background, foreground) = match icon_pack {
        SceneIconPack::Outline => (colors.icon_background, colors.icon_foreground),
        SceneIconPack::Solid => (colors.solid_icon_background, colors.solid_icon_foreground),
    };
    let name = canonical_name(name);
    svg.push_str("<g data-icon-id=\"");
    write_xml(svg, id);
    svg.push_str("\" role=\"img\" aria-label=\"");
    write_xml(svg, id);
    write!(
        svg,
        "\" transform=\"translate({} {}) scale({} {})\">",
        number(bounds.x),
        number(bounds.y),
        number(bounds.width / 30.0),
        number(bounds.height / 30.0),
    )
    .expect("writing to a String cannot fail");
    write!(
        svg,
        "<rect width=\"30\" height=\"30\" rx=\"9\" fill=\"{}\"/>",
        background
    )
    .expect("writing to a String cannot fail");

    if name.starts_with("data:image/") && is_safe_image_source(name) {
        svg.push_str("<image x=\"3\" y=\"3\" width=\"24\" height=\"24\" preserveAspectRatio=\"xMidYMid meet\" href=\"");
        write_xml(svg, name);
        svg.push_str("\"/>");
    } else if icon_pack == SceneIconPack::Solid {
        write_solid_icon(svg, name, 3.0, 3.0, foreground, background);
    } else {
        write_outline_icon(svg, name, 3.0, 3.0, foreground, background);
    }

    svg.push_str("</g>");
}

fn canonical_name(name: &str) -> &str {
    let name = name.strip_prefix("lucide/").unwrap_or(name);
    if name == "send" { "mail" } else { name }
}

fn write_outline_icon(svg: &mut String, name: &str, x: f32, y: f32, ink: &str, paper: &str) {
    write!(
        svg,
        "<g transform=\"translate({} {})\" stroke=\"{}\" stroke-width=\"1.7\" stroke-linecap=\"round\" stroke-linejoin=\"round\" fill=\"none\">",
        number(x),
        number(y),
        ink
    )
    .expect("writing to a String cannot fail");
    match name {
        "browser" => svg.push_str("<rect x=\"2\" y=\"3\" width=\"20\" height=\"17\" rx=\"2.5\"/><path d=\"M2 8h20M6 5.5h.01M9 5.5h.01M6 11.5h12\"/>") ,
        "server" => svg.push_str("<rect x=\"3\" y=\"3\" width=\"18\" height=\"7\" rx=\"2\"/><rect x=\"3\" y=\"14\" width=\"18\" height=\"7\" rx=\"2\"/><path d=\"M7 6.5h.01M7 17.5h.01M11 6.5h6M11 17.5h6\"/>") ,
        "database" => svg.push_str("<ellipse cx=\"12\" cy=\"5\" rx=\"8.5\" ry=\"3\"/><path d=\"M3.5 5v7c0 1.7 3.8 3 8.5 3s8.5-1.3 8.5-3V5M3.5 12v7c0 1.7 3.8 3 8.5 3s8.5-1.3 8.5-3v-7\"/>") ,
        "shield" => svg.push_str("<path d=\"M12 22s8-4 8-11V5l-8-3-8 3v6c0 7 8 11 8 11z\"/><path d=\"m9 12 2 2 4-4\"/>") ,
        "queue" => write!(svg, "<path d=\"M4 5h16M4 12h16M4 19h16\"/><circle cx=\"7\" cy=\"5\" r=\"1.5\" fill=\"{paper}\"/><circle cx=\"13\" cy=\"12\" r=\"1.5\" fill=\"{paper}\"/><circle cx=\"17\" cy=\"19\" r=\"1.5\" fill=\"{paper}\"/>").expect("writing to a String cannot fail"),
        "cloud" => svg.push_str("<path d=\"M7.5 18H7a5 5 0 1 1 1.6-9.7A6.5 6.5 0 0 1 21 11a4 4 0 0 1-1 7h-1.5\"/><path d=\"m9 15 3-3 3 3m-3-3v9\"/>") ,
        "code" => svg.push_str("<path d=\"m8 7-5 5 5 5m8-10 5 5-5 5m-3-12-2 14\"/>") ,
        "user" => svg.push_str("<circle cx=\"12\" cy=\"8\" r=\"4\"/><path d=\"M5 21a7 7 0 0 1 14 0\"/>") ,
        "layers" => svg.push_str("<path d=\"m12 3 9 5-9 5-9-5 9-5zM3 12l9 5 9-5M3 16l9 5 9-5\"/>") ,
        "sparkles" => svg.push_str("<path d=\"m12 3 1.9 5.8L20 11l-6.1 2.2L12 19l-1.9-5.8L4 11l6.1-2.2L12 3zM19 15l1 2.5 2.5 1-2.5 1L19 22l-1-2.5-2.5-1 2.5-1 1-2.5z\"/>") ,
        "mail" => svg.push_str("<rect x=\"3\" y=\"5\" width=\"18\" height=\"14\" rx=\"2\"/><path d=\"m4 7 8 6 8-6\"/>") ,
        "message" => svg.push_str("<path d=\"M21 11.5a8.4 8.4 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.4 8.4 0 0 1-3.8-.9L3 21l1.9-5.7a8.4 8.4 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.4 8.4 0 0 1 3.8-.9h.5a8.5 8.5 0 0 1 8 8v.5z\"/>") ,
        "package" => svg.push_str("<path d=\"m12 3 9 5-9 5-9-5 9-5zM3 8v9l9 5 9-5V8M12 13v9\"/>") ,
        "file" => svg.push_str("<path d=\"M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z\"/><path d=\"M14 2v6h6M8 13h8M8 17h8\"/>") ,
        _ => svg.push_str("<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"M12 11v5m0-8h.01\"/>") ,
    }
    svg.push_str("</g>");
}

fn write_solid_icon(svg: &mut String, name: &str, x: f32, y: f32, ink: &str, paper: &str) {
    write!(
        svg,
        "<g transform=\"translate({} {}) scale(0.75)\" stroke-linecap=\"round\" stroke-linejoin=\"round\">",
        number(x),
        number(y)
    )
    .expect("writing to a String cannot fail");
    match name {
        "browser" => write!(svg, "<rect x=\"4\" y=\"5\" width=\"24\" height=\"20\" rx=\"3\" fill=\"{ink}\"/><path d=\"M4 10h24M9 7.6h.1m3.5 0h.1\" stroke=\"{paper}\" stroke-width=\"1.8\"/><path d=\"M9 15h14M9 19h9\" stroke=\"{paper}\" stroke-width=\"1.8\"/>") .expect("writing to a String cannot fail"),
        "server" => write!(svg, "<rect x=\"5\" y=\"4\" width=\"22\" height=\"10\" rx=\"2\" fill=\"{ink}\"/><rect x=\"5\" y=\"17\" width=\"22\" height=\"10\" rx=\"2\" fill=\"{ink}\"/><path d=\"M9 9h.1m4 0h9M9 22h.1m4 0h9\" stroke=\"{paper}\" stroke-width=\"1.8\"/>") .expect("writing to a String cannot fail"),
        "database" => write!(svg, "<path d=\"M5 7c0-2.2 4.9-4 11-4s11 1.8 11 4v16c0 2.2-4.9 4-11 4S5 25.2 5 23V7Z\" fill=\"{ink}\"/><ellipse cx=\"16\" cy=\"7\" rx=\"11\" ry=\"4\" fill=\"{ink}\" stroke=\"{paper}\" stroke-width=\"1.5\"/><path d=\"M5 15c0 2.2 4.9 4 11 4s11-1.8 11-4M5 22c0 2.2 4.9 4 11 4s11-1.8 11-4\" fill=\"none\" stroke=\"{paper}\" stroke-width=\"1.5\"/>") .expect("writing to a String cannot fail"),
        "shield" => write!(svg, "<path d=\"M16 3 27 7v8c0 6.6-4.4 11-11 14C9.4 26 5 21.6 5 15V7l11-4Z\" fill=\"{ink}\"/><path d=\"m11 16 3.2 3.2 6.8-7\" fill=\"none\" stroke=\"{paper}\" stroke-width=\"2.2\"/>") .expect("writing to a String cannot fail"),
        "queue" => write!(svg, "<path d=\"M10 8h17M10 16h17M10 24h17\" stroke=\"{ink}\" stroke-width=\"3\"/><circle cx=\"5.5\" cy=\"8\" r=\"2.5\" fill=\"{ink}\"/><circle cx=\"5.5\" cy=\"16\" r=\"2.5\" fill=\"{ink}\"/><circle cx=\"5.5\" cy=\"24\" r=\"2.5\" fill=\"{ink}\"/>") .expect("writing to a String cannot fail"),
        "cloud" => write!(svg, "<path d=\"M10 26h13a7 7 0 0 0 0-14h-1a9 9 0 0 0-17-2 7 7 0 0 0 5 16Z\" fill=\"{ink}\"/>") .expect("writing to a String cannot fail"),
        "code" => write!(svg, "<path d=\"m11 9-7 7 7 7m10-14 7 7-7 7m-3-17-4 20\" fill=\"none\" stroke=\"{ink}\" stroke-width=\"3\"/>") .expect("writing to a String cannot fail"),
        "user" => write!(svg, "<circle cx=\"16\" cy=\"10\" r=\"6\" fill=\"{ink}\"/><path d=\"M5 28a11 11 0 0 1 22 0\" fill=\"{ink}\"/>") .expect("writing to a String cannot fail"),
        "layers" => write!(svg, "<path d=\"m16 3 13 8-13 8L3 11l13-8Zm-13 14 13 8 13-8M3 23l13 8 13-8\" fill=\"{ink}\" stroke=\"{paper}\" stroke-width=\"1.4\"/>") .expect("writing to a String cannot fail"),
        "sparkles" => write!(svg, "<path d=\"m16 3 3 9 9 3-9 3-3 9-3-9-9-3 9-3 3-9Zm10 18 1.2 3.8L31 26l-3.8 1.2L26 31l-1.2-3.8L21 26l3.8-1.2L26 21Z\" fill=\"{ink}\"/>") .expect("writing to a String cannot fail"),
        "mail" => write!(svg, "<path d=\"M4 6h24v20H4z\" fill=\"{ink}\"/><path d=\"m5 8 11 9L27 8\" fill=\"none\" stroke=\"{paper}\" stroke-width=\"2\"/>") .expect("writing to a String cannot fail"),
        "message" => write!(svg, "<path d=\"M16 4a12 12 0 0 0-10 18l-2 6 6-2A12 12 0 1 0 16 4Z\" fill=\"{ink}\"/>") .expect("writing to a String cannot fail"),
        "package" => write!(svg, "<path d=\"m16 3 13 7v14l-13 7-13-7V10l13-7Z\" fill=\"{ink}\"/><path d=\"m3 10 13 7 13-7m-13 7v14\" fill=\"none\" stroke=\"{paper}\" stroke-width=\"1.8\"/>") .expect("writing to a String cannot fail"),
        "file" => write!(svg, "<path d=\"M7 3h12l7 7v20H7V3Z\" fill=\"{ink}\"/><path d=\"M19 3v8h7M11 18h11m-11 5h11\" fill=\"none\" stroke=\"{paper}\" stroke-width=\"1.8\"/>") .expect("writing to a String cannot fail"),
        _ => write!(svg, "<path d=\"m16 3 13 8-13 8L3 11l13-8Zm-13 14 13 8 13-8M3 23l13 8 13-8\" fill=\"{ink}\" stroke=\"{paper}\" stroke-width=\"1.4\"/>") .expect("writing to a String cannot fail"),
    }
    svg.push_str("</g>");
}
