use schemra_ir::{ActionIr, HighlightOrigin, NodeId, TokenId};
use schemra_syntax::Diagnostic;

use crate::compiler::{
    HighlightProperty, LoweredItem, LoweringContext, ValidatedConstruct, highlight_property,
};
use crate::{BodySpec, ConstructSpec, Scope};

use super::{ACTION_SCOPE, MOVE_HEADER, MOVE_PROPERTIES, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "move",
    contexts: ACTION_SCOPE,
    header: MOVE_HEADER,
    body: BodySpec::Properties(Scope::Action),
    properties: MOVE_PROPERTIES,
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, None);

fn lower(
    construct: &ValidatedConstruct<'_>,
    context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    let arguments = construct.arguments();
    let destination = NodeId(arguments[4].text.clone());
    let span = construct.statement.span;
    let movement = ActionIr::Move {
        token: TokenId(arguments[0].text.clone()),
        token_span: arguments[0].span,
        from: NodeId(arguments[2].text.clone()),
        from_span: arguments[2].span,
        to: destination.clone(),
        to_span: arguments[4].span,
        duration_ms: crate::compiler::duration_property(
            construct.body().unwrap_or_default(),
            "duration",
        ),
        span,
    };

    let highlight = match highlight_property(construct.body().unwrap_or_default(), "highlight") {
        Some(HighlightProperty::Enabled {
            duration_ms,
            effect,
            side,
        }) => Some((duration_ms, effect, side)),
        Some(HighlightProperty::Off) => None,
        None => context.scenario_highlight(),
    };

    let action = if let Some((duration_ms, effect, side)) = highlight {
        ActionIr::Sequence {
            actions: vec![
                movement,
                ActionIr::Highlight {
                    node: destination.clone(),
                    node_span: arguments[4].span,
                    duration_ms: Some(duration_ms),
                    effect,
                    origin: side.map_or_else(
                        || HighlightOrigin::Arrival {
                            from: NodeId(arguments[2].text.clone()),
                            to: destination.clone(),
                        },
                        |side| HighlightOrigin::Side { side },
                    ),
                    span,
                },
            ],
            span,
        }
    } else {
        movement
    };

    Ok(LoweredItem::Action(action))
}
