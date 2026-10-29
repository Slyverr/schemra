use schemra_compiler::{CompletionKind, complete_at, complete_at_with_nodes};

fn labels(source: &str, cursor: usize, explicit: bool) -> Vec<String> {
    complete_at(source, cursor, explicit)
        .expect("completion list")
        .items
        .into_iter()
        .map(|item| item.label)
        .collect()
}

#[test]
fn root_and_nested_suggestions_come_from_supported_schemas() {
    assert_eq!(labels("", 0, true), ["scenario", "visual"]);

    let source = "visual app {\n  node api {}\n  ";
    let choices = labels(source, source.len(), true);
    assert!(choices.contains(&"group".to_owned()));
    assert!(choices.contains(&"node".to_owned()));
    assert!(choices.contains(&"direction".to_owned()));
    assert!(!choices.contains(&"component".to_owned()));
    assert!(!choices.contains(&"theme".to_owned()));
}

#[test]
fn automatic_completion_is_limited_to_action_scope() {
    let scenario = "scenario flow for visual app {\n  \n}";
    let cursor = scenario.find("  \n").unwrap() + 2;
    let choices = labels(scenario, cursor, false);
    assert!(choices.contains(&"move".to_owned()));
    assert!(choices.contains(&"parallel".to_owned()));
    assert!(choices.contains(&"repeat".to_owned()));

    let visual = "visual app {\n  \n}";
    let cursor = visual.find("  \n").unwrap() + 2;
    assert!(complete_at(visual, cursor, false).is_none());
}

#[test]
fn enum_values_and_highlight_origins_are_filtered_by_prefix() {
    let source = "visual app {\n direction: ";
    let choices = complete_at(source, source.len(), true).expect("direction choices");
    assert_eq!(
        choices
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        [
            "left-to-right",
            "right-to-left",
            "top-to-bottom",
            "bottom-to-top"
        ]
    );
    assert!(
        choices
            .items
            .iter()
            .all(|item| item.kind == CompletionKind::Value)
    );

    let source = "scenario flow for visual app {\n  highlight: ";
    assert_eq!(
        labels(source, source.len(), true),
        ["off", "trace", "pulse"]
    );

    let source = "scenario flow for visual app {\n  highlight: 200ms trace from l";
    assert_eq!(labels(source, source.len(), true), ["left"]);

    let source = "scenario flow for visual app {\n  highlight: 200ms trace ";
    assert_eq!(labels(source, source.len(), true), ["from"]);

    let source = "scenario flow for visual app {\n  highlight server 500ms trace from r";
    assert_eq!(labels(source, source.len(), true), ["right"]);
}

#[test]
fn action_headers_offer_declared_tokens_and_project_nodes() {
    let nodes = ["client".to_owned(), "service".to_owned()];
    let source = "scenario flow for visual app {\n  token request\n  move ";
    assert_eq!(
        complete_at_with_nodes(source, source.len(), true, &nodes)
            .unwrap()
            .items[0]
            .label,
        "request"
    );

    let source = "scenario flow for visual app {\n  token request\n  move request from ";
    assert_eq!(labels_with_nodes(source, &nodes), ["client", "service"]);
}

#[test]
fn edge_completions_offer_operators_then_target_nodes() {
    let source = "visual app {\n node client {}\n client ";
    assert_eq!(labels(source, source.len(), true), ["->", "--"]);

    let source = "visual app {\n client -> ";
    let nodes = ["client".to_owned(), "service".to_owned()];
    assert_eq!(labels_with_nodes(source, &nodes), ["client", "service"]);
}

#[test]
fn completion_ranges_are_utf8_safe_and_replace_only_the_current_prefix() {
    let source = "// café\nvis";
    let list = complete_at(source, source.len(), true).expect("visual prefix");
    assert_eq!(&source[list.start..list.end], "vis");
    assert_eq!(list.items[0].label, "visual");

    let source = "visual app {\n direct";
    let cursor = source.len() - 1;
    let list = complete_at(source, cursor, true).expect("clamped character boundary");
    assert!(source.is_char_boundary(list.start));
    assert!(source.is_char_boundary(list.end));
}

fn labels_with_nodes(source: &str, nodes: &[String]) -> Vec<String> {
    complete_at_with_nodes(source, source.len(), true, nodes)
        .expect("completion list")
        .items
        .into_iter()
        .map(|item| item.label)
        .collect()
}
