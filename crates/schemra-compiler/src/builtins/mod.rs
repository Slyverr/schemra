mod node;
mod visual;

use crate::ConstructHandler;
use crate::compiler::BuiltinHandler;

pub(super) fn handlers() -> [&'static dyn ConstructHandler; 2] {
    [&visual::HANDLER, &node::HANDLER]
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
];

pub(super) const fn handler(
    spec: &'static crate::ConstructSpec,
    lower: crate::compiler::LowerFn,
) -> BuiltinHandler {
    crate::compiler::handler(spec, lower)
}
