//! Data-driven schemas for the Schemra construct language.

mod registry;
mod schema;

pub use registry::{Registry, RegistryBuilder, RegistryError};
pub use schema::{
    BodySpec, ConstructSpec, HeaderKind, HeaderParam, PropertyKind, PropertySpec, Scope,
};
