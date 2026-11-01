use schemra_engine::{SceneTimelineSampler, compile_timeline, layout_visual, render_svg};
use schemra_ir::{
    ActionIr, Direction, EdgeDirectionality, EdgeIr, GroupId, GroupIr, GroupLayout, NodeId, NodeIr,
    NodeLayout, NodeTextAlign, ScenarioId, ScenarioIr, SourceLocation, SourceSpan, TokenId,
    VisualId, VisualIr,
};
use schemra_scene::SceneRouteStatus;

fn span() -> SourceSpan {
    SourceSpan::point(SourceLocation {
        offset: 0,
        line: 1,
        column: 1,
    })
}

fn visual(direction: Direction) -> VisualIr {
    VisualIr {
        id: VisualId("routing".into()),
        id_span: span(),
        title: "Routing".into(),
        direction,
        direction_explicit: false,
        groups: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        span: span(),
    }
}

fn node(id: &str, group: Option<&str>) -> NodeIr {
    NodeIr {
        id: NodeId(id.into()),
        id_span: span(),
        label: id.into(),
        icon: None,
        detail: None,
        layout: NodeLayout::default(),
        text_align: NodeTextAlign::default(),
        group: group.map(|id| GroupId(id.into())),
        span: span(),
    }
}

fn edge(from: &str, to: &str) -> EdgeIr {
    EdgeIr {
        from: NodeId(from.into()),
        from_span: span(),
        to: NodeId(to.into()),
        to_span: span(),
        directionality: EdgeDirectionality::Directed,
        span: span(),
    }
}

fn undirected_edge(from: &str, to: &str) -> EdgeIr {
    EdgeIr {
        from: NodeId(from.into()),
        from_span: span(),
        to: NodeId(to.into()),
        to_span: span(),
        directionality: EdgeDirectionality::Undirected,
        span: span(),
    }
}

#[test]
fn direct_route_is_one_cubic_with_endpoints_on_selected_node_faces() {
    for (direction, source_side, target_side) in [
        (Direction::LeftToRight, "right", "left"),
        (Direction::RightToLeft, "left", "right"),
        (Direction::TopToBottom, "bottom", "top"),
        (Direction::BottomToTop, "top", "bottom"),
    ] {
        let mut model = visual(direction);
        model.nodes = vec![node("client", None), node("proxy", None)];
        model.edges.push(edge("client", "proxy"));
        let scene = layout_visual(&model);

        let link = &scene.edges[0];
        let client = scene.nodes.iter().find(|node| node.id == "client").unwrap();
        let proxy = scene.nodes.iter().find(|node| node.id == "proxy").unwrap();
        assert_eq!(link.path.len(), 1);
        assert!(is_straight(&link.path[0]));
        assert_eq!(
            link.path[0].start,
            centered_port(client.bounds, source_side)
        );
        assert_eq!(link.path[0].end, centered_port(proxy.bounds, target_side));
        let diagnostic = link.route_diagnostic.as_ref().unwrap();
        assert_eq!(diagnostic.status, SceneRouteStatus::Routed);
        assert_eq!(diagnostic.source_port, source_side);
        assert_eq!(diagnostic.target_port, target_side);
        assert_eq!(diagnostic.selected_routing, "straight");
        let chord = (link.path[0].end.x - link.path[0].start.x)
            .hypot(link.path[0].end.y - link.path[0].start.y);
        assert!((diagnostic.route_length - chord).abs() < 0.01);
    }
}

