//! Deterministic straight-port selection that avoids intervening nodes.

use std::cmp::Ordering;

use schemra_scene::{CubicBezier, Rect};

use super::Side;
use super::geometry::{
    direct_connector, path_length, path_obstacle_intersections, paths_conflict, side_anchor,
};

#[derive(Clone, Debug)]
pub(super) struct DirectChoice {
    pub(super) source_side: Side,
    pub(super) target_side: Side,
    pub(super) path: Vec<CubicBezier>,
    pub(super) original_direct_clear: bool,
    pub(super) candidates_considered: usize,
    pub(super) rejected_for_obstacles: usize,
    pub(super) rejected_for_route_conflicts: usize,
    pub(super) geometric_lower_bound: f32,
    pub(super) selection_reason: String,
}

pub(super) struct SharedTargetRequest<'a> {
    pub(super) index: usize,
    pub(super) source: Rect,
    pub(super) preferred_source: Side,
    pub(super) obstacles: &'a [Rect],
}

#[derive(Clone)]
struct Candidate {
    source_side: Side,
    target_side: Side,
    path: Vec<CubicBezier>,
    intersections: usize,
    route_conflicts: usize,
    changed_source: bool,
    changed_target: bool,
    source_rank: usize,
    target_rank: usize,
    length: f32,
}

struct CandidateRequest<'obstacles, 'paths> {
    source: Rect,
    target: Rect,
    source_side: Side,
    target_side: Side,
    obstacles: &'obstacles [Rect],
    other_paths: &'obstacles [&'paths [CubicBezier]],
    source_rank: usize,
    target_rank: usize,
}

/// Selects one direct connector, preferring the natural pair unless it is blocked.
pub(super) fn choose_direct_pair(
    source: Rect,
    target: Rect,
    preferred_source: Side,
    preferred_target: Side,
    obstacles: &[Rect],
    other_paths: &[&[CubicBezier]],
) -> DirectChoice {
    let original = candidate(CandidateRequest {
        source,
        target,
        source_side: preferred_source,
        target_side: preferred_target,
        obstacles,
        other_paths,
        source_rank: 0,
        target_rank: 0,
    });
    if original.intersections == 0 && original.route_conflicts == 0 {
        return DirectChoice {
            source_side: preferred_source,
            target_side: preferred_target,
            path: original.path,
            original_direct_clear: true,
            candidates_considered: 1,
            rejected_for_obstacles: 0,
            rejected_for_route_conflicts: 0,
            geometric_lower_bound: original.length,
            selection_reason: "the natural straight connector was clear".into(),
        };
    }

    let source_sides = ordered_sides(preferred_source);
    let target_sides = ordered_sides(preferred_target);
    let mut best: Option<Candidate> = None;
    let mut rejected_for_obstacles = 0;
    let mut rejected_for_route_conflicts = 0;
    for (target_rank, target_side) in target_sides.iter().copied().enumerate() {
        for (source_rank, source_side) in source_sides.iter().copied().enumerate() {
            let next = candidate(CandidateRequest {
                source,
                target,
                source_side,
                target_side,
                obstacles,
                other_paths,
                source_rank,
                target_rank,
            });
            rejected_for_obstacles += usize::from(next.intersections > 0);
            rejected_for_route_conflicts += usize::from(next.route_conflicts > 0);
            if best
                .as_ref()
                .is_none_or(|current| compare_candidates(&next, current).is_lt())
            {
                best = Some(next);
            }
        }
    }
    let selected = best.expect("each node has four port faces");
    let selection_reason = if original.route_conflicts > selected.route_conflicts {
        "the natural straight connector crossed another route; selected a clearer direct connector"
    } else if selected.intersections == 0 {
        "the natural connector was blocked; selected a clear straight connector from alternate ports"
    } else {
        "no obstacle-free port pair exists; selected the least obstructed straight connector"
    };
    DirectChoice {
        source_side: selected.source_side,
        target_side: selected.target_side,
        path: selected.path,
        original_direct_clear: false,
        candidates_considered: source_sides.len() * target_sides.len(),
        rejected_for_obstacles,
        rejected_for_route_conflicts,
        geometric_lower_bound: original.length,
        selection_reason: selection_reason.into(),
    }
}

