use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::{BodySpec, ConstructSpec, HeaderKind, PropertyKind, PropertySpec, Scope};

/// Immutable set of construct schemas available to compiler stages.
#[derive(Default)]
pub struct Registry {
    constructs: BTreeMap<&'static str, &'static ConstructSpec>,
}

impl Registry {
    pub fn builder() -> RegistryBuilder {
        RegistryBuilder::default()
    }

    pub fn spec(&self, name: &str) -> Option<&'static ConstructSpec> {
        self.constructs.get(name).copied()
    }

    pub fn constructs_in_scope(
        &self,
        scope: Scope,
    ) -> impl Iterator<Item = &'static ConstructSpec> + '_ {
        self.constructs
            .values()
            .copied()
            .filter(move |spec| spec.contexts.contains(&scope))
    }

    pub fn properties_for_scope(
        &self,
        scope: Scope,
    ) -> impl Iterator<Item = &'static PropertySpec> + '_ {
        self.constructs
            .values()
            .copied()
            .filter(move |spec| body_scope(spec.body) == Some(scope))
            .flat_map(|spec| spec.properties)
    }

    pub fn property_spec(&self, scope: Scope, name: &str) -> Option<&'static PropertySpec> {
        self.properties_for_scope(scope)
            .find(|property| property.name == name)
    }
}

#[derive(Default)]
pub struct RegistryBuilder {
    constructs: BTreeMap<&'static str, &'static ConstructSpec>,
}

impl RegistryBuilder {
    pub fn register(&mut self, spec: &'static ConstructSpec) -> Result<&mut Self, RegistryError> {
        if self.constructs.contains_key(spec.name) {
            return Err(RegistryError {
                name: spec.name,
                reason: "construct name is already registered",
            });
        }
        if !valid_name(spec.name) {
            return Err(RegistryError {
                name: spec.name,
                reason: "construct name must be an ASCII identifier",
            });
        }
        if spec.contexts.is_empty() {
            return Err(RegistryError {
                name: spec.name,
                reason: "construct must declare at least one legal scope",
            });
        }
        if spec
            .contexts
            .iter()
            .enumerate()
            .any(|(index, scope)| spec.contexts[..index].contains(scope))
        {
            return Err(RegistryError {
                name: spec.name,
                reason: "construct scopes must be unique",
            });
        }
        if !body_matches_contexts(spec) {
            return Err(RegistryError {
                name: spec.name,
                reason: "construct body is incompatible with its allowed scopes",
            });
        }

        let mut optional_seen = false;
        for parameter in spec.header {
            optional_seen |= parameter.optional;
            if optional_seen && !parameter.optional {
                return Err(RegistryError {
                    name: spec.name,
                    reason: "optional header parameters must be trailing",
                });
            }
            if matches!(parameter.kind, HeaderKind::Literal(value) if !valid_name(value)) {
                return Err(RegistryError {
                    name: spec.name,
                    reason: "literal header terms must be ASCII identifiers",
                });
            }
        }

        let mut property_names = HashSet::new();
        for property in spec.properties {
            if !valid_name(property.name) || !property_names.insert(property.name) {
                return Err(RegistryError {
                    name: spec.name,
                    reason: "property names must be valid and unique",
                });
            }
            if matches!(property.kind, PropertyKind::Value(values) if values.is_empty()) {
                return Err(RegistryError {
                    name: spec.name,
                    reason: "enum properties must declare at least one accepted value",
                });
            }
            if let PropertyKind::Value(values) = property.kind
                && values.iter().collect::<BTreeSet<_>>().len() != values.len()
            {
                return Err(RegistryError {
                    name: spec.name,
                    reason: "enum property values must be unique",
                });
            }
            if matches!(property.kind, PropertyKind::Value(values) if values.iter().any(|value| !valid_bare_value(value)))
            {
                return Err(RegistryError {
                    name: spec.name,
                    reason: "enum property values must be valid bare values",
                });
            }

            let conflicts_with_existing = self.constructs.values().any(|registered| {
                body_scope(registered.body) == body_scope(spec.body)
                    && registered
                        .properties
                        .iter()
                        .any(|known| known.name == property.name && known.kind != property.kind)
            });
            if conflicts_with_existing {
                return Err(RegistryError {
                    name: spec.name,
                    reason: "property schemas conflict within a scope",
                });
            }
        }

        self.constructs.insert(spec.name, spec);
        Ok(self)
    }

    pub fn build(self) -> Registry {
        Registry {
            constructs: self.constructs,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryError {
    pub name: &'static str,
    pub reason: &'static str,
}

impl Display for RegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid construct `{}`: {}",
            self.name, self.reason
        )
    }
}

impl Error for RegistryError {}

fn body_matches_contexts(spec: &ConstructSpec) -> bool {
    let compatible_parents: &[Scope] = match spec.body {
        BodySpec::None => return spec.properties.is_empty(),
        BodySpec::Properties(Scope::Node) => &[Scope::Visual, Scope::Group],
        BodySpec::Properties(Scope::Token) => &[Scope::Scenario],
        BodySpec::Properties(Scope::Action) => &[Scope::Scenario, Scope::Action],
        BodySpec::Properties(Scope::Edge) => &[Scope::Visual],
        BodySpec::Properties(_) => return false,
        BodySpec::Scope(Scope::Visual | Scope::Scenario) => &[Scope::File],
        BodySpec::Scope(Scope::Group) => &[Scope::Visual],
        BodySpec::Scope(Scope::Action) => &[Scope::Scenario, Scope::Action],
        BodySpec::Scope(_) => return false,
    };

    spec.contexts
        .iter()
        .all(|context| compatible_parents.contains(context))
}

fn body_scope(body: BodySpec) -> Option<Scope> {
    match body {
        BodySpec::None => None,
        BodySpec::Properties(scope) | BodySpec::Scope(scope) => Some(scope),
    }
}

fn valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn valid_bare_value(value: &str) -> bool {
    !value.is_empty() && value.split('-').all(valid_name)
}
