use crate::compilation;
use schemra_compiler::{
    ProjectSource, complete_at, complete_at_with_nodes, prepare_project_rename,
    rename_project as apply_project_rename,
};
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) fn format_source(source: &str) -> String {
    schemra_syntax::format_source(source)
}

pub(super) fn complete(source: &str, cursor_offset: usize, explicit: bool) -> String {
    serde_json::to_string(&complete_at(source, cursor_offset, explicit))
        .unwrap_or_else(|_| "null".to_owned())
}

pub(super) fn complete_with_nodes(
    source: &str,
    cursor_offset: usize,
    explicit: bool,
    context_json: &str,
) -> String {
    let node_ids = serde_json::from_str::<CompletionContext>(context_json)
        .map(|context| context.node_ids)
        .unwrap_or_default();
    serde_json::to_string(&complete_at_with_nodes(
        source,
        cursor_offset,
        explicit,
        &node_ids,
    ))
    .unwrap_or_else(|_| "null".to_owned())
}

pub(super) fn rename_project(request_json: &str) -> String {
    let result = (|| -> Result<Value, String> {
        if compilation::validate_project_request_size(request_json.len()).is_err() {
            return Err(
                "The project rename request exceeds the configured JSON size limit.".into(),
            );
        }
        let request: RenameRequest = serde_json::from_str(request_json)
            .map_err(|error| format!("Could not read the project rename request: {error}"))?;
        compilation::validate_project_source_list(&request.sources)?;

        if let Some(new_name) = request.new_name {
            let result =
                apply_project_rename(&request.sources, &request.path, request.offset, &new_name)?;
            Ok(json!({"error": null, "selection": result.selection, "edits": result.edits}))
        } else {
            let selection =
                prepare_project_rename(&request.sources, &request.path, request.offset)?;
            Ok(json!({"error": null, "selection": selection, "edits": []}))
        }
    })();

    result
        .unwrap_or_else(|error| json!({"error": error, "selection": null, "edits": []}))
        .to_string()
}

#[derive(Deserialize)]
struct RenameRequest {
    sources: Vec<ProjectSource>,
    path: String,
    offset: usize,
    #[serde(default)]
    new_name: Option<String>,
}

#[derive(Deserialize)]
struct CompletionContext {
    node_ids: Vec<String>,
}
