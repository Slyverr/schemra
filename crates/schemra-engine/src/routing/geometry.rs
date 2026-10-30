use schemra_scene::{CubicBezier, FlowDirection, Point, Rect};

const CURVE_FLATNESS: f64 = 0.35;
const MAX_FLATTEN_DEPTH: u8 = 12;
const PATH_JOIN_EPSILON: f64 = 0.5;

#[derive(Clone, Copy, Debug)]
struct DPoint {
    x: f64,
    y: f64,
}

impl From<Point> for DPoint {
    fn from(point: Point) -> Self {
        Self {
            x: f64::from(point.x),
            y: f64::from(point.y),
        }
    }
}

impl DPoint {
    fn midpoint(self, other: Self) -> Self {
        Self {
            x: (self.x + other.x) * 0.5,
            y: (self.y + other.y) * 0.5,
        }
    }

    fn to_point(self) -> Point {
        Point {
            x: self.x as f32,
            y: self.y as f32,
        }
    }

    fn distance(self, other: Self) -> f64 {
        (other.x - self.x).hypot(other.y - self.y)
    }
}

/// Cached arc-length samples for repeatedly positioning markers on a route.
#[derive(Clone, Debug)]
pub(crate) struct RouteGeometry {
    segments: Vec<CubicBezier>,
    samples: Vec<RouteSample>,
    length: f64,
}

#[derive(Clone, Copy, Debug)]
struct RouteSample {
    segment: usize,
    parameter: f64,
    point: DPoint,
    distance: f64,
}

impl RouteGeometry {
    pub(crate) fn new(path: &[CubicBezier]) -> Self {
        let mut segments = Vec::with_capacity(path.len());
        let mut samples = Vec::new();

        for (segment_index, segment) in path.iter().copied().enumerate() {
            let Some(curve_samples) = flatten_cubic_samples(segment) else {
                return Self::default();
            };
            if samples.last().zip(curve_samples.first()).is_some_and(
                |(previous, next): (&RouteSample, &CurveSample)| {
                    previous.point.distance(next.point) > PATH_JOIN_EPSILON
                },
            ) {
                return Self::default();
            }

            for sample in curve_samples {
                let distance = samples.last().map_or(0.0, |previous: &RouteSample| {
                    previous.distance + previous.point.distance(sample.point)
                });
                samples.push(RouteSample {
                    segment: segment_index,
                    parameter: sample.parameter,
                    point: sample.point,
                    distance,
                });
            }
            segments.push(segment);
        }
        let length = samples.last().map_or(0.0, |sample| sample.distance);
        Self {
            segments,
            samples,
            length,
        }
    }

    pub(crate) fn point_at_progress(&self, progress: f32) -> Option<Point> {
        let first = self.samples.first()?;
        if self.samples.len() == 1 || self.length <= f64::EPSILON {
            return Some(first.point.to_point());
        }

        let target = self.length * f64::from(progress.clamp(0.0, 1.0));
        let after_index = self
            .samples
            .partition_point(|sample| sample.distance < target);
        if after_index == 0 {
            return Some(first.point.to_point());
        }
        let Some(after) = self.samples.get(after_index) else {
            return self.samples.last().map(|sample| sample.point.to_point());
        };
        let before = &self.samples[after_index - 1];
        let interval_length = after.distance - before.distance;
        let amount = if interval_length <= f64::EPSILON {
            0.0
        } else {
            (target - before.distance) / interval_length
        };
        let parameter = before.parameter + (after.parameter - before.parameter) * amount;
        self.segments
            .get(after.segment)
            .map(|segment| evaluate_cubic(*segment, parameter).to_point())
    }
}

