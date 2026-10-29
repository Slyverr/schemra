use base64::{Engine as _, engine::general_purpose::STANDARD};
use schemra_engine::{node_text_width, render_svg, wrap_text_lines};
use schemra_scene::{
    FlowDirection, Rect, Scene, SceneFont, SceneNode, SceneNodeLayout, SceneNodeShape, SceneStyle,
    SceneTextAlign, SceneTheme,
};

fn scene(nodes: Vec<SceneNode>) -> Scene {
    Scene {
        title: "Font rendering".into(),
        direction: FlowDirection::LeftToRight,
        theme: SceneTheme::default(),
        width: 800.0,
        height: 180.0,
        groups: Vec::new(),
        nodes,
        texts: Vec::new(),
        images: Vec::new(),
        icons: Vec::new(),
        edges: Vec::new(),
    }
}

fn node(id: &str, label: &str, font: SceneFont) -> SceneNode {
    SceneNode {
        id: id.into(),
        label: label.into(),
        icon: None,
        detail: None,
        layout: SceneNodeLayout::Horizontal,
        text_align: SceneTextAlign::Left,
        shape: SceneNodeShape::Round,
        style: SceneStyle {
            font: Some(font),
            ..SceneStyle::default()
        },
        bounds: Rect {
            x: 10.0,
            y: 10.0,
            width: 240.0,
            height: 60.0,
        },
        group: None,
    }
}

#[test]
fn public_wrapping_uses_glyph_width_and_preserves_blank_lines() {
    let narrow = wrap_text_lines(SceneFont::Sans, "iiiiiiiiiiii", 80.0, 17.0, 650);
    let wide = wrap_text_lines(SceneFont::Sans, "WWWWWWWWWWWW", 80.0, 17.0, 650);
    let blank_lines = wrap_text_lines(SceneFont::Serif, "first\n\nlast", 220.0, 14.0, 450);

    assert_eq!(narrow.len(), 1);
    assert!(wide.len() > narrow.len());
    assert_eq!(blank_lines, ["first", "", "last"]);
}

#[test]
fn public_node_text_width_accounts_for_layout_and_icon_space() {
    assert_eq!(
        node_text_width(252.0, SceneNodeLayout::Horizontal, true, 12.0),
        174.0
    );
    assert_eq!(
        node_text_width(252.0, SceneNodeLayout::Vertical, true, 12.0),
        216.0
    );
}

#[test]
fn embeds_only_the_bundled_profiles_used_by_nodes() {
    let scene = scene(vec![
        node("sans", "Schemra", SceneFont::Sans),
        node("serif", "Crème", SceneFont::Serif),
        node("mono", "Δοκιμή", SceneFont::Mono),
    ]);
    let svg = render_svg(&scene).unwrap();

    assert_eq!(svg.matches("@font-face{").count(), 3);
    for font in [
        include_bytes!("../src/assets/fonts/inter-v4.1-subset.woff2").as_slice(),
        include_bytes!("../src/assets/fonts/noto-serif-v2.015-subset.woff2").as_slice(),
        include_bytes!("../src/assets/fonts/noto-sans-mono-v2.013-subset.woff2").as_slice(),
    ] {
        assert!(svg.contains(&STANDARD.encode(font)));
    }
    assert!(svg.contains("font-family=\"'Inter Variable', sans-serif\""));
    assert!(svg.contains("font-family=\"'Noto Serif', serif\""));
    assert!(svg.contains("font-family=\"'Noto Sans Mono', monospace\""));
}

#[test]
fn unicode_text_remains_in_logical_order_in_the_svg() {
    let visual = scene(vec![node(
        "unicode",
        "Crème Δοκιμή Журнал € →",
        SceneFont::Serif,
    )]);
    let svg = render_svg(&visual).unwrap();

    let glyph_run = ["Crème", "Δοκιμή", "Журнал", "€", "→"].map(|text| {
        svg.find(text)
            .unwrap_or_else(|| panic!("missing rendered text {text:?}"))
    });
    assert!(glyph_run.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(svg.contains("data:font/woff2;base64,"));
    assert!(svg.contains("<text"));
}
