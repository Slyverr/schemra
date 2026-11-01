mod group;
mod highlight;
mod move_action;
mod node;
mod parallel;
mod repeat;
mod scenario;
mod token;
mod visual;
mod wait;

use crate::ConstructHandler;
use crate::compiler::BuiltinHandler;

pub(super) fn handlers() -> [&'static dyn ConstructHandler; 10] {
    [
        &visual::HANDLER,
        &group::HANDLER,
        &node::HANDLER,
        &scenario::HANDLER,
        &token::HANDLER,
        &move_action::HANDLER,
        &wait::HANDLER,
        &highlight::HANDLER,
        &parallel::HANDLER,
        &repeat::HANDLER,
    ]
}

pub(super) const ID: crate::HeaderParam = crate::HeaderParam {
    kind: crate::HeaderKind::Identifier,
    optional: false,
};

pub(super) const OPTIONAL_TITLE: crate::HeaderParam = crate::HeaderParam {
    kind: crate::HeaderKind::String,
    optional: true,
};

pub(super) const FILE_SCOPE: &[crate::Scope] = &[crate::Scope::File];
pub(super) const VISUAL_AND_GROUP_SCOPE: &[crate::Scope] =
    &[crate::Scope::Visual, crate::Scope::Group];
pub(super) const VISUAL_SCOPE: &[crate::Scope] = &[crate::Scope::Visual];
pub(super) const SCENARIO_SCOPE: &[crate::Scope] = &[crate::Scope::Scenario];
pub(super) const ACTION_SCOPE: &[crate::Scope] = &[crate::Scope::Scenario, crate::Scope::Action];

pub(super) const VISUAL_PROPERTIES: &[crate::PropertySpec] = &[crate::PropertySpec {
    name: "direction",
    kind: crate::PropertyKind::Value(&[
        "left-to-right",
        "right-to-left",
        "top-to-bottom",
        "bottom-to-top",
    ]),
}];

pub(super) const NODE_PROPERTIES: &[crate::PropertySpec] = &[
    crate::PropertySpec {
        name: "icon",
        kind: crate::PropertyKind::String,
    },
    crate::PropertySpec {
        name: "detail",
        kind: crate::PropertyKind::String,
    },
    crate::PropertySpec {
        name: "layout",
        kind: crate::PropertyKind::Value(&["horizontal", "vertical", "row", "column"]),
    },
    crate::PropertySpec {
        name: "text_align",
        kind: crate::PropertyKind::Value(&["left", "center"]),
    },
];

pub(super) const TOKEN_PROPERTIES: &[crate::PropertySpec] = &[
    crate::PropertySpec {
        name: "icon",
        kind: crate::PropertyKind::String,
    },
    crate::PropertySpec {
        name: "char",
        kind: crate::PropertyKind::String,
    },
];

pub(super) const SCENARIO_PROPERTIES: &[crate::PropertySpec] = &[crate::PropertySpec {
    name: "highlight",
    kind: crate::PropertyKind::Highlight,
}];

pub(super) const MOVE_PROPERTIES: &[crate::PropertySpec] = &[
    crate::PropertySpec {
        name: "duration",
        kind: crate::PropertyKind::PositiveDuration,
    },
    crate::PropertySpec {
        name: "highlight",
        kind: crate::PropertyKind::Highlight,
    },
];

pub(super) const SCENARIO_HEADER: &[crate::HeaderParam] = &[
    ID,
    crate::HeaderParam {
        kind: crate::HeaderKind::Literal("for"),
        optional: false,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::Literal("visual"),
        optional: false,
    },
    ID,
];

pub(super) const TOKEN_HEADER: &[crate::HeaderParam] = &[ID];

pub(super) const MOVE_HEADER: &[crate::HeaderParam] = &[
    crate::HeaderParam {
        kind: crate::HeaderKind::TokenIdentifier,
        optional: false,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::Literal("from"),
        optional: false,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::NodeIdentifier,
        optional: false,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::Arrow,
        optional: false,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::NodeIdentifier,
        optional: false,
    },
];

pub(super) const WAIT_HEADER: &[crate::HeaderParam] = &[crate::HeaderParam {
    kind: crate::HeaderKind::Duration,
    optional: false,
}];

pub(super) const REPEAT_HEADER: &[crate::HeaderParam] = &[crate::HeaderParam {
    kind: crate::HeaderKind::PositiveInteger,
    optional: false,
}];

pub(super) const HIGHLIGHT_HEADER: &[crate::HeaderParam] = &[
    crate::HeaderParam {
        kind: crate::HeaderKind::NodeIdentifier,
        optional: false,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::Duration,
        optional: false,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::HighlightEffect,
        optional: true,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::Literal("from"),
        optional: true,
    },
    crate::HeaderParam {
        kind: crate::HeaderKind::HighlightSide,
        optional: true,
    },
];

pub(super) const GROUP_PROPERTIES: &[crate::PropertySpec] = &[
    crate::PropertySpec {
        name: "layout",
        kind: crate::PropertyKind::Value(&["auto", "grid"]),
    },
    crate::PropertySpec {
        name: "columns",
        kind: crate::PropertyKind::PositiveInteger,
    },
];

pub(super) const fn handler(
    spec: &'static crate::ConstructSpec,
    lower: crate::compiler::LowerFn,
    validate: Option<crate::compiler::ValidateFn>,
) -> BuiltinHandler {
    crate::compiler::handler(spec, lower, validate)
}
