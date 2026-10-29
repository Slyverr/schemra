use schemra_ir::{ActionIr, HighlightEffect, HighlightOrigin, NodeId};
use schemra_syntax::Diagnostic;

use crate::compiler::{
    LoweredItem, LoweringContext, ValidatedConstruct, duration_value, highlight_side,
};
use crate::{BodySpec, ConstructSpec};

use super::{ACTION_SCOPE, HIGHLIGHT_HEADER, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "highlight",
    contexts: ACTION_SCOPE,
    header: HIGHLIGHT_HEADER,
    body: BodySpec::None,
    properties: &[],
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, None);

fn lower(
    construct: &ValidatedConstruct<'_>,
    _context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    let arguments = construct.arguments();
    let effect = arguments.get(2).map_or(HighlightEffect::Trace, |token| {
        if token.text == "pulse" {
            HighlightEffect::Pulse
        } else {
            HighlightEffect::Trace
        }
    });
    let side = arguments
        .iter()
        .position(|token| token.text == "from")
        .and_then(|index| arguments.get(index + 1))
        .and_then(highlight_side);
    if side.is_some() && effect == HighlightEffect::Pulse {
        return Err(Diagnostic::error(
            "E_HIGHLIGHT_ORIGIN_EFFECT",
            "Only trace highlights can specify a start side.",
            construct.statement.span,
            None::<String>,
        ));
    }

    Ok(LoweredItem::Action(ActionIr::Highlight {
        node: NodeId(arguments[0].text.clone()),
        node_span: arguments[0].span,
        duration_ms: Some(duration_value(&arguments[1]).unwrap_or_default()),
        effect,
        origin: side.map_or(HighlightOrigin::Default, |side| HighlightOrigin::Side {
            side,
        }),
        span: construct.statement.span,
    }))
}
