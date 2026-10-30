//! Data-driven schemas for the Schemra construct language.

mod compiler;
mod completion;
mod editor;
mod editor_metadata;
mod project;
mod registry;
mod rename;
mod schema;
mod visual_repeat;

pub use compiler::{
    ConstructHandler, LoweredItem, LoweringContext, ValidatedConstruct, compile, compile_sources,
    compile_sources_with_registry, validate_scenario_for_visual, validate_scenarios_for_visual,
};
pub use completion::{
    CompletionItem, CompletionKind, CompletionList, complete_at, complete_at_with_nodes,
};
pub use editor::{SyntaxToken, SyntaxTokenKind, highlight_tokens};
pub use editor_metadata::{
    DeclarationInfo, DeclarationKind, EditorSymbol, EditorSymbolKind, EditorSymbolRole,
    editor_metadata,
};
pub use project::{
    ProjectAnalysis, ProjectFileAnalysis, ProjectSource, ProjectVisualAnalysis, analyze_project,
};
pub use registry::{Registry, RegistryBuilder, RegistryError};
pub use rename::{
    RenameEdit, RenameResult, RenameSelection, prepare_project_rename, rename_project,
};
pub use schema::{
    BodySpec, ConstructSpec, HeaderKind, HeaderParam, PropertyKind, PropertySpec, Scope,
};