/// Keeps a fan on its shared target face while choosing clear source ports.
pub(super) fn choose_shared_target_face(
    target: Rect,
    preferred_target: Side,
    requests: &[SharedTargetRequest<'_>],
) -> Vec<(usize, DirectChoice)> {
    if requests.is_empty() {
        return Vec::new();
    }
    let original = requests
        .iter()
        .map(|request| {
            candidate(CandidateRequest {
                source: request.source,
                target,
                source_side: request.preferred_source,
                target_side: preferred_target,
                obstacles: request.obstacles,
                other_paths: &[],
                source_rank: 0,
                target_rank: 0,
            })
        })
        .collect::<Vec<_>>();
    if original
        .iter()
        .all(|candidate| candidate.intersections == 0)
    {
        return requests
            .iter()
            .zip(original)
            .map(|(request, candidate)| {
                (
                    request.index,
                    DirectChoice {
                        source_side: candidate.source_side,
                        target_side: candidate.target_side,
                        path: candidate.path,
                        original_direct_clear: true,
                        candidates_considered: 1,
                        rejected_for_obstacles: 0,
                        rejected_for_route_conflicts: 0,
                        geometric_lower_bound: candidate.length,
                        selection_reason: "the shared target face has clear straight connectors"
                            .into(),
                    },
                )
            })
            .collect();
    }

    requests
        .iter()
        .zip(original)
        .map(|(request, original)| {
            let source_sides = ordered_sides(request.preferred_source);
            let mut best: Option<Candidate> = None;
            let mut rejected_for_obstacles = 0;
            for (source_rank, source_side) in source_sides.iter().copied().enumerate() {
                let next = candidate(CandidateRequest {
                    source: request.source,
                    target,
                    source_side,
                    target_side: preferred_target,
                    obstacles: request.obstacles,
                    other_paths: &[],
                    source_rank,
                    target_rank: 0,
                });
                rejected_for_obstacles += usize::from(next.intersections > 0);
                if best
                    .as_ref()
                    .is_none_or(|current| compare_candidates(&next, current).is_lt())
                {
                    best = Some(next);
                }
            }
            let selected = best.expect("each node has four port faces");
            let selection_reason = if original.intersections == 0 {
                "kept the natural straight connector on the shared target face"
            } else if selected.intersections == 0 {
                "the natural connector was blocked; selected a clear source port on the shared target face"
            } else {
                "no clear source port exists; retained the least obstructed route on the shared target face"
            };
            (
                request.index,
                DirectChoice {
                    source_side: selected.source_side,
                    target_side: preferred_target,
                    path: selected.path,
                    original_direct_clear: original.intersections == 0,
                    candidates_considered: source_sides.len(),
                    rejected_for_obstacles,
                    rejected_for_route_conflicts: 0,
                    geometric_lower_bound: original.length,
                    selection_reason: selection_reason.into(),
                },
            )
        })
        .collect()
}

fn candidate(request: CandidateRequest<'_, '_>) -> Candidate {
    let CandidateRequest {
        source,
        target,
        source_side,
        target_side,
        obstacles,
        other_paths,
        source_rank,
        target_rank,
    } = request;
    let start = side_anchor(source, source_side);
    let end = side_anchor(target, target_side);
    let path = vec![direct_connector(start, end)];
    Candidate {
        source_side,
        target_side,
        intersections: path_obstacle_intersections(&path, obstacles),
        route_conflicts: other_paths
            .iter()
            .filter(|other| paths_conflict(&path, other))
            .count(),
        changed_source: source_rank > 0,
        changed_target: target_rank > 0,
        source_rank,
        target_rank,
        length: path_length(&path),
        path,
    }
}

fn ordered_sides(preferred: Side) -> Vec<Side> {
    let mut sides = vec![preferred];
    for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        if !sides.contains(&side) {
            sides.push(side);
        }
    }
    sides
}

fn compare_candidates(left: &Candidate, right: &Candidate) -> Ordering {
    left.intersections
        .cmp(&right.intersections)
        .then(left.route_conflicts.cmp(&right.route_conflicts))
        .then(left.changed_target.cmp(&right.changed_target))
        .then(left.changed_source.cmp(&right.changed_source))
        .then_with(|| left.length.total_cmp(&right.length))
        .then(left.target_rank.cmp(&right.target_rank))
        .then(left.source_rank.cmp(&right.source_rank))
}
