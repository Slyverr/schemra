use schemra_ir::{GroupId, GroupIr, GroupLayout};
use schemra_syntax::{Diagnostic, StatementKind};

use crate::compiler::{LoweredItem, LoweringContext, ValidatedConstruct};
use crate::{BodySpec, ConstructSpec, Scope};

use super::{GROUP_PROPERTIES, OPTIONAL_TITLE, VISUAL_SCOPE, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "group",
    contexts: VISUAL_SCOPE,
    header: &[super::ID, OPTIONAL_TITLE],
    body: BodySpec::Scope(Scope::Group),
    properties: GROUP_PROPERTIES,
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, Some(validate));

fn lower(
    construct: &ValidatedConstruct<'_>,
    context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    let arguments = construct.arguments();
    let Some(id) = arguments.first() else {
        return Err(Diagnostic::error(
            "E_LOWERING_INPUT",
            "A validated group is missing its identifier.",
            construct.statement.span,
            None::<String>,
        ));
    };

    let id_text = id.text.clone();
    let group_id = GroupId(id_text.clone());
    let body = construct.body().unwrap_or_default();
    let title = arguments
        .get(1)
        .and_then(|token| match &token.kind {
            schemra_syntax::TokenKind::String(value) => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or(&id_text)
        .to_owned();
    let layout = match crate::compiler::scalar_property(body, "layout").as_deref() {
        Some("grid") => GroupLayout::Grid {
            columns: crate::compiler::integer_property(body, "columns"),
        },
        _ => GroupLayout::Auto,
    };
    let group = GroupIr {
        id: group_id.clone(),
        id_span: id.span,
        title,
        layout,
        span: construct.statement.span,
    };
    let nodes = context
        .lower_children(construct, Some(group_id))?
        .into_iter()
        .filter_map(|item| match item {
            LoweredItem::Node(node) => Some(node),
            _ => None,
        })
        .collect();

    Ok(LoweredItem::Group(group, nodes))
}

fn validate(construct: &ValidatedConstruct<'_>, diagnostics: &mut Vec<Diagnostic>) {
    let body = construct.body().unwrap_or_default();
    let has_grid = body.iter().any(|statement| {
        matches!(
            &statement.kind,
            StatementKind::Property { name, value, .. }
                if name.text == "layout" && value.text == "grid"
        )
    });
    if let Some(span) = body.iter().find_map(|statement| match &statement.kind {
        StatementKind::Property { name, .. } if name.text == "columns" => Some(statement.span),
        _ => None,
    }) && !has_grid
    {
        diagnostics.push(Diagnostic::error(
            "E_COLUMNS_WITHOUT_GRID",
            "`columns` is only valid when the group uses `layout: grid`.",
            span,
            None::<String>,
        ));
    }
}
