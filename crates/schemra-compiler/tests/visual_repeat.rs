use schemra_compiler::{CompletionKind, compile, complete_at};
use schemra_ir::{EdgeDirectionality, GroupId, NodeId};

#[test]
fn expands_indexed_nodes_and_repeated_links_inside_a_grid_group() {
    let module = compile(
        r#"
visual traffic {
  group clients "Clients" {
    layout: grid
    columns: 2
    repeat 2..=3 as i {
      node user[i] "User {i}" { icon: "user" }
    }
  }
  node proxy "Proxy" {}
  repeat 2..=3 as i { user[i] -> proxy }
}
"#,
    )
    .expect("visual repeats compile into ordinary graph elements");

    let visual = &module.visuals[0];
    assert_eq!(
        visual
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.label.as_str(), node.group.clone()))
            .collect::<Vec<_>>(),
        [
            (
                NodeId("user_2".into()),
                "User 2",
                Some(GroupId("clients".into()))
            ),
            (
                NodeId("user_3".into()),
                "User 3",
                Some(GroupId("clients".into()))
            ),
            (NodeId("proxy".into()), "Proxy", None),
        ]
    );
    assert_eq!(visual.edges.len(), 2);
    assert_eq!(visual.edges[0].from, NodeId("user_2".into()));
    assert_eq!(visual.edges[1].from, NodeId("user_3".into()));
    assert!(visual.edges.iter().all(|edge| {
        edge.to == NodeId("proxy".into()) && edge.directionality == EdgeDirectionality::Directed
    }));
}

#[test]
fn assigns_stable_suffixes_to_unindexed_node_ids_and_links() {
    let module = compile(
        r#"
visual copies {
  repeat 2 {
    node source "Source" {}
    node destination "Destination" {}
    source -- destination
  }
}
"#,
    )
    .expect("unindexed structural repeats receive stable IDs");

    let visual = &module.visuals[0];
    assert_eq!(
        visual
            .nodes
            .iter()
            .map(|node| node.id.0.as_str())
            .collect::<Vec<_>>(),
        ["source_1", "destination_1", "source_2", "destination_2"]
    );
    assert_eq!(visual.edges[0].from, NodeId("source_1".into()));
    assert_eq!(visual.edges[1].to, NodeId("destination_2".into()));
}

#[test]
fn validates_zero_count_templates_without_emitting_their_nodes() {
    let module = compile(
        r#"
visual optional {
  node proxy {}
  repeat 0 as i { node client[i] "Client {i}" {} }
}
"#,
    )
    .expect("a zero count keeps a valid template but emits no elements");

    assert_eq!(module.visuals[0].nodes.len(), 1);
}

#[test]
fn rejects_descending_visual_repeat_ranges() {
    let diagnostics =
        compile("visual map {\n  node proxy {}\n  repeat 4..=2 as i { node user[i] {} }\n}")
            .expect_err("descending ranges are invalid");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_VISUAL_REPEAT_RANGE")
    );
}

#[test]
fn rejects_group_declarations_inside_visual_repeats() {
    let diagnostics = compile(
        r#"
visual map {
  repeat 2 as i {
    group clients "Clients" { node user[i] "User {i}" {} }
    node proxy {}
  }
}
"#,
    )
    .expect_err("visual repeat bodies only contain nodes and links");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_REPEAT_BODY_STATEMENT")
    );
}

#[test]
fn rejects_repeat_indices_without_a_matching_binding() {
    let diagnostics = compile("visual map { repeat 2 as i { node user[j] \"User\" {} } }")
        .expect_err("an unresolved repeat binding is invalid");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_VISUAL_REPEAT_INDEX")
    );
}

#[test]
fn rejects_visual_repeat_counts_above_the_expansion_limit() {
    let diagnostics = compile("visual map {\n  repeat 1001 { node user {} }\n}")
        .expect_err("visual repeat expansion is bounded");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_VISUAL_REPEAT_COUNT")
    );
}

#[test]
fn rejects_indexed_node_ids_outside_visual_repeat_blocks() {
    let diagnostics = compile("visual map { node user[i] \"User\" {} }")
        .expect_err("indexed node IDs require a repeat binding");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E_VISUAL_REPEAT_INDEX_SCOPE")
    );
}

#[test]
fn suggests_repeat_at_visual_and_group_statement_boundaries() {
    for source in [
        "visual map {\n  rep",
        "visual map {\n  group clients {\n    rep",
        "visual map {\n  repeat 2 as i {\n    rep",
    ] {
        let completion = complete_at(source, source.len(), false)
            .expect("repeat keyword should be suggested in visual contexts");
        let item = completion
            .items
            .iter()
            .find(|item| item.label == "repeat")
            .expect("repeat completion is present");
        assert_eq!(item.kind, CompletionKind::Keyword);
    }
}
