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
    ] {
        assert!(
            compile(source).is_err(),
            "unsupported syntax compiled:\n{source}"
        );
    }
}
