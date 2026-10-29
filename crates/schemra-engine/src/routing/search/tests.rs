use super::orthogonal_route;
use crate::routing::geometry::segment_intersects_rect;
use schemra_scene::{Point, Rect};

fn point(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn routes_around_an_obstacle_without_crossing_it() {
    let start = point(0.0, 20.0);
    let end = point(100.0, 20.0);
    let obstacle = rect(40.0, 0.0, 20.0, 40.0);

    let route = orthogonal_route(start, end, &[obstacle]).unwrap();

    assert_eq!(route.first(), Some(&start));
    assert_eq!(route.last(), Some(&end));
    assert!(route.len() >= 4, "the blocking rectangle requires a detour");
    assert!(
        route
            .windows(2)
            .all(|segment| !segment_intersects_rect(segment[0], segment[1], obstacle))
    );
    assert!(route.windows(2).all(|segment| {
        (segment[0].x - segment[1].x).abs() <= f32::EPSILON
            || (segment[0].y - segment[1].y).abs() <= f32::EPSILON
    }));
}

#[test]
fn equal_geometry_is_stable_when_obstacles_are_reordered() {
    let start = point(0.0, 10.0);
    let end = point(100.0, 10.0);
    let obstacles = [rect(40.0, -10.0, 20.0, 30.0), rect(70.0, 25.0, 10.0, 20.0)];

    let forward = orthogonal_route(start, end, &obstacles).unwrap();
    let reversed = orthogonal_route(start, end, &[obstacles[1], obstacles[0]]).unwrap();

    assert_eq!(forward, reversed);
}

#[test]
fn returns_the_shortest_bend_count_for_a_clear_diagonal() {
    let start = point(0.0, 0.0);
    let end = point(100.0, 50.0);

    let route = orthogonal_route(start, end, &[]).unwrap();

    assert_eq!(route.first(), Some(&start));
    assert_eq!(route.last(), Some(&end));
    assert_eq!(route.len(), 3);
    assert!(route.windows(2).all(|segment| {
        (segment[0].x - segment[1].x).abs() <= f32::EPSILON
            || (segment[0].y - segment[1].y).abs() <= f32::EPSILON
    }));
}

#[test]
fn rejects_invalid_geometry_and_endpoints_inside_obstacles() {
    assert!(orthogonal_route(point(f32::NAN, 0.0), point(10.0, 10.0), &[]).is_none());
    assert!(
        orthogonal_route(
            point(1.0, 1.0),
            point(10.0, 10.0),
            &[rect(0.0, 0.0, 4.0, 4.0)]
        )
        .is_none()
    );
}
