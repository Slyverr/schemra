use schemra_compiler::compile;
use schemra_ir::Direction;

fn diagnostic_codes(source: &str) -> Vec<String> {
    compile(source)
        .expect_err("source should be rejected")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn accepts_each_supported_visual_direction_and_node_text_property() {
    let directions = [
        ("left-to-right", Direction::LeftToRight),
        ("right-to-left", Direction::RightToLeft),
        ("top-to-bottom", Direction::TopToBottom),
        ("bottom-to-top", Direction::BottomToTop),
    ];

    for (name, expected) in directions {
        let source = format!(
            "visual network {{\n direction: {name}\n node api \"API\" {{\n icon: \"lucide/server\"\n detail: \"Public endpoint\"\n }}\n}}"
        );
        let module = compile(&source).expect("supported properties compile");
        let visual = &module.visuals[0];

        assert_eq!(visual.direction, expected);
        assert_eq!(visual.nodes[0].icon.as_deref(), Some("lucide/server"));
        assert_eq!(visual.nodes[0].detail.as_deref(), Some("Public endpoint"));
    }
}

#[test]
fn rejects_unknown_visual_properties_and_invalid_property_types_or_values() {
    for source in [
        "visual network {\n direction: diagonal\n node api {}\n}",
        "visual network {\n camera: 2\n node api {}\n}",
        "visual network {\n theme: traffic\n node api {}\n}",
        "visual network {\n node api { icon: 3 }\n}",
        "visual network {\n node api { detail: visible }\n}",
    ] {
        assert!(
            !diagnostic_codes(source).is_empty(),
            "unsupported property syntax compiled:\n{source}"
        );
    }

    assert!(
        diagnostic_codes("visual network { direction: diagonal node api {} }")
            .contains(&"E_PROPERTY_TYPE".into())
    );
    assert!(
        diagnostic_codes("visual network { node api { icon: 3 } }")
            .contains(&"E_PROPERTY_TYPE".into())
    );
    assert!(
        diagnostic_codes("visual network { camera: 2 node api {} }")
            .contains(&"E_UNKNOWN_PROPERTY".into())
    );
}

#[test]
fn rejects_constructs_outside_the_supported_v0_surface() {
    for source in [
        "component service {}",
        "camera { zoom: 2 }",
        "theme traffic {}",
        "state online = false",
        "model request {}",
        "let count = 3",
        "input count: number = 3",
        "import Service from \"service.schemra\"",
        "use Service()",
        "visual network { node api {}; box card {} }",
        "visual network { node api {}; text caption \"hello\" }",
    ] {
        assert!(
            compile(source).is_err(),
            "unsupported syntax compiled:\n{source}"
        );
    }
}

#[test]
fn rejects_deferred_scenario_actions() {
    let visual = "visual net { node a {}; node b {}; a -> b }";
    for action in [
        "emit request from a to b",
        "send request from a to b",
        "set a opacity 0.5",
        "if ready { wait 1s }",
        "if ready { wait 1s } else { wait 1s }",
        "when ready { wait 1s }",
        "at 1s { wait 1s }",
        "bind online = true",
    ] {
        let source =
            format!("{visual}\nscenario flow for visual net {{ token request {{}} {action} }}");
        assert!(
            compile(&source).is_err(),
            "unsupported scenario action compiled:\n{action}"
        );
    }
}

#[test]
fn rejects_deferred_visual_and_node_properties() {
    for property in [
        "theme", "camera", "width", "shape", "style", "variant", "font",
    ] {
        let source = format!("visual net {{ node api {{}}; {property}: unsupported }}");
        assert!(
            compile(&source).is_err(),
            "unsupported visual property compiled: {property}"
        );
    }

    for property in ["processing: sequential", "service: 850"] {
        let source = format!("visual net {{ node queue {{ {property} }} }}");
        assert!(
            compile(&source).is_err(),
            "unsupported node property compiled: {property}"
        );
    }
}
