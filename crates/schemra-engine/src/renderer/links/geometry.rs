//! Cubic-route measurements used to place direction cues and labels.

use schemra_scene::{CubicBezier, Point, Rect, SceneNode};

const LABEL_FONT_SIZE: f32 = 12.0;
const LABEL_BASELINE_GAP: f32 = 17.0;
const LABEL_HEIGHT: f32 = 14.0;
const LABEL_HALO: f32 = 2.5;
const LABEL_NODE_CLEARANCE: f32 = 3.0;
const LABEL_OFFSETS: [f32; 9] = [0.0, -18.0, 18.0, -36.0, 36.0, -54.0, 54.0, -72.0, 72.0];
const LABEL_ROUTE_FRACTIONS: [f32; 9] = [0.5, 0.4, 0.6, 0.3, 0.7, 0.2, 0.8, 0.1, 0.9];
const CURVE_FLATNESS_TOLERANCE: f32 = 0.2;
const CURVE_MAX_SUBDIVISION_DEPTH: u8 = 10;
const EDGE_GEOMETRY_EPSILON: f32 = 0.001;

#[derive(Clone, Copy, Debug)]
struct PathSample {
    segment: usize,
    t: f32,
    point: Point,
    distance: f32,
}

pub(super) fn route_label_baseline(
    path: &[CubicBezier],
    label: &str,
    nodes: &[SceneNode],
) -> Point {
    let samples = flatten_route(path);
    let length = samples.last().map_or(0.0, |sample| sample.distance);
    let width = estimated_label_width(label);
    let mut fallback = point_at_distance(&samples, path, length / 2.0).point;
    fallback.y -= LABEL_BASELINE_GAP;

    for fraction in LABEL_ROUTE_FRACTIONS {
        let sample = point_at_distance(&samples, path, length * fraction);
        let Some(segment) = path.get(sample.segment).copied() else {
            continue;
        };
        let tangent = bezier_tangent(segment, sample.t);
        let tangent_length = tangent.x.hypot(tangent.y);
        let normal = if tangent_length <= EDGE_GEOMETRY_EPSILON {
            Point { x: 0.0, y: -1.0 }
        } else {
            Point {
                x: -tangent.y / tangent_length,
                y: tangent.x / tangent_length,
            }
        };
        let base = Point {
            x: sample.point.x,
            y: sample.point.y - LABEL_BASELINE_GAP,
        };
        for offset_distance in LABEL_OFFSETS {
            let baseline = offset(base, normal, offset_distance);
            if !label_overlaps_node(baseline, width, nodes) {
                return baseline;
            }
        }
    }

    fallback
}

/// Places an inline chevron away from a shared endpoint and aligns it to the
/// local route tangent. The returned tangent points in the direction of travel.
pub(super) fn route_direction_cue(path: &[CubicBezier], at_start: bool) -> Option<(Point, Point)> {
    let samples = flatten_route(path);
    let length = samples.last()?.distance;
    if length <= EDGE_GEOMETRY_EPSILON {
        return None;
    }
    let fraction = if at_start { 0.32 } else { 0.68 };
    let sample = point_at_distance(&samples, path, length * fraction);
    let mut tangent = bezier_tangent(path.get(sample.segment).copied()?, sample.t);
    let tangent_length = tangent.x.hypot(tangent.y);
    if tangent_length <= EDGE_GEOMETRY_EPSILON {
        return None;
    }
    tangent.x /= tangent_length;
    tangent.y /= tangent_length;
    if at_start {
        tangent.x = -tangent.x;
        tangent.y = -tangent.y;
    }
    Some((sample.point, tangent))
}

fn point_at_distance(samples: &[PathSample], path: &[CubicBezier], target: f32) -> PathSample {
    for pair in samples.windows(2) {
        let [before, after] = pair else { continue };
        if target > after.distance {
            continue;
        }
        let segment_length = after.distance - before.distance;
        if before.segment != after.segment {
            return PathSample {
                segment: before.segment,
                t: before.t,
                point: before.point,
                distance: target,
            };
        }
        let progress = if segment_length <= EDGE_GEOMETRY_EPSILON {
            0.0
        } else {
            (target - before.distance) / segment_length
        };
        let segment = before.segment;
        let t = before.t + (after.t - before.t) * progress;
        return PathSample {
            segment,
            t,
            point: bezier_point(path[segment], t),
            distance: target,
        };
    }
    *samples.last().unwrap_or(&PathSample {
        segment: path.len().saturating_sub(1),
        t: 1.0,
        point: path
            .last()
            .map_or_else(Point::default, |segment| segment.end),
        distance: target,
    })
}

fn flatten_route(path: &[CubicBezier]) -> Vec<PathSample> {
    let Some(first) = path.first().copied() else {
        return Vec::new();
    };
    let mut points = vec![PathSample {
        segment: 0,
        t: 0.0,
        point: first.start,
        distance: 0.0,
    }];
    for (segment_index, segment) in path.iter().copied().enumerate() {
        if segment_index > 0 {
            points.push(PathSample {
                segment: segment_index,
                t: 0.0,
                point: segment.start,
                distance: 0.0,
            });
        }
        flatten_cubic(segment, segment_index, 0.0, 1.0, 0, &mut points);
    }
    let mut total_length = 0.0;
    for index in 1..points.len() {
        total_length += point_distance(points[index - 1].point, points[index].point);
        points[index].distance = total_length;
    }
    points
}

