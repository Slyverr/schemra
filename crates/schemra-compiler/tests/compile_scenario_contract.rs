use schemra_compiler::{compile, compile_sources};
use schemra_ir::{ScenarioId, TokenId, VisualId};

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
fn resolves_scenario_visuals_across_separate_sources() {
    let module = compile_sources(&[
        "visual network { node client {} }",
        "scenario requests for visual network { token request {} }",
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
