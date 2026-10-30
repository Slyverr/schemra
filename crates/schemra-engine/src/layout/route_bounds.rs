use schemra_scene::{CubicBezier, Point, SceneEdge, SceneGroup, SceneNode};

use super::CANVAS_PADDING;

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl Bounds {
    fn include(&mut self, other: Self) {
        self.min_x = self.min_x.min(other.min_x);
        self.max_x = self.max_x.max(other.max_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_y = self.max_y.max(other.max_y);
    }
}

pub(super) fn fit_scene(
    nodes: &mut [SceneNode],
    groups: &mut [SceneGroup],
    edges: &mut [SceneEdge],
    rank_extent: f32,
) -> (f32, f32) {
    let max_x = nodes
        .iter()
        .map(|node| node.bounds.x + node.bounds.width)
        .chain(
            groups
                .iter()
                .map(|group| group.bounds.x + group.bounds.width),
        )
        .reduce(f32::max)
        .unwrap_or(CANVAS_PADDING);
    let max_y = nodes
        .iter()
        .map(|node| node.bounds.y + node.bounds.height)
        .chain(
            groups
                .iter()
                .map(|group| group.bounds.y + group.bounds.height),
        )
        .reduce(f32::max)
        .unwrap_or(CANVAS_PADDING);
    let base_width = (max_x + CANVAS_PADDING).max(rank_extent + CANVAS_PADDING * 2.0);
    let base_height = max_y + CANVAS_PADDING;
    let path_bounds = edges
        .iter()
        .flat_map(|edge| edge.path.iter().copied().map(cubic_bounds))
        .reduce(|mut left, right| {
            left.include(right);
            left
        });
    let offset = path_bounds.map_or(Point::default(), |bounds| Point {
        x: (CANVAS_PADDING - bounds.min_x).max(0.0),
        y: (CANVAS_PADDING - bounds.min_y).max(0.0),
    });
    if offset.x > 0.0 || offset.y > 0.0 {
        for node in nodes {
            translate_rect(&mut node.bounds, offset);
        }
        for group in groups {
            translate_rect(&mut group.bounds, offset);
        }
        for edge in edges {
            for segment in &mut edge.path {
                translate_point(&mut segment.start, offset);
                translate_point(&mut segment.control1, offset);
                translate_point(&mut segment.control2, offset);
                translate_point(&mut segment.end, offset);
            }
        }
    }
    let width = path_bounds.map_or(base_width, |bounds| {
        (base_width + offset.x).max(bounds.max_x + offset.x + CANVAS_PADDING)
    });
    let height = path_bounds.map_or(base_height, |bounds| {
        (base_height + offset.y).max(bounds.max_y + offset.y + CANVAS_PADDING)
    });
    (width, height)
}

fn cubic_bounds(path: CubicBezier) -> Bounds {
    let (min_x, max_x) =
        cubic_axis_bounds(path.start.x, path.control1.x, path.control2.x, path.end.x);
    let (min_y, max_y) =
        cubic_axis_bounds(path.start.y, path.control1.y, path.control2.y, path.end.y);
    Bounds {
        min_x,
        max_x,
        min_y,
        max_y,
    }
}

fn cubic_axis_bounds(start: f32, control1: f32, control2: f32, end: f32) -> (f32, f32) {
    let mut min = start.min(end);
    let mut max = start.max(end);
    let quadratic = -start + 3.0 * control1 - 3.0 * control2 + end;
    let linear = 2.0 * (start - 2.0 * control1 + control2);
    let constant = control1 - start;
    let roots = if quadratic.abs() < 1e-6 {
        if linear.abs() < 1e-6 {
            [None, None]
        } else {
            [Some(-constant / linear), None]
        }
    } else {
        let discriminant = linear * linear - 4.0 * quadratic * constant;
        if discriminant < 0.0 {
            [None, None]
        } else {
            let root = discriminant.sqrt();
            [
                Some((-linear - root) / (2.0 * quadratic)),
                Some((-linear + root) / (2.0 * quadratic)),
            ]
        }
    };
    for t in roots.into_iter().flatten().filter(|t| *t > 0.0 && *t < 1.0) {
        let value = cubic_axis_value(start, control1, control2, end, t);
        min = min.min(value);
        max = max.max(value);
    }
    (min, max)
}

fn cubic_axis_value(start: f32, control1: f32, control2: f32, end: f32, t: f32) -> f32 {
    let one_minus_t = 1.0 - t;
    one_minus_t.powi(3) * start
        + 3.0 * one_minus_t.powi(2) * t * control1
        + 3.0 * one_minus_t * t.powi(2) * control2
        + t.powi(3) * end
}

fn translate_rect(rect: &mut schemra_scene::Rect, offset: Point) {
    rect.x += offset.x;
    rect.y += offset.y;
}

fn translate_point(point: &mut Point, offset: Point) {
    point.x += offset.x;
    point.y += offset.y;
}
