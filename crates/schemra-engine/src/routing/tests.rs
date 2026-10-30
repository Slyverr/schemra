use super::geometry::{direct_connector, natural_sides, paths_conflict, side_anchor};
use super::{RoutingMode, route_edges_with_mode};
use schemra_scene::{
    FlowDirection, Rect, SceneEdge, SceneNode, SceneNodeLayout, SceneNodeShape, SceneStyle,
    SceneTextAlign,
};

fn node(id: &str, bounds: Rect) -> SceneNode {
    SceneNode {
        id: id.into(),
        label: id.into(),
        icon: None,
        detail: None,
        layout: SceneNodeLayout::Horizontal,
        text_align: SceneTextAlign::Left,
        shape: SceneNodeShape::Round,
        style: SceneStyle::default(),
        bounds,
        group: None,
    }
}

fn edge(from: &str, to: &str) -> SceneEdge {
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
        path: Vec::new(),
        route_diagnostic: None,
    }
}

#[test]
fn auto_uses_a_clear_detour_when_the_direct_connector_hits_a_node() {
    let nodes = [
        node(
            "source",
            Rect {
                x: 0.0,
                y: 40.0,
                width: 20.0,
                height: 20.0,
            },
        ),
        node(
            "obstacle",
            Rect {
                x: 45.0,
                y: 10.0,
                width: 25.0,
                height: 80.0,
            },
        ),
        node(
            "target",
            Rect {
                x: 120.0,
                y: 40.0,
                width: 20.0,
                height: 20.0,
            },
        ),
    ];
    let mut edges = [edge("source", "target")];

    route_edges_with_mode(
        &mut edges,
        &nodes,
        FlowDirection::LeftToRight,
        RoutingMode::Auto,
    );

    let route = &edges[0].path;
    let diagnostic = edges[0].route_diagnostic.as_ref().unwrap();
    assert!(
        route.len() >= 3,
        "the obstacle should force a path around it: {route:?}; {diagnostic:?}"
    );
    assert_eq!(diagnostic.selected_routing, "auto");
    assert!(!diagnostic.direct_candidate_clear);
    assert_eq!(diagnostic.obstacle_intersections, 0);
    assert!(diagnostic.route_length > diagnostic.geometric_lower_bound);
    assert_eq!(route.first().unwrap().start.x, 20.0);
    assert_eq!(route.last().unwrap().end.x, 120.0);
    assert!(route.iter().all(|segment| {
        (segment.start.x - segment.end.x).abs() <= f32::EPSILON
            || (segment.start.y - segment.end.y).abs() <= f32::EPSILON
    }));
}

#[test]
fn straight_mode_keeps_one_direct_path_when_all_ports_are_obstructed() {
    let nodes = [
        node(
            "source",
            Rect {
                x: 0.0,
                y: 40.0,
                width: 20.0,
                height: 20.0,
            },
        ),
        node(
            "obstacle",
            Rect {
                x: 45.0,
                y: 10.0,
                width: 25.0,
                height: 80.0,
            },
        ),
        node(
            "target",
            Rect {
                x: 120.0,
                y: 40.0,
                width: 20.0,
                height: 20.0,
            },
        ),
    ];
    let mut edges = [edge("source", "target")];

    route_edges_with_mode(
        &mut edges,
        &nodes,
        FlowDirection::LeftToRight,
        RoutingMode::Straight,
    );

    let diagnostic = edges[0].route_diagnostic.as_ref().unwrap();
    assert_eq!(edges[0].path.len(), 1);
    assert_eq!(diagnostic.selected_routing, "straight");
    assert_eq!(diagnostic.obstacle_intersections, 1);
}

#[test]
fn straight_mode_selects_an_alternate_port_to_avoid_an_intermediate_node() {
    let nodes = [
        node(
            "source",
            Rect {
                x: 0.0,
                y: 40.0,
                width: 20.0,
                height: 20.0,
            },
        ),
        node(
            "obstacle",
            Rect {
                x: 45.0,
                y: 48.0,
                width: 25.0,
                height: 4.0,
            },
        ),
        node(
            "target",
            Rect {
                x: 120.0,
                y: 40.0,
                width: 20.0,
                height: 20.0,
            },
        ),
    ];
    let mut edges = [edge("source", "target")];

    route_edges_with_mode(
        &mut edges,
        &nodes,
        FlowDirection::LeftToRight,
        RoutingMode::Straight,
    );

    let diagnostic = edges[0].route_diagnostic.as_ref().unwrap();
    assert_eq!(edges[0].path.len(), 1);
    assert_eq!(diagnostic.source_port, "top");
    assert_eq!(diagnostic.target_port, "left");
    assert_eq!(diagnostic.obstacle_intersections, 0);
    assert!(!diagnostic.direct_candidate_clear);
}

#[test]
fn straight_mode_reassigns_ports_to_avoid_an_avoidable_route_crossing() {
    let bounds = [
        Rect {
            x: 119.9,
            y: 141.2,
            width: 24.0,
            height: 18.0,
        },
        Rect {
            x: 190.1,
            y: 83.2,
            width: 24.0,
            height: 18.0,
        },
        Rect {
            x: 145.8,
            y: 138.9,
            width: 24.0,
            height: 18.0,
        },
        Rect {
            x: 78.4,
            y: 185.2,
            width: 24.0,
            height: 18.0,
        },
    ];
    let nodes = [
        node("source-a", bounds[0]),
        node("target-a", bounds[1]),
        node("source-b", bounds[2]),
        node("target-b", bounds[3]),
    ];
    let direction = FlowDirection::LeftToRight;
    let first_sides = natural_sides(bounds[0], bounds[1], direction);
    let second_sides = natural_sides(bounds[2], bounds[3], direction);
    let first_natural = [direct_connector(
        side_anchor(bounds[0], first_sides.0),
        side_anchor(bounds[1], first_sides.1),
    )];
    let second_natural = [direct_connector(
        side_anchor(bounds[2], second_sides.0),
        side_anchor(bounds[3], second_sides.1),
    )];
    assert!(paths_conflict(&first_natural, &second_natural));

    let mut edges = [edge("source-a", "target-a"), edge("source-b", "target-b")];
    route_edges_with_mode(&mut edges, &nodes, direction, RoutingMode::Straight);
    let mut reversed_edges = [edge("source-b", "target-b"), edge("source-a", "target-a")];
    route_edges_with_mode(
        &mut reversed_edges,
        &nodes,
        direction,
        RoutingMode::Straight,
    );

    assert!(edges.iter().all(|edge| edge.path.len() == 1));
    assert!(edges.iter().all(|edge| {
        edge.route_diagnostic.as_ref().is_some_and(|diagnostic| {
            diagnostic.obstacle_intersections == 0 && diagnostic.route_conflicts == 0
        })
    }));
    for routed in &edges {
        let reversed = reversed_edges
            .iter()
            .find(|candidate| candidate.from == routed.from && candidate.to == routed.to)
            .expect("reversed input should contain the same link");
        assert_eq!(routed.path, reversed.path);
        assert_eq!(routed.route_diagnostic, reversed.route_diagnostic);
    }
    assert!(
        edges
            .iter()
            .zip([first_sides, second_sides])
            .any(|(edge, (source, target))| {
                edge.route_diagnostic.as_ref().is_some_and(|diagnostic| {
                    diagnostic.source_port != source.name()
                        || diagnostic.target_port != target.name()
                })
            }),
        "at least one route should change its natural port pair to remove the crossing"
    );
}
