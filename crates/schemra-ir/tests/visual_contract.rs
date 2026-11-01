use schemra_ir::{
    Direction, EdgeDirectionality, EdgeIr, GroupId, GroupIr, GroupLayout, NodeId, NodeIr,
    NodeLayout, NodeTextAlign, SourceLocation, SourceSpan, VisualId, VisualIr,
};

fn span(start: usize, end: usize) -> SourceSpan {
    SourceSpan {
        start: SourceLocation {
            offset: start,
            line: 1,
            column: start + 1,
        },
        end: SourceLocation {
            offset: end,
            line: 1,
            column: end + 1,
        },
    }
}

#[test]
fn visual_ir_models_typed_groups_nodes_and_link_endpoints() {
    let visual_span = span(0, 45);
    let node_span = span(16, 29);
    let visual = VisualIr {
        id: VisualId("service_map".into()),
        id_span: span(7, 18),
        title: "Service map".into(),
        direction: Direction::TopToBottom,
        direction_explicit: true,
        groups: vec![GroupIr {
            id: GroupId("application".into()),
            id_span: span(20, 31),
            title: "Application".into(),
            layout: GroupLayout::Grid { columns: Some(2) },
            span: span(20, 45),
        }],
        nodes: vec![NodeIr {
            id: NodeId("proxy".into()),
            id_span: span(16, 21),
            label: "Reverse proxy".into(),
            icon: Some("server".into()),
            detail: None,
            layout: NodeLayout::Vertical,
            text_align: NodeTextAlign::Center,
            group: Some(GroupId("application".into())),
            span: node_span,
        }],
        edges: vec![EdgeIr {
            from: NodeId("proxy".into()),
            from_span: span(31, 36),
            to: NodeId("backend".into()),
            to_span: span(40, 47),
            directionality: EdgeDirectionality::Directed,
            span: span(31, 47),
        }],
        span: visual_span,
    };

    assert_eq!(visual.nodes[0].group, Some(GroupId("application".into())));
    assert_eq!(visual.nodes[0].layout, NodeLayout::Vertical);
    assert_eq!(visual.nodes[0].text_align, NodeTextAlign::Center);
    assert_eq!(visual.edges[0].from, visual.nodes[0].id);
    assert_eq!(visual.edges[0].directionality, EdgeDirectionality::Directed);
    assert_eq!(visual.edges[0].from_span, span(31, 36));
    assert_eq!(visual.span, visual_span);
}

#[test]
fn visual_layout_and_direction_defaults_are_explicit_and_stable() {
    assert_eq!(Direction::default(), Direction::LeftToRight);
    assert_eq!(GroupLayout::default(), GroupLayout::Auto);
    assert_eq!(NodeLayout::default(), NodeLayout::Horizontal);
    assert_eq!(NodeTextAlign::default(), NodeTextAlign::Left);
}

#[test]
fn strong_ids_keep_their_string_json_representation() {
    let id = NodeId("worker".into());

    assert_eq!(serde_json::to_string(&id).expect("id JSON"), "\"worker\"");
    assert_eq!(
        serde_json::from_str::<NodeId>("\"worker\"").expect("decode id"),
        id
    );
}
