//! Data-driven schemas for the Schemra construct language.

mod compiler;
mod registry;
mod schema;

pub use compiler::{
    ConstructHandler, LoweredItem, LoweringContext, ValidatedConstruct, compile, compile_sources,
    compile_sources_with_registry, validate_scenario_for_visual,
};
pub use registry::{Registry, RegistryBuilder, RegistryError};
pub use schema::{
    BodySpec, ConstructSpec, HeaderKind, HeaderParam, PropertyKind, PropertySpec, Scope,
};
