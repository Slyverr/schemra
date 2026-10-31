use schemra_wasm::{compile_project_schemra, compile_schemra};
use serde_json::{Value, json};

const VISUAL: &str = r#"visual network "Network" {
  node client "Client" {}
  node server "Server" {}
  client -> server
}
"#;

const SCENARIO: &str = r#"scenario requests for visual network {
  token packet { icon: "mail" }
  token acknowledgment { char: "✓" }
  move packet from client -> server { duration: 100ms }
  wait 25ms
  highlight server 50ms trace
}
"#;

fn parse_json(json: &str) -> Value {
    serde_json::from_str(json).expect("API response is valid JSON")
}

#[test]
fn compile_schemra_returns_scene_svg_and_scheduled_scenario() {
    let output = parse_json(&compile_schemra(&format!("{VISUAL}\n{SCENARIO}")));

    assert_eq!(output["ok"], true);
    assert_eq!(output["title"], "Network");
    assert_eq!(output["stats"]["nodes"], 2);
    assert_eq!(output["stats"]["edges"], 1);
    assert_eq!(output["scene"]["nodes"].as_array().map(Vec::len), Some(2));
    assert!(
        output["svg"]
            .as_str()
            .is_some_and(|svg| svg.contains("<svg"))
    );
    assert_eq!(output["scenarios"][0]["duration_ms"], 175);
    assert_eq!(output["scenarios"][0]["tokens"][0]["icon"], "mail");
    assert_eq!(output["scenarios"][0]["tokens"][1]["char"], "✓");
    assert_eq!(output["scenarios"][0]["effects"][0]["kind"], "move");
    assert_eq!(output["scenarios"][0]["effects"][0]["start_ms"], 0);
    assert_eq!(output["scenarios"][0]["effects"][0]["end_ms"], 100);
    assert_eq!(output["scenarios"][0]["effects"][1]["kind"], "wait");
    assert_eq!(output["scenarios"][0]["effects"][2]["kind"], "highlight");
}

#[test]
fn compile_schemra_returns_source_diagnostics_without_scene_for_invalid_text() {
    let output = parse_json(&compile_schemra("visual network { node client {"));

    assert_eq!(output["ok"], false);
    assert!(output["svg"].is_null());
    assert!(output["scene"].is_null());
    assert!(!output["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn compile_schemra_rejects_oversized_source_before_compiling() {
    let source = " ".repeat(2 * 1024 * 1024 + 1);
    let output = parse_json(&compile_schemra(&source));

    assert_eq!(output["ok"], false);
    assert_eq!(output["diagnostics"][0]["code"], "E_INPUT_LIMIT");
    assert!(output["tokens"].as_array().unwrap().is_empty());
}

#[test]
fn project_compilation_links_scenario_files_to_their_visual() {
    let sources = json!({
        "sources": [
            { "id": "visual", "path": "visuals/network.schemra", "source": VISUAL },
            { "id": "scenario", "path": "scenarios/requests.schemra", "source": SCENARIO }
        ]
    });
    let output = parse_json(&compile_project_schemra(&sources.to_string()));

    assert!(output["error"].is_null());
    assert_eq!(output["files"].as_array().map(Vec::len), Some(2));
    assert_eq!(output["visuals"].as_array().map(Vec::len), Some(1));
    assert_eq!(output["visuals"][0]["id"], "network");
    assert_eq!(
        output["visuals"][0]["output"]["scenarios"][0]["id"],
        "requests"
    );
    assert_eq!(output["files"][1]["output"]["title"], "Network");
}

#[test]
fn project_compilation_reports_invalid_json_and_unknown_request_fields() {
    let malformed = parse_json(&compile_project_schemra("{"));
    assert!(malformed["error"].as_str().is_some());
    assert!(malformed["files"].as_array().unwrap().is_empty());

    let unexpected = parse_json(&compile_project_schemra(
        r#"{"sources": [], "extra": true}"#,
    ));
    assert!(unexpected["error"].as_str().is_some());
}

#[test]
fn project_compilation_enforces_file_count_and_path_limits() {
    let sources = (0..513)
        .map(|index| json!({"id": index.to_string(), "path": "a.schemra", "source": ""}))
        .collect::<Vec<_>>();
    let too_many = parse_json(&compile_project_schemra(
        &json!({"sources": sources}).to_string(),
    ));
    assert!(
        too_many["error"]
            .as_str()
            .unwrap()
            .contains("513 source files")
    );

    let long_path = "x".repeat(4097);
    let invalid_path = parse_json(&compile_project_schemra(
        &json!({"sources": [{"id": "file", "path": long_path, "source": ""}]}).to_string(),
    ));
    assert!(
        invalid_path["error"]
            .as_str()
            .unwrap()
            .contains("path limit")
    );
}

#[test]
fn project_compilation_enforces_aggregate_source_size() {
    let source = " ".repeat(1900 * 1024);
    let sources = (0..9)
        .map(|index| {
            json!({
                "id": index.to_string(),
                "path": format!("{index}.schemra"),
                "source": source
            })
        })
        .collect::<Vec<_>>();
    let output = parse_json(&compile_project_schemra(
        &json!({"sources": sources}).to_string(),
    ));

    assert!(
        output["error"]
            .as_str()
            .unwrap()
            .contains("aggregate limit")
    );
}

#[test]
fn project_compilation_rejects_visuals_over_the_link_limit() {
    let mut source = String::from("visual network {\n");
    for index in 0..32 {
        source.push_str(&format!(
            "node left_{index} {{}}\nnode right_{index} {{}}\n"
        ));
    }
    for left in 0..32 {
        for right in 0..32 {
            if left * 32 + right == 1001 {
                break;
            }
            source.push_str(&format!("left_{left} -> right_{right}\n"));
        }
    }
    source.push('}');

    let project =
        json!({"sources": [{"id": "visual", "path": "visual.schemra", "source": source}]});
    let output = parse_json(&compile_project_schemra(&project.to_string()));

    assert_eq!(output["visuals"][0]["output"]["ok"], false);
    assert_eq!(
        output["visuals"][0]["output"]["diagnostics"][0]["code"],
        "E_VISUAL_LINK_LIMIT"
    );
    assert!(output["visuals"][0]["output"]["scene"].is_null());
}
