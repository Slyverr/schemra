use schemra_renderer::render_svg;
use schemra_scene::{
    FlowDirection, Rect, Scene, SceneIconPack, SceneNode, SceneNodeLayout, SceneNodeShape,
    SceneStyle, SceneTextAlign, SceneTheme, SceneThemeVariant, SceneVariant,
};

fn scene(node: SceneNode) -> Scene {
    Scene {
        title: "Node rendering".into(),
        direction: FlowDirection::LeftToRight,
        theme: SceneTheme::default(),
        width: 300.0,
        height: 180.0,
        groups: Vec::new(),
        nodes: vec![node],
        texts: Vec::new(),
        images: Vec::new(),
        icons: Vec::new(),
        edges: Vec::new(),
    }
}

fn node(icon: Option<&str>) -> SceneNode {
    SceneNode {
        id: "service".into(),
        label: "Service".into(),
        icon: icon.map(str::to_owned),
        detail: None,
        layout: SceneNodeLayout::Horizontal,
        text_align: SceneTextAlign::Left,
        shape: SceneNodeShape::Round,
        style: SceneStyle::default(),
        bounds: Rect {
            x: 20.0,
            y: 20.0,
            width: 240.0,
            height: 80.0,
        },
        group: None,
    }
}

#[test]
fn renders_a_builtin_node_icon_with_the_selected_icon_pack() {
    let mut visual = scene(node(Some("lucide/server")));
    visual.theme.colors.icon_background = Some("#dbeafe".into());
    let outline = render_svg(&visual).unwrap();

    assert!(outline.contains("data-icon-pack=\"outline\""));
    assert!(outline.contains("data-node-icon=\"server\""));
    assert!(outline.contains("width=\"30\" height=\"30\" rx=\"9\" fill=\"#dbeafe\""));
    assert!(outline.contains("stroke=\"#52525b\" stroke-width=\"1.7\""));

    visual.theme.icon_pack = SceneIconPack::Solid;
    let solid = render_svg(&visual).unwrap();

    assert!(solid.contains("data-icon-pack=\"solid\""));
    assert!(solid.contains("fill=\"#52525b\""));
    assert!(
        solid.contains(
            "<rect x=\"5\" y=\"4\" width=\"22\" height=\"10\" rx=\"2\" fill=\"#ffffff\"/>"
        )
    );
}

#[test]
fn renders_custom_variant_colors_on_node_fill_stroke_and_text() {
    let mut visual = scene(node(None));
    visual.theme.variants.push(SceneThemeVariant {
        variant: SceneVariant::Accent,
        fill: Some("#eff6ff".into()),
        stroke: Some("#2563eb".into()),
        text: Some("#1d4ed8".into()),
    });
    visual.nodes[0].style.variant = SceneVariant::Accent;
    visual.nodes[0].detail = Some("Handles requests".into());
    let svg = render_svg(&visual).unwrap();

    assert!(svg.contains("data-node-variant=\"accent\""));
    assert!(svg.contains("fill=\"#eff6ff\" stroke=\"#2563eb\""));
    assert!(svg.contains("fill=\"#1d4ed8\""));
    assert!(svg.contains("data-node-text=\"detail\""));
    assert!(svg.contains("Handles requests"));
}

#[test]
fn node_detail_and_vertical_layout_follow_style_spacing_and_weight() {
    let mut visual = scene(node(Some("database")));
    visual.nodes[0].layout = SceneNodeLayout::Vertical;
    visual.nodes[0].text_align = SceneTextAlign::Center;
    visual.nodes[0].detail = Some("Stores shared data".into());
    visual.nodes[0].style.spacing = Some(16.0);
    visual.nodes[0].style.weight = Some(500);
    let svg = render_svg(&visual).unwrap();

    assert!(svg.contains("data-node-text=\"label\""));
    assert!(svg.contains("data-node-text=\"detail\""));
    assert!(svg.contains("text-anchor=\"middle\""));
    assert!(svg.contains("font-weight=\"500\""));
    assert!(svg.contains("Stores shared data"));
}

#[test]
fn custom_radius_is_clamped_to_the_node_and_unknown_icons_are_escaped() {
    let mut visual = scene(node(Some("</g><script>")));
    visual.nodes[0].style.radius = Some(500.0);
    let svg = render_svg(&visual).unwrap();

    assert!(svg.contains("rx=\"40\""));
    assert!(svg.contains("data-node-icon=\"&lt;/g&gt;&lt;script&gt;\""));
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("<circle cx=\"12\" cy=\"12\" r=\"8\"/>"));
}
