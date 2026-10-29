use schemra_scene::{CubicBezier, FlowDirection, Point, Rect};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    pub(super) const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Top, Self::Bottom];

    pub(super) fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Top => 2,
            Self::Bottom => 3,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
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
    let (primary_delta, secondary_delta, horizontal_flow) = match direction {
        FlowDirection::LeftToRight | FlowDirection::RightToLeft => (dx, dy, true),
        FlowDirection::TopToBottom | FlowDirection::BottomToTop => (dy, dx, false),
    };

    if primary_delta.abs() > 0.5 {
        return if horizontal_flow {
            if primary_delta > 0.0 {
                (Side::Right, Side::Left)
            } else {
                (Side::Left, Side::Right)
            }
        } else if primary_delta > 0.0 {
            (Side::Bottom, Side::Top)
        } else {
            (Side::Top, Side::Bottom)
        };
    }

    if secondary_delta.abs() > 0.5 {
        return if horizontal_flow {
            if secondary_delta > 0.0 {
                (Side::Bottom, Side::Top)
            } else {
                (Side::Top, Side::Bottom)
            }
        } else if secondary_delta > 0.0 {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        };
    }

    match direction {
        FlowDirection::LeftToRight => (Side::Right, Side::Left),
        FlowDirection::RightToLeft => (Side::Left, Side::Right),
        FlowDirection::TopToBottom => (Side::Bottom, Side::Top),
        FlowDirection::BottomToTop => (Side::Top, Side::Bottom),
    }
}

pub(super) fn port_anchor(rect: Rect, side: Side, offset: f32) -> Point {
    let rect = normalized(rect);
    let point = match side {
        Side::Left => Point {
            x: rect.x,
            y: rect.y + rect.height * 0.5 + offset,
        },
        Side::Right => Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height * 0.5 + offset,
        },
        Side::Top => Point {
            x: rect.x + rect.width * 0.5 + offset,
            y: rect.y,
        },
        Side::Bottom => Point {
            x: rect.x + rect.width * 0.5 + offset,
            y: rect.y + rect.height,
        },
    };
    Point {
        x: finite(point.x),
        y: finite(point.y),
    }
}

pub(super) fn direct_connector(start: Point, end: Point) -> CubicBezier {
    CubicBezier {
        start,
        control1: lerp(start, end, 1.0 / 3.0),
        control2: lerp(start, end, 2.0 / 3.0),
        end,
    }
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