#[test]
fn direct_route_diagnostics_report_an_intermediate_node_obstruction() {
    let mut model = visual(Direction::LeftToRight);
    model.nodes = vec![
        node("source", None),
        node("other-source", None),
        node("obstacle", None),
        node("target", None),
    ];
    model.edges = vec![
        edge("source", "target"),
        edge("other-source", "obstacle"),
        edge("obstacle", "target"),
    ];

    let scene = layout_visual(&model);
    let route = scene
        .edges
        .iter()
        .find(|edge| edge.from == "source" && edge.to == "target")
        .unwrap();
    let diagnostic = route.route_diagnostic.as_ref().unwrap();

    assert_eq!(diagnostic.obstacle_intersections, 1);
    assert!(!diagnostic.direct_candidate_clear);
    assert!(diagnostic.candidates_considered > 1);
    assert!(diagnostic.selection_reason.contains("least obstructed"));
}

#[test]
fn grid_fans_share_one_clear_target_port() {
    let mut model = visual(Direction::LeftToRight);
    model.groups.push(GroupIr {
        id: GroupId("clients".into()),
        id_span: span(),
        title: "Clients".into(),
        layout: GroupLayout::Grid { columns: Some(3) },
        span: span(),
    });
    model.nodes = (0..6)
        .map(|index| node(&format!("client-{index}"), Some("clients")))
        .chain(std::iter::once(node("proxy", None)))
        .collect();
    model.edges = (0..6)
        .map(|index| edge(&format!("client-{index}"), "proxy"))
        .collect();

    let scene = layout_visual(&model);
    let proxy = scene.nodes.iter().find(|node| node.id == "proxy").unwrap();
    let expected_target = (proxy.bounds.x, proxy.bounds.y + proxy.bounds.height * 0.5);
    let mut target_points = scene
        .edges
        .iter()
        .map(|edge| {
            let diagnostic = edge.route_diagnostic.as_ref().unwrap();
            assert_eq!(diagnostic.status, SceneRouteStatus::Routed);
            assert_eq!(diagnostic.target_port, "left");
            assert_eq!(edge.path.len(), 1);
            assert!(is_straight(&edge.path[0]));
            assert_eq!((edge.path[0].end.x, edge.path[0].end.y), expected_target);
            (edge.path[0].end.y, edge.path[0].end.x)
        })
        .collect::<Vec<_>>();
    target_points.sort_by(|left, right| left.0.total_cmp(&right.0));
    target_points.dedup_by(|left, right| {
        (left.0 - right.0).abs() < f32::EPSILON && (left.1 - right.1).abs() < f32::EPSILON
    });
    assert_eq!(
        target_points.len(),
        1,
        "fan links should share one target port"
    );

    let conflicts = scene
        .edges
        .iter()
        .filter_map(|edge| {
            let count = edge.route_diagnostic.as_ref()?.route_conflicts;
            (count > 0).then_some((&edge.from, &edge.to, count))
        })
        .collect::<Vec<_>>();
    assert!(
        conflicts.is_empty(),
        "the six-link grid fan should stay separated: {conflicts:?}"
    );
}

#[test]
fn mixed_direction_fan_uses_the_natural_centered_face_for_each_edge() {
    let mut model = visual(Direction::LeftToRight);
    model.groups.push(GroupIr {
        id: GroupId("data".into()),
        id_span: span(),
        title: "Data".into(),
        layout: GroupLayout::Grid { columns: Some(1) },
        span: span(),
    });
    model.nodes = vec![node("backend", None)]
        .into_iter()
        .chain([
            node("audit_worker", Some("data")),
            node("database", Some("data")),
        ])
        .collect();
    model.edges = vec![
        edge("backend", "database"),
        edge("audit_worker", "database"),
    ];

    let scene = layout_visual(&model);
    let backend_link = scene
        .edges
        .iter()
        .find(|edge| edge.from == "backend")
        .unwrap();
    let audit_link = scene
        .edges
        .iter()
        .find(|edge| edge.from == "audit_worker")
        .unwrap();
    let audit = scene
        .nodes
        .iter()
        .find(|node| node.id == "audit_worker")
        .unwrap();
    let database = scene
        .nodes
        .iter()
        .find(|node| node.id == "database")
        .unwrap();

    assert_eq!(
        backend_link.route_diagnostic.as_ref().unwrap().target_port,
        "left",
    );
    assert_eq!(backend_link.path[0].end.x, database.bounds.x);
    assert_eq!(
        backend_link.path[0].end.y,
        database.bounds.y + database.bounds.height * 0.5
    );
    assert_eq!(
        audit_link.route_diagnostic.as_ref().unwrap().source_port,
        "bottom"
    );
    assert_eq!(
        audit_link.route_diagnostic.as_ref().unwrap().target_port,
        "top"
    );
    assert_eq!(
        audit_link.path[0].start,
        schemra_scene::Point {
            x: audit.bounds.x + audit.bounds.width * 0.5,
            y: audit.bounds.y + audit.bounds.height,
        }
    );
    assert_eq!(
        audit_link.path[0].end,
        schemra_scene::Point {
            x: database.bounds.x + database.bounds.width * 0.5,
            y: database.bounds.y,
        }
    );
    assert_ne!(backend_link.path[0].end, audit_link.path[0].end);
}

