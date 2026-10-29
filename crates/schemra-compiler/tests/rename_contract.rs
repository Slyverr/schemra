use schemra_compiler::{EditorSymbolKind, ProjectSource, prepare_project_rename, rename_project};

fn source(id: &str, path: &str, source: &str) -> ProjectSource {
    ProjectSource {
        id: id.to_owned(),
        path: path.to_owned(),
        source: source.to_owned(),
    }
}

fn project_sources() -> Vec<ProjectSource> {
    vec![
        source(
            "visual",
            "visuals/app.schemra",
            "visual app {\n node client {}\n node proxy {}\n client -> proxy\n}",
        ),
        source(
            "scenario",
            "scenarios/flow.schemra",
            "scenario flow for visual app {\n token request {}\n move request from client -> proxy {}\n}",
        ),
    ]
}

#[test]
fn visual_rename_updates_definition_and_cross_file_scenario_reference() {
    let sources = project_sources();
    let offset = sources[0].source.find("app").unwrap() + 1;
    let rename = rename_project(&sources, &sources[0].path, offset, "service_map")
        .expect("visual rename should preserve project links");

    assert_eq!(rename.selection.name, "app");
    assert_eq!(rename.selection.kind, EditorSymbolKind::Visual);
    assert_eq!(rename.edits.len(), 2);
    assert_eq!(rename.edits[0].path, "scenarios/flow.schemra");
    assert_eq!(rename.edits[1].path, "visuals/app.schemra");
    assert!(
        rename
            .edits
            .iter()
            .all(|edit| edit.replacement == "service_map")
    );
    assert_eq!(sources[0].source.matches("app").count(), 1);
    assert_eq!(sources[1].source.matches("app").count(), 1);
}

#[test]
fn node_rename_updates_definition_edges_and_scenario_actions() {
    let sources = project_sources();
    let offset = sources[0].source.find("client").unwrap() + 2;
    let rename = rename_project(&sources, &sources[0].path, offset, "browser")
        .expect("node rename should update all linked uses");

    assert_eq!(rename.selection.kind, EditorSymbolKind::Node);
    assert_eq!(rename.selection.name, "client");
    assert_eq!(rename.edits.len(), 3);
    assert!(
        rename
            .edits
            .iter()
            .all(|edit| edit.replacement == "browser")
    );
    assert_eq!(rename.edits[0].path, "scenarios/flow.schemra");
    assert_eq!(rename.edits[1].path, "visuals/app.schemra");
    assert_eq!(rename.edits[2].path, "visuals/app.schemra");
}

#[test]
fn prepare_rename_returns_exact_source_selection() {
    let sources = project_sources();
    let offset = sources[1].source.find("flow").unwrap() + 1;
    let selection = prepare_project_rename(&sources, &sources[1].path, offset)
        .expect("scenario identifier is renameable");

    assert_eq!(selection.name, "flow");
    assert_eq!(selection.kind, EditorSymbolKind::Scenario);
    assert_eq!(
        &sources[1].source[selection.span.start.offset..selection.span.end.offset],
        "flow"
    );
}

#[test]
fn collisions_and_invalid_identifiers_are_rejected_without_changing_sources() {
    let sources = project_sources();
    let original = sources.clone();
    let client_offset = sources[0].source.find("client").unwrap();

    assert!(rename_project(&sources, &sources[0].path, client_offset, "proxy").is_err());
    assert!(rename_project(&sources, &sources[0].path, client_offset, "two words").is_err());
    assert_eq!(sources[0].source, original[0].source);
    assert_eq!(sources[1].source, original[1].source);
}

#[test]
fn rename_requires_a_valid_path_offset_and_error_free_project() {
    let sources = project_sources();
    assert!(prepare_project_rename(&sources, "missing.schemra", 0).is_err());
    assert!(prepare_project_rename(&sources, &sources[0].path, usize::MAX).is_err());

    let broken = vec![source(
        "broken",
        "broken.schemra",
        "visual app { node client {} }\nscenario flow for visual unknown { wait 1ms }",
    )];
    assert!(prepare_project_rename(&broken, "broken.schemra", 7).is_err());
}

#[test]
fn same_scenario_id_on_different_visuals_remains_unambiguous() {
    let sources = vec![
        source(
            "first",
            "visuals/first.schemra",
            "visual first { node a {} }",
        ),
        source(
            "second",
            "visuals/second.schemra",
            "visual second { node b {} }",
        ),
        source(
            "first_scenario",
            "scenarios/first.schemra",
            "scenario flow for visual first {\n wait 1ms\n}",
        ),
        source(
            "second_scenario",
            "scenarios/second.schemra",
            "scenario flow for visual second {\n wait 1ms\n}",
        ),
    ];
    let offset = sources[2].source.find("flow").unwrap();

    let rename = rename_project(&sources, &sources[2].path, offset, "first_flow")
        .expect("scenario IDs are scoped by their visual");
    assert_eq!(rename.edits.len(), 1);
    assert_eq!(rename.edits[0].path, "scenarios/first.schemra");
}
