use schemra_ir::ActionIr;
use schemra_syntax::Diagnostic;

use crate::compiler::{LoweredItem, LoweringContext, ValidatedConstruct, duration_value};
use crate::{BodySpec, ConstructSpec};

use super::{ACTION_SCOPE, WAIT_HEADER, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "wait",
    contexts: ACTION_SCOPE,
    header: WAIT_HEADER,
    body: BodySpec::None,
    properties: &[],
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, None);

fn lower(
    construct: &ValidatedConstruct<'_>,
    _context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    Ok(LoweredItem::Action(ActionIr::Wait {
        duration_ms: duration_value(&construct.arguments()[0]).unwrap_or_default(),
        span: construct.statement.span,
    }))
}
