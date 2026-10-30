use schemra_renderer::{SvgRenderError, render_svg};
use schemra_scene::{
    FlowDirection, Rect, Scene, SceneFont, SceneGroup, SceneNode, SceneNodeLayout, SceneNodeShape,
    ScenePalette, SceneStyle, SceneTextAlign, SceneTheme, SceneVariant,
};

fn make_scene(title: &str, node: SceneNode) -> Scene {
    Scene {
        title: title.into(),
        direction: FlowDirection::LeftToRight,
        theme: SceneTheme::default(),
        width: 180.0,
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
            width: 160.0,
            height: 50.0,
        },
        group: None,
    }
}

fn group() -> SceneGroup {
    SceneGroup {
        name: "application".into(),
        title: "Application".into(),
        bounds: Rect {
            x: 4.0,
            y: 5.0,
            width: 172.0,
            height: 62.0,
        },
        style: SceneStyle::default(),
    }
}

#[test]
fn renders_a_static_node_as_the_expected_svg() {
    let svg = render_svg(&make_scene("Gateway", node())).unwrap();

    assert!(
        svg.contains(
            "data-palette=\"paper\" data-font-profile=\"sans\" data-icon-pack=\"outline\""
        )
    );
    assert!(svg.contains("id=\"schemra-shadow\""));
    assert!(svg.contains("data-node-shape=\"round\" data-node-variant=\"default\""));
    assert!(
        svg.contains("width=\"160\" height=\"50\" rx=\"16\" fill=\"#ffffff\" stroke=\"#e4e4e7\"")
    );
    assert!(svg.contains("data-node-text=\"label\""));
    assert!(svg.contains("font-family=\"'Inter Variable', sans-serif\""));
    assert!(svg.contains("<tspan>Gateway</tspan>"));
}

#[test]
fn keeps_node_dimensions_and_tightens_title_to_detail_spacing() {
    let mut node = node();
    node.detail = Some("Routes traffic".into());
    let svg = render_svg(&make_scene("Gateway", node)).unwrap();

    assert!(
        svg.contains("width=\"160\" height=\"50\" rx=\"16\" fill=\"#ffffff\" stroke=\"#e4e4e7\"")
    );
    assert!(svg.contains("data-node-text=\"label\" data-node-text-width=\"124\" data-node-text-line-height=\"22\" x=\"28\" y=\"32\""));
    assert!(svg.contains("data-node-text=\"detail\" data-node-text-width=\"124\" data-node-text-line-height=\"18\" x=\"28\" y=\"56\""));
}

#[test]
fn escapes_scene_text_and_node_attributes() {
    let mut node = node();
    node.id = "api\"><script>".into();
    node.label = "A & <gateway> \"service\"".into();
    node.detail = Some("routes & validates".into());

    let mut scene = make_scene("Flow <one> & two\0", node);
    scene.width = 300.0;
    scene.nodes[0].bounds.width = 280.0;
    let svg = render_svg(&scene).unwrap();

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
    assert!(svg.contains("font-family=\"'Noto Sans Mono', monospace\""));
    assert!(!svg.contains("untrusted.test"));
}

#[test]
fn serializes_rectangular_and_cylindrical_node_shapes() {
    let mut scene = make_scene("Shapes", node());
    scene.nodes[0].shape = SceneNodeShape::Rectangle;
    let rectangle = render_svg(&scene).unwrap();
    assert!(rectangle.contains("width=\"160\" height=\"50\" rx=\"0\""));

    scene.nodes[0].shape = SceneNodeShape::Cylinder;
    let cylinder = render_svg(&scene).unwrap();
    assert!(cylinder.contains(
        "data-node-background=\"true\" d=\"M 10 17 C 10 11 170 11 170 17 V 55 C 170 61 10 61 10 55 Z\""
    ));
}

#[test]
fn renders_group_backgrounds_and_titles_around_the_node_layer() {
    let mut scene = make_scene("Application", node());
    scene.groups.push(group());

    let svg = render_svg(&scene).unwrap();
    let background = svg.find("<rect data-group-id=\"application\"").unwrap();
    let label = svg.find("<text data-group-id=\"application\"").unwrap();
    let node = svg.find("<g data-node-id=\"api\"").unwrap();

    assert!(background < label && label < node);
    assert!(svg.contains(
		"x=\"4\" y=\"5\" width=\"172\" height=\"62\" rx=\"18\" fill=\"#fafafa\" stroke=\"#d4d4d8\" stroke-width=\"1.5\" stroke-dasharray=\"5 5\""
	));
    assert!(svg.contains("x=\"20\" y=\"29\" font-family=\"'Inter Variable', sans-serif\""));
    assert!(svg.contains("Application</tspan>"));
}

#[test]
fn applies_group_variant_colors_and_opacity() {
    let mut scene = make_scene("Application", node());
    let mut group = group();
    group.style.variant = SceneVariant::Accent;
    group.style.radius = Some(12.0);
    group.style.opacity = Some(0.5);
    scene.groups.push(group);

    let svg = render_svg(&scene).unwrap();

    assert!(svg.contains("rx=\"12\" fill=\"#eff6ff\" stroke=\"#2563eb\""));
    assert!(svg.contains("stroke-dasharray=\"5 5\" opacity=\"0.5\""));
    assert!(svg.contains("fill=\"#1d4ed8\" stroke=\"#ffffff\""));
}

#[test]
fn escapes_group_identifiers_and_titles() {
    let mut scene = make_scene("Application", node());
    let mut group = group();
    group.name = "app&<one>".into();
    group.title = "API & <services>".into();
    scene.groups.push(group);

    let svg = render_svg(&scene).unwrap();

    assert!(svg.contains("data-group-id=\"app&amp;&lt;one&gt;\""));
    assert!(svg.contains("API &amp; &lt;services&gt;"));
}

#[test]
fn rejects_invalid_group_geometry_and_style() {
    let mut scene = make_scene("Application", node());
    let mut invalid_bounds = group();
    invalid_bounds.bounds.width = f32::INFINITY;
    scene.groups.push(invalid_bounds);
    assert_eq!(render_svg(&scene), Err(SvgRenderError::InvalidGroupBounds));

    let mut scene = make_scene("Application", node());
    let mut invalid_style = group();
    invalid_style.style.opacity = Some(f32::NAN);
    scene.groups.push(invalid_style);
    assert_eq!(render_svg(&scene), Err(SvgRenderError::InvalidGroupStyle));
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
