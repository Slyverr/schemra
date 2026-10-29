use schemra_engine::layout_visual;
use schemra_ir::{
    Direction, EdgeDirectionality, EdgeIr, GroupId, GroupIr, GroupLayout, NodeId, NodeIr,
    SourceLocation, SourceSpan, VisualId, VisualIr,
};

fn span() -> SourceSpan {
    SourceSpan::point(SourceLocation {
        offset: 0,
        line: 1,
        column: 1,
    })
}

fn visual(direction: Direction) -> VisualIr {
    VisualIr {
        id: VisualId("network".into()),
        id_span: span(),
        title: "Network".into(),
        direction,
        direction_explicit: false,
        routing: None,
        bend: None,
        groups: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        span: span(),
    }
}

fn node(id: &str, label: &str, detail: Option<&str>, group: Option<&str>) -> NodeIr {
    NodeIr {
        id: NodeId(id.into()),
        id_span: span(),
        label: label.into(),
        icon: Some("lucide/server".into()),
        detail: detail.map(str::to_owned),
        group: group.map(|id| GroupId(id.into())),
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
fn graph_ranks_follow_direction_and_edges_remain_in_scene() {
    let mut model = visual(Direction::LeftToRight);
    model.nodes = vec![
        node("client", "Client", None, None),
        node("proxy", "Proxy", None, None),
    ];
    model.edges.push(edge("client", "proxy"));

    let scene = layout_visual(&model);
    assert!(scene.nodes[0].bounds.x < scene.nodes[1].bounds.x);
    assert_eq!(scene.edges.len(), 1);
    assert_eq!(
        (scene.edges[0].from.as_str(), scene.edges[0].to.as_str()),
        ("client", "proxy")
    );
    assert_eq!(scene.edges[0].path.len(), 1);

    model.direction = Direction::RightToLeft;
    let reversed = layout_visual(&model);
    assert!(reversed.nodes[0].bounds.x > reversed.nodes[1].bounds.x);
}

#[test]
fn configured_group_grid_places_members_and_contains_them() {
    let mut model = visual(Direction::LeftToRight);
    model.groups.push(GroupIr {
        id: GroupId("clients".into()),
        id_span: span(),
        title: "Clients".into(),
        layout: GroupLayout::Grid { columns: Some(3) },
        span: span(),
    });
    model.nodes = (1..=5)
        .map(|number| {
            node(
                &format!("bot-{number}"),
                &format!("Bot {number}"),
                None,
                Some("clients"),
            )
        })
        .collect();

    let scene = layout_visual(&model);
    let first_row = &scene.nodes[..3];
    assert!(
        first_row
            .windows(2)
            .all(|pair| pair[0].bounds.x < pair[1].bounds.x)
    );
    assert!(
        first_row
            .iter()
            .all(|item| item.bounds.y == first_row[0].bounds.y)
    );
    assert!(scene.nodes[3].bounds.y > scene.nodes[0].bounds.y);

    let group = &scene.groups[0];
    assert!(scene.nodes.iter().all(|item| {
        item.bounds.x >= group.bounds.x
            && item.bounds.y >= group.bounds.y
            && item.bounds.x + item.bounds.width <= group.bounds.x + group.bounds.width
            && item.bounds.y + item.bounds.height <= group.bounds.y + group.bounds.height
    }));
}

#[test]
fn node_height_grows_when_glyph_aware_wrapping_adds_lines() {
    let mut model = visual(Direction::TopToBottom);
    model.nodes = vec![
        node("short", "Proxy", Some("Handles requests"), None),
        node(
            "long",
            "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW",
            Some("WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW"),
            None,
        ),
    ];

    let scene = layout_visual(&model);
    assert!(scene.nodes[1].bounds.height > scene.nodes[0].bounds.height);
    assert!(scene.nodes[0].bounds.x < scene.nodes[1].bounds.x);
}
