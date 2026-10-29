/// The syntactic context in which a construct or property is valid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    File,
    Visual,
    Group,
    Node,
    Scenario,
    Token,
    Action,
    Edge,
}

/// A required or optional term in a construct header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeaderKind {
    Identifier,
    NodeIdentifier,
    TokenIdentifier,
    String,
    Integer,
    PositiveInteger,
    Duration,
    HighlightEffect,
    HighlightSide,
    Literal(&'static str),
    Arrow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeaderParam {
    pub kind: HeaderKind,
    pub optional: bool,
}

/// Describes whether a construct has no body, a property body, or a nested scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BodySpec {
    None,
    Properties(Scope),
    Scope(Scope),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropertyKind {
    String,
    Integer,
    PositiveInteger,
    Duration,
    PositiveDuration,
    Highlight,
    Value(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PropertySpec {
    pub name: &'static str,
    pub kind: PropertyKind,
}

/// Static grammar metadata registered for an invocation name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstructSpec {
    pub name: &'static str,
    pub contexts: &'static [Scope],
    pub header: &'static [HeaderParam],
    pub body: BodySpec,
    pub properties: &'static [PropertySpec],
}
