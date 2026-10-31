use schemra_engine::{
    SceneGeometrySampler, SceneTimelineSampler, TimelineSampling, compile_timeline,
};
use schemra_ir::{
    ActionIr, HighlightEffect, HighlightOrigin, HighlightSide, NodeId, ScenarioId, ScenarioIr,
    SourceLocation, SourceSpan, TokenId, VisualId,
};
use schemra_scene::{
    CubicBezier, FlowDirection, Point, Rect, Scene, SceneEdge, SceneFont, SceneIconPack, SceneNode,
    SceneNodeLayout, SceneNodeShape, SceneStyle, SceneTextAlign, SceneTheme,
};

fn span() -> SourceSpan {
    SourceSpan::point(SourceLocation {
        offset: 0,
        line: 1,
        column: 1,
    })
}

fn scenario(actions: Vec<ActionIr>) -> ScenarioIr {
    let span = span();
    ScenarioIr {
        id: ScenarioId("scene-sampling".into()),
        id_span: span,
        visual: VisualId("network".into()),
        visual_span: span,
        tokens: Vec::new(),
        actions,
        span,
    }
}

fn movement(from: &str, to: &str, duration_ms: Option<u64>) -> ActionIr {
    let span = span();
    ActionIr::Move {
        token: TokenId("request".into()),
        token_span: span,
        from: NodeId(from.into()),
        from_span: span,
        to: NodeId(to.into()),
        to_span: span,
        duration_ms,
        span,
    }
}

fn highlight(node: &str, origin: HighlightOrigin) -> ActionIr {
    let span = span();
    ActionIr::Highlight {
        node: NodeId(node.into()),
        node_span: span,
        duration_ms: Some(1_000),
        effect: HighlightEffect::Trace,
        origin,
        span,
    }
}

fn point(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn scene(path: Vec<CubicBezier>) -> Scene {
    let node = |id: &str, x| SceneNode {
        id: id.into(),
        label: id.into(),
        icon: None,
        detail: None,
        layout: SceneNodeLayout::Horizontal,
        text_align: SceneTextAlign::Left,
        shape: SceneNodeShape::Round,
        style: SceneStyle::default(),
        bounds: Rect {
            x,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        },
        group: None,
    };
    Scene {
        title: "Timeline".into(),
        direction: FlowDirection::LeftToRight,
        theme: SceneTheme {
            font: SceneFont::Sans,
            icon_pack: SceneIconPack::Outline,
            ..SceneTheme::default()
        },
        width: 140.0,
        height: 60.0,
        groups: Vec::new(),
        nodes: vec![node("a", 0.0), node("b", 100.0)],
        texts: Vec::new(),
        images: Vec::new(),
        icons: Vec::new(),
        edges: vec![SceneEdge {
            id: Some("a-to-b".into()),
            source_id: None,
            reciprocal_edge_index: None,
            from: "a".into(),
            to: "b".into(),
            arrow_at_start: false,
            arrow_at_end: true,
            label: None,
            style: SceneStyle::default(),
            group: None,
            path,
            route_diagnostic: None,
        }],
    }
}

fn line() -> Vec<CubicBezier> {
    vec![CubicBezier {
        start: point(20.0, 20.0),
        control1: point(46.666_668, 20.0),
        control2: point(73.333_336, 20.0),
        end: point(100.0, 20.0),
    }]
}

struct SamplingInput {
    scene: Scene,
    timeline: schemra_ir::Timeline,
}

impl SamplingInput {
    fn sampler(&self) -> SceneTimelineSampler<'_> {
        SceneTimelineSampler::new(&self.scene, &self.timeline)
    }
}

fn sampler(actions: Vec<ActionIr>) -> SamplingInput {
    SamplingInput {
        scene: scene(line()),
        timeline: compile_timeline(&scenario(actions)).unwrap(),
    }
}

#[test]
fn forward_marker_samples_the_resolved_link_and_is_consumed_at_arrival() {
    let input = sampler(vec![movement("a", "b", Some(1_000))]);
    let sampler = input.sampler();
    let frame = sampler.sample(250);

    assert_eq!(frame.tokens.len(), 1);
    assert!((frame.tokens[0].x - 40.0).abs() < 0.1);
    assert_eq!(frame.active_edges[0].traversal_direction, 1);
    assert!(sampler.sample(1_000).tokens.is_empty());
}

#[test]
fn simultaneous_same_direction_markers_separate_smoothly_on_the_link() {
    let span = span();
    let input = SamplingInput {
        scene: scene(line()),
        timeline: compile_timeline(&scenario(vec![ActionIr::Parallel {
            actions: vec![
                movement("a", "b", Some(1_000)),
                movement("a", "b", Some(1_000)),
            ],
            span,
        }]))
        .unwrap(),
    };
    let frame = input.sampler().sample(500);

    assert_eq!(frame.tokens.len(), 2);
    assert_eq!(frame.active_edges.len(), 2);
    assert!((frame.tokens[0].x - frame.tokens[1].x).abs() < 0.1);
    assert!((frame.tokens[0].y - frame.tokens[1].y).abs() > 20.0);
    assert!(((frame.tokens[0].y + frame.tokens[1].y) / 2.0 - 20.0).abs() < 0.1);
}

