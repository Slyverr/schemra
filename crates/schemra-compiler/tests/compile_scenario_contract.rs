use schemra_compiler::{compile, compile_sources};
use schemra_ir::{
    ActionIr, HighlightEffect, HighlightOrigin, HighlightSide, ScenarioId, TokenId, VisualId,
};

fn diagnostic_codes(source: &str) -> Vec<String> {
    compile(source)
        .expect_err("source should be rejected")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn compiles_scenario_visual_reference_and_reusable_token_style() {
    let source = r#"
visual network { node client {} }
scenario request_response for visual network {
  token request { icon: "mail" }
  wait 1ms
}
"#;

    let module = compile(source).expect("valid scenario compiles");
    assert_eq!(module.scenarios.len(), 1);

    let scenario = &module.scenarios[0];
    assert_eq!(scenario.id, ScenarioId("request_response".into()));
    assert_eq!(scenario.visual, VisualId("network".into()));
    assert_eq!(scenario.tokens.len(), 1);
    assert_eq!(scenario.tokens[0].id, TokenId("request".into()));
    assert_eq!(scenario.tokens[0].icon.as_deref(), Some("mail"));
    assert!(scenario.id_span.start.offset < scenario.id_span.end.offset);
    assert!(scenario.visual_span.start.offset < scenario.visual_span.end.offset);
}

#[test]
fn compiles_a_single_character_marker_style() {
    let module = compile(
        r#"
visual network {
  node client {}
  node server {}
  client -> server
}
scenario requests for visual network {
  token acknowledgment { char: "✓" }
  move acknowledgment from client -> server { duration: 1ms }
}
"#,
    )
    .expect("single-character marker style compiles");

    assert_eq!(module.scenarios[0].tokens[0].char.as_deref(), Some("✓"));
}

#[test]
fn rejects_invalid_character_marker_styles() {
    let too_long = diagnostic_codes(
        r#"visual network {
  node client {}
  node server {}
  client -> server
}
scenario requests for visual network {
  token packet { char: "AB" }
  move packet from client -> server { duration: 1ms }
}"#,
    );
    assert!(
        too_long.contains(&"E_SCENARIO_TOKEN_CHAR".into()),
        "{too_long:?}"
    );

    let both_styles = diagnostic_codes(
        r#"visual network {
  node client {}
  node server {}
  client -> server
}
scenario requests for visual network {
  token packet {
    icon: "mail"
    char: "R"
  }
  move packet from client -> server { duration: 1ms }
}"#,
    );
    assert!(
        both_styles.contains(&"E_SCENARIO_TOKEN_STYLE".into()),
        "{both_styles:?}"
    );
}

#[test]
fn resolves_scenario_visuals_across_separate_sources() {
    let module = compile_sources(&[
        "visual network { node client {} }",
        "scenario requests for visual network {\n token request {}\n wait 1ms\n}",
    ])
    .expect("scenario can reference a visual declared in another source");

    assert_eq!(module.visuals[0].id, VisualId("network".into()));
    assert_eq!(module.scenarios[0].visual, VisualId("network".into()));
}

#[test]
fn rejects_missing_scenario_visuals_and_duplicate_declarations() {
    assert!(
        diagnostic_codes(
            "visual network { node client {} }\nscenario requests for visual missing {}"
        )
        .contains(&"E_SCENARIO_VISUAL_TARGET".into())
    );

    assert!(
        diagnostic_codes(
            "scenario requests for visual network {}\nscenario requests for visual network {}"
        )
        .contains(&"E_DUPLICATE_SCENARIO".into())
    );

    let codes = diagnostic_codes(
        "scenario requests for visual network {\n token packet {}\n token packet {}\n}",
    );
    assert!(codes.contains(&"E_DUPLICATE_TOKEN".into()), "{codes:?}");
}

#[test]
fn token_properties_are_validated_in_their_own_scope() {
    assert!(
        diagnostic_codes("scenario requests for visual network { token packet { icon: 2 } }")
            .contains(&"E_PROPERTY_TYPE".into())
    );

    assert!(
        diagnostic_codes("scenario requests for visual network { token packet { camera: 2 } }")
            .contains(&"E_UNKNOWN_PROPERTY".into())
    );
}

