//! Straight link routing with shared faces and non-overlapping fan geometry.

use std::collections::{HashMap, HashSet};

use schemra_scene::{
    CubicBezier, FlowDirection, Rect, SceneEdge, SceneNode, SceneRouteDiagnostic, SceneRouteStatus,
};

mod geometry;
mod search;

pub(crate) use geometry::RouteGeometry;

use geometry::{Side, center, direct_connector, natural_sides, side_anchor};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum RoutingMode {
    Auto,
    #[default]
    Straight,
}

#[derive(Clone, Copy, Debug)]
struct EdgePlan {
    source_side: Side,
    target_side: Side,
}

/// Routes each edge as a straight connector between the four centered node ports.
pub(super) fn route_edges(edges: &mut [SceneEdge], nodes: &[SceneNode], direction: FlowDirection) {
    route_edges_with_mode(edges, nodes, direction, RoutingMode::Straight);
}

fn route_edges_with_mode(
    edges: &mut [SceneEdge],
    nodes: &[SceneNode],
    direction: FlowDirection,
    mode: RoutingMode,
) {
    for edge in edges.iter_mut() {
        edge.path.clear();
        edge.route_diagnostic = None;
        edge.reciprocal_edge_index = None;
    }
    if edges.is_empty() {
        return;
    }

    let node_bounds = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.bounds))
        .collect::<HashMap<_, _>>();
    let reciprocal_pairs = compatible_reciprocals(edges);
    let mut reverse_indices = HashSet::new();
    let mut reciprocal_for = HashMap::new();
    for (left, right) in reciprocal_pairs {
        let (canonical, reverse) =
            choose_reciprocal_orientation(left, right, edges, &node_bounds, direction);
        reverse_indices.insert(reverse);
        reciprocal_for.insert(canonical, reverse);
    }
    let route_indices = (0..edges.len())
        .filter(|index| !reverse_indices.contains(index))
        .collect::<Vec<_>>();

    let natural = route_indices
        .iter()
        .filter_map(|index| {
            let edge = &edges[*index];
            let source = *node_bounds.get(edge.from.as_str())?;
            let target = *node_bounds.get(edge.to.as_str())?;
            Some((*index, natural_sides(source, target, direction)))
        })
        .collect::<HashMap<_, _>>();
    let shared_target_sides = shared_target_sides(&edges, &route_indices, &node_bounds, direction);
    let mut plans = HashMap::with_capacity(route_indices.len());
    for index in &route_indices {
        let edge = &edges[*index];
        if edge.from == edge.to {
            edges[*index].route_diagnostic = Some(unsupported_self_link_diagnostic());
            continue;
        }
        let Some((natural_source, natural_target)) = natural.get(index).copied() else {
            edges[*index].route_diagnostic = Some(missing_node_diagnostic(edge, nodes));
            continue;
        };
        let (source_side, target_side) = shared_target_sides
            .get(edge.to.as_str())
            .copied()
            .map_or((natural_source, natural_target), |target_side| {
                (target_side.opposite(), target_side)
            });
        plans.insert(
            *index,
            EdgePlan {
                source_side,
                target_side,
            },
        );
    }

    for index in &route_indices {
        let Some(edge) = edges.get(*index) else {
            continue;
        };
        let (Some(plan), Some(source), Some(target)) = (
            plans.get(index),
            node_bounds.get(edge.from.as_str()).copied(),
            node_bounds.get(edge.to.as_str()).copied(),
        ) else {
            continue;
        };
        let start = side_anchor(source, plan.source_side);
        let end = side_anchor(target, plan.target_side);
        let direct_path = vec![direct_connector(start, end)];
        let obstacles = nodes
            .iter()
            .filter(|node| node.id != edge.from && node.id != edge.to)
            .map(|node| node.bounds)
            .collect::<Vec<_>>();
        let direct_obstruction_count =
            geometry::path_obstacle_intersections(&direct_path, &obstacles);
        let path = if mode == RoutingMode::Auto && direct_obstruction_count > 0 {
            search::orthogonal_route(start, end, &obstacles)
                .map(|points| geometry::polyline_connectors(&points))
                .filter(|path| !path.is_empty())
                .unwrap_or_else(|| direct_path.clone())
        } else {
            direct_path.clone()
        };
        let obstruction_count = geometry::path_obstacle_intersections(&path, &obstacles);
        let length = geometry::path_length(&path);
        let geometric_lower_bound = geometry::path_length(&direct_path);
        let selected_auto_route =
            mode == RoutingMode::Auto && direct_obstruction_count > 0 && obstruction_count == 0;
        edges[*index].path = path;
        edges[*index].route_diagnostic = Some(SceneRouteDiagnostic {
            status: SceneRouteStatus::Routed,
            source_port: plans
                .get(index)
                .map_or_else(String::new, |plan| plan.source_side.name().to_owned()),
            target_port: plans
                .get(index)
                .map_or_else(String::new, |plan| plan.target_side.name().to_owned()),
            candidates_considered: if mode == RoutingMode::Auto && direct_obstruction_count > 0 {
                2
            } else {
                1
            },
            rejected_for_obstacles: usize::from(direct_obstruction_count > 0),
            rejected_for_route_conflicts: 0,
            obstacle_intersections: obstruction_count,
            route_conflicts: 0,
            candidate_rejections: Vec::new(),
            route_length: length,
            geometric_lower_bound,
            detour_ratio: if geometric_lower_bound > f32::EPSILON {
                length / geometric_lower_bound
            } else {
                1.0
            },
            direct_candidate_clear: direct_obstruction_count == 0,
            selected_routing: if mode == RoutingMode::Auto {
                "auto".into()
            } else {
                "straight".into()
            },
            selection_reason: if selected_auto_route {
                "direct path was obstructed; selected a deterministic clear route".into()
            } else if mode == RoutingMode::Auto && direct_obstruction_count == 0 {
                "direct path was clear".into()
            } else if mode == RoutingMode::Auto {
                "no clear detour was found; retained the direct path".into()
            } else {
                "straight connector between assigned face anchors".into()
            },
            message: (obstruction_count > 0).then(|| {
                if mode == RoutingMode::Auto {
                    format!("The selected route intersects {obstruction_count} node(s).")
                } else {
                    format!("The direct connector passes beneath {obstruction_count} node(s); nodes render above links.")
                }
            }),
        });
        if let Some(reverse) = reciprocal_for.get(index).copied() {
            edges[reverse].path = reverse_path(&edges[*index].path);
            edges[reverse].route_diagnostic = edges[*index]
                .route_diagnostic
                .clone()
                .map(reverse_diagnostic);
            edges[*index].reciprocal_edge_index = Some(reverse);
            edges[reverse].reciprocal_edge_index = Some(*index);
        }
    }

    mark_route_conflicts(edges);
}