#[test]
fn opposing_markers_use_opposite_lanes_on_the_shared_link() {
    let span = span();
    let input = SamplingInput {
        scene: scene(line()),
        timeline: compile_timeline(&scenario(vec![ActionIr::Parallel {
            actions: vec![
                movement("a", "b", Some(1_000)),
                movement("b", "a", Some(1_000)),
            ],
            span,
        }]))
        .unwrap(),
    };
    let frame = input.sampler().sample(500);

    assert_eq!(frame.tokens.len(), 2);
    assert!((frame.tokens[0].x - frame.tokens[1].x).abs() < 0.1);
    assert!((frame.tokens[0].y - frame.tokens[1].y).abs() > 20.0);
    assert!(((frame.tokens[0].y + frame.tokens[1].y) / 2.0 - 20.0).abs() < 0.1);
}

#[test]
fn crowded_markers_follow_the_curve_normal() {
    let span = span();
    let scene = scene(vec![CubicBezier {
        start: point(20.0, 20.0),
        control1: point(20.0, 60.0),
        control2: point(100.0, 60.0),
        end: point(100.0, 20.0),
    }]);
    let timeline = compile_timeline(&scenario(vec![ActionIr::Parallel {
        actions: vec![
            movement("a", "b", Some(1_000)),
            movement("a", "b", Some(1_000)),
        ],
        span,
    }]))
    .unwrap();
    let frame = SceneTimelineSampler::new(&scene, &timeline).sample(500);

    assert_eq!(frame.tokens.len(), 2);
    let midpoint_x = (frame.tokens[0].x + frame.tokens[1].x) / 2.0;
    let midpoint_y = (frame.tokens[0].y + frame.tokens[1].y) / 2.0;
    let separation =
        (frame.tokens[0].x - frame.tokens[1].x).hypot(frame.tokens[0].y - frame.tokens[1].y);

    assert!((midpoint_x - 60.0).abs() < 0.2);
    assert!((midpoint_y - 50.0).abs() < 0.2);
    assert!(separation > 20.0);
}

#[test]
fn cached_scene_geometry_can_sample_multiple_timelines() {
    let scene = scene(line());
    let forward = compile_timeline(&scenario(vec![movement("a", "b", Some(1_000))])).unwrap();
    let reverse = compile_timeline(&scenario(vec![movement("b", "a", Some(1_000))])).unwrap();
    let geometry = SceneGeometrySampler::new(&scene);

    assert_eq!(
        geometry.sample(&forward, 250),
        SceneTimelineSampler::new(&scene, &forward).sample(250)
    );
    assert_eq!(
        geometry.sample(&reverse, 250),
        SceneTimelineSampler::new(&scene, &reverse).sample(250)
    );
}

#[test]
fn reverse_marker_samples_the_same_link_in_reverse() {
    let input = sampler(vec![movement("b", "a", Some(1_000))]);
    let sampler = input.sampler();
    let frame = sampler.sample(250);

    assert!((frame.tokens[0].x - 80.0).abs() < 0.1);
    assert_eq!(frame.active_edges[0].traversal_direction, -1);
}

#[test]
fn wait_keeps_the_marker_hidden_until_the_move_starts() {
    let wait = ActionIr::Wait {
        duration_ms: 250,
        span: span(),
    };
    let input = sampler(vec![wait, movement("a", "b", Some(1_000))]);
    let sampler = input.sampler();

    assert!(sampler.sample(249).tokens.is_empty());
    assert!((sampler.sample(250).tokens[0].x - 20.0).abs() < 0.1);
}

#[test]
fn marker_position_tracks_the_cubic_curve_instead_of_its_endpoint_chord() {
    let curve = vec![CubicBezier {
        start: point(20.0, 20.0),
        control1: point(20.0, 60.0),
        control2: point(100.0, 60.0),
        end: point(100.0, 20.0),
    }];
    let scene = scene(curve);
    let timeline = compile_timeline(&scenario(vec![movement("a", "b", Some(1_000))])).unwrap();
    let sampler = SceneTimelineSampler::new(&scene, &timeline);
    let frame = sampler.sample(500);

    assert!((frame.tokens[0].x - 60.0).abs() < 0.2);
    assert!((frame.tokens[0].y - 50.0).abs() < 0.2);
}