fn flatten_cubic(
    path: CubicBezier,
    segment_index: usize,
    t_start: f32,
    t_end: f32,
    depth: u8,
    output: &mut Vec<PathSample>,
) {
    let CubicBezier {
        start,
        control1,
        control2,
        end,
    } = path;
    let midpoint = |a: Point, b: Point| Point {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
    };
    let p01 = midpoint(start, control1);
    let p12 = midpoint(control1, control2);
    let p23 = midpoint(control2, end);
    let p012 = midpoint(p01, p12);
    let p123 = midpoint(p12, p23);
    let p0123 = midpoint(p012, p123);
    let flatness = distance_from_chord(control1, start, end)
        .max(distance_from_chord(control2, start, end))
        .max(point_distance(p0123, midpoint(start, end)));
    if depth >= CURVE_MAX_SUBDIVISION_DEPTH || flatness <= CURVE_FLATNESS_TOLERANCE {
        output.push(PathSample {
            segment: segment_index,
            t: t_end,
            point: end,
            distance: 0.0,
        });
        return;
    }

    let t_mid = (t_start + t_end) * 0.5;
    flatten_cubic(
        CubicBezier {
            start,
            control1: p01,
            control2: p012,
            end: p0123,
        },
        segment_index,
        t_start,
        t_mid,
        depth + 1,
        output,
    );
    flatten_cubic(
        CubicBezier {
            start: p0123,
            control1: p123,
            control2: p23,
            end,
        },
        segment_index,
        t_mid,
        t_end,
        depth + 1,
        output,
    );
}

fn distance_from_chord(point: Point, start: Point, end: Point) -> f32 {
    let line = vector_between(start, end);
    let length_squared = line.x * line.x + line.y * line.y;
    if length_squared <= EDGE_GEOMETRY_EPSILON * EDGE_GEOMETRY_EPSILON {
        return point_distance(start, point);
    }
    let relative = vector_between(start, point);
    let projection = ((relative.x * line.x + relative.y * line.y) / length_squared).clamp(0.0, 1.0);
    let closest = offset(start, line, projection);
    point_distance(closest, point)
}

fn vector_between(start: Point, end: Point) -> Point {
    Point {
        x: end.x - start.x,
        y: end.y - start.y,
    }
}

fn point_distance(start: Point, end: Point) -> f32 {
    let vector = vector_between(start, end);
    vector.x.hypot(vector.y)
}

fn offset(point: Point, direction: Point, distance: f32) -> Point {
    Point {
        x: point.x + direction.x * distance,
        y: point.y + direction.y * distance,
    }
}

fn bezier_point(path: CubicBezier, t: f32) -> Point {
    let u = 1.0 - t;
    Point {
        x: u.powi(3) * path.start.x
            + 3.0 * u.powi(2) * t * path.control1.x
            + 3.0 * u * t.powi(2) * path.control2.x
            + t.powi(3) * path.end.x,
        y: u.powi(3) * path.start.y
            + 3.0 * u.powi(2) * t * path.control1.y
            + 3.0 * u * t.powi(2) * path.control2.y
            + t.powi(3) * path.end.y,
    }
}

fn bezier_tangent(path: CubicBezier, t: f32) -> Point {
    let inverse = 1.0 - t;
    Point {
        x: 3.0 * inverse.powi(2) * (path.control1.x - path.start.x)
            + 6.0 * inverse * t * (path.control2.x - path.control1.x)
            + 3.0 * t.powi(2) * (path.end.x - path.control2.x),
        y: 3.0 * inverse.powi(2) * (path.control1.y - path.start.y)
            + 6.0 * inverse * t * (path.control2.y - path.control1.y)
            + 3.0 * t.powi(2) * (path.end.y - path.control2.y),
    }
}

fn estimated_label_width(label: &str) -> f32 {
    label.chars().count() as f32 * LABEL_FONT_SIZE * 0.56 + 8.0
}

fn label_overlaps_node(baseline: Point, width: f32, nodes: &[SceneNode]) -> bool {
    let text_bounds = Rect {
        x: baseline.x - width / 2.0 - LABEL_HALO,
        y: baseline.y - LABEL_HEIGHT - LABEL_HALO,
        width: width + LABEL_HALO * 2.0,
        height: LABEL_HEIGHT + LABEL_HALO * 2.0,
    };
    nodes
        .iter()
        .any(|node| rects_overlap(text_bounds, expand_rect(node.bounds, LABEL_NODE_CLEARANCE)))
}

fn expand_rect(rect: Rect, amount: f32) -> Rect {
    Rect {
        x: rect.x - amount,
        y: rect.y - amount,
        width: rect.width + amount * 2.0,
        height: rect.height + amount * 2.0,
    }
}

fn rects_overlap(left: Rect, right: Rect) -> bool {
    left.x < right.x + right.width
        && left.x + left.width > right.x
        && left.y < right.y + right.height
        && left.y + left.height > right.y
}
