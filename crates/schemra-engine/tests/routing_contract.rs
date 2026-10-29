use schemra_engine::layout_visual;
use schemra_ir::{
    Direction, EdgeDirectionality, EdgeIr, GroupId, GroupIr, GroupLayout, NodeId, NodeIr,
    SourceLocation, SourceSpan, VisualId, VisualIr,
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
        assert_eq!(link.path.len(), 1);
        assert!(is_straight(&link.path[0]));
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
    let mut target_points = scene
        .edges
        .iter()
        .map(|edge| {
            let diagnostic = edge.route_diagnostic.as_ref().unwrap();
            assert_eq!(diagnostic.status, SceneRouteStatus::Routed);
            assert_eq!(diagnostic.target_port, "left");
            assert_eq!(edge.path.len(), 1);
            assert!(is_straight(&edge.path[0]));
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
fn grid_fans_remain_separated_across_group_sizes_and_column_counts() {
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
            let mut endpoints = scene
                .edges
                .iter()
                .map(|edge| (edge.path[0].end.x, edge.path[0].end.y))
                .collect::<Vec<_>>();
            endpoints
                .sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.total_cmp(&right.1)));
            endpoints.dedup_by(|left, right| {
                (left.0 - right.0).abs() < f32::EPSILON && (left.1 - right.1).abs() < f32::EPSILON
            });
            assert_eq!(
                endpoints.len(),
                1,
                "fan links should share one target port for {count} nodes in {columns} columns"
            );
            let conflicts = scene
                .edges
                .iter()
                .filter_map(|edge| {
                    let diagnostic = edge.route_diagnostic.as_ref()?;
                    (diagnostic.target_port != "left" || diagnostic.route_conflicts > 0).then_some(
                        (
                            &edge.from,
                            diagnostic.target_port.as_str(),
                            diagnostic.route_conflicts,
                        ),
                    )
                })
                .collect::<Vec<_>>();
            assert!(
                conflicts.is_empty(),
                "fan routes should stay conflict-free for {count} nodes in {columns} columns: {conflicts:?}"
            );
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
fn self_link_does_not_create_a_zero_length_dot() {
    let mut model = visual(Direction::LeftToRight);
    model.nodes.push(node("worker", None));
    model.edges.push(edge("worker", "worker"));
    let scene = layout_visual(&model);

    assert!(scene.edges[0].path.is_empty());
    assert_eq!(
        scene.edges[0].route_diagnostic.as_ref().unwrap().status,
        SceneRouteStatus::Unroutable
    );
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