#[test]
fn outgoing_links_from_one_node_share_its_single_face_port() {
    let mut model = visual(Direction::LeftToRight);
    model.groups.push(GroupIr {
        id: GroupId("services".into()),
        id_span: span(),
        title: "Services".into(),
        layout: GroupLayout::Grid { columns: Some(1) },
        span: span(),
    });
    model.nodes = vec![node("proxy", None)]
        .into_iter()
        .chain((0..3).map(|index| node(&format!("service-{index}"), Some("services"))))
        .collect();
    model.edges = (0..3)
        .map(|index| edge("proxy", &format!("service-{index}")))
        .collect();

    let scene = layout_visual(&model);
    let proxy = scene.nodes.iter().find(|node| node.id == "proxy").unwrap();
    let expected_port = schemra_scene::Point {
        x: proxy.bounds.x + proxy.bounds.width,
        y: proxy.bounds.y + proxy.bounds.height * 0.5,
    };
    assert!(scene.edges.iter().all(|edge| {
        edge.route_diagnostic.as_ref().unwrap().source_port == "right"
            && edge.path[0].start == expected_port
    }));
}

#[test]
fn grid_fans_use_centered_ports_across_group_sizes_and_column_counts() {
    for count in 1..=12 {
        for columns in 1..=count {
            let mut model = visual(Direction::LeftToRight);
            model.groups.push(GroupIr {
                id: GroupId("clients".into()),
                id_span: span(),
                title: "Clients".into(),
                layout: GroupLayout::Grid {
                    columns: Some(columns as u32),
                },
                span: span(),
            });
            model.nodes = (0..count)
                .map(|index| node(&format!("client-{index}"), Some("clients")))
                .chain(std::iter::once(node("proxy", None)))
                .collect();
            model.edges = (0..count)
                .map(|index| edge(&format!("client-{index}"), "proxy"))
                .collect();

            let scene = layout_visual(&model);
            let proxy = scene.nodes.iter().find(|node| node.id == "proxy").unwrap();
            for link in &scene.edges {
                let diagnostic = link.route_diagnostic.as_ref().unwrap();
                let source = scene
                    .nodes
                    .iter()
                    .find(|node| node.id == link.from)
                    .unwrap();
                assert_eq!(diagnostic.status, SceneRouteStatus::Routed);
                assert_eq!(diagnostic.target_port, "left");
                assert_eq!(link.path.len(), 1);
                assert!(is_straight(&link.path[0]));
                assert_eq!(
                    link.path[0].end,
                    schemra_scene::Point {
                        x: proxy.bounds.x,
                        y: proxy.bounds.y + proxy.bounds.height * 0.5,
                    },
                    "fan links must share the one centered target port for {count} nodes in {columns} columns"
                );
                assert_eq!(
                    link.path[0].start,
                    centered_port(source.bounds, diagnostic.source_port.as_str()),
                    "source ports must be centered for {count} nodes in {columns} columns"
                );
            }
        }
    }
}

