use schemra_syntax::format_source;

#[test]
fn normalizes_line_endings_and_nested_block_indentation() {
    let source = "\r\nvisual map \"Map\" {\r\nnode api \"API\" {\r\nicon: \"lucide/server\"\r\n}\r\n}\r\n\r\n";
    let expected =
        "visual map \"Map\" {\n  node api \"API\" {\n    icon: \"lucide/server\"\n  }\n}\n";

    assert_eq!(format_source(source), expected);
}

#[test]
fn ignores_braces_inside_strings_and_comments() {
    let source = "visual map \"Map {\\\"quoted\\\"\" {\nnode api \"API }\" {\nicon: \"lucide/server\" // } {\n}\n// visual close }\n}";
    let expected = "visual map \"Map {\\\"quoted\\\"\" {\n  node api \"API }\" {\n    icon: \"lucide/server\" // } {\n  }\n  // visual close }\n}\n";

    assert_eq!(format_source(source), expected);
}

#[test]
fn formatting_is_idempotent_and_preserves_internal_spacing() {
    let source = "visual   map \"Map\"   {\n\n    node api  \"API\" {}  \n}\n";
    let formatted = format_source(source);

    assert_eq!(
        formatted,
        "visual   map \"Map\"   {\n\n  node api  \"API\" {}\n}\n"
    );
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_scenario_actions_and_properties_idempotently() {
    let source = "scenario flow \"Flow\" for visual map {\ntoken request { icon: \"lucide/mail\" }\ntoken retry { icon: \"lucide/rotate-cw\" }\nmove request from api -> db { duration: 800ms }\n}";
    let formatted = format_source(source);

    assert_eq!(
        formatted,
        "scenario flow \"Flow\" for visual map {\n  token request { icon: \"lucide/mail\" }\n  token retry { icon: \"lucide/rotate-cw\" }\n  move request from api -> db { duration: 800ms }\n}\n"
    );
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn preserves_directed_and_undirected_link_operators() {
    let source = "visual map \"Map\" {\nnode a \"A\" {}\nnode b \"B\" {}\nnode c \"C\" {}\na -> b\nb -- c\n}";
    let formatted = format_source(source);

    assert!(formatted.contains("  a -> b\n"));
    assert!(formatted.contains("  b -- c\n"));
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn formats_node_properties_at_their_block_depth() {
    let source = r#"visual map "Map" {
node queue "Work queue" {
icon: "lucide/server"
detail: "Ready"
}
}"#;
    let formatted = format_source(source);

    assert_eq!(
        formatted,
        "visual map \"Map\" {\n  node queue \"Work queue\" {\n    icon: \"lucide/server\"\n    detail: \"Ready\"\n  }\n}\n"
    );
}

#[test]
fn leaves_malformed_statements_intact_while_formatting_blocks() {
    let source = "scenario flow \"Flow\" for visual map {\nmove request from api -> db { duration 800ms } // route\n}";
    let formatted = format_source(source);

    assert_eq!(
        formatted,
        "scenario flow \"Flow\" for visual map {\n  move request from api -> db { duration 800ms } // route\n}\n"
    );
    assert_eq!(format_source(&formatted), formatted);
}

#[test]
fn empty_source_stays_empty_and_unmatched_braces_do_not_underflow() {
    assert_eq!(format_source(" \r\n\t\n"), "");
    assert_eq!(
        format_source("}\n    node api \"API\" {"),
        "}\nnode api \"API\" {\n"
    );
}
