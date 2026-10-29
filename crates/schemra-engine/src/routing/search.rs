//! Bounded, deterministic visibility-grid search for obstacle-avoiding routes.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use schemra_scene::{Point, Rect};

use super::geometry::segment_intersects_rect;

const OBSTACLE_CLEARANCE: f32 = 1.0;
const OUTER_CHANNEL: f32 = 48.0;
const MAX_EXPANSIONS: u32 = 4;
const MAX_AXIS_COORDINATES: usize = 72;
const BEND_COST: u64 = 22_000;
const COST_SCALE: f64 = 1_000.0;
const POINT_EPSILON: f32 = 0.001;

/// Finds a short rectilinear path around rectangular obstacles.
///
/// The graph uses obstacle boundaries and endpoint coordinates, so its cost is
/// independent of pixel resolution. Search expands locally only if blocked.
pub(super) fn orthogonal_route(start: Point, end: Point, obstacles: &[Rect]) -> Option<Vec<Point>> {
    if !point_is_finite(start)
        || !point_is_finite(end)
        || obstacles.iter().any(|rect| !rect_is_finite(*rect))
    {
        return None;
    }
    if point_distance(start, end) <= POINT_EPSILON {
        return Some(vec![start, end]);
    }

    let obstacles = obstacles
        .iter()
        .copied()
        .map(expand_obstacle)
        .collect::<Vec<_>>();
    if obstacles.iter().any(|rect| !rect_is_finite(*rect)) {
        return None;
    }
    if obstacles
        .iter()
        .any(|rect| point_is_inside(start, *rect) || point_is_inside(end, *rect))
    {
        return None;
    }

    let span = (end.x - start.x)
        .abs()
        .max((end.y - start.y).abs())
        .max(1.0);
    let base_margin = OUTER_CHANNEL.max(span * 0.15);
    for expansion in 0..MAX_EXPANSIONS {
        let margin = base_margin * (1_u32 << expansion) as f32;
        let bounds = Rect {
            x: start.x.min(end.x) - margin,
            y: start.y.min(end.y) - margin,
            width: (start.x - end.x).abs() + margin * 2.0,
            height: (start.y - end.y).abs() + margin * 2.0,
        };
        if !rect_is_finite(bounds) {
            return None;
        }
        let relevant = obstacles
            .iter()
            .copied()
            .filter(|obstacle| rects_intersect(*obstacle, bounds))
            .collect::<Vec<_>>();
        if let Some(path) = search_in_bounds(start, end, &relevant, bounds) {
            return Some(path);
        }
    }
    None
}

fn search_in_bounds(
    start: Point,
    end: Point,
    obstacles: &[Rect],
    bounds: Rect,
) -> Option<Vec<Point>> {
    let mut xs = vec![bounds.x, bounds.x + bounds.width, start.x, end.x];
    let mut ys = vec![bounds.y, bounds.y + bounds.height, start.y, end.y];
    for obstacle in obstacles {
        xs.extend([
            obstacle.x.max(bounds.x),
            (obstacle.x + obstacle.width).min(bounds.x + bounds.width),
        ]);
        ys.extend([
            obstacle.y.max(bounds.y),
            (obstacle.y + obstacle.height).min(bounds.y + bounds.height),
        ]);
    }
    sort_coordinates(&mut xs);
    sort_coordinates(&mut ys);
    if xs.len() > MAX_AXIS_COORDINATES || ys.len() > MAX_AXIS_COORDINATES {
        return None;
    }

    let width = xs.len();
    let height = ys.len();
    let mut point_ids = vec![None; width.checked_mul(height)?];
    let mut points = Vec::with_capacity(point_ids.len());
    for (row, y) in ys.iter().copied().enumerate() {
        for (column, x) in xs.iter().copied().enumerate() {
            let point = Point { x, y };
            if obstacles
                .iter()
                .any(|obstacle| point_is_inside(point, *obstacle))
            {
                continue;
            }
            let id = points.len();
            point_ids[row * width + column] = Some(id);
            points.push(point);
        }
    }
    let start_id =
        point_ids[(coordinate_index(&ys, start.y)? * width) + coordinate_index(&xs, start.x)?]?;
    let end_id =
        point_ids[(coordinate_index(&ys, end.y)? * width) + coordinate_index(&xs, end.x)?]?;

    let mut graph = vec![Vec::<(usize, u8, u64)>::new(); points.len()];
    for row in 0..height {
        for column in 0..width {
            let Some(current) = point_ids[row * width + column] else {
                continue;
            };
            if column + 1 < width
                && let Some(next) = point_ids[row * width + column + 1]
                && segment_is_clear(points[current], points[next], obstacles)
            {
                let cost = scaled_distance(points[current], points[next])?;
                graph[current].push((next, 1, cost));
                graph[next].push((current, 1, cost));
            }
            if row + 1 < height
                && let Some(next) = point_ids[(row + 1) * width + column]
                && segment_is_clear(points[current], points[next], obstacles)
            {
                let cost = scaled_distance(points[current], points[next])?;
                graph[current].push((next, 2, cost));
                graph[next].push((current, 2, cost));
            }
        }
    }

    let state_count = points.len().checked_mul(3)?;
    let mut costs = vec![u64::MAX; state_count];
    let mut previous = vec![None; state_count];
    let start_state = start_id * 3;
    costs[start_state] = 0;
    let mut queue = BinaryHeap::from([Reverse((0_u64, start_state))]);

    while let Some(Reverse((cost, state))) = queue.pop() {
        if cost != costs[state] {
            continue;
        }
        let current = state / 3;
        let previous_axis = (state % 3) as u8;
        for &(next, axis, distance) in &graph[current] {
            let next_state = next * 3 + usize::from(axis);
            let bend = u64::from(previous_axis != 0 && previous_axis != axis) * BEND_COST;
            let next_cost = cost.saturating_add(distance).saturating_add(bend);
            if next_cost < costs[next_state] {
                costs[next_state] = next_cost;
                previous[next_state] = Some(state);
                queue.push(Reverse((next_cost, next_state)));
            }
        }
    }

    let end_state = (0..3)
        .map(|axis| end_id * 3 + axis)
        .min_by_key(|state| (costs[*state], *state))?;
    if costs[end_state] == u64::MAX {
        return None;
    }
    let mut route = Vec::new();
    let mut state = end_state;
    loop {
        route.push(points[state / 3]);
        if state == start_state {
            break;
        }
        state = previous[state]?;
    }
    route.reverse();
    route[0] = start;
    *route.last_mut()? = end;
    simplify_collinear(&mut route);
    Some(route)
}

