use schemra_ir::{
    ActionIr, HighlightEffect, HighlightOrigin, HighlightSide, ModuleIr, NodeId, ScenarioId,
    ScenarioIr, SourceLocation, SourceSpan, TokenId, TokenIr, VisualId,
};

fn span(start: usize, end: usize) -> SourceSpan {
    SourceSpan {
        start: SourceLocation {
            offset: start,
            line: 1,
            column: start + 1,
        },
        end: SourceLocation {
            offset: end,
            line: 1,
            column: end + 1,
        },
    }
}

#[test]
fn module_models_scenarios_and_reusable_token_styles() {
    let scenario = ScenarioIr {
        id: ScenarioId("request_response".into()),
        id_span: span(9, 25),
        visual: VisualId("service_map".into()),
        visual_span: span(42, 53),
        tokens: vec![TokenIr {
            id: TokenId("request".into()),
            id_span: span(64, 71),
            icon: Some("mail".into()),
            char: None,
            span: span(58, 81),
        }],
        actions: Vec::new(),
        span: span(0, 84),
    };
    let module = ModuleIr {
        visuals: Vec::new(),
        scenarios: vec![scenario.clone()],
    };

    assert_eq!(module.scenarios[0], scenario);
    assert_eq!(module.scenarios[0].tokens[0].icon.as_deref(), Some("mail"));
    assert_eq!(module.scenarios[0].visual, VisualId("service_map".into()));
}

#[test]
fn action_tree_preserves_timing_parallelism_and_highlight_origin() {
    let actions = vec![ActionIr::Parallel {
        actions: vec![
            ActionIr::Move {
                token: TokenId("request".into()),
                token_span: span(20, 27),
                from: NodeId("client".into()),
                from_span: span(33, 39),
                to: NodeId("proxy".into()),
                to_span: span(43, 48),
                duration_ms: Some(350),
                span: span(15, 58),
            },
            ActionIr::Sequence {
                actions: vec![
                    ActionIr::Wait {
                        duration_ms: 120,
                        span: span(60, 73),
                    },
                    ActionIr::Highlight {
                        node: NodeId("proxy".into()),
                        node_span: span(84, 89),
                        duration_ms: Some(200),
                        effect: HighlightEffect::Trace,
                        origin: HighlightOrigin::Side {
                            side: HighlightSide::Left,
                        },
                        span: span(74, 101),
                    },
                ],
                span: span(60, 101),
            },
            ActionIr::Repeat {
                times: 3,
                body: vec![ActionIr::Wait {
                    duration_ms: 25,
                    span: span(115, 127),
                }],
                span: span(102, 130),
            },
        ],
        span: span(0, 131),
    }];

    let ActionIr::Parallel {
        actions: scheduled, ..
    } = &actions[0]
    else {
        unreachable!("outer action should remain parallel");
    };
    assert_eq!(scheduled.len(), 3);
    assert!(matches!(
        scheduled[0],
        ActionIr::Move {
            duration_ms: Some(350),
            ..
        }
    ));
    assert!(matches!(scheduled[1], ActionIr::Sequence { .. }));
    assert!(matches!(scheduled[2], ActionIr::Repeat { times: 3, .. }));

    let ActionIr::Sequence { actions, .. } = &scheduled[1] else {
        unreachable!("nested action should remain a sequence");
    };
    let ActionIr::Highlight { origin, .. } = &actions[1] else {
        unreachable!("second sequenced action should remain a highlight");
    };
    assert_eq!(
        origin,
        &HighlightOrigin::Side {
            side: HighlightSide::Left
        }
    );
}

#[test]
fn scenario_and_token_ids_keep_their_string_json_representation() {
    let scenario = ScenarioId("request_response".into());
    let token = TokenId("request".into());

    assert_eq!(
        serde_json::to_string(&scenario).expect("scenario JSON"),
        "\"request_response\""
    );
    assert_eq!(
        serde_json::to_string(&token).expect("token JSON"),
        "\"request\""
    );
    assert_eq!(
        serde_json::from_str::<ScenarioId>("\"request_response\"").unwrap(),
        scenario
    );
    assert_eq!(
        serde_json::from_str::<TokenId>("\"request\"").unwrap(),
        token
    );
}
