use schemra_compiler::{
    BodySpec, ConstructHandler, ConstructSpec, HeaderKind, HeaderParam, LoweredItem,
    LoweringContext, PropertyKind, PropertySpec, Registry, Scope, ValidatedConstruct,
    compile_sources_with_registry,
};
use schemra_ir::VisualId;
use schemra_syntax::{Diagnostic, StatementKind, parse_syntax};

static FILE_SCOPE: &[Scope] = &[Scope::File];
static VISUAL_SCOPE: &[Scope] = &[Scope::Visual];
static VISUAL_OR_GROUP: &[Scope] = &[Scope::Visual, Scope::Group];
static CUSTOM_HEADER: &[HeaderParam] = &[HeaderParam {
    kind: HeaderKind::String,
    optional: true,
}];
static NODE_PROPERTIES: &[PropertySpec] = &[PropertySpec {
    name: "icon",
    kind: PropertyKind::String,
}];
static INTEGER_NODE_PROPERTIES: &[PropertySpec] = &[PropertySpec {
    name: "icon",
    kind: PropertyKind::Integer,
}];
static GRID_VALUES: &[&str] = &["grid", "grid"];
static INVALID_ENUM: &[PropertySpec] = &[PropertySpec {
    name: "layout",
    kind: PropertyKind::Value(GRID_VALUES),
}];

static CUSTOM_VISUAL: ConstructSpec = ConstructSpec {
    name: "customvisual",
    contexts: FILE_SCOPE,
    header: CUSTOM_HEADER,
    body: BodySpec::Scope(Scope::Visual),
    properties: &[],
};
static NODE_SCHEMA: ConstructSpec = ConstructSpec {
    name: "node_schema",
    contexts: VISUAL_OR_GROUP,
    header: &[],
    body: BodySpec::Properties(Scope::Node),
    properties: NODE_PROPERTIES,
};
static CONFLICTING_NODE_SCHEMA: ConstructSpec = ConstructSpec {
    name: "other_node_schema",
    contexts: VISUAL_OR_GROUP,
    header: &[],
    body: BodySpec::Properties(Scope::Node),
    properties: INTEGER_NODE_PROPERTIES,
};
static INVALID_ENUM_SCHEMA: ConstructSpec = ConstructSpec {
    name: "invalid_enum",
    contexts: VISUAL_SCOPE,
    header: &[],
    body: BodySpec::Properties(Scope::Node),
    properties: INVALID_ENUM,
};

static CUSTOM_EXTENSION: ConstructSpec = ConstructSpec {
    name: "custom_extension",
    contexts: FILE_SCOPE,
    header: &[],
    body: BodySpec::None,
    properties: &[],
};

struct CustomExtension;

static CUSTOM_EXTENSION_HANDLER: CustomExtension = CustomExtension;

impl ConstructHandler for CustomExtension {
    fn spec(&self) -> &'static ConstructSpec {
        &CUSTOM_EXTENSION
    }

    fn lower(
        &self,
        _construct: &ValidatedConstruct<'_>,
        _context: &mut LoweringContext<'_>,
    ) -> Result<LoweredItem, Diagnostic> {
        Ok(LoweredItem::Ignored)
    }
}

#[test]
fn custom_constructs_register_without_changing_generic_parser_behavior() {
    let syntax = parse_syntax("customvisual \"Custom map\" {}");
    assert!(syntax.diagnostics.is_empty());
    assert!(matches!(
        syntax.statements[0].kind,
        StatementKind::Invocation { .. }
    ));

    let mut builder = Registry::builder();
    builder.register(&CUSTOM_VISUAL).expect("valid schema");
    let registry = builder.build();

    let StatementKind::Invocation { name, .. } = &syntax.statements[0].kind else {
        panic!("the generic parser should retain the custom invocation");
    };

    assert_eq!(registry.spec("customvisual"), Some(&CUSTOM_VISUAL));
    assert_eq!(registry.spec(&name.text), Some(&CUSTOM_VISUAL));
}

#[test]
fn duplicate_names_and_conflicting_property_types_are_rejected() {
    let mut builder = Registry::builder();
    builder.register(&NODE_SCHEMA).expect("first schema");
    assert!(builder.register(&NODE_SCHEMA).is_err());
    assert!(builder.register(&CONFLICTING_NODE_SCHEMA).is_err());
}

#[test]
fn invalid_enum_schemas_are_rejected() {
    let mut builder = Registry::builder();
    let failure = builder.register(&INVALID_ENUM_SCHEMA);
    assert!(failure.is_err());
}

#[test]
fn registered_schema_queries_are_scope_specific() {
    let mut builder = Registry::builder();
    builder
        .register(&CUSTOM_VISUAL)
        .expect("custom visual schema");
    builder.register(&NODE_SCHEMA).expect("node schema");
    let registry = builder.build();

    assert_eq!(registry.constructs_in_scope(Scope::File).count(), 1);
    assert_eq!(registry.properties_for_scope(Scope::Node).count(), 1);
    assert_eq!(
        registry
            .property_spec(Scope::Node, "icon")
            .map(|property| property.kind),
        Some(PropertyKind::String)
    );
    assert_eq!(registry.property_spec(Scope::Group, "icon"), None);
}

#[test]
fn builtin_builder_accepts_extensions_and_compiles_them_with_builtin_constructs() {
    let mut builder = Registry::builder_with_builtins().expect("builtin handlers register");
    builder
        .register_handler(&CUSTOM_EXTENSION_HANDLER)
        .expect("custom handler registers alongside builtins");
    let registry = builder.build();

    let module = compile_sources_with_registry(
        &["visual network { node client {} }\ncustom_extension"],
        &registry,
    )
    .expect("builtin visual and custom extension compile together");

    assert_eq!(module.visuals.len(), 1);
    assert_eq!(module.visuals[0].id, VisualId("network".into()));
    assert!(registry.handler("custom_extension").is_some());
}
