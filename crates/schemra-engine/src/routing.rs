//! Straight link routing with shared faces and non-overlapping fan geometry.

use std::collections::{HashMap, HashSet};

use schemra_scene::{
    CubicBezier, FlowDirection, Point, Rect, SceneEdge, SceneNode, SceneRouteDiagnostic,
    SceneRouteStatus,
};

mod geometry;
mod search;

use geometry::{Side, center, direct_connector, natural_sides, port_anchor};

#[cfg(test)]
mod tests;

const PORT_MARGIN: f32 = 8.0;
const PORT_SPACING: f32 = 12.0;
const MAX_PORT_SPREAD: f32 = 36.0;

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

#[derive(Clone, Debug)]
struct Endpoint {
    edge_index: usize,
    is_source: bool,
    node_id: String,
    neighbor_id: String,
    neighbor: Point,
    bounds: Rect,
    side: Side,
}

/// Routes each edge as a straight connector. A fan shares a selected node
/// face. Incoming fans share one target anchor; when that would make links
/// overlap collinearly, the anchor shifts by the smallest clear amount.
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
    let target_faces = common_target_faces(edges, &route_indices, &natural, direction);
    let mut plans = HashMap::with_capacity(route_indices.len());
    for index in &route_indices {
        let edge = &edges[*index];
        if edge.from == edge.to {
            edges[*index].route_diagnostic = Some(unsupported_self_link_diagnostic());
            continue;
        }
        let Some((source_side, natural_target)) = natural.get(index).copied() else {
            edges[*index].route_diagnostic = Some(missing_node_diagnostic(edge, nodes));
            continue;
        };
        plans.insert(
            *index,
            EdgePlan {
                source_side,
                target_side: target_faces
                    .get(edge.to.as_str())
                    .copied()
                    .unwrap_or(natural_target),
            },
        );
    }

    let mut endpoints = HashMap::<(String, Side), Vec<Endpoint>>::new();
    for index in &route_indices {
        let (Some(plan), Some(edge)) = (plans.get(index), edges.get(*index)) else {
            continue;
        };
        let (Some(source_bounds), Some(target_bounds)) = (
            node_bounds.get(edge.from.as_str()).copied(),
            node_bounds.get(edge.to.as_str()).copied(),
        ) else {
            continue;
        };
        endpoints
            .entry((edge.from.clone(), plan.source_side))
            .or_default()
            .push(Endpoint {
                edge_index: *index,
                is_source: true,
                node_id: edge.from.clone(),
                neighbor_id: edge.to.clone(),
                neighbor: center(target_bounds),
                bounds: source_bounds,
                side: plan.source_side,
            });
        endpoints
            .entry((edge.to.clone(), plan.target_side))
            .or_default()
            .push(Endpoint {
                edge_index: *index,
                is_source: false,
                node_id: edge.to.clone(),
                neighbor_id: edge.from.clone(),
                neighbor: center(source_bounds),
                bounds: target_bounds,
                side: plan.target_side,
            });
    }

    let mut anchors = HashMap::<(usize, bool), Point>::new();
    for (_, mut face_endpoints) in endpoints {
        let target_fan = face_endpoints.iter().all(|endpoint| !endpoint.is_source);
        let count = face_endpoints.len();
        let bounds = face_endpoints[0].bounds;
        let side = face_endpoints[0].side;
        let tangent_extent = match side {
            Side::Left | Side::Right => bounds.height,
            Side::Top | Side::Bottom => bounds.width,
        };
        let usable_spread = (tangent_extent * 0.5 - PORT_MARGIN).max(0.0);
        let requested_spread = count.saturating_sub(1) as f32 * PORT_SPACING * 0.5;
        let centered_spread = usable_spread.min(requested_spread).min(MAX_PORT_SPREAD);
        let target_fan_offset =
            target_fan.then(|| shared_fan_offset(&face_endpoints, side, bounds, usable_spread));
        if !target_fan {
            face_endpoints.sort_by(endpoint_order);
        }
        for (position, endpoint) in face_endpoints.iter().enumerate() {
            let offset = if let Some(offset) = target_fan_offset {
                offset
            } else if count <= 1 {
                0.0
            } else {
                -centered_spread + 2.0 * centered_spread * position as f32 / (count - 1) as f32
            };
            anchors.insert(
                (endpoint.edge_index, endpoint.is_source),
                port_anchor(endpoint.bounds, endpoint.side, offset),
            );
        }
    }

    for index in &route_indices {
        let Some(edge) = edges.get(*index) else {
            continue;
        };
        let (Some(start), Some(end)) =
            (anchors.get(&(*index, true)), anchors.get(&(*index, false)))
        else {
            continue;
        };
        let direct_path = vec![direct_connector(*start, *end)];
        let obstacles = nodes
            .iter()
            .filter(|node| node.id != edge.from && node.id != edge.to)
            .map(|node| node.bounds)
            .collect::<Vec<_>>();
        let direct_obstruction_count =
            geometry::path_obstacle_intersections(&direct_path, &obstacles);
        let path = if mode == RoutingMode::Auto && direct_obstruction_count > 0 {
            search::orthogonal_route(*start, *end, &obstacles)
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

fn common_target_faces(
    edges: &[SceneEdge],
    route_indices: &[usize],
    natural: &HashMap<usize, (Side, Side)>,
    direction: FlowDirection,
) -> HashMap<String, Side> {
    let mut votes = HashMap::<String, [usize; 4]>::new();
    for index in route_indices {
        if let (Some(edge), Some((_, side))) = (edges.get(*index), natural.get(index)) {
            votes.entry(edge.to.clone()).or_default()[side.index()] += 1;
        }
    }
    votes
        .into_iter()
        .map(|(target, counts)| {
            let preferred = target_side(direction);
            let maximum = counts.iter().copied().max().unwrap_or_default();
            let side = if counts[preferred.index()] == maximum {
                preferred
            } else {
                Side::ALL
                    .into_iter()
                    .find(|side| counts[side.index()] == maximum)
                    .unwrap_or(preferred)
            };
            (target, side)
        })
        .collect()
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

fn endpoint_order(left: &Endpoint, right: &Endpoint) -> std::cmp::Ordering {
    let vertical_face = matches!(left.side, Side::Left | Side::Right);
    let left_center = center(left.bounds);
    let left_tangent = if vertical_face {
        left.neighbor.y
    } else {
        left.neighbor.x
    };
    let right_tangent = if vertical_face {
        right.neighbor.y
    } else {
        right.neighbor.x
    };
    let tangent_center = if vertical_face {
        left_center.y
    } else {
        left_center.x
    };
    let mut order = left_tangent.total_cmp(&right_tangent);
    if order == std::cmp::Ordering::Equal {
        let left_cross = if vertical_face {
            left.neighbor.x
        } else {
            left.neighbor.y
        };
        let right_cross = if vertical_face {
            right.neighbor.x
        } else {
            right.neighbor.y
        };
        let tangent_above_center = left_tangent <= tangent_center;
        let reverse_cross = tangent_above_center;
        order = if reverse_cross {
            right_cross.total_cmp(&left_cross)
        } else {
            left_cross.total_cmp(&right_cross)
        };
        if order == std::cmp::Ordering::Equal {
            order = left_cross.total_cmp(&right_cross);
        }
    }
    order
        .then_with(|| left.neighbor_id.cmp(&right.neighbor_id))
        .then_with(|| left.node_id.cmp(&right.node_id))
        .then_with(|| left.edge_index.cmp(&right.edge_index))
}

fn shared_fan_offset(endpoints: &[Endpoint], side: Side, bounds: Rect, usable_spread: f32) -> f32 {
    if endpoints.len() <= 1 || usable_spread <= f32::EPSILON {
        return 0.0;
    }
    let sources = endpoints
        .iter()
        .map(|endpoint| endpoint.neighbor)
        .collect::<Vec<_>>();
    let step = (PORT_SPACING * 0.5).min(usable_spread);
    let steps = (usable_spread / step).floor() as usize;
    let candidates = std::iter::once(0.0)
        .chain((1..=steps).flat_map(|index| {
            let distance = index as f32 * step;
            [distance, -distance]
        }))
        .collect::<Vec<_>>();
    candidates
        .into_iter()
        .min_by(|left, right| {
            let left_collisions = collinear_fan_pairs(&sources, port_anchor(bounds, side, *left));
            let right_collisions = collinear_fan_pairs(&sources, port_anchor(bounds, side, *right));
            left_collisions
                .cmp(&right_collisions)
                .then_with(|| left.abs().total_cmp(&right.abs()))
                .then_with(|| right.total_cmp(left))
        })
        .unwrap_or(0.0)
}

fn collinear_fan_pairs(sources: &[Point], target: Point) -> usize {
    let mut overlaps = 0;
    for (left_index, left_source) in sources.iter().enumerate() {
        let left_ray = vector_between(target, *left_source);
        let left_length = left_ray.x.hypot(left_ray.y);
        if left_length <= f32::EPSILON {
            continue;
        }
        for right_source in sources.iter().skip(left_index + 1) {
            let right_ray = vector_between(target, *right_source);
            let right_length = right_ray.x.hypot(right_ray.y);
            if right_length <= f32::EPSILON {
                continue;
            }
            let cross = left_ray.x * right_ray.y - left_ray.y * right_ray.x;
            let dot = left_ray.x * right_ray.x + left_ray.y * right_ray.y;
            if dot > 0.0 && cross.abs() <= left_length * right_length * 1.0e-4 {
                overlaps += 1;
            }
        }
    }
    overlaps
}

fn vector_between(from: Point, to: Point) -> Point {
    Point {
        x: to.x - from.x,
        y: to.y - from.y,
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

fn target_side(direction: FlowDirection) -> Side {
    match direction {
        FlowDirection::LeftToRight => Side::Left,
        FlowDirection::RightToLeft => Side::Right,
        FlowDirection::TopToBottom => Side::Top,
        FlowDirection::BottomToTop => Side::Bottom,
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
