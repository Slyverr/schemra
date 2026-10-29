use schemra_scene::{
    CubicBezier, FlowDirection, Point, Scene, SceneEdge, SceneRouteDiagnostic, SceneRouteStatus,
    SceneStyle, SceneTheme, is_safe_image_source,
};

#[test]
fn scene_geometry_and_link_semantics_round_trip() {
    let scene = Scene {
        title: "Request flow".into(),
        direction: FlowDirection::LeftToRight,
        theme: SceneTheme::default(),
        width: 480.0,
        height: 240.0,
        groups: Vec::new(),
        nodes: Vec::new(),
        texts: Vec::new(),
        images: Vec::new(),
        icons: Vec::new(),
        edges: vec![SceneEdge {
            id: Some("request-link".into()),
            source_id: Some("request-link".into()),
            reciprocal_edge_index: None,
            from: "client".into(),
            to: "proxy".into(),
            arrow_at_start: false,
            arrow_at_end: true,
            label: Some("request".into()),
            style: SceneStyle::default(),
            group: None,
            path: vec![CubicBezier {
                start: Point { x: 20.0, y: 40.0 },
                control1: Point { x: 80.0, y: 40.0 },
                control2: Point { x: 120.0, y: 100.0 },
                end: Point { x: 180.0, y: 100.0 },
            }],
            route_diagnostic: None,
        }],
    };

    let encoded = serde_json::to_value(&scene).unwrap();
    let decoded: Scene = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, scene);
}

#[test]
fn routing_diagnostic_wire_format_is_stable() {
    let diagnostic = SceneRouteDiagnostic {
        status: SceneRouteStatus::Unroutable,
        source_port: "right".into(),
        target_port: "left".into(),
        candidates_considered: 4,
        rejected_for_obstacles: 2,
        rejected_for_route_conflicts: 1,
        obstacle_intersections: 2,
        route_conflicts: 1,
        candidate_rejections: vec!["node-overlap".into()],
        route_length: 80.0,
        geometric_lower_bound: 40.0,
        detour_ratio: 2.0,
        direct_candidate_clear: false,
        selected_routing: "straight".into(),
        selection_reason: "all candidates blocked".into(),
        message: Some("No clear route was found.".into()),
    };

    let value = serde_json::to_value(diagnostic).unwrap();
    assert_eq!(value["status"], "unroutable");
    assert_eq!(value["sourcePort"], "right");
    assert_eq!(value["candidateRejections"][0], "node-overlap");
    assert_eq!(value["selectedRouting"], "straight");
}

#[test]
fn image_sources_allow_local_raster_assets_and_reject_unsafe_paths() {
    assert!(is_safe_image_source("assets/brand.png"));
    assert!(is_safe_image_source("./images/photo.webp"));
    assert!(is_safe_image_source("data:image/png;base64,aGVsbG8="));

    for unsafe_source in [
        "https://example.test/brand.png",
        "../private.png",
        "/etc/passwd.png",
        "data:image/svg+xml;base64,PHN2Zz4=",
        "data:image/png;base64,not base64!",
        "data:image/png;base64,aGVsbG8===",
        "assets/brand.svg",
    ] {
        assert!(!is_safe_image_source(unsafe_source), "{unsafe_source}");
    }
}
