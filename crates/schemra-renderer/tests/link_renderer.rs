use schemra_renderer::{SvgRenderError, render_svg};
use schemra_scene::{
    CubicBezier, FlowDirection, Point, Rect, Scene, SceneEdge, SceneNode, SceneNodeLayout,
    SceneNodeShape, SceneStyle, SceneTextAlign, SceneTheme, SceneThemeVariant, SceneVariant,
};

fn make_scene() -> Scene {
    Scene {
        title: "Links".into(),
        direction: FlowDirection::LeftToRight,
        theme: SceneTheme::default(),
        width: 180.0,
        height: 72.0,
        groups: Vec::new(),
        nodes: vec![SceneNode {
            id: "node".into(),
            label: "Node".into(),
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
        }],
        texts: Vec::new(),
        images: Vec::new(),
        icons: Vec::new(),
        edges: Vec::new(),
    }
}

fn cubic(start: Point, control1: Point, control2: Point, end: Point) -> CubicBezier {
    CubicBezier {
        start,
        control1,
        control2,
        end,
    }
}

fn edge(from: &str, to: &str, path: Vec<CubicBezier>) -> SceneEdge {
    SceneEdge {
        id: None,
        source_id: None,
        reciprocal_edge_index: None,
        from: from.into(),
        to: to.into(),
        arrow_at_start: false,
        arrow_at_end: true,
        label: None,
        style: SceneStyle::default(),
        group: None,
        path,
        route_diagnostic: None,
    }
}

fn straight_cubic(start_x: f32, end_x: f32) -> CubicBezier {
    cubic(
        Point {
            x: start_x,
            y: 20.0,
        },
        Point {
            x: start_x + (end_x - start_x) / 3.0,
            y: 20.0,
        },
        Point {
            x: start_x + (end_x - start_x) * 2.0 / 3.0,
            y: 20.0,
        },
        Point { x: end_x, y: 20.0 },
    )
}

#[test]
fn writes_colored_paths_and_tangent_aligned_direction_cues_behind_nodes() {
    let mut scene = make_scene();
    scene
        .edges
        .push(edge("client", "server", vec![straight_cubic(0.0, 80.0)]));

    let svg = render_svg(&scene).unwrap();

    assert!(svg.contains("class=\"schemra-edge-path\""));
    assert!(svg.contains("stroke=\"#71717a\""));
    assert!(svg.contains("data-arrow-start=\"false\" data-arrow-end=\"true\""));
    assert!(!svg.contains("class=\"schemra-edge-direction-cue\""));
    assert_eq!(svg.matches("class=\"schemra-edge-path\"").count(), 1);
    assert!(svg.contains("M 36 23.5 L 44 20 L 36 16.5"));
    assert!(
        svg.find("class=\"schemra-edge-path\"").unwrap()
            < svg.find("data-node-id=\"node\"").unwrap(),
        "links must be serialized below nodes"
    );
}

#[test]
fn reciprocal_directions_share_one_path_with_cues_at_both_ends() {
    let mut scene = make_scene();
    let mut forward = edge("client", "server", vec![straight_cubic(10.0, 80.0)]);
    forward.reciprocal_edge_index = Some(1);
    let mut reverse = edge("server", "client", vec![straight_cubic(80.0, 10.0)]);
    reverse.reciprocal_edge_index = Some(0);
    scene.edges = vec![forward, reverse];

    let svg = render_svg(&scene).unwrap();

    assert_eq!(svg.matches("class=\"schemra-edge-path\"").count(), 1);
    assert!(svg.contains("data-edge-indices=\"0,1\""));
    assert!(svg.contains("data-arrow-start=\"true\" data-arrow-end=\"true\""));
    assert_eq!(
        path_attribute(&svg, "d").matches(" M ").count(),
        2,
        "both direction cues are subpaths of the canonical edge path"
    );
}

#[test]
fn direction_cue_tracks_the_curve_instead_of_its_endpoint_chord() {
    let mut scene = make_scene();
    scene.edges.push(edge(
        "client",
        "server",
        vec![cubic(
            Point { x: 0.0, y: 20.0 },
            Point { x: 30.0, y: 10.0 },
            Point { x: 50.0, y: 30.0 },
            Point { x: 80.0, y: 20.0 },
        )],
    ));

    let svg = render_svg(&scene).unwrap();
    let cue_data = path_attribute(&svg, "d").split(" M ").nth(1).unwrap();
    let ordinates = cue_data
        .split_whitespace()
        .filter_map(|part| part.parse::<f32>().ok())
        .skip(1)
        .step_by(2)
        .collect::<Vec<_>>();

    assert_eq!(ordinates.len(), 3);
    let wings_midpoint = (ordinates[0] + ordinates[2]) / 2.0;
    assert!((wings_midpoint - ordinates[1]).abs() > 0.2);
}

