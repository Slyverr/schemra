use schemra_engine::{TimelineSampling, compile_timeline};
use schemra_ir::{
    ActionIr, HighlightEffect, HighlightOrigin, NodeId, ScenarioId, ScenarioIr, SourceLocation,
    SourceSpan, TokenId, TokenPosition, VisualId,
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
        id: ScenarioId("contract".into()),
        id_span: span,
        visual: VisualId("network".into()),
        visual_span: span,
        tokens: Vec::new(),
        actions,
        span,
    }
}

fn movement(token: &str, from: &str, to: &str, duration_ms: Option<u64>) -> ActionIr {
    let span = span();
    ActionIr::Move {
        token: TokenId(token.into()),
        token_span: span,
        from: NodeId(from.into()),
        from_span: span,
        to: NodeId(to.into()),
        to_span: span,
        duration_ms,
        span,
    }
}

#[test]
fn parallel_branches_start_together_and_advance_by_the_longest_duration() {
    let result = compile_timeline(&scenario(vec![ActionIr::Parallel {
        actions: vec![
            movement("packet", "client", "proxy", Some(800)),
            movement("packet", "proxy", "client", Some(1_200)),
        ],
        span: span(),
    }]))
    .unwrap();

    assert_eq!(result.duration_ms, 1_200);
    assert!(result.events.iter().all(|event| event.start_ms == 0));
    let frame = result.sample(400);
    assert_eq!(frame.tokens.len(), 2);
    assert_eq!(frame.tokens[0].token, frame.tokens[1].token);
    assert_ne!(frame.tokens[0].instance_id, frame.tokens[1].instance_id);
    assert!(matches!(
        &frame.tokens[0].position,
        TokenPosition::Moving {
            progress_micros: 500_000,
            ..
        }
    ));
}

#[test]
fn wait_delays_movement_and_movement_is_consumed_at_its_end() {
    let result = compile_timeline(&scenario(vec![
        ActionIr::Wait {
            duration_ms: 250,
            span: span(),
        },
        movement("request", "client", "proxy", Some(400)),
    ]))
    .unwrap();

    assert_eq!(result.duration_ms, 650);
    assert!(result.sample(249).tokens.is_empty());
    assert_eq!(result.sample(250).tokens.len(), 1);
    assert_eq!(result.sample(649).tokens.len(), 1);
    assert!(result.sample(650).tokens.is_empty());
}

#[test]
fn repeated_actions_expand_at_fixed_intervals_and_default_move_time_is_one_second() {
    let result = compile_timeline(&scenario(vec![ActionIr::Repeat {
        times: 3,
        body: vec![movement("request", "client", "proxy", None)],
        span: span(),
    }]))
    .unwrap();

    assert_eq!(result.duration_ms, 3_000);
    assert_eq!(
        result
            .events
            .iter()
            .map(|event| (event.start_ms, event.end_ms))
            .collect::<Vec<_>>(),
        [(0, 1_000), (1_000, 2_000), (2_000, 3_000)]
    );
}

#[test]
fn highlight_intervals_are_sampled_without_changing_node_ordering() {
    let span = span();
    let result = compile_timeline(&scenario(vec![ActionIr::Parallel {
        actions: vec![
            ActionIr::Highlight {
                node: NodeId("zeta".into()),
                node_span: span,
                duration_ms: Some(300),
                effect: HighlightEffect::Trace,
                origin: HighlightOrigin::Default,
                span,
            },
            ActionIr::Highlight {
                node: NodeId("alpha".into()),
                node_span: span,
                duration_ms: Some(300),
                effect: HighlightEffect::Pulse,
                origin: HighlightOrigin::Default,
                span,
            },
        ],
        span,
    }]))
    .unwrap();

    assert_eq!(
        result.sample(150).highlighted_nodes,
        [NodeId("alpha".into()), NodeId("zeta".into())]
    );
    assert!(result.sample(300).highlighted_nodes.is_empty());
}

#[test]
fn duration_and_expanded_event_limits_return_source_spanned_errors() {
    let too_long = compile_timeline(&scenario(vec![movement(
        "request",
        "client",
        "proxy",
        Some(120_001),
    )]))
    .unwrap_err();
    assert_eq!(too_long.code, "E_TIMELINE_DURATION_LIMIT");

    let too_many = compile_timeline(&scenario(vec![ActionIr::Repeat {
        times: 501,
        body: vec![movement("request", "client", "proxy", Some(0))],
        span: span(),
    }]))
    .unwrap_err();
    assert_eq!(too_many.code, "E_TIMELINE_EVENT_LIMIT");

    let zero_repeat = compile_timeline(&scenario(vec![ActionIr::Repeat {
        times: 0,
        body: vec![movement("request", "client", "proxy", Some(0))],
        span: span(),
    }]))
    .unwrap_err();
    assert_eq!(zero_repeat.code, "E_REPEAT_COUNT");
}
