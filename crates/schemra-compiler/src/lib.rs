//! Data-driven schemas for the Schemra construct language.

mod compiler;
mod editor;
mod editor_metadata;
mod registry;
mod schema;

pub use compiler::{
    ConstructHandler, LoweredItem, LoweringContext, ValidatedConstruct, compile, compile_sources,
    compile_sources_with_registry, validate_scenario_for_visual,
};
pub use editor::{SyntaxToken, SyntaxTokenKind, highlight_tokens};
pub use editor_metadata::{
    DeclarationInfo, DeclarationKind, EditorSymbol, EditorSymbolKind, EditorSymbolRole,
    editor_metadata,
};
pub use registry::{Registry, RegistryBuilder, RegistryError};
pub use schema::{
    BodySpec, ConstructSpec, HeaderKind, HeaderParam, PropertyKind, PropertySpec, Scope,
};
