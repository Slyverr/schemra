use schemra_compiler::{
    DeclarationKind, EditorSymbolKind, EditorSymbolRole, SyntaxTokenKind, compile, editor_metadata,
    highlight_tokens,
};

fn token_kinds(source: &str) -> Vec<(&str, SyntaxTokenKind)> {
    highlight_tokens(source)
        .into_iter()
        .map(|token| {
            let span = token.span.start.offset..token.span.end.offset;
            (&source[span], token.kind)
        })
        .collect()
}

#[test]
fn syntax_coloring_uses_construct_and_property_schemas() {
    let source =
        "visual app { direction: left-to-right\n node client {}\n client -> api // edge\n}\n";
    let tokens = token_kinds(source);

    for (text, kind) in [
        ("visual", SyntaxTokenKind::Keyword),
        ("direction", SyntaxTokenKind::Property),
        ("left-to-right", SyntaxTokenKind::Value),
        ("node", SyntaxTokenKind::Keyword),
        ("client", SyntaxTokenKind::Identifier),
        ("->", SyntaxTokenKind::Operator),
        ("// edge", SyntaxTokenKind::Comment),
    ] {
        assert!(
            tokens.contains(&(text, kind)),
            "missing {text:?} as {kind:?}: {tokens:?}"
        );
    }
    assert!(
        !tokens
            .iter()
            .any(|(_, kind)| *kind == SyntaxTokenKind::Number)
    );
}

#[test]
fn node_presentation_properties_and_values_use_registered_colors() {
    let source = "visual app { node api { layout: vertical\n text_align: center } }";
    let tokens = token_kinds(source);

    for (text, kind) in [
        ("layout", SyntaxTokenKind::Property),
        ("vertical", SyntaxTokenKind::Value),
        ("text_align", SyntaxTokenKind::Property),
        ("center", SyntaxTokenKind::Value),
    ] {
        assert!(
            tokens.contains(&(text, kind)),
            "missing {text:?}: {tokens:?}"
        );
    }
}

#[test]
fn scenario_action_keywords_and_highlight_values_are_colored() {
    let source = "scenario flow for visual app {\n token packet { char: \"R\" }\n highlight: 200ms trace from left\n move packet from client -> api {\n  duration: 1s\n  highlight: off\n }\n}";
    let tokens = token_kinds(source);

    for (text, kind) in [
        ("scenario", SyntaxTokenKind::Keyword),
        ("for", SyntaxTokenKind::Keyword),
        ("visual", SyntaxTokenKind::Keyword),
        ("highlight", SyntaxTokenKind::Property),
        ("200ms", SyntaxTokenKind::Number),
        ("trace", SyntaxTokenKind::Value),
        ("from", SyntaxTokenKind::Keyword),
        ("left", SyntaxTokenKind::Value),
        ("move", SyntaxTokenKind::Keyword),
        ("char", SyntaxTokenKind::Property),
        ("1s", SyntaxTokenKind::Number),
        ("off", SyntaxTokenKind::Value),
    ] {
        assert!(
            tokens.contains(&(text, kind)),
            "missing {text:?} as {kind:?}: {tokens:?}"
        );
    }
}

#[test]
fn semicolons_compile_and_are_colored_as_punctuation() {
    let source = "visual app { node api {} }; scenario flow for visual app { wait 1s; }";

    assert!(compile(source).is_ok());
    assert!(token_kinds(source).contains(&(";", SyntaxTokenKind::Punctuation)));
}

#[test]
fn metadata_reports_declarations_and_node_references_with_source_spans() {
    let source = "visual app {\n node client {}\n node api {}\n client -> api\n}\nscenario flow for visual app {\n token packet {}\n move packet from client -> api {}\n highlight api 10ms\n}";
    let module = compile(source).expect("source compiles");
    let (declarations, symbols) = editor_metadata(&module);

    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].kind, DeclarationKind::Visual);
    assert_eq!(declarations[0].id.as_deref(), Some("app"));
    assert_eq!(declarations[1].kind, DeclarationKind::Scenario);
    assert_eq!(declarations[1].id.as_deref(), Some("flow"));
    assert_eq!(declarations[1].visual_id.as_deref(), Some("app"));

    let app_visual_definitions = symbols
        .iter()
        .filter(|symbol| {
            symbol.target_key == "visual:app" && symbol.role == EditorSymbolRole::Definition
        })
        .count();
    assert_eq!(app_visual_definitions, 1);
    assert!(symbols.iter().any(|symbol| {
        symbol.name == "client"
            && symbol.kind == EditorSymbolKind::Node
            && symbol.role == EditorSymbolRole::Definition
            && &source[symbol.span.start.offset..symbol.span.end.offset] == "client"
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol.name == "api"
            && symbol.target_key == "node:app:api"
            && symbol.role == EditorSymbolRole::Reference
            && &source[symbol.span.start.offset..symbol.span.end.offset] == "api"
    }));
}
