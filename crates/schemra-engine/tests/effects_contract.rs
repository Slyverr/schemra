use schemra_engine::{TimelineSampling, compile_timeline};
use schemra_ir::{
    ActionIr, HighlightEffect, HighlightOrigin, HighlightSide, NodeId, ScenarioId, ScenarioIr,
    SourceLocation, SourceSpan, VisualId,
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
        id: ScenarioId("effects".into()),
        id_span: span,
        visual: VisualId("network".into()),
        visual_span: span,
        tokens: Vec::new(),
        actions,
        span,
    }
}

fn highlight(effect: HighlightEffect, origin: HighlightOrigin) -> ActionIr {
    let span = span();
    ActionIr::Highlight {
        node: NodeId("proxy".into()),
        node_span: span,
        duration_ms: Some(1_000),
        effect,
        origin,
        span,
    }
}

fn sample(effect: HighlightEffect, time_ms: u64) -> schemra_ir::TimelineFrame {
    compile_timeline(&scenario(vec![highlight(effect, HighlightOrigin::Default)]))
        .unwrap()
        .sample(time_ms)
}

#[test]
fn trace_starts_at_the_declared_origin_with_full_opacity() {
    let frame = sample(HighlightEffect::Trace, 0);

    assert_eq!(frame.highlights[0].progress_micros, 0);
    assert_eq!(frame.highlights[0].pulse_opacity, 1.0);
}

#[test]
fn trace_reaches_the_full_border_by_eighty_percent_of_its_duration() {
    let frame = sample(HighlightEffect::Trace, 800);

    assert_eq!(frame.highlights[0].progress_micros, 1_000_000);
    assert_eq!(frame.highlights[0].pulse_opacity, 1.0);
}

#[test]
fn trace_fades_during_its_final_fifth() {
    let frame = sample(HighlightEffect::Trace, 900);

    assert_eq!(frame.highlights[0].progress_micros, 1_000_000);
    assert!((frame.highlights[0].pulse_opacity - 0.5).abs() < 1e-6);
}

#[test]
fn pulse_starts_at_its_base_opacity() {
    let frame = sample(HighlightEffect::Pulse, 0);

    assert_eq!(frame.highlights[0].progress_micros, 0);
    assert!((frame.highlights[0].pulse_opacity - 0.35).abs() < 1e-6);
}

#[test]
fn pulse_reaches_full_opacity_at_its_midpoint() {
    let frame = sample(HighlightEffect::Pulse, 500);

    assert_eq!(frame.highlights[0].progress_micros, 500_000);
    assert!((frame.highlights[0].pulse_opacity - 1.0).abs() < 1e-6);
}

#[test]
fn highlight_sampling_preserves_the_declared_port_origin() {
    let timeline = compile_timeline(&scenario(vec![highlight(
        HighlightEffect::Trace,
        HighlightOrigin::Side {
            side: HighlightSide::Left,
        },
    )]))
    .unwrap();

    assert_eq!(
        timeline.sample(250).highlights[0].origin,
        HighlightOrigin::Side {
            side: HighlightSide::Left,
        }
    );
}

#[test]
fn simultaneous_highlights_remain_independent_instances() {
    let timeline = compile_timeline(&scenario(vec![ActionIr::Parallel {
        actions: vec![
            highlight(HighlightEffect::Trace, HighlightOrigin::Default),
            highlight(
                HighlightEffect::Pulse,
                HighlightOrigin::Side {
                    side: HighlightSide::Bottom,
                },
            ),
        ],
        span: span(),
    }]))
    .unwrap();
    let frame = timeline.sample(500);

    assert_eq!(frame.highlighted_nodes, vec![NodeId("proxy".into())]);
    assert_eq!(frame.highlights.len(), 2);
    assert_ne!(
        frame.highlights[0].instance_id,
        frame.highlights[1].instance_id
    );
    assert_eq!(frame.highlights[0].effect, HighlightEffect::Trace);
    assert_eq!(frame.highlights[1].effect, HighlightEffect::Pulse);
}

#[test]
fn highlight_is_inactive_at_its_exclusive_end_time() {
    let frame = sample(HighlightEffect::Pulse, 1_000);

    assert!(frame.highlights.is_empty());
}
