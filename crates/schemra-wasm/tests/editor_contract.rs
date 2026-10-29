use schemra_wasm::{
    complete_schemra, complete_schemra_with_nodes, format_schemra, rename_project_schemra,
};
use serde_json::{Value, json};

fn parse_json(response: &str) -> Value {
    serde_json::from_str(response).expect("editor API response is valid JSON")
}

fn project_sources() -> Value {
    json!({
        "sources": [
            {
                "id": "visual",
                "path": "visual.schemra",
                "source": "visual network {\n node client {}\n node server {}\n client -> server\n}"
            },
            {
                "id": "scenario",
                "path": "scenario.schemra",
                "source": "scenario request for visual network {\n token packet {}\n move packet from client -> server {}\n}"
            }
        ]
    })
}

#[test]
fn formatter_api_matches_the_shared_formatter_and_is_idempotent() {
    let source = "visual app{node api{}}";
    let formatted = format_schemra(source);

    assert_eq!(formatted, schemra_syntax::format_source(source));
    assert_eq!(format_schemra(&formatted), formatted);
}

#[test]
fn completion_api_returns_utf8_safe_ranges_and_node_context() {
    let source = "// café\nvis";
    let completion = parse_json(&complete_schemra(source, source.len(), true));

    assert_eq!(
        &source[completion["start"].as_u64().unwrap() as usize
            ..completion["end"].as_u64().unwrap() as usize],
        "vis"
    );
    assert_eq!(completion["items"][0]["label"], "visual");

    let edge_source = "visual app {\n client -> ";
    let context = json!({"node_ids": ["client", "service"]});
    let completion = parse_json(&complete_schemra_with_nodes(
        edge_source,
        edge_source.len(),
        true,
        &context.to_string(),
    ));
    let labels = completion["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(labels, ["client", "service"]);
}

#[test]
fn rename_api_prepares_and_applies_edits_across_project_files() {
    let project = project_sources();
    let visual_source = project["sources"][0]["source"].as_str().unwrap();
    let offset = visual_source.find("client").unwrap();
    let sources = project["sources"].clone();
    let prepared = parse_json(&rename_project_schemra(
        &json!({
            "sources": sources,
            "path": "visual.schemra",
            "offset": offset
        })
        .to_string(),
    ));

    assert!(prepared["error"].is_null());
    assert_eq!(prepared["selection"]["name"], "client");
    assert_eq!(prepared["selection"]["kind"], "node");
    assert!(prepared["edits"].as_array().unwrap().is_empty());

    let sources = project["sources"].clone();
    let applied = parse_json(&rename_project_schemra(
        &json!({
            "sources": sources,
            "path": "visual.schemra",
            "offset": offset,
            "new_name": "frontend"
        })
        .to_string(),
    ));

    assert!(applied["error"].is_null());
    assert_eq!(applied["selection"]["name"], "client");
    assert_eq!(applied["edits"].as_array().map(Vec::len), Some(3));
    assert!(
        applied["edits"].as_array().unwrap().iter().any(|edit| {
            edit["path"] == "scenario.schemra" && edit["replacement"] == "frontend"
        })
    );
    assert!(
        project["sources"][0]["source"]
            .as_str()
            .unwrap()
            .contains("node client")
    );
}

#[test]
fn rename_api_returns_structured_errors_for_bad_requests_and_conflicts() {
    let malformed = parse_json(&rename_project_schemra("{"));
    assert!(malformed["error"].as_str().is_some());
    assert!(malformed["selection"].is_null());
    assert!(malformed["edits"].as_array().unwrap().is_empty());

    let project = project_sources();
    let visual_source = project["sources"][0]["source"].as_str().unwrap();
    let request = json!({
        "sources": project["sources"].clone(),
        "path": "visual.schemra",
        "offset": visual_source.find("client").unwrap(),
        "new_name": "server"
    });
    let conflict = parse_json(&rename_project_schemra(&request.to_string()));
    assert!(conflict["error"].as_str().is_some());
    assert!(conflict["edits"].as_array().unwrap().is_empty());
}
