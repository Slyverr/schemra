use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::OnceLock;

use schemra_syntax::{
    Diagnostic, SourceLocation, SourceSpan, Statement, StatementKind, SyntaxFile, Token, TokenKind,
};

use crate::compiler::{ConstructHandler, ValidatedConstruct, builtin_registry};
use crate::{BodySpec, ConstructSpec, HeaderKind, PropertyKind, PropertySpec, Scope};

/// Immutable set of construct schemas available to compiler stages.
#[derive(Default)]
pub struct Registry {
    constructs: BTreeMap<&'static str, &'static ConstructSpec>,
    handlers: BTreeMap<&'static str, &'static dyn ConstructHandler>,
}

impl Registry {
    pub fn builder() -> RegistryBuilder {
        RegistryBuilder::default()
    }

    /// Creates a builder preloaded with the builtin constructs and handlers.
    pub fn builder_with_builtins() -> Result<RegistryBuilder, RegistryError> {
        crate::compiler::builtin_builder()
    }

    pub fn builtin() -> Result<Self, RegistryError> {
        builtin_registry()
    }

    pub fn builtin_shared() -> Result<&'static Self, RegistryError> {
        static REGISTRY: OnceLock<Result<Registry, RegistryError>> = OnceLock::new();

        match REGISTRY.get_or_init(builtin_registry) {
            Ok(registry) => Ok(registry),
            Err(error) => Err(error.clone()),
        }
    }

    pub fn spec(&self, name: &str) -> Option<&'static ConstructSpec> {
        self.constructs.get(name).copied()
    }

    pub fn handler(&self, name: &str) -> Option<&'static dyn ConstructHandler> {
        self.handlers.get(name).copied()
    }

    pub(crate) fn header_is_valid(&self, name: &str, arguments: &[Token]) -> bool {
        self.spec(name)
            .is_some_and(|spec| header_matches_spec(spec, arguments))
    }

    /// Validate generic syntax against the registered construct and property schemas.
    pub fn validate(&self, file: &SyntaxFile) -> Vec<Diagnostic> {
        let mut diagnostics = file.diagnostics.clone();
        validate_scope(&file.statements, Scope::File, self, &mut diagnostics);
        diagnostics
    }

    pub(crate) fn validate_statements_in_scope(
        &self,
        statements: &[Statement],
        scope: Scope,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        validate_scope(statements, scope, self, &mut diagnostics);
        diagnostics
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

    pub(crate) fn is_highlighted_word(&self, text: &str) -> bool {
        self.constructs.values().any(|spec| {
            spec.name == text
                || spec.header.iter().any(
                    |parameter| matches!(parameter.kind, HeaderKind::Literal(literal) if literal == text),
                )
                || spec.properties.iter().any(|property| property.name == text)
        })
    }

    pub(crate) fn is_registered_property_value(&self, name: &str, value: &str) -> bool {
        self.constructs.values().any(|spec| {
            spec.properties.iter().any(|property| {
                property.name == name
                    && match property.kind {
                        PropertyKind::Value(values) => values.contains(&value),
                        PropertyKind::Highlight => matches!(
                            value,
                            "off" | "trace" | "pulse" | "left" | "right" | "top" | "bottom"
                        ),
                        _ => false,
                    }
            })
        })
    }
}

#[derive(Default)]
pub struct RegistryBuilder {
    constructs: BTreeMap<&'static str, &'static ConstructSpec>,
    handlers: BTreeMap<&'static str, &'static dyn ConstructHandler>,
}

