use schemra_engine::{SvgRenderError, render_svg};
use schemra_scene::{
    FlowDirection, Rect, Scene, SceneFont, SceneNode, SceneNodeLayout, SceneNodeShape,
    ScenePalette, SceneStyle, SceneTextAlign, SceneTheme,
};

fn make_scene(title: &str, node: SceneNode) -> Scene {
    Scene {
        title: title.into(),
        direction: FlowDirection::LeftToRight,
        theme: SceneTheme::default(),
        width: 120.0,
        height: 72.0,
        groups: Vec::new(),
        nodes: vec![node],
        texts: Vec::new(),
        images: Vec::new(),
        icons: Vec::new(),
        edges: Vec::new(),
    }
}

fn node() -> SceneNode {
    SceneNode {
        id: "api".into(),
        label: "Gateway".into(),
        icon: None,
        detail: None,
        layout: SceneNodeLayout::Horizontal,
        text_align: SceneTextAlign::Left,
        shape: SceneNodeShape::Round,
        style: SceneStyle::default(),
        bounds: Rect {
            x: 10.0,
            y: 11.0,
            width: 100.0,
            height: 50.0,
        },
        group: None,
    }
}

#[test]
fn renders_a_static_node_as_the_expected_svg() {
    let svg = render_svg(&make_scene("Gateway", node())).unwrap();

    assert_eq!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-labelledby=\"schemra-title schemra-description\" width=\"120\" height=\"72\" viewBox=\"0 0 120 72\"><title id=\"schemra-title\">Gateway</title><desc id=\"schemra-description\">Schemra architecture visual</desc><rect width=\"120\" height=\"72\" fill=\"#ffffff\"/><g data-node-id=\"api\" data-node-label=\"Gateway\" data-node-shape=\"round\"><rect x=\"10\" y=\"11\" width=\"100\" height=\"50\" rx=\"12\" fill=\"#ffffff\" stroke=\"#e4e4e7\" stroke-width=\"1\"/><text x=\"26\" y=\"41\" text-anchor=\"start\" font-family=\"sans-serif\" font-size=\"16\" font-weight=\"600\" fill=\"#18181b\">Gateway</text></g></svg>"
    );
}

#[test]
fn escapes_scene_text_and_node_attributes() {
    let mut node = node();
    node.id = "api\"><script>".into();
    node.label = "A & <gateway> \"service\"".into();
    node.detail = Some("routes & validates".into());

    let svg = render_svg(&make_scene("Flow <one> & two\0", node)).unwrap();

    assert!(svg.contains("<title id=\"schemra-title\">Flow &lt;one&gt; &amp; two�</title>"));
    assert!(svg.contains("data-node-id=\"api&quot;&gt;&lt;script&gt;\""));
    assert!(svg.contains("A &amp; &lt;gateway&gt; &quot;service&quot;"));
    assert!(svg.contains("routes &amp; validates"));
    assert!(!svg.contains("<script>"));
}

#[test]
fn uses_scene_palette_and_font_enums_without_interpolating_arbitrary_paint() {
    let mut scene = make_scene("Palette", node());
    scene.theme.palette = ScenePalette::Midnight;
    scene.nodes[0].style.font = Some(SceneFont::Mono);
    scene.nodes[0].style.stroke = Some("rebeccapurple".into());
    scene.nodes[0].style.fill = Some("url(https://untrusted.test/x)".into());

    let svg = render_svg(&scene).unwrap();

    assert!(svg.contains("fill=\"#0f172a\""));
    assert!(svg.contains("fill=\"#1e293b\" stroke=\"rebeccapurple\""));
    assert!(svg.contains("font-family=\"monospace\""));
    assert!(!svg.contains("untrusted.test"));
}

#[test]
fn serializes_rectangular_and_cylindrical_node_shapes() {
    let mut scene = make_scene("Shapes", node());
    scene.nodes[0].shape = SceneNodeShape::Rectangle;
    let rectangle = render_svg(&scene).unwrap();
    assert!(rectangle.contains("width=\"100\" height=\"50\" rx=\"0\""));

    scene.nodes[0].shape = SceneNodeShape::Cylinder;
    let cylinder = render_svg(&scene).unwrap();
    assert!(
        cylinder.contains("<path d=\"M 10 17 C 10 11 110 11 110 17 V 55 C 110 61 10 61 10 55 Z\"")
    );
}

#[test]
fn rejects_non_finite_or_non_positive_canvas_and_node_bounds() {
    let mut scene = make_scene("Invalid", node());
    scene.width = f32::INFINITY;
    assert_eq!(render_svg(&scene), Err(SvgRenderError::InvalidCanvasBounds));

    let mut scene = make_scene("Invalid", node());
    scene.nodes[0].bounds.height = 0.0;
    assert_eq!(render_svg(&scene), Err(SvgRenderError::InvalidNodeBounds));

    let mut scene = make_scene("Invalid", node());
    scene.nodes[0].style.opacity = Some(f32::NAN);
    assert_eq!(render_svg(&scene), Err(SvgRenderError::InvalidNodeStyle));
}
