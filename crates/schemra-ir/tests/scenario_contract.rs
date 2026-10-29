use schemra_ir::{
    ModuleIr, ScenarioId, ScenarioIr, SourceLocation, SourceSpan, TokenId, TokenIr, VisualId,
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
            span: span(58, 81),
        }],
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
