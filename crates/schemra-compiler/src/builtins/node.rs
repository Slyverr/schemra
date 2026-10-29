use schemra_ir::{NodeId, NodeIr};
use schemra_syntax::{Diagnostic, Statement, StatementKind, TokenKind};

use crate::compiler::{LoweredItem, LoweringContext, ValidatedConstruct};
use crate::{BodySpec, ConstructSpec, Scope};

use super::{NODE_PROPERTIES, OPTIONAL_TITLE, VISUAL_AND_GROUP_SCOPE, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "node",
    contexts: VISUAL_AND_GROUP_SCOPE,
    header: &[super::ID, OPTIONAL_TITLE],
    body: BodySpec::Properties(Scope::Node),
    properties: NODE_PROPERTIES,
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower);

fn lower(
    construct: &ValidatedConstruct<'_>,
    _context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    let arguments = construct.arguments();
    let Some(id) = arguments.first() else {
        return Err(Diagnostic::error(
            "E_LOWERING_INPUT",
            "A validated node is missing its identifier.",
            construct.statement.span,
            None::<String>,
        ));
    };
    let id_text = id.text.clone();
    let body = construct.body().unwrap_or_default();
    Ok(LoweredItem::Node(NodeIr {
        id: NodeId(id_text.clone()),
        id_span: id.span,
        label: arguments
            .get(1)
            .and_then(|token| match &token.kind {
                TokenKind::String(value) => Some(value.as_str()),
                _ => None,
            })
            .unwrap_or(&id_text)
            .to_owned(),
        icon: string_property(body, "icon"),
        detail: string_property(body, "detail"),
        group: construct.parent_group.clone(),
        span: construct.statement.span,
    }))
}

fn string_property(statements: &[Statement], name: &str) -> Option<String> {
    statements.iter().find_map(|statement| {
        let StatementKind::Property {
            name: property_name,
            value,
            ..
        } = &statement.kind
        else {
            return None;
        };
        if property_name.text != name {
            return None;
        }
        match &value.kind {
            TokenKind::String(value) => Some(value.clone()),
            _ => None,
        }
    })
}
