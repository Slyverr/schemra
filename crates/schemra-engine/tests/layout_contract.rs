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
fn grid_rows_center_nodes_with_different_heights() {
    for direction in [
        Direction::LeftToRight,
        Direction::RightToLeft,
        Direction::TopToBottom,
        Direction::BottomToTop,
    ] {
        let mut model = visual(direction);
        model.groups.push(GroupIr {
            id: GroupId("clients".into()),
            id_span: span(),
            title: "Clients".into(),
            layout: GroupLayout::Grid { columns: Some(2) },
            span: span(),
        });
        model.nodes = vec![
            node("short", "Short", None, Some("clients")),
            node(
                "tall",
                "Tall",
                Some("A longer description that wraps across several lines in the card."),
                Some("clients"),
            ),
        ];

        let scene = layout_visual(&model);
        let short = &scene.nodes[0].bounds;
        let tall = &scene.nodes[1].bounds;
        assert!(tall.height > short.height, "expected varied node heights");
        assert!(
            ((short.y + short.height / 2.0) - (tall.y + tall.height / 2.0)).abs() < 0.01,
            "grid row nodes should share their vertical center in {direction:?}"
        );
    }
}

#[test]
fn independent_blocks_do_not_overlap_multiline_groups_in_any_direction() {
    for direction in [
        Direction::LeftToRight,
        Direction::RightToLeft,
        Direction::TopToBottom,
        Direction::BottomToTop,
    ] {
        let mut model = visual(direction);
        model.groups.push(GroupIr {
            id: GroupId("clients".into()),
            id_span: span(),
            title: "Clients".into(),
            layout: GroupLayout::Grid { columns: Some(2) },
            span: span(),
        });
        model.nodes = (1..=4)
            .map(|number| {
                node(
                    &format!("user-{number}"),
                    &format!("User {number}"),
                    None,
                    Some("clients"),
                )
            })
            .collect();
        model
            .nodes
            .push(node("audit-worker", "Audit worker", None, None));

        let scene = layout_visual(&model);
        for (index, left) in scene.nodes.iter().enumerate() {
            for right in &scene.nodes[index + 1..] {
                let overlaps = left.bounds.x < right.bounds.x + right.bounds.width
                    && right.bounds.x < left.bounds.x + left.bounds.width
                    && left.bounds.y < right.bounds.y + right.bounds.height
                    && right.bounds.y < left.bounds.y + left.bounds.height;
                assert!(
                    !overlaps,
                    "nodes {} and {} overlap in {direction:?}",
                    left.id, right.id
                );
            }
        }
    }
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
