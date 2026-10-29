use schemra_ir::{EdgeDirectionality, EdgeIr, GroupId, ModuleIr, NodeId, NodeIr, VisualIr};
use schemra_syntax::{Diagnostic, Statement, StatementKind, Token, TokenKind, parse_syntax};

use crate::registry::Registry;
use crate::{BodySpec, ConstructSpec, RegistryError, Scope};

/// A construct after syntax and schema validation, ready for lowering.
pub struct ValidatedConstruct<'a> {
    pub statement: &'a Statement,
    pub scope: Scope,
    pub parent_group: Option<GroupId>,
}

impl<'a> ValidatedConstruct<'a> {
    pub fn name(&self) -> Option<&str> {
        match &self.statement.kind {
            StatementKind::Invocation { name, .. } => Some(&name.text),
            _ => None,
        }
    }

    pub fn arguments(&self) -> &[Token] {
        match &self.statement.kind {
            StatementKind::Invocation { arguments, .. } => arguments,
            _ => &[],
        }
    }

    pub fn body(&self) -> Option<&'a [Statement]> {
        match &self.statement.kind {
            StatementKind::Invocation { body, .. } => body.as_deref(),
            _ => None,
        }
    }
}

/// Typed value produced by a registered construct handler.
#[derive(Clone, Debug, PartialEq)]
pub enum LoweredItem {
    Visual(VisualIr),
    Node(NodeIr),
    Ignored,
}

/// Gives construct handlers access to registered child handlers.
pub struct LoweringContext<'a> {
    registry: &'a Registry,
}

impl LoweringContext<'_> {
    pub fn lower_child(
        &mut self,
        statement: &Statement,
        scope: Scope,
    ) -> Result<LoweredItem, Diagnostic> {
        self.lower_child_in_group(statement, scope, None)
    }

    pub fn lower_child_in_group(
        &mut self,
        statement: &Statement,
        scope: Scope,
        parent_group: Option<GroupId>,
    ) -> Result<LoweredItem, Diagnostic> {
        let StatementKind::Invocation { name, .. } = &statement.kind else {
            return Ok(LoweredItem::Ignored);
        };
        let Some(handler) = self.registry.handler(&name.text) else {
            return Err(Diagnostic::error(
                "E_CONSTRUCT_HANDLER",
                format!(
                    "Construct `{}` has no registered compiler handler.",
                    name.text
                ),
                name.span,
                None::<String>,
            ));
        };
        handler.lower(
            &ValidatedConstruct {
                statement,
                scope,
                parent_group,
            },
            self,
        )
    }

    pub fn lower_children(
        &mut self,
        construct: &ValidatedConstruct<'_>,
        parent_group: Option<GroupId>,
    ) -> Result<Vec<LoweredItem>, Diagnostic> {
        let Some(scope) = construct
            .name()
            .and_then(|name| self.registry.spec(name))
            .and_then(|spec| match spec.body {
                BodySpec::Scope(scope) => Some(scope),
                BodySpec::None | BodySpec::Properties(_) => None,
            })
        else {
            return Ok(Vec::new());
        };
        let Some(statements) = construct.body() else {
            return Ok(Vec::new());
        };

        statements
            .iter()
            .filter(|statement| matches!(statement.kind, StatementKind::Invocation { .. }))
            .map(|statement| self.lower_child_in_group(statement, scope, parent_group.clone()))
            .collect()
    }

    pub fn lower_edges(&self, statements: &[Statement]) -> Vec<EdgeIr> {
        statements
            .iter()
            .filter_map(|statement| {
                let StatementKind::Edge {
                    from, operator, to, ..
                } = &statement.kind
                else {
                    return None;
                };
                Some(EdgeIr {
                    from: NodeId(from.text.clone()),
                    from_span: from.span,
                    to: NodeId(to.text.clone()),
                    to_span: to.span,
                    directionality: if matches!(operator.kind, TokenKind::Undirected) {
                        EdgeDirectionality::Undirected
                    } else {
                        EdgeDirectionality::Directed
                    },
                    span: statement.span,
                })
            })
            .collect()
    }
}

