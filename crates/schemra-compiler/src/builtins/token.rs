use schemra_ir::{TokenId, TokenIr};
use schemra_syntax::Diagnostic;

use crate::compiler::{LoweredItem, LoweringContext, ValidatedConstruct};
use crate::{BodySpec, ConstructSpec, Scope};

use super::{SCENARIO_SCOPE, TOKEN_HEADER, TOKEN_PROPERTIES, handler};

static SPEC: ConstructSpec = ConstructSpec {
    name: "token",
    contexts: SCENARIO_SCOPE,
    header: TOKEN_HEADER,
    body: BodySpec::Properties(Scope::Token),
    properties: TOKEN_PROPERTIES,
};

pub(super) static HANDLER: super::super::BuiltinHandler = handler(&SPEC, lower, None);

fn lower(
    construct: &ValidatedConstruct<'_>,
    _context: &mut LoweringContext<'_>,
) -> Result<LoweredItem, Diagnostic> {
    let id = &construct.arguments()[0];
    Ok(LoweredItem::Token(TokenIr {
        id: TokenId(id.text.clone()),
        id_span: id.span,
        icon: crate::compiler::string_property(construct.body().unwrap_or_default(), "icon"),
        span: construct.statement.span,
    }))
}
