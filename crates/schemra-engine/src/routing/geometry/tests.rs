use super::{direct_connector, path_length, path_obstacle_intersections};
use schemra_scene::{CubicBezier, Point, Rect};

#[test]
fn measures_a_direct_cubic_as_its_exact_chord_length() {
    let path = [direct_connector(
        Point { x: 0.0, y: 0.0 },
        Point { x: 100.0, y: 0.0 },
    )];

    assert!((path_length(&path) - 100.0).abs() < f32::EPSILON);
}

#[test]
fn measures_curved_paths_beyond_their_endpoint_chord() {
    let path = [CubicBezier {
        start: Point { x: 0.0, y: 0.0 },
        control1: Point { x: 25.0, y: 80.0 },
        control2: Point { x: 75.0, y: 80.0 },
        end: Point { x: 100.0, y: 0.0 },
    }];

    let length = path_length(&path);
    assert!(length > 100.0);
    assert!(length < 200.0);
}

#[test]
fn detects_curved_obstacle_intersections_that_the_endpoint_chord_misses() {
    let path = [CubicBezier {
        start: Point { x: 0.0, y: 0.0 },
        control1: Point { x: 25.0, y: 80.0 },
        control2: Point { x: 75.0, y: 80.0 },
        end: Point { x: 100.0, y: 0.0 },
    }];
    let obstacles = [Rect {
        x: 45.0,
        y: 54.0,
        width: 10.0,
        height: 12.0,
    }];

    assert_eq!(path_obstacle_intersections(&path, &obstacles), 1);
}

#[test]
fn counts_an_obstacle_once_when_multiple_segments_cross_it() {
    let path = [
        direct_connector(Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }),
        direct_connector(Point { x: 10.0, y: 0.0 }, Point { x: 20.0, y: 0.0 }),
    ];
    let obstacles = [Rect {
        x: 8.0,
        y: -1.0,
        width: 4.0,
        height: 2.0,
    }];

    assert_eq!(path_obstacle_intersections(&path, &obstacles), 1);
}

#[test]
fn invalid_geometry_has_finite_measure_and_conservative_collision_result() {
    let path = [CubicBezier {
        start: Point {
            x: f32::NAN,
            y: 0.0,
        },
        control1: Point { x: 1.0, y: 2.0 },
        control2: Point { x: 3.0, y: 2.0 },
        end: Point { x: 4.0, y: 0.0 },
    }];
    let obstacles = [Rect {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    }];

    assert_eq!(path_length(&path), f32::MAX);
    assert_eq!(path_obstacle_intersections(&path, &obstacles), 1);
}

#[test]
fn disconnected_segments_are_not_treated_as_one_path() {
    let path = [
        direct_connector(Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }),
        direct_connector(Point { x: 100.0, y: 0.0 }, Point { x: 110.0, y: 0.0 }),
    ];
    let obstacles = [Rect {
        x: 40.0,
        y: -1.0,
        width: 10.0,
        height: 2.0,
    }];

    assert_eq!(path_length(&path), f32::MAX);
    assert_eq!(path_obstacle_intersections(&path, &obstacles), 1);
}