fn expand_obstacle(rect: Rect) -> Rect {
    let left = rect.x.min(rect.x + rect.width) - OBSTACLE_CLEARANCE;
    let top = rect.y.min(rect.y + rect.height) - OBSTACLE_CLEARANCE;
    let right = rect.x.max(rect.x + rect.width) + OBSTACLE_CLEARANCE;
    let bottom = rect.y.max(rect.y + rect.height) + OBSTACLE_CLEARANCE;
    Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

fn sort_coordinates(coordinates: &mut Vec<f32>) {
    coordinates.sort_by(f32::total_cmp);
    coordinates.dedup_by(|left, right| *left == *right);
}

fn coordinate_index(coordinates: &[f32], value: f32) -> Option<usize> {
    coordinates
        .binary_search_by(|candidate| {
            candidate
                .partial_cmp(&value)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok()
}

fn point_is_inside(point: Point, rect: Rect) -> bool {
    let left = rect.x.min(rect.x + rect.width);
    let right = rect.x.max(rect.x + rect.width);
    let top = rect.y.min(rect.y + rect.height);
    let bottom = rect.y.max(rect.y + rect.height);
    point.x > left && point.x < right && point.y > top && point.y < bottom
}

fn segment_is_clear(start: Point, end: Point, obstacles: &[Rect]) -> bool {
    obstacles
        .iter()
        .all(|obstacle| !segment_intersects_rect(start, end, *obstacle))
}

fn scaled_distance(start: Point, end: Point) -> Option<u64> {
    let distance = (f64::from(end.x) - f64::from(start.x))
        .hypot(f64::from(end.y) - f64::from(start.y))
        * COST_SCALE;
    (distance.is_finite() && distance < u64::MAX as f64).then_some(distance.round() as u64)
}

fn point_distance(start: Point, end: Point) -> f32 {
    (end.x - start.x).hypot(end.y - start.y)
}

fn rect_is_finite(rect: Rect) -> bool {
    [
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        rect.x + rect.width,
        rect.y + rect.height,
    ]
    .into_iter()
    .all(f32::is_finite)
}

fn rects_intersect(left: Rect, right: Rect) -> bool {
    let left_x = left.x.min(left.x + left.width)..=left.x.max(left.x + left.width);
    let left_y = left.y.min(left.y + left.height)..=left.y.max(left.y + left.height);
    let right_x = right.x.min(right.x + right.width)..=right.x.max(right.x + right.width);
    let right_y = right.y.min(right.y + right.height)..=right.y.max(right.y + right.height);
    left_x.start() <= right_x.end()
        && left_x.end() >= right_x.start()
        && left_y.start() <= right_y.end()
        && left_y.end() >= right_y.start()
}

fn point_is_finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn simplify_collinear(points: &mut Vec<Point>) {
    let mut index = 1;
    while index + 1 < points.len() {
        let previous = points[index - 1];
        let current = points[index];
        let next = points[index + 1];
        let cross = (current.x - previous.x) * (next.y - current.y)
            - (current.y - previous.y) * (next.x - current.x);
        if cross.abs() <= POINT_EPSILON {
            points.remove(index);
        } else {
            index += 1;
        }
    }
}

#[cfg(test)]
mod tests;