#[test]
fn sampled_marker_stays_on_the_exact_cubic_between_arc_length_samples() {
    let curve = vec![CubicBezier {
        start: point(20.0, 20.0),
        control1: point(20.0, 60.0),
        control2: point(100.0, 60.0),
        end: point(100.0, 20.0),
    }];
    let scene = scene(curve);
    let timeline = compile_timeline(&scenario(vec![movement("a", "b", Some(1_000))])).unwrap();
    let sampler = SceneTimelineSampler::new(&scene, &timeline);
    let marker = &sampler.sample(250).tokens[0];

    let normalized_x = (marker.x - 20.0) / 80.0;
    let mut lower = 0.0_f32;
    let mut upper = 1.0_f32;
    for _ in 0..32 {
        let parameter = (lower + upper) * 0.5;
        let curve_x = 3.0 * parameter.powi(2) - 2.0 * parameter.powi(3);
        if curve_x < normalized_x {
            lower = parameter;
        } else {
            upper = parameter;
        }
    }
    let parameter = (lower + upper) * 0.5;
    let expected_y = 20.0 + 120.0 * parameter * (1.0 - parameter);

    assert!((marker.y - expected_y).abs() < 0.01);
}

#[test]
fn marker_traverses_connected_cubic_segments_as_one_route() {
    let path = vec![
        CubicBezier {
            start: point(20.0, 20.0),
            control1: point(33.333_332, 20.0),
            control2: point(46.666_668, 20.0),
            end: point(60.0, 20.0),
        },
        CubicBezier {
            start: point(60.0, 20.0),
            control1: point(73.333_336, 20.0),
            control2: point(86.666_664, 20.0),
            end: point(100.0, 20.0),
        },
    ];
    let scene = scene(path);
    let timeline = compile_timeline(&scenario(vec![movement("a", "b", Some(1_000))])).unwrap();
    let sampler = SceneTimelineSampler::new(&scene, &timeline);

    assert!((sampler.sample(250).tokens[0].x - 40.0).abs() < 0.1);
    assert!((sampler.sample(500).tokens[0].x - 60.0).abs() < 0.1);
    assert!((sampler.sample(750).tokens[0].x - 80.0).abs() < 0.1);
}

#[test]
fn marker_is_not_sampled_across_a_disconnected_route_gap() {
    let disconnected_path = vec![
        CubicBezier {
            start: point(20.0, 20.0),
            control1: point(28.0, 20.0),
            control2: point(37.0, 20.0),
            end: point(45.0, 20.0),
        },
        CubicBezier {
            start: point(75.0, 20.0),
            control1: point(83.0, 20.0),
            control2: point(92.0, 20.0),
            end: point(100.0, 20.0),
        },
    ];
    let scene = scene(disconnected_path);
    let timeline = compile_timeline(&scenario(vec![movement("a", "b", Some(1_000))])).unwrap();
    let sampler = SceneTimelineSampler::new(&scene, &timeline);

    assert!(sampler.sample(500).tokens.is_empty());
}

#[test]
fn all_explicit_highlight_sides_resolve_to_the_matching_node_border_port() {
    for (side, expected) in [
        (HighlightSide::Left, [100.0, 20.0]),
        (HighlightSide::Right, [120.0, 20.0]),
        (HighlightSide::Top, [110.0, 10.0]),
        (HighlightSide::Bottom, [110.0, 30.0]),
    ] {
        let input = sampler(vec![highlight("b", HighlightOrigin::Side { side })]);
        let sampler = input.sampler();

        assert_eq!(sampler.sample(500).highlights[0].origin, Some(expected));
    }
}

#[test]
fn arrival_highlight_origin_resolves_to_the_destination_link_port() {
    let input = sampler(vec![highlight(
        "b",
        HighlightOrigin::Arrival {
            from: NodeId("a".into()),
            to: NodeId("b".into()),
        },
    )]);
    let sampler = input.sampler();

    assert_eq!(
        sampler.sample(500).highlights[0].origin,
        Some([100.0, 20.0])
    );
}

#[test]
fn reverse_arrival_highlight_origin_resolves_to_the_reverse_destination_port() {
    let input = sampler(vec![highlight(
        "a",
        HighlightOrigin::Arrival {
            from: NodeId("b".into()),
            to: NodeId("a".into()),
        },
    )]);
    let sampler = input.sampler();

    assert_eq!(sampler.sample(500).highlights[0].origin, Some([20.0, 20.0]));
}

#[test]
fn default_highlight_origin_resolves_to_the_node_top_center() {
    let input = sampler(vec![highlight("b", HighlightOrigin::Default)]);
    let sampler = input.sampler();

    assert_eq!(
        sampler.sample(500).highlights[0].origin,
        Some([110.0, 10.0])
    );
}

#[test]
fn cached_sampling_matches_uncached_random_access_across_seeks() {
    let scene = scene(line());
    let timeline = compile_timeline(&scenario(vec![
        movement("a", "b", Some(1_000)),
        highlight(
            "b",
            HighlightOrigin::Arrival {
                from: NodeId("a".into()),
                to: NodeId("b".into()),
            },
        ),
    ]))
    .unwrap();
    let sampler = SceneTimelineSampler::new(&scene, &timeline);

    for time_ms in [900, 0, 400, 1_000, 400] {
        assert_eq!(
            sampler.sample(time_ms),
            timeline.sample_scene(&scene, time_ms)
        );
    }
}