fn shared_target_sides(
    edges: &[SceneEdge],
    route_indices: &[usize],
    bounds: &HashMap<&str, Rect>,
    direction: FlowDirection,
) -> HashMap<String, Side> {
    let mut sources_by_target = HashMap::<&str, Vec<Rect>>::new();
    for index in route_indices {
        let Some(edge) = edges.get(*index) else {
            continue;
        };
        let Some(source) = bounds.get(edge.from.as_str()).copied() else {
            continue;
        };
        sources_by_target
            .entry(edge.to.as_str())
            .or_default()
            .push(source);
    }

    sources_by_target
        .into_iter()
        .filter_map(|(target_id, sources)| {
            let target = bounds.get(target_id).copied()?;
            let target_side = shared_flow_face(&sources, target, direction)?;
            Some((target_id.to_owned(), target_side))
        })
        .collect()
}

fn shared_flow_face(sources: &[Rect], target: Rect, direction: FlowDirection) -> Option<Side> {
    let target_right = target.x + target.width;
    let target_bottom = target.y + target.height;
    let all_on_flow_side = match direction {
        FlowDirection::LeftToRight => sources
            .iter()
            .all(|source| source.x + source.width <= target.x + 0.5),
        FlowDirection::RightToLeft => sources.iter().all(|source| source.x >= target_right - 0.5),
        FlowDirection::TopToBottom => sources
            .iter()
            .all(|source| source.y + source.height <= target.y + 0.5),
        FlowDirection::BottomToTop => sources.iter().all(|source| source.y >= target_bottom - 0.5),
    };
    all_on_flow_side.then(|| target_side_for_flow(direction))
}

fn target_side_for_flow(direction: FlowDirection) -> Side {
    match direction {
        FlowDirection::LeftToRight => Side::Left,
        FlowDirection::RightToLeft => Side::Right,
        FlowDirection::TopToBottom => Side::Top,
        FlowDirection::BottomToTop => Side::Bottom,
    }
}

fn compatible_reciprocals(edges: &[SceneEdge]) -> Vec<(usize, usize)> {
    let mut pairs = HashMap::<(String, String), (Vec<usize>, Vec<usize>)>::new();
    for (index, edge) in edges.iter().enumerate() {
        if edge.from == edge.to || edge.label.is_some() {
            continue;
        }
        let (key, forward) = if edge.from < edge.to {
            ((edge.from.clone(), edge.to.clone()), true)
        } else {
            ((edge.to.clone(), edge.from.clone()), false)
        };
        let entry = pairs.entry(key).or_default();
        if forward {
            entry.0.push(index);
        } else {
            entry.1.push(index);
        }
    }
    let mut result = pairs
        .into_values()
        .filter_map(|(forward, reverse)| {
            let ([forward], [reverse]) = (forward.as_slice(), reverse.as_slice()) else {
                return None;
            };
            (edges[*forward].style == edges[*reverse].style).then_some((*forward, *reverse))
        })
        .collect::<Vec<_>>();
    result.sort_unstable();
    result
}