#[test]
fn fan_routes_are_independent_of_edge_declaration_order() {
    let mut model = visual(Direction::LeftToRight);
    model.groups.push(GroupIr {
        id: GroupId("clients".into()),
        id_span: span(),
        title: "Clients".into(),
        layout: GroupLayout::Grid { columns: Some(3) },
        span: span(),
    });
    model.nodes = (0..6)
        .map(|index| node(&format!("client-{index}"), Some("clients")))
        .chain(std::iter::once(node("proxy", None)))
        .collect();
    model.edges = (0..6)
        .map(|index| edge(&format!("client-{index}"), "proxy"))
        .collect();

    let initial = layout_visual(&model);
    model.edges.reverse();
    let reordered = layout_visual(&model);

    for link in &initial.edges {
        let same_link = reordered
            .edges
            .iter()
            .find(|candidate| candidate.from == link.from && candidate.to == link.to)
            .expect("the reordered model retains every link");
        assert_eq!(
            link.path, same_link.path,
            "route for {} is stable",
            link.from
        );
    }
}

#[test]
fn six_client_service_diagram_keeps_shared_ports_and_reciprocal_routes_stable() {
    let mut model = visual(Direction::LeftToRight);
    model.groups = vec![
        GroupIr {
            id: GroupId("clients".into()),
            id_span: span(),
            title: "Clients".into(),
            layout: GroupLayout::Grid { columns: Some(3) },
            span: span(),
        },
        GroupIr {
            id: GroupId("application".into()),
            id_span: span(),
            title: "Application".into(),
            layout: GroupLayout::Auto,
            span: span(),
        },
        GroupIr {
            id: GroupId("data".into()),
            id_span: span(),
            title: "Data".into(),
            layout: GroupLayout::Auto,
            span: span(),
        },
    ];
    model.nodes = (1..=6)
        .map(|index| node(&format!("bot_{index}"), Some("clients")))
        .chain([
            node("reverse_proxy", Some("application")),
            node("backend", Some("application")),
            node("database", Some("data")),
        ])
        .collect();
    model.edges = (1..=6)
        .map(|index| undirected_edge(&format!("bot_{index}"), "reverse_proxy"))
        .chain([
            edge("reverse_proxy", "backend"),
            edge("backend", "reverse_proxy"),
            edge("backend", "database"),
            edge("database", "backend"),
        ])
        .collect();

    let first = layout_visual(&model);
    let repeated = layout_visual(&model);
    assert_eq!(first, repeated);

    let bot_routes = first
        .edges
        .iter()
        .filter(|edge| edge.from.starts_with("bot_") && edge.to == "reverse_proxy")
        .collect::<Vec<_>>();
    assert_eq!(first.edges.len(), 10);
    assert_eq!(bot_routes.len(), 6);
    assert!(
        bot_routes.iter().all(|route| {
            let diagnostic = route.route_diagnostic.as_ref().unwrap();
            diagnostic.status == SceneRouteStatus::Routed
                && diagnostic.target_port == "left"
                && diagnostic.route_conflicts == 0
                && route.path.len() == 1
                && is_straight(&route.path[0])
        }),
        "bot fan paths should retain the shared target face and direct geometry: {bot_routes:#?}"
    );
    for (upper, lower) in [("bot_1", "bot_4"), ("bot_2", "bot_5"), ("bot_3", "bot_6")] {
        let route_length = |source: &str| {
            bot_routes
                .iter()
                .find(|route| route.from == source)
                .unwrap()
                .route_diagnostic
                .as_ref()
                .unwrap()
                .route_length
        };
        assert!(
            (route_length(upper) - route_length(lower)).abs() < 36.0,
            "mirrored clients should have comparable route lengths: {upper} / {lower}"
        );
    }
    for (from, to) in [
        ("reverse_proxy", "backend"),
        ("backend", "reverse_proxy"),
        ("backend", "database"),
        ("database", "backend"),
    ] {
        let route = first
            .edges
            .iter()
            .find(|route| route.from == from && route.to == to)
            .unwrap();
        let reverse = &first.edges[route.reciprocal_edge_index.unwrap()];
        assert_eq!(reverse.from, to);
        assert_eq!(reverse.to, from);
        assert_eq!(route.path[0].start, reverse.path[0].end);
        assert_eq!(route.path[0].control1, reverse.path[0].control2);
        assert_eq!(route.path[0].control2, reverse.path[0].control1);
        assert_eq!(route.path[0].end, reverse.path[0].start);
    }
}

