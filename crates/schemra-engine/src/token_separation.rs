use std::collections::HashMap;

use schemra_ir::{SceneEdgeProgress, SceneTokenPosition};
use schemra_scene::Point;

use crate::routing::RouteGeometry;

const TOKEN_MARKER_RADIUS: f32 = 11.0;
const EXTRA_LANE_GAP: f32 = 1.0;

#[derive(Clone, Copy)]
struct Traveler {
    token_index: usize,
    instance_id: usize,
    path_coordinate: f32,
    travel_progress: f32,
    normal: Point,
    same_direction_offset: f32,
}

#[derive(Default)]
struct EdgeTravelers {
    forward: Vec<Traveler>,
    reverse: Vec<Traveler>,
}

pub(super) fn separate_crowded_tokens(
    tokens: &mut [SceneTokenPosition],
    edges: &[SceneEdgeProgress],
    route_geometry: &[RouteGeometry],
) {
    let mut travelers_by_edge = HashMap::<usize, EdgeTravelers>::new();
    for (token_index, (token, edge)) in tokens.iter().zip(edges).enumerate() {
        let Some(geometry) = route_geometry.get(edge.edge_index) else {
            continue;
        };
        let direction = match edge.traversal_direction {
            1 => 1,
            -1 => -1,
            _ => continue,
        };
        let travel_progress = edge.progress_micros as f32 / 1_000_000.0;
        let path_coordinate = if direction == 1 {
            travel_progress
        } else {
            1.0 - travel_progress
        };
        let Some(mut normal) = geometry.normal_at_progress(path_coordinate) else {
            continue;
        };
        if direction == -1 {
            normal.x = -normal.x;
            normal.y = -normal.y;
        }

        let traveler = Traveler {
            token_index,
            instance_id: token.instance_id,
            path_coordinate,
            travel_progress,
            normal,
            same_direction_offset: 0.0,
        };
        let edge_travelers = travelers_by_edge.entry(edge.edge_index).or_default();
        if direction == 1 {
            edge_travelers.forward.push(traveler);
        } else {
            edge_travelers.reverse.push(traveler);
        }
    }

    for (edge_index, mut travelers) in travelers_by_edge {
        let Some(geometry) = route_geometry.get(edge_index) else {
            continue;
        };
        let path_length = geometry.length();
        let maximum_offset = (TOKEN_MARKER_RADIUS + EXTRA_LANE_GAP).min(path_length / 2.0);
        if maximum_offset <= 0.0 {
            continue;
        }

        let cluster_distance = TOKEN_MARKER_RADIUS * 2.0 + maximum_offset * 2.0;
        for same_direction in [&mut travelers.forward, &mut travelers.reverse] {
            assign_symmetric_lanes(same_direction, path_length, cluster_distance);
        }
        separate_direction(&travelers.forward, &travelers.reverse, tokens, path_length);
        separate_direction(&travelers.reverse, &travelers.forward, tokens, path_length);
    }
}

fn assign_symmetric_lanes(travelers: &mut [Traveler], path_length: f32, cluster_distance: f32) {
    travelers.sort_by(|left, right| {
        left.path_coordinate
            .total_cmp(&right.path_coordinate)
            .then_with(|| left.instance_id.cmp(&right.instance_id))
    });

    let mut cluster_start = 0;
    while cluster_start < travelers.len() {
        let mut cluster_end = cluster_start + 1;
        while cluster_end < travelers.len()
            && (travelers[cluster_end].path_coordinate - travelers[cluster_end - 1].path_coordinate)
                .abs()
                * path_length
                < cluster_distance
        {
            cluster_end += 1;
        }

        let cluster = &mut travelers[cluster_start..cluster_end];
        if cluster.len() > 1 {
            let last_index = cluster.len() - 1;
            for index in 0..cluster.len() {
                let path_coordinate = cluster[index].path_coordinate;
                let travel_progress = cluster[index].travel_progress;
                let nearest_distance = index
                    .checked_sub(1)
                    .map(|previous| {
                        (path_coordinate - cluster[previous].path_coordinate).abs() * path_length
                    })
                    .into_iter()
                    .chain(
                        cluster.get(index + 1).map(|next| {
                            (path_coordinate - next.path_coordinate).abs() * path_length
                        }),
                    )
                    .fold(f32::INFINITY, f32::min);
                let lane = if last_index == 1 {
                    index as f32 * 2.0 - 1.0
                } else {
                    index as f32 / last_index as f32 * 2.0 - 1.0
                };
                cluster[index].same_direction_offset =
                    lane * temporary_lane_offset(travel_progress, nearest_distance, path_length);
            }
        }
        cluster_start = cluster_end;
    }
}

fn separate_direction(
    travelers: &[Traveler],
    opposing: &[Traveler],
    tokens: &mut [SceneTokenPosition],
    path_length: f32,
) {
    for traveler in travelers {
        let closest_opposing_distance = opposing
            .iter()
            .map(|other| (traveler.path_coordinate - other.path_coordinate).abs() * path_length)
            .fold(f32::INFINITY, f32::min);
        let opposing_offset = temporary_lane_offset(
            traveler.travel_progress,
            closest_opposing_distance,
            path_length,
        );
        let same_direction_offset =
            traveler.same_direction_offset * if opposing_offset > 0.0 { 0.35 } else { 1.0 };
        let offset = (opposing_offset + same_direction_offset).clamp(
            -(TOKEN_MARKER_RADIUS + EXTRA_LANE_GAP).min(path_length / 2.0),
            (TOKEN_MARKER_RADIUS + EXTRA_LANE_GAP).min(path_length / 2.0),
        );
        let token = &mut tokens[traveler.token_index];
        token.x += traveler.normal.x * offset;
        token.y += traveler.normal.y * offset;
    }
}

fn temporary_lane_offset(progress: f32, closest_distance: f32, path_length: f32) -> f32 {
    let maximum_offset = (TOKEN_MARKER_RADIUS + EXTRA_LANE_GAP).min(path_length / 2.0);
    if maximum_offset <= 0.0 {
        return 0.0;
    }

    let influence_distance = TOKEN_MARKER_RADIUS * 2.0 + maximum_offset * 2.0;
    let proximity = (1.0 - closest_distance / influence_distance).clamp(0.0, 1.0);
    let eased_proximity = proximity.powi(2) * (3.0 - 2.0 * proximity);
    let endpoint_taper = (std::f32::consts::PI * progress.clamp(0.0, 1.0)).sin();
    maximum_offset * eased_proximity * endpoint_taper
}