fn choose_reciprocal_orientation(
    left: usize,
    right: usize,
    edges: &[SceneEdge],
    bounds: &HashMap<&str, Rect>,
    direction: FlowDirection,
) -> (usize, usize) {
    let progress = |index: usize| {
        let edge = &edges[index];
        let (Some(source), Some(target)) =
            (bounds.get(edge.from.as_str()), bounds.get(edge.to.as_str()))
        else {
            return f32::NEG_INFINITY;
        };
        let source = center(*source);
        let target = center(*target);
        match direction {
            FlowDirection::LeftToRight => target.x - source.x,
            FlowDirection::RightToLeft => source.x - target.x,
            FlowDirection::TopToBottom => target.y - source.y,
            FlowDirection::BottomToTop => source.y - target.y,
        }
    };
    if progress(right) > progress(left)
        || (progress(right) == progress(left)
            && (edges[right].from.as_str(), edges[right].to.as_str())
                < (edges[left].from.as_str(), edges[left].to.as_str()))
    {
        (right, left)
    } else {
        (left, right)
    }
}

fn unsupported_self_link_diagnostic() -> SceneRouteDiagnostic {
    SceneRouteDiagnostic {
        status: SceneRouteStatus::Unroutable,
        source_port: "right".into(),
        target_port: "right".into(),
        candidates_considered: 0,
        rejected_for_obstacles: 0,
        rejected_for_route_conflicts: 0,
        obstacle_intersections: 0,
        route_conflicts: 0,
        candidate_rejections: vec![
            "a self-link cannot be represented by a straight segment".into(),
        ],
        route_length: 0.0,
        geometric_lower_bound: 0.0,
        detour_ratio: 0.0,
        direct_candidate_clear: false,
        selected_routing: "straight".into(),
        selection_reason: "a loop needs non-straight geometry".into(),
        message: Some("No non-zero straight route exists for a self-link.".into()),
    }
}

fn mark_route_conflicts(edges: &mut [SceneEdge]) {
    let mut conflicts = vec![0usize; edges.len()];
    for left in 0..edges.len() {
        if edges[left].path.is_empty() {
            continue;
        }
        for right in left + 1..edges.len() {
            if edges[right].path.is_empty()
                || edges[left].reciprocal_edge_index == Some(right)
                || edges[right].reciprocal_edge_index == Some(left)
            {
                continue;
            }
            if edges[left].path.iter().any(|left_segment| {
                edges[right].path.iter().any(|right_segment| {
                    geometry::segments_conflict(
                        left_segment.start,
                        left_segment.end,
                        right_segment.start,
                        right_segment.end,
                    )
                })
            }) {
                conflicts[left] += 1;
                conflicts[right] += 1;
            }
        }
    }
    for (edge, count) in edges.iter_mut().zip(conflicts) {
        if let Some(diagnostic) = edge.route_diagnostic.as_mut() {
            diagnostic.route_conflicts = count;
            if count > 0 {
                diagnostic.message = Some(format!(
                    "This {} route intersects {count} other route(s).",
                    diagnostic.selected_routing
                ));
            }
        }
    }
}

fn missing_node_diagnostic(edge: &SceneEdge, nodes: &[SceneNode]) -> SceneRouteDiagnostic {
    let from_exists = nodes.iter().any(|node| node.id == edge.from);
    let to_exists = nodes.iter().any(|node| node.id == edge.to);
    let message = match (from_exists, to_exists) {
        (false, false) => "The source and target nodes are missing.",
        (false, true) => "The source node is missing.",
        (true, false) => "The target node is missing.",
        (true, true) => "The route endpoints are invalid.",
    };
    SceneRouteDiagnostic {
        status: SceneRouteStatus::Unroutable,
        source_port: String::new(),
        target_port: String::new(),
        candidates_considered: 0,
        rejected_for_obstacles: 0,
        rejected_for_route_conflicts: 0,
        obstacle_intersections: 0,
        route_conflicts: 0,
        candidate_rejections: Vec::new(),
        route_length: 0.0,
        geometric_lower_bound: 0.0,
        detour_ratio: 0.0,
        direct_candidate_clear: false,
        selected_routing: "straight".into(),
        selection_reason: "missing endpoint node".into(),
        message: Some(message.into()),
    }
}

fn reverse_path(path: &[CubicBezier]) -> Vec<CubicBezier> {
    path.iter()
        .rev()
        .map(|segment| CubicBezier {
            start: segment.end,
            control1: segment.control2,
            control2: segment.control1,
            end: segment.start,
        })
        .collect()
}

fn reverse_diagnostic(mut diagnostic: SceneRouteDiagnostic) -> SceneRouteDiagnostic {
    std::mem::swap(&mut diagnostic.source_port, &mut diagnostic.target_port);
    diagnostic
}
