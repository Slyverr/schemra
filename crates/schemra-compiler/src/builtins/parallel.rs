use schemra_ir::ActionIr;
use schemra_syntax::Diagnostic;

use crate::compiler::{LoweredItem, LoweringContext, ValidatedConstruct};
use crate::{BodySpec, ConstructSpec, Scope};

use super::{ACTION_SCOPE, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "parallel",
    contexts: ACTION_SCOPE,
    header: &[],
    body: BodySpec::Scope(Scope::Action),
    properties: &[],
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, None);

fn lower(
    construct: &ValidatedConstruct<'_>,
    context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    Ok(LoweredItem::Action(ActionIr::Parallel {
        actions: context.lower_action_children(construct)?,
        span: construct.statement.span,
    }))
}
