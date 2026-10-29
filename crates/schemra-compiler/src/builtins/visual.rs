use schemra_ir::{Direction, VisualId, VisualIr};
use schemra_syntax::{Diagnostic, StatementKind};

use crate::compiler::{LoweredItem, LoweringContext, ValidatedConstruct};
use crate::{BodySpec, ConstructSpec, Scope};

use super::{FILE_SCOPE, OPTIONAL_TITLE, VISUAL_PROPERTIES, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "visual",
    contexts: FILE_SCOPE,
    header: &[super::ID, OPTIONAL_TITLE],
    body: BodySpec::Scope(Scope::Visual),
    properties: VISUAL_PROPERTIES,
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, None);

fn lower(
    construct: &ValidatedConstruct<'_>,
    context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    let arguments = construct.arguments();
    let Some(id) = arguments.first() else {
        return Err(Diagnostic::error(
            "E_LOWERING_INPUT",
            "A validated visual is missing its identifier.",
            construct.statement.span,
            None::<String>,
        ));
    };
    let id_text = id.text.clone();
    let title = arguments
        .get(1)
        .and_then(|token| match &token.kind {
            schemra_syntax::TokenKind::String(value) => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or(&id_text)
        .to_owned();
    let body = construct.body().unwrap_or_default();
    let mut visual = VisualIr {
        id: VisualId(id_text),
        id_span: id.span,
        title,
        direction: Direction::default(),
        direction_explicit: false,
        routing: None,
        bend: None,
        groups: Vec::new(),
        nodes: Vec::new(),
        edges: context.lower_edges(body),
        span: construct.statement.span,
    };

    for child in body {
        if let StatementKind::Property { name, value, .. } = &child.kind
            && name.text == "direction"
        {
            visual.direction_explicit = true;
            visual.direction = match value.text.as_str() {
                "right-to-left" => Direction::RightToLeft,
                "top-to-bottom" => Direction::TopToBottom,
                "bottom-to-top" => Direction::BottomToTop,
                _ => Direction::LeftToRight,
            };
        }
    }

    for child in context.lower_children(construct, None)? {
        match child {
            LoweredItem::Group(group, nodes) => {
                visual.groups.push(group);
                visual.nodes.extend(nodes);
            }
            LoweredItem::Node(node) => visual.nodes.push(node),
            _ => {}
        }
    }
    Ok(LoweredItem::Visual(visual))
}
