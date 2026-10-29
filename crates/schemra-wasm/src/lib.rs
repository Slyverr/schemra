//! Browser-facing JSON API backed by the shared compiler and rendering crates.

mod compilation;
mod editor_api;
mod playback_api;

pub use playback_api::ScenarioSampler;

/// Compiles one Schemra source document and returns its JSON result.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn compile_schemra(source: &str) -> String {
    compilation::compile_source_json(source)
}

/// Compiles and links the source documents in a project JSON request.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn compile_project_schemra(sources_json: &str) -> String {
    compilation::compile_project_json(sources_json)
}

/// Formats source with the shared language formatter.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn format_schemra(source: &str) -> String {
    editor_api::format_source(source)
}

/// Returns context-aware language completions as JSON.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn complete_schemra(source: &str, cursor_offset: usize, explicit: bool) -> String {
    editor_api::complete(source, cursor_offset, explicit)
}

/// Returns completions with project node IDs supplied as a JSON context object.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn complete_schemra_with_nodes(
    source: &str,
    cursor_offset: usize,
    explicit: bool,
    context_json: &str,
) -> String {
    editor_api::complete_with_nodes(source, cursor_offset, explicit, context_json)
}

/// Prepares or applies an identifier rename across a linked project.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn rename_project_schemra(request_json: &str) -> String {
    editor_api::rename_project(request_json)
}
