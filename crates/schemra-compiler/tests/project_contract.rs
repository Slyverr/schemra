use schemra_compiler::{ProjectSource, analyze_project};

fn source(id: &str, path: &str, source: &str) -> ProjectSource {
    ProjectSource {
        id: id.to_owned(),
        path: path.to_owned(),
        source: source.to_owned(),
    }
}

#[test]
fn project_analysis_links_scenario_sources_to_visuals() {
    let analysis = analyze_project(&[
        source(
            "visual",
            "visuals/network.schemra",
            r#"visual network "Network" {
  node client "Client" {}
  node api "API" {}
  client -> api
}"#,
        ),
        source(
            "scenario",
            "scenarios/request.schemra",
            r#"scenario request for visual network {
  token packet { icon: "mail" }
  move packet from client -> api { duration: 200ms }
}"#,
        ),
    ]);

    assert_eq!(analysis.visuals.len(), 1);
    assert_eq!(analysis.visuals[0].id, "network");
    assert_eq!(analysis.visuals[0].source_index, 0);
    assert_eq!(analysis.visuals[0].scenarios.len(), 1);
    assert_eq!(analysis.files[1].kind, "scenario");
    assert_eq!(
        analysis.files[1].resolved_visual_id.as_deref(),
        Some("network")
    );
    assert!(
        analysis
            .files
            .iter()
            .all(|file| file.diagnostics.is_empty())
    );
}

#[test]
fn duplicate_visual_ids_and_unknown_scenario_targets_are_diagnosed() {
    let analysis = analyze_project(&[
        source(
            "first",
            "first.schemra",
            "visual shared \"First\" {\n  node a \"A\" {}\n  node b \"B\" {}\n  a -> b\n}",
        ),
        source(
            "duplicate",
            "duplicate.schemra",
            "visual shared \"Duplicate\" {\n  node a \"A\" {}\n  node b \"B\" {}\n  a -> b\n}",
        ),
        source(
            "scenario",
            "scenario.schemra",
            "scenario request for visual missing {\n}",
        ),
    ]);

    assert_eq!(analysis.visuals.len(), 1);
    assert_eq!(analysis.files[1].kind, "invalid");
    assert!(
        analysis.files[1]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_DUPLICATE_VISUAL_ID")
    );
    assert!(
        analysis.files[2]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_UNKNOWN_VISUAL_TARGET")
    );
}

#[test]
fn scenario_file_with_multiple_visual_targets_is_not_partially_linked() {
    let analysis = analyze_project(&[
        source("first", "first.schemra", "visual first { node a {} }"),
        source("second", "second.schemra", "visual second { node a {} }"),
        source(
            "scenarios",
            "scenarios.schemra",
            "scenario first_flow for visual first {\n}\nscenario second_flow for visual second {\n}",
        ),
    ]);

    assert!(
        analysis.files[2]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_MULTIPLE_SCENARIO_TARGETS")
    );
    assert!(analysis.files[2].resolved_scenarios.is_empty());
    assert!(
        analysis
            .visuals
            .iter()
            .all(|visual| visual.scenarios.is_empty())
    );
}

#[test]
fn visual_and_file_order_follow_input_order() {
    let analysis = analyze_project(&[
        source("first", "first.schemra", "visual zeta { node a {} }"),
        source("second", "second.schemra", "visual alpha { node b {} }"),
    ]);

    assert_eq!(analysis.files[0].id, "first");
    assert_eq!(analysis.files[1].id, "second");
    assert_eq!(
        analysis
            .visuals
            .iter()
            .map(|visual| visual.id.as_str())
            .collect::<Vec<_>>(),
        ["zeta", "alpha"]
    );
}

#[test]
fn duplicate_scenario_ids_are_rejected_across_files() {
    let analysis = analyze_project(&[
        source("visual", "visual.schemra", "visual app { node api {} }"),
        source(
            "first",
            "first.schemra",
            "scenario flow for visual app {\n wait 1ms\n}",
        ),
        source(
            "second",
            "second.schemra",
            "scenario flow for visual app {\n wait 2ms\n}",
        ),
    ]);

    assert_eq!(analysis.visuals[0].scenarios.len(), 1);
    assert!(analysis.files[1].diagnostics.is_empty());
    assert!(
        analysis.files[2]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_DUPLICATE_SCENARIO")
    );
    assert!(analysis.files[2].resolved_scenarios.is_empty());
}

#[test]
fn source_with_multiple_visual_declarations_is_reported_as_invalid() {
    let analysis = analyze_project(&[source(
        "multiple",
        "multiple.schemra",
        "visual first { node a {} }\nvisual second { node b {} }",
    )]);

    assert_eq!(analysis.files[0].kind, "invalid");
    assert!(
        analysis.files[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_MULTIPLE_VISUALS")
    );
    assert!(analysis.visuals.is_empty());
}
