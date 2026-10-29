use schemra_ir::ActionIr;
use schemra_syntax::Diagnostic;

use crate::compiler::{LoweredItem, LoweringContext, ValidatedConstruct};
use crate::{BodySpec, ConstructSpec, Scope};

use super::{ACTION_SCOPE, REPEAT_HEADER, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "repeat",
    contexts: ACTION_SCOPE,
    header: REPEAT_HEADER,
    body: BodySpec::Scope(Scope::Action),
    properties: &[],
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, None);

fn lower(
    construct: &ValidatedConstruct<'_>,
    context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    Ok(LoweredItem::Action(ActionIr::Repeat {
        times: construct.arguments()[0].text.parse().unwrap_or_default(),
        body: context.lower_action_children(construct)?,
        span: construct.statement.span,
    }))
}