impl Default for RouteGeometry {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            samples: Vec::new(),
            length: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
struct CurveSample {
    parameter: f64,
    point: DPoint,
}

fn evaluate_cubic(cubic: CubicBezier, parameter: f64) -> DPoint {
    let [start, control1, control2, end]: [DPoint; 4] = [
        cubic.start.into(),
        cubic.control1.into(),
        cubic.control2.into(),
        cubic.end.into(),
    ];
    let inverse = 1.0 - parameter;
    let start_weight = inverse * inverse * inverse;
    let control1_weight = 3.0 * inverse * inverse * parameter;
    let control2_weight = 3.0 * inverse * parameter * parameter;
    let end_weight = parameter * parameter * parameter;
    DPoint {
        x: start.x * start_weight
            + control1.x * control1_weight
            + control2.x * control2_weight
            + end.x * end_weight,
        y: start.y * start_weight
            + control1.y * control1_weight
            + control2.y * control2_weight
            + end.y * end_weight,
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
    pub(super) fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

pub(super) fn center(rect: Rect) -> Point {
    Point {
        x: rect.x + rect.width * 0.5,
        y: rect.y + rect.height * 0.5,
    }
}

pub(super) fn natural_sides(source: Rect, target: Rect, direction: FlowDirection) -> (Side, Side) {
    let source_center = center(source);
    let target_center = center(target);
    let dx = target_center.x - source_center.x;
    let dy = target_center.y - source_center.y;
    let horizontal_clearance = dx.abs() - (source.width + target.width) * 0.5;
    let vertical_clearance = dy.abs() - (source.height + target.height) * 0.5;
    let (flow_x, flow_y) = match direction {
        FlowDirection::LeftToRight => (1.0, 0.0),
        FlowDirection::RightToLeft => (-1.0, 0.0),
        FlowDirection::TopToBottom => (0.0, 1.0),
        FlowDirection::BottomToTop => (0.0, -1.0),
    };
    let horizontal = if (horizontal_clearance - vertical_clearance).abs() > 0.5 {
        horizontal_clearance > vertical_clearance
    } else {
        matches!(
            direction,
            FlowDirection::LeftToRight | FlowDirection::RightToLeft
        )
    };

    if horizontal {
        let target_is_right = if dx.abs() > 0.5 {
            dx > 0.0
        } else {
            flow_x >= 0.0
        };
        if target_is_right {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        }
    } else {
        let target_is_below = if dy.abs() > 0.5 {
            dy > 0.0
        } else {
            flow_y >= 0.0
        };
        if target_is_below {
            (Side::Bottom, Side::Top)
        } else {
            (Side::Top, Side::Bottom)
        }
    }
}

pub(super) fn side_anchor(rect: Rect, side: Side) -> Point {
    let rect = normalized(rect);
    let point = match side {
        Side::Left => Point {
            x: rect.x,
            y: rect.y + rect.height * 0.5,
        },
        Side::Right => Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height * 0.5,
        },
        Side::Top => Point {
            x: rect.x + rect.width * 0.5,
            y: rect.y,
        },
        Side::Bottom => Point {
            x: rect.x + rect.width * 0.5,
            y: rect.y + rect.height,
        },
    };
    Point {
        x: finite(point.x),
        y: finite(point.y),
    }
}

pub(super) fn self_loop_route(bounds: Rect, reach: f32) -> Vec<CubicBezier> {
    let start = side_anchor(bounds, Side::Right);
    let above = Point {
        x: start.x + reach,
        y: bounds.y - reach,
    };
    let below = Point {
        x: start.x + reach,
        y: bounds.y + bounds.height + reach,
    };
    vec![
        CubicBezier {
            start,
            control1: Point {
                x: start.x + reach,
                y: start.y,
            },
            control2: Point {
                x: above.x,
                y: above.y + reach,
            },
            end: above,
        },
        CubicBezier {
            start: above,
            control1: Point {
                x: above.x + reach,
                y: above.y - reach,
            },
            control2: Point {
                x: below.x + reach,
                y: below.y + reach,
            },
            end: below,
        },
        CubicBezier {
            start: below,
            control1: Point {
                x: below.x,
                y: below.y - reach,
            },
            control2: Point {
                x: start.x + reach,
                y: start.y,
            },
            end: start,
        },
    ]
}

pub(super) fn direct_connector(start: Point, end: Point) -> CubicBezier {
    CubicBezier {
        start,
        control1: lerp(start, end, 1.0 / 3.0),
        control2: lerp(start, end, 2.0 / 3.0),
        end,
    }
}

pub(super) fn polyline_connectors(points: &[Point]) -> Vec<CubicBezier> {
    points
        .windows(2)
        .map(|pair| direct_connector(pair[0], pair[1]))
        .collect()
}

/// Measure a cubic path by adaptively flattening its segments. Invalid paths
/// return a finite sentinel so diagnostics never contain NaN or infinity.
pub(super) fn path_length(path: &[CubicBezier]) -> f32 {
    let Some(points) = flatten_path(path) else {
        return f32::MAX;
    };
    let total = points
        .windows(2)
        .map(|pair| point_distance64(pair[0], pair[1]))
        .sum::<f64>();
    if !total.is_finite() || total >= f64::from(f32::MAX) {
        return f32::MAX;
    }
    total as f32
}

/// Count obstacle rectangles intersected by any flattened part of a cubic
/// path. Invalid paths or obstacles conservatively count as an intersection.
pub(super) fn path_obstacle_intersections(path: &[CubicBezier], obstacles: &[Rect]) -> usize {
    if path.is_empty() {
        return 0;
    }
    let Some(points) = flatten_path(path) else {
        return obstacles.len().max(1);
    };
    obstacles
        .iter()
        .filter(|obstacle| {
            let Some(rect) = checked_rect(**obstacle) else {
                return true;
            };
            points
                .windows(2)
                .any(|pair| segment_intersects_rect(pair[0], pair[1], rect))
        })
        .count()
}

pub(super) fn segment_intersects_rect(start: Point, end: Point, rect: Rect) -> bool {
    let rect = normalized(rect);
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let mut near = 0.0f32;
    let mut far = 1.0f32;
    for (origin, delta, minimum, maximum) in [
        (start.x, dx, rect.x, rect.x + rect.width),
        (start.y, dy, rect.y, rect.y + rect.height),
    ] {
        if delta.abs() <= f32::EPSILON {
            if origin <= minimum || origin >= maximum {
                return false;
            }
            continue;
        }
        let first = (minimum - origin) / delta;
        let second = (maximum - origin) / delta;
        near = near.max(first.min(second));
        far = far.min(first.max(second));
        if near >= far {
            return false;
        }
    }
    far > 0.001 && near < 0.999
}

pub(super) fn segments_conflict(a: Point, b: Point, c: Point, d: Point) -> bool {
    let ab = Point {
        x: b.x - a.x,
        y: b.y - a.y,
    };
    let cd = Point {
        x: d.x - c.x,
        y: d.y - c.y,
    };
    let denominator = cross(ab, cd);
    let ac = Point {
        x: c.x - a.x,
        y: c.y - a.y,
    };
    if denominator.abs() <= 0.001 {
        if cross(ac, ab).abs() > 0.001 {
            return false;
        }
        let use_x = ab.x.abs() >= ab.y.abs();
        let (a0, a1, c0, c1) = if use_x {
            (a.x.min(b.x), a.x.max(b.x), c.x.min(d.x), c.x.max(d.x))
        } else {
            (a.y.min(b.y), a.y.max(b.y), c.y.min(d.y), c.y.max(d.y))
        };
        return a1.min(c1) - a0.max(c0) > 1.0;
    }
    let t = cross(ac, cd) / denominator;
    let u = cross(ac, ab) / denominator;
    (0.001..0.999).contains(&t) && (0.001..0.999).contains(&u)
}

pub(super) fn paths_conflict(left: &[CubicBezier], right: &[CubicBezier]) -> bool {
    left.iter().any(|left_segment| {
        right.iter().any(|right_segment| {
            segments_conflict(
                left_segment.start,
                left_segment.end,
                right_segment.start,
                right_segment.end,
            )
        })
    })
}

fn normalized(rect: Rect) -> Rect {
    let x1 = finite(rect.x);
    let y1 = finite(rect.y);
    let x2 = finite(rect.x + rect.width);
    let y2 = finite(rect.y + rect.height);
    Rect {
        x: x1.min(x2),
        y: y1.min(y2),
        width: (x2 - x1).abs(),
        height: (y2 - y1).abs(),
    }
}

fn flatten_path(path: &[CubicBezier]) -> Option<Vec<Point>> {
    let mut flattened = Vec::new();
    for segment in path {
        let points = flatten_cubic(*segment)?;
        if flattened
            .last()
            .zip(points.first())
            .is_some_and(|(previous, next)| point_distance64(*previous, *next) > PATH_JOIN_EPSILON)
        {
            return None;
        }
        let skip_first = flattened.last() == points.first();
        flattened.extend(points.into_iter().skip(usize::from(skip_first)));
    }
    Some(flattened)
}

fn flatten_cubic(cubic: CubicBezier) -> Option<Vec<Point>> {
    flatten_cubic_samples(cubic).map(|samples| {
        samples
            .into_iter()
            .map(|sample| sample.point.to_point())
            .collect()
    })
}

fn flatten_cubic_samples(cubic: CubicBezier) -> Option<Vec<CurveSample>> {
    let points = [cubic.start, cubic.control1, cubic.control2, cubic.end];
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return None;
    }
    let points = points.map(DPoint::from);
    let mut flattened = vec![CurveSample {
        parameter: 0.0,
        point: points[0],
    }];
    flatten_cubic_recursive(points, 0.0, 1.0, 0, &mut flattened);
    Some(flattened)
}

fn flatten_cubic_recursive(
    points: [DPoint; 4],
    start_parameter: f64,
    end_parameter: f64,
    depth: u8,
    output: &mut Vec<CurveSample>,
) {
    if depth >= MAX_FLATTEN_DEPTH || cubic_flatness(points) <= CURVE_FLATNESS {
        output.push(CurveSample {
            parameter: end_parameter,
            point: points[3],
        });
        return;
    }

    let p01 = points[0].midpoint(points[1]);
    let p12 = points[1].midpoint(points[2]);
    let p23 = points[2].midpoint(points[3]);
    let p012 = p01.midpoint(p12);
    let p123 = p12.midpoint(p23);
    let midpoint = p012.midpoint(p123);
    let middle_parameter = (start_parameter + end_parameter) * 0.5;
    flatten_cubic_recursive(
        [points[0], p01, p012, midpoint],
        start_parameter,
        middle_parameter,
        depth + 1,
        output,
    );
    flatten_cubic_recursive(
        [midpoint, p123, p23, points[3]],
        middle_parameter,
        end_parameter,
        depth + 1,
        output,
    );
}

fn cubic_flatness(points: [DPoint; 4]) -> f64 {
    point_line_distance(points[1], points[0], points[3])
        .max(point_line_distance(points[2], points[0], points[3]))
}

fn point_line_distance(point: DPoint, start: DPoint, end: DPoint) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON {
        (point.x - start.x).hypot(point.y - start.y)
    } else {
        ((point.x - start.x) * dy - (point.y - start.y) * dx).abs() / length
    }
}

fn point_distance64(left: Point, right: Point) -> f64 {
    (f64::from(right.x) - f64::from(left.x)).hypot(f64::from(right.y) - f64::from(left.y))
}

fn checked_rect(rect: Rect) -> Option<Rect> {
    let right = rect.x + rect.width;
    let bottom = rect.y + rect.height;
    (rect.x.is_finite() && rect.y.is_finite() && right.is_finite() && bottom.is_finite())
        .then(|| normalized(rect))
}

#[cfg(test)]
mod tests;

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn lerp(start: Point, end: Point, amount: f32) -> Point {
    Point {
        x: start.x + (end.x - start.x) * amount,
        y: start.y + (end.y - start.y) * amount,
    }
}

fn cross(left: Point, right: Point) -> f32 {
    left.x * right.y - left.y * right.x
}