#[test]
fn compiles_parallel_moves_waits_repeats_and_explicit_highlights() {
    let source = r#"
visual network {
  node client {}
  node proxy {}
  node store {}
  client -> proxy
  proxy -> store
}
scenario requests for visual network {
  token packet { icon: "mail" }
  highlight: 200ms trace
  parallel {
    move packet from client -> proxy { duration: 350ms }
    move packet from proxy -> store {
      duration: 500ms
      highlight: off
    }
  }
  wait 100ms
  repeat 2 {
    highlight store 300ms pulse
  }
  highlight store 250ms trace from left
}
"#;

    let module = compile(source).expect("valid animation scenario compiles");
    let scenario = &module.scenarios[0];
    assert_eq!(scenario.actions.len(), 4);

    let ActionIr::Parallel { actions, .. } = &scenario.actions[0] else {
        panic!("first action should be parallel");
    };
    assert_eq!(actions.len(), 2);
    let ActionIr::Sequence {
        actions: first_move,
        ..
    } = &actions[0]
    else {
        panic!("the scenario default should add an arrival highlight after the first move");
    };
    assert!(matches!(
        first_move[0],
        ActionIr::Move {
            duration_ms: Some(350),
            ..
        }
    ));
    assert!(matches!(
        first_move[1],
        ActionIr::Highlight {
            duration_ms: Some(200),
            effect: HighlightEffect::Trace,
            origin: HighlightOrigin::Arrival { .. },
            ..
        }
    ));
    assert!(matches!(
        actions[1],
        ActionIr::Move {
            duration_ms: Some(500),
            ..
        }
    ));
    assert!(matches!(
        scenario.actions[1],
        ActionIr::Wait {
            duration_ms: 100,
            ..
        }
    ));
    assert!(matches!(
        scenario.actions[2],
        ActionIr::Repeat { times: 2, .. }
    ));
    assert!(matches!(
        scenario.actions[3],
        ActionIr::Highlight {
            effect: HighlightEffect::Trace,
            origin: HighlightOrigin::Side {
                side: HighlightSide::Left
            },
            ..
        }
    ));
}

#[test]
fn rejects_pulse_highlights_with_an_explicit_start_side() {
    let diagnostics = compile(
        "visual network { node proxy {} }\nscenario requests for visual network {\n wait 1ms\n highlight proxy 200ms pulse from left\n}",
    )
    .expect_err("pulse effects do not have a trace start side");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_HIGHLIGHT_ORIGIN_EFFECT")
    );
}

#[test]
fn validates_move_tokens_nodes_links_and_marker_icons_against_visual() {
    let diagnostics = compile(
        r#"
visual network {
  node client {}
  node proxy {}
}
scenario requests for visual network {
  token packet { icon: "not-an-icon" }
  move unknown from client -> proxy {}
}
"#,
    )
    .expect_err("undeclared markers and links should be diagnosed");
    let codes = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(codes.contains(&"E_SCENARIO_TOKEN_ICON"), "{codes:?}");
    assert!(codes.contains(&"E_UNKNOWN_TOKEN"));
    assert!(codes.contains(&"E_MOVE_NO_EDGE"));
}

#[test]
fn validates_highlight_nodes_against_the_scenario_visual() {
    let diagnostics = compile(
        "visual network { node proxy {} }\nscenario requests for visual network {\n wait 1ms\n highlight missing 200ms trace\n}",
    )
    .expect_err("a highlight must name a node in the visual");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_UNKNOWN_HIGHLIGHT_NODE")
    );
}

#[test]
fn rejects_scenarios_without_actions_when_the_visual_is_available() {
    let diagnostics =
        compile("visual network { node proxy {} }\nscenario requests for visual network {}")
            .expect_err("a scenario needs at least one action");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_EMPTY_SCENARIO")
    );
}
