use schemra_engine::{VisualLimits, check_visual_limits};
use schemra_ir::{
    Direction, EdgeDirectionality, EdgeIr, GroupId, GroupIr, GroupLayout, NodeId, NodeIr,
    SourceLocation, SourceSpan, VisualId, VisualIr,
};

fn span() -> SourceSpan {
    SourceSpan::point(SourceLocation {
        offset: 12,
        line: 2,
        column: 3,
    })
}

fn visual() -> VisualIr {
    VisualIr {
        id: VisualId("network".into()),
        id_span: span(),
        title: "Network".into(),
        direction: Direction::LeftToRight,
        direction_explicit: false,
        routing: None,
        bend: None,
        groups: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        span: span(),
    }
}

fn node(id: &str) -> NodeIr {
    NodeIr {
        id: NodeId(id.into()),
        id_span: span(),
        label: id.into(),
        icon: None,
        detail: None,
        group: None,
        span: span(),
    }
}

fn edge(from: &str, to: &str) -> EdgeIr {
    EdgeIr {
        from: NodeId(from.into()),
        from_span: span(),
        to: NodeId(to.into()),
        to_span: span(),
        directionality: EdgeDirectionality::Directed,
        routing: None,
        bend: None,
        span: span(),
    }
}

#[test]
fn accepts_visuals_at_both_configured_limits() {
    let mut model = visual();
    model.groups.push(GroupIr {
        id: GroupId("clients".into()),
        id_span: span(),
        title: "Clients".into(),
        layout: GroupLayout::Auto,
        span: span(),
    });
    model.nodes = vec![node("client"), node("server")];
    model.edges.push(edge("client", "server"));

    check_visual_limits(
        &model,
        VisualLimits {
            max_visual_elements: 4,
            max_visual_links: 1,
        },
    )
    .expect("counts equal to the configured bounds should be accepted");
}

#[test]
fn reports_element_limit_with_the_visual_span() {
    let mut model = visual();
    model.nodes.push(node("client"));

    let error = check_visual_limits(
        &model,
        VisualLimits {
            max_visual_elements: 0,
            ..VisualLimits::default()
        },
    )
    .expect_err("one node exceeds a zero element limit");

    assert_eq!(error.code, "E_VISUAL_ELEMENT_LIMIT");
    assert_eq!(error.span, span());
    assert_eq!(
        error.message,
        "This visual exceeds the configured limit of 0 elements."
    );
}

#[test]
fn reports_link_limit_with_the_visual_span() {
    let mut model = visual();
    model.nodes = vec![node("client"), node("proxy"), node("server")];
    model.edges = vec![edge("client", "proxy"), edge("proxy", "server")];

    let error = check_visual_limits(
        &model,
        VisualLimits {
            max_visual_links: 1,
            ..VisualLimits::default()
        },
    )
    .expect_err("two links exceed the configured limit of one");

    assert_eq!(error.code, "E_VISUAL_LINK_LIMIT");
    assert_eq!(error.span, span());
    assert_eq!(
        error.message,
        "This visual exceeds the configured limit of 1 links."
    );
}

#[test]
fn uses_finite_defaults_for_visual_element_and_link_limits() {
    let limits = VisualLimits::default();
    assert_eq!(limits.max_visual_elements, 10_000);
    assert_eq!(limits.max_visual_links, 1_000);
}