fn path_attribute<'a>(svg: &'a str, name: &str) -> &'a str {
    let path = svg.split("class=\"schemra-edge-path\"").nth(1).unwrap();
    let prefix = format!("{name}=\"");
    path.split_once(&prefix)
        .unwrap()
        .1
        .split_once('"')
        .unwrap()
        .0
}

#[test]
fn resolves_semantic_edge_color_and_escapes_edge_attributes() {
    let mut scene = make_scene();
    let mut edge = edge("from<&", "to\"node", vec![straight_cubic(0.0, 80.0)]);
    edge.id = Some("edge<&\"".into());
    edge.style.stroke = Some("#123456".into());
    edge.style.opacity = Some(0.35);
    scene.edges.push(edge);

    let svg = render_svg(&scene).unwrap();

    assert!(svg.contains("stroke=\"#123456\""));
    assert!(svg.contains("opacity=\"0.35\""));
    assert!(svg.contains("data-from=\"from&lt;&amp;\""));
    assert!(svg.contains("data-to=\"to&quot;node\""));
    assert!(svg.contains("data-edge-id=\"edge&lt;&amp;&quot;\""));
}

#[test]
fn applies_theme_variant_and_renders_escaped_edge_labels() {
    let mut scene = make_scene();
    scene.theme.variants.push(SceneThemeVariant {
        variant: SceneVariant::Accent,
        fill: None,
        stroke: Some("#2255aa".into()),
        text: Some("#123456".into()),
    });
    let mut edge = edge("client", "server", vec![straight_cubic(0.0, 80.0)]);
    edge.label = Some("request <read>".into());
    edge.style.variant = SceneVariant::Accent;
    edge.style.font = Some(schemra_scene::SceneFont::Serif);
    scene.edges.push(edge);

    let svg = render_svg(&scene).unwrap();

    assert!(svg.contains("stroke=\"#2255aa\""));
    assert!(svg.contains("data-edge-index=\"0\""));
    assert!(svg.contains("request &lt;read&gt;"));
    assert!(svg.contains("fill=\"#123456\""));
    assert!(svg.contains("paint-order=\"stroke\""));
    assert!(svg.contains("@font-face{font-family:&apos;Noto Serif&apos;"));
    assert!(svg.contains("font-family=\"'Noto Serif', serif\""));
}

#[test]
fn disconnected_segments_start_new_subpaths_instead_of_drawing_a_bridge() {
    let mut scene = make_scene();
    scene.edges.push(edge(
        "client",
        "server",
        vec![straight_cubic(0.0, 10.0), straight_cubic(30.0, 40.0)],
    ));

    let svg = render_svg(&scene).unwrap();
    let path = svg
        .split("class=\"schemra-edge-path\"")
        .nth(1)
        .unwrap()
        .split("/>")
        .next()
        .unwrap();

    assert!(path.contains("M 0 20 C"));
    assert!(path.contains("10 20 M 30 20 C"));
    let path_data = path
        .split("d=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let route_segments = path_data.split(" M ").take(2).collect::<Vec<_>>().join(" ");
    assert!(!route_segments.contains(" L "));
}

#[test]
fn empty_and_degenerate_paths_do_not_leave_stray_dots() {
    let mut scene = make_scene();
    scene.edges.push(edge("empty", "target", Vec::new()));
    scene.edges.push(edge(
        "point",
        "target",
        vec![cubic(
            Point { x: 4.0, y: 4.0 },
            Point { x: 4.0, y: 4.0 },
            Point { x: 4.0, y: 4.0 },
            Point { x: 4.0, y: 4.0 },
        )],
    ));

    let svg = render_svg(&scene).unwrap();

    assert!(!svg.contains("class=\"schemra-edge-path\""));
}

#[test]
fn invalid_edge_geometry_and_opacity_are_rejected() {
    let mut scene = make_scene();
    scene.edges.push(edge(
        "client",
        "server",
        vec![cubic(
            Point {
                x: f32::NAN,
                y: 0.0,
            },
            Point::default(),
            Point::default(),
            Point::default(),
        )],
    ));
    assert_eq!(render_svg(&scene), Err(SvgRenderError::InvalidEdgeGeometry));

    let mut scene = make_scene();
    let mut edge = edge("client", "server", vec![straight_cubic(0.0, 80.0)]);
    edge.style.opacity = Some(f32::INFINITY);
    scene.edges.push(edge);
    assert_eq!(render_svg(&scene), Err(SvgRenderError::InvalidEdgeStyle));
}
