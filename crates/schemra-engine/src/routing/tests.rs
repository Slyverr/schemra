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
