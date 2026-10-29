use schemra_syntax::{StatementKind, TokenKind, parse_syntax};

#[test]
fn parses_invocations_properties_and_directed_or_undirected_edges() {
    let file = parse_syntax(
        "visual graph { direction: left-to-right\n highlight: 250ms trace\n a -> b\n a -- c }\nscenario flow for visual graph { wait 2s }",
    );

    assert!(file.diagnostics.is_empty(), "{:?}", file.diagnostics);
    assert_eq!(file.statements.len(), 2);

    let StatementKind::Invocation {
        body: Some(visual), ..
    } = &file.statements[0].kind
    else {
        panic!("expected an invocation with a body");
    };
    let StatementKind::Property {
        additional_values, ..
    } = &visual[1].kind
    else {
        panic!("expected a property with an additional value");
    };
    assert_eq!(additional_values.len(), 1);
    assert_eq!(additional_values[0].text, "trace");
    assert!(matches!(visual[0].kind, StatementKind::Property { .. }));
    assert!(matches!(visual[2].kind, StatementKind::Edge { .. }));
    assert!(matches!(visual[3].kind, StatementKind::Edge { .. }));

    let StatementKind::Invocation {
        body: Some(scenario),
        ..
    } = &file.statements[1].kind
    else {
        panic!("expected a scenario invocation with a body");
    };
    let StatementKind::Invocation { arguments, .. } = &scenario[0].kind else {
        panic!("expected the nested wait invocation");
    };
    assert!(matches!(arguments[0].kind, TokenKind::Duration(_)));
}

#[test]
fn retains_comments_and_tracks_unicode_source_spans() {
    let file = parse_syntax("// note\nvisual g \"🌱\" {}");

    assert_eq!(file.comments.len(), 1);
    assert!(file.diagnostics.is_empty());
    assert_eq!(file.statements[0].span.start.offset, 8);
    assert_eq!(file.statements[0].span.start.line, 2);
}

#[test]
fn recovers_after_invalid_characters_and_malformed_statements() {
    let file = parse_syntax("visual a {}\n@ bad\nvisual b {}");

    assert!(!file.diagnostics.is_empty());
    assert_eq!(file.statements.len(), 3);
    assert!(matches!(
        file.statements[2].kind,
        StatementKind::Invocation { .. }
    ));
}

#[test]
fn diagnoses_missing_separators_between_sibling_statements() {
    let file = parse_syntax("visual graph { direction: left-to-right columns: 2 }");

    assert!(
        file.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_SYNTAX_SEPARATOR")
    );
}

#[test]
fn reports_excessive_nesting_and_recovers_to_following_statements() {
    let supported_depth = format!("{}{}", "block {".repeat(32), "}".repeat(32));
    assert!(parse_syntax(&supported_depth).diagnostics.is_empty());

    let nesting = 256;
    let source = format!(
        "{}{}\nvisual after {{}}",
        "block {".repeat(nesting),
        "}".repeat(nesting)
    );
    let file = parse_syntax(&source);

    assert!(
        file.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_SYNTAX_DEPTH")
    );
    assert!(matches!(
        file.statements.last().map(|statement| &statement.kind),
        Some(StatementKind::Invocation { name, .. }) if name.text == "visual"
    ));
}
