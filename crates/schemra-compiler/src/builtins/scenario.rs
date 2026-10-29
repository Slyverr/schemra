use schemra_ir::{ScenarioId, ScenarioIr, VisualId};
use schemra_syntax::Diagnostic;

use crate::compiler::{LoweredItem, LoweringContext, ValidatedConstruct};
use crate::{BodySpec, ConstructSpec, Scope};

use super::{FILE_SCOPE, SCENARIO_HEADER, SCENARIO_PROPERTIES, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "scenario",
    contexts: FILE_SCOPE,
    header: SCENARIO_HEADER,
    body: BodySpec::Scope(Scope::Scenario),
    properties: SCENARIO_PROPERTIES,
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, None);

fn lower(
    construct: &ValidatedConstruct<'_>,
    context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    let arguments = construct.arguments();
    let mut scenario = ScenarioIr {
        id: ScenarioId(arguments[0].text.clone()),
        id_span: arguments[0].span,
        visual: VisualId(arguments[3].text.clone()),
        visual_span: arguments[3].span,
        tokens: Vec::new(),
        span: construct.statement.span,
    };

    for child in context.lower_children(construct, None)? {
        if let LoweredItem::Token(token) = child {
            scenario.tokens.push(token);
        }
    }

    Ok(LoweredItem::Scenario(scenario))
}