/// Lowers a registered construct from validated generic syntax.
pub trait ConstructHandler: Send + Sync {
    fn spec(&self) -> &'static ConstructSpec;

    fn lower(
        &self,
        construct: &ValidatedConstruct<'_>,
        context: &mut LoweringContext<'_>,
    ) -> Result<LoweredItem, Diagnostic>;
}

pub(super) type LowerFn = for<'a, 'r> fn(
    &ValidatedConstruct<'a>,
    &mut LoweringContext<'r>,
) -> Result<LoweredItem, Diagnostic>;

pub(super) struct BuiltinHandler {
    spec: &'static ConstructSpec,
    lower: LowerFn,
}

impl ConstructHandler for BuiltinHandler {
    fn spec(&self) -> &'static ConstructSpec {
        self.spec
    }

    fn lower(
        &self,
        construct: &ValidatedConstruct<'_>,
        context: &mut LoweringContext<'_>,
    ) -> Result<LoweredItem, Diagnostic> {
        (self.lower)(construct, context)
    }
}

pub(super) const fn handler(spec: &'static ConstructSpec, lower: LowerFn) -> BuiltinHandler {
    BuiltinHandler { spec, lower }
}

/// Parse, validate, and lower a source file into typed visual IR.
pub fn compile(source: &str) -> Result<ModuleIr, Vec<Diagnostic>> {
    compile_sources(&[source])
}

pub fn compile_sources(sources: &[&str]) -> Result<ModuleIr, Vec<Diagnostic>> {
    let registry = Registry::builtin_shared().map_err(registry_diagnostic)?;
    compile_sources_with_registry(sources, registry)
}

pub fn compile_sources_with_registry(
    sources: &[&str],
    registry: &Registry,
) -> Result<ModuleIr, Vec<Diagnostic>> {
    let mut syntax_files = Vec::with_capacity(sources.len());
    let mut diagnostics = Vec::new();
    for source in sources {
        let syntax = parse_syntax(source);
        diagnostics.extend(registry.validate(&syntax));
        syntax_files.push(syntax);
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut module = ModuleIr::default();
    for syntax in &syntax_files {
        let mut context = LoweringContext { registry };
        for statement in &syntax.statements {
            match context.lower_child(statement, Scope::File) {
                Ok(LoweredItem::Visual(visual)) => module.visuals.push(visual),
                Ok(LoweredItem::Node(_) | LoweredItem::Ignored) => {}
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }
    }
    validate_visuals(&module, &mut diagnostics);
    if diagnostics.is_empty() {
        Ok(module)
    } else {
        Err(diagnostics)
    }
}

fn registry_diagnostic(error: RegistryError) -> Vec<Diagnostic> {
    vec![Diagnostic::error(
        "E_REGISTRY",
        format!("Could not construct the builtin registry: {error}."),
        schemra_syntax::SourceSpan::point(schemra_syntax::SourceLocation {
            offset: 0,
            line: 1,
            column: 1,
        }),
        None::<String>,
    )]
}

fn validate_visuals(module: &ModuleIr, diagnostics: &mut Vec<Diagnostic>) {
    for visual in &module.visuals {
        if visual.title.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "E_EMPTY_VISUAL_TITLE",
                "A visual title cannot be empty.",
                visual.span,
                None::<String>,
            ));
        }
        if visual.nodes.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E_EMPTY_VISUAL",
                "A visual needs at least one node.",
                visual.span,
                None::<String>,
            ));
        }
        for node in &visual.nodes {
            if node.label.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "E_EMPTY_NODE_LABEL",
                    format!("Node `{}` needs a visible label.", node.id.0),
                    node.span,
                    None::<String>,
                ));
            }
        }
    }
}

#[path = "builtins/mod.rs"]
mod builtins;

pub(crate) fn builtin_registry() -> Result<Registry, RegistryError> {
    let mut builder = Registry::builder();
    for handler in builtins::handlers() {
        builder.register_handler(handler)?;
    }
    Ok(builder.build())
}