impl RegistryBuilder {
    pub fn register_handler(
        &mut self,
        handler: &'static dyn ConstructHandler,
    ) -> Result<&mut Self, RegistryError> {
        self.register(handler.spec())?;
        self.handlers.insert(handler.spec().name, handler);
        Ok(self)
    }

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
            handlers: self.handlers,
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
        BodySpec::OptionalProperties(Scope::Node) => &[Scope::Visual, Scope::Group],
        BodySpec::Properties(Scope::Token) => &[Scope::Scenario],
        BodySpec::OptionalProperties(Scope::Token) => &[Scope::Scenario],
        BodySpec::Properties(Scope::Action) => &[Scope::Scenario, Scope::Action],
        BodySpec::OptionalProperties(Scope::Action) => &[Scope::Scenario, Scope::Action],
        BodySpec::Properties(Scope::Edge) => &[Scope::Visual],
        BodySpec::OptionalProperties(Scope::Edge) => &[Scope::Visual],
        BodySpec::Properties(_) => return false,
        BodySpec::OptionalProperties(_) => return false,
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
        BodySpec::Properties(scope)
        | BodySpec::OptionalProperties(scope)
        | BodySpec::Scope(scope) => Some(scope),
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

fn validate_scope(
    statements: &[Statement],
    scope: Scope,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut properties_seen = HashSet::new();
    for statement in statements {
        match &statement.kind {
            StatementKind::Error => {}
            StatementKind::Property {
                name,
                value,
                additional_values,
            } => {
                if !properties_seen.insert(name.text.as_str()) {
                    diagnostics.push(Diagnostic::error(
                        "E_DUPLICATE_PROPERTY",
                        format!(
                            "Property `{}` is declared more than once in this block.",
                            name.text
                        ),
                        name.span,
                        None::<String>,
                    ));
                    continue;
                }

                let values = std::iter::once(value)
                    .chain(additional_values.iter())
                    .collect::<Vec<_>>();
                match registry.property_spec(scope, &name.text) {
                    Some(spec) if property_matches(spec.kind, &values) => {}
                    Some(spec) => diagnostics.push(Diagnostic::error(
                        "E_PROPERTY_TYPE",
                        format!("Property `{}` has a value of the wrong type.", name.text),
                        value.span,
                        Some(format!("Expected {}.", property_kind_name(spec.kind))),
                    )),
                    None => diagnostics.push(Diagnostic::error(
                        "E_UNKNOWN_PROPERTY",
                        format!("Property `{}` is not valid in this scope.", name.text),
                        name.span,
                        Some(format!(
                            "Supported properties: {}.",
                            properties_for_scope(scope, registry)
                        )),
                    )),
                }
            }
            StatementKind::Edge { properties, .. } => {
                if scope != Scope::Visual {
                    diagnostics.push(Diagnostic::error(
                        "E_SCOPE_EDGE",
                        "Edges are only valid in a visual scope.",
                        statement.span,
                        None::<String>,
                    ));
                }
                if let Some(properties) = properties {
                    validate_scope(properties, Scope::Edge, registry, diagnostics);
                }
            }
            StatementKind::Invocation {
                name,
                arguments,
                body,
            } => {
                let Some(spec) = registry.spec(&name.text) else {
                    diagnostics.push(Diagnostic::error(
                        "E_UNKNOWN_CONSTRUCT",
                        format!("Unknown construct `{}` in {scope:?} scope.", name.text),
                        name.span,
                        Some(format!(
                            "Registered constructs here: {}.",
                            constructs_for_scope(scope, registry)
                        )),
                    ));
                    continue;
                };

                if !spec.contexts.contains(&scope) {
                    diagnostics.push(Diagnostic::error(
                        "E_CONSTRUCT_SCOPE",
                        format!(
                            "Construct `{}` is not allowed in {scope:?} scope.",
                            name.text
                        ),
                        name.span,
                        Some(format!(
                            "Allowed scopes: {}.",
                            scopes_display(spec.contexts)
                        )),
                    ));
                }
                let valid_header = validate_header(spec, arguments, diagnostics);
                match (spec.body, body) {
                    (BodySpec::None, Some(_)) => diagnostics.push(Diagnostic::error(
                        "E_UNEXPECTED_BODY",
                        format!("Construct `{}` does not accept a body.", name.text),
                        statement.span,
                        None::<String>,
                    )),
                    (BodySpec::Scope(_) | BodySpec::Properties(_), None) => {
                        diagnostics.push(Diagnostic::error(
                            "E_EXPECTED_BODY",
                            format!("Construct `{}` requires a body.", name.text),
                            statement.span,
                            None::<String>,
                        ));
                    }
                    (BodySpec::Properties(child_scope), Some(children))
                    | (BodySpec::OptionalProperties(child_scope), Some(children))
                    | (BodySpec::Scope(child_scope), Some(children)) => {
                        validate_scope(children, child_scope, registry, diagnostics);
                    }
                    (BodySpec::OptionalProperties(_), None) => {}
                    (BodySpec::None, None) => {}
                }
                if valid_header && let Some(handler) = registry.handler(&name.text) {
                    handler.validate(&ValidatedConstruct::new(statement, scope), diagnostics);
                }
            }
        }
    }
}

fn properties_for_scope(scope: Scope, registry: &Registry) -> String {
    let properties = registry
        .properties_for_scope(scope)
        .map(|property| property.name)
        .collect::<BTreeSet<_>>();
    if properties.is_empty() {
        "none".into()
    } else {
        properties.into_iter().collect::<Vec<_>>().join(", ")
    }
}

fn constructs_for_scope(scope: Scope, registry: &Registry) -> String {
    let names = registry
        .constructs_in_scope(scope)
        .map(|spec| spec.name)
        .collect::<Vec<_>>();
    if names.is_empty() {
        "none".into()
    } else {
        names.join(", ")
    }
}

fn scopes_display(scopes: &[Scope]) -> String {
    scopes
        .iter()
        .map(|scope| format!("{scope:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn validate_header(
    spec: &ConstructSpec,
    arguments: &[Token],
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut argument_index = 0;
    let mut valid = true;
    for parameter in spec.header {
        let Some(argument) = arguments.get(argument_index) else {
            if !parameter.optional {
                valid = false;
                diagnostics.push(Diagnostic::error(
                    "E_CONSTRUCT_HEADER",
                    format!("Construct `{}` has an incomplete header.", spec.name),
                    arguments.last().map_or_else(
                        || {
                            SourceSpan::point(SourceLocation {
                                offset: 0,
                                line: 1,
                                column: 1,
                            })
                        },
                        |token| token.span,
                    ),
                    Some(format!("Expected: {}.", signature(spec))),
                ));
            }
            continue;
        };

        if !header_matches(parameter.kind, argument) {
            if parameter.optional {
                continue;
            }
            diagnostics.push(Diagnostic::error(
                "E_CONSTRUCT_HEADER",
                format!("Construct `{}` has an invalid header argument.", spec.name),
                argument.span,
                Some(format!("Expected: {}.", signature(spec))),
            ));
            return false;
        }
        argument_index += 1;
    }
    if argument_index != arguments.len() {
        valid = false;
        diagnostics.push(Diagnostic::error(
            "E_CONSTRUCT_HEADER",
            format!("Construct `{}` has too many header arguments.", spec.name),
            arguments[argument_index].span,
            Some(format!("Expected: {}.", signature(spec))),
        ));
    }
    valid
}

fn header_matches_spec(spec: &ConstructSpec, arguments: &[Token]) -> bool {
    let mut argument_index = 0;
    for parameter in spec.header {
        let Some(argument) = arguments.get(argument_index) else {
            if parameter.optional {
                continue;
            }
            return false;
        };

        if !header_matches(parameter.kind, argument) {
            if parameter.optional {
                continue;
            }
            return false;
        }
        argument_index += 1;
    }
    argument_index == arguments.len()
}

fn signature(spec: &ConstructSpec) -> String {
    let arguments = spec
        .header
        .iter()
        .map(|parameter| {
            let expected = match parameter.kind {
                HeaderKind::Identifier => "identifier",
                HeaderKind::NodeIdentifier => "node identifier",
                HeaderKind::TokenIdentifier => "token identifier",
                HeaderKind::String => "string",
                HeaderKind::Integer => "integer",
                HeaderKind::PositiveInteger => "positive integer",
                HeaderKind::Duration => "duration",
                HeaderKind::HighlightEffect => "trace or pulse",
                HeaderKind::HighlightSide => "left, right, top, or bottom",
                HeaderKind::Literal(value) => value,
                HeaderKind::Arrow => "->",
            };
            if parameter.optional {
                format!("[{expected}]")
            } else {
                expected.to_owned()
            }
        })
        .collect::<Vec<_>>();
    format!("{} {}", spec.name, arguments.join(" "))
        .trim()
        .to_owned()
}

fn header_matches(kind: HeaderKind, token: &Token) -> bool {
    match kind {
        HeaderKind::Identifier | HeaderKind::NodeIdentifier | HeaderKind::TokenIdentifier => {
            matches!(token.kind, TokenKind::Identifier)
        }
        HeaderKind::String => matches!(token.kind, TokenKind::String(_)),
        HeaderKind::Integer => matches!(token.kind, TokenKind::Integer(_)),
        HeaderKind::PositiveInteger => match &token.kind {
            TokenKind::Integer(value) => value.parse::<u32>().is_ok_and(|value| value > 0),
            _ => false,
        },
        HeaderKind::Duration => duration_value(token).is_some(),
        HeaderKind::HighlightEffect => matches!(token.text.as_str(), "trace" | "pulse"),
        HeaderKind::HighlightSide => {
            matches!(token.text.as_str(), "left" | "right" | "top" | "bottom")
        }
        HeaderKind::Literal(expected) => token.text == expected,
        HeaderKind::Arrow => matches!(token.kind, TokenKind::Arrow),
    }
}

fn property_matches(kind: PropertyKind, values: &[&Token]) -> bool {
    let Some(token) = values.first().copied() else {
        return false;
    };
    match kind {
        PropertyKind::String => values.len() == 1 && matches!(token.kind, TokenKind::String(_)),
        PropertyKind::Integer => values.len() == 1 && matches!(token.kind, TokenKind::Integer(_)),
        PropertyKind::PositiveInteger => match &token.kind {
            TokenKind::Integer(value) if values.len() == 1 => {
                value.parse::<u64>().is_ok_and(|value| value > 0)
            }
            _ => false,
        },
        PropertyKind::Duration => values.len() == 1 && duration_value(token).is_some(),
        PropertyKind::PositiveDuration => {
            values.len() == 1 && duration_value(token).is_some_and(|duration| duration > 0)
        }
        PropertyKind::Highlight => {
            if token.text == "off" {
                values.len() == 1
                    && matches!(token.kind, TokenKind::Identifier | TokenKind::BareValue(_))
            } else {
                duration_value(token).is_some_and(|duration| duration > 0)
                    && match &values[1..] {
                        [] => true,
                        [effect] => matches!(effect.text.as_str(), "trace" | "pulse"),
                        [from, side] => {
                            from.text == "from"
                                && matches!(side.text.as_str(), "left" | "right" | "top" | "bottom")
                        }
                        [effect, from, side] => {
                            effect.text == "trace"
                                && from.text == "from"
                                && matches!(side.text.as_str(), "left" | "right" | "top" | "bottom")
                        }
                        _ => false,
                    }
            }
        }
        PropertyKind::Value(options) => {
            values.len() == 1 && options.iter().any(|value| token.text == *value)
        }
    }
}

fn duration_value(token: &Token) -> Option<u64> {
    let TokenKind::Duration(value) = &token.kind else {
        return None;
    };
    if let Some(milliseconds) = value.strip_suffix("ms") {
        milliseconds.parse().ok()
    } else if let Some(seconds) = value.strip_suffix('s') {
        seconds.parse::<u64>().ok()?.checked_mul(1_000)
    } else {
        None
    }
}

fn property_kind_name(kind: PropertyKind) -> String {
    match kind {
        PropertyKind::String => "a string".into(),
        PropertyKind::Integer => "an integer".into(),
        PropertyKind::PositiveInteger => "a positive integer".into(),
        PropertyKind::Duration => "a duration such as 500ms or 2s".into(),
        PropertyKind::PositiveDuration => "a positive duration such as 500ms or 2s".into(),
        PropertyKind::Highlight => {
            "a positive duration, optionally followed by `trace` or `pulse`, or `off`".into()
        }
        PropertyKind::Value(values) => format!("one of {}", values.join(", ")),
    }
}