#[test]
fn one_hundred_node_fan_in_is_deterministic_and_keeps_one_centered_target_port() {
    const SOURCE_COUNT: usize = 100;
    let mut model = visual(Direction::LeftToRight);
    model.groups.push(GroupIr {
        id: GroupId("clients".into()),
        id_span: span(),
        title: "Clients".into(),
        layout: GroupLayout::Grid { columns: Some(10) },
        span: span(),
    });
    model.nodes = (0..SOURCE_COUNT)
        .map(|index| node(&format!("client_{index}"), Some("clients")))
        .chain(std::iter::once(node("collector", None)))
        .collect();
    model.edges = (0..SOURCE_COUNT)
        .map(|index| edge(&format!("client_{index}"), "collector"))
        .collect();

    let first = layout_visual(&model);
    let second = layout_visual(&model);

    assert_eq!(first.edges.len(), SOURCE_COUNT);
    assert_eq!(first.edges, second.edges);
    assert!(first.edges.iter().all(|edge| {
        edge.path.len() == 1
            && is_straight(&edge.path[0])
            && edge.route_diagnostic.as_ref().is_some_and(|diagnostic| {
                diagnostic.status == SceneRouteStatus::Routed
                    && diagnostic.target_port == "left"
                    && diagnostic.route_conflicts == 0
            })
    }));
    let target_points = first
        .edges
        .iter()
        .map(|edge| (edge.path[0].end.x, edge.path[0].end.y))
        .collect::<Vec<_>>();
    assert!(target_points.iter().all(|point| *point == target_points[0]));
}

#[test]
fn reciprocal_links_share_and_reverse_one_canonical_route() {
    let mut model = visual(Direction::LeftToRight);
    model.nodes = vec![node("client", None), node("proxy", None)];
    model.edges = vec![edge("client", "proxy"), edge("proxy", "client")];
    let scene = layout_visual(&model);

    assert_eq!(scene.edges[0].reciprocal_edge_index, Some(1));
    assert_eq!(scene.edges[1].reciprocal_edge_index, Some(0));
    let forward = scene.edges[0].path[0];
    let reverse = scene.edges[1].path[0];
    assert_eq!(forward.start, reverse.end);
    assert_eq!(forward.control1, reverse.control2);
    assert_eq!(forward.control2, reverse.control1);
    assert_eq!(forward.end, reverse.start);
}

#[test]
fn self_link_is_a_visible_loop_with_canvas_padding() {
    let mut model = visual(Direction::LeftToRight);
    model.nodes.push(node("worker", None));
    model.edges.push(edge("worker", "worker"));
    let scene = layout_visual(&model);
    let link = &scene.edges[0];

    assert_eq!(link.path.len(), 3);
    assert_eq!(
        link.path.first().unwrap().start,
        link.path.last().unwrap().end
    );
    assert!(link.path.iter().all(|segment| {
        [
            segment.start,
            segment.control1,
            segment.control2,
            segment.end,
        ]
        .into_iter()
        .all(|point| point.x.is_finite() && point.y.is_finite())
    }));
    let diagnostic = link.route_diagnostic.as_ref().unwrap();
    assert_eq!(diagnostic.status, SceneRouteStatus::Routed);
    assert_eq!(diagnostic.source_port, "right");
    assert_eq!(diagnostic.target_port, "right");
    assert!(diagnostic.route_length > 0.0);

    for segment in &link.path {
        for step in 0..=20 {
            let point = evaluate(*segment, step as f32 / 20.0);
            assert!(point.x >= 48.0 - 0.01 && point.y >= 48.0 - 0.01);
            assert!(point.x <= scene.width - 48.0 + 0.01);
            assert!(point.y <= scene.height - 48.0 + 0.01);
        }
    }

    let svg = render_svg(&scene).unwrap();
    assert!(svg.contains("data-from=\"worker\" data-to=\"worker\""));
    assert!(svg.contains("data-arrow-start=\"false\" data-arrow-end=\"true\""));
    assert!(svg.contains("class=\"schemra-edge-path\""));
}

#[test]
fn self_link_playback_follows_the_loop_and_consumes_the_marker_at_arrival() {
    let mut model = visual(Direction::LeftToRight);
    model.nodes.push(node("worker", None));
    model.edges.push(edge("worker", "worker"));
    let scene = layout_visual(&model);
    let span = span();
    let timeline = compile_timeline(&ScenarioIr {
        id: ScenarioId("self-loop".into()),
        id_span: span,
        visual: VisualId("routing".into()),
        visual_span: span,
        tokens: Vec::new(),
        actions: vec![ActionIr::Move {
            token: TokenId("request".into()),
            token_span: span,
            from: NodeId("worker".into()),
            from_span: span,
            to: NodeId("worker".into()),
            to_span: span,
            duration_ms: Some(1_000),
            span,
        }],
        span,
    })
    .unwrap();
    let sampler = SceneTimelineSampler::new(&scene, &timeline);
    let marker = sampler.sample(500).tokens;
    let start = scene.edges[0].path[0].start;

    assert_eq!(marker.len(), 1);
    assert!((marker[0].x - start.x).hypot(marker[0].y - start.y) > 1.0);
    assert!(sampler.sample(1_000).tokens.is_empty());
}

fn evaluate(segment: schemra_scene::CubicBezier, t: f32) -> schemra_scene::Point {
    let inverse = 1.0 - t;
    schemra_scene::Point {
        x: inverse.powi(3) * segment.start.x
            + 3.0 * inverse.powi(2) * t * segment.control1.x
            + 3.0 * inverse * t.powi(2) * segment.control2.x
            + t.powi(3) * segment.end.x,
        y: inverse.powi(3) * segment.start.y
            + 3.0 * inverse.powi(2) * t * segment.control1.y
            + 3.0 * inverse * t.powi(2) * segment.control2.y
            + t.powi(3) * segment.end.y,
    }
}

fn is_straight(segment: &schemra_scene::CubicBezier) -> bool {
    let dx = segment.end.x - segment.start.x;
    let dy = segment.end.y - segment.start.y;
    let chord = dx.hypot(dy);
    chord > f32::EPSILON
        && [segment.control1, segment.control2]
            .into_iter()
            .all(|point| {
                let cross = (point.x - segment.start.x) * dy - (point.y - segment.start.y) * dx;
                cross.abs() / chord < 0.001
            })
}

fn centered_port(bounds: schemra_scene::Rect, side: &str) -> schemra_scene::Point {
    use schemra_scene::Point;

    match side {
        "left" => Point {
            x: bounds.x,
            y: bounds.y + bounds.height * 0.5,
        },
        "right" => Point {
            x: bounds.x + bounds.width,
            y: bounds.y + bounds.height * 0.5,
        },
        "top" => Point {
            x: bounds.x + bounds.width * 0.5,
            y: bounds.y,
        },
        "bottom" => Point {
            x: bounds.x + bounds.width * 0.5,
            y: bounds.y + bounds.height,
        },
        unexpected => panic!("unexpected port side: {unexpected}"),
    }
}
