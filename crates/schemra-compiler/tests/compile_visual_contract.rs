use schemra_compiler::{compile, compile_sources};
use schemra_ir::{Direction, EdgeDirectionality, GroupId, GroupLayout, NodeId, VisualId};

fn diagnostic_codes(source: &str) -> Vec<String> {
    compile(source)
        .expect_err("source should be rejected")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn compiles_visual_nodes_properties_and_edges_through_the_public_api() {
    let source = r#"
visual network "Service map" {
  direction: top-to-bottom
  node proxy "Reverse proxy" {
    icon: "lucide/server"
    detail: "Routes requests"
  }
  node backend "Backend" {}
  proxy -> backend
  proxy -- backend
}
"#;

    let module = compile(source).expect("valid visual compiles");
    assert_eq!(module.visuals.len(), 1);

    let visual = &module.visuals[0];
    assert_eq!(visual.id, VisualId("network".into()));
    assert_eq!(visual.title, "Service map");
    assert_eq!(visual.direction, Direction::TopToBottom);
    assert!(visual.direction_explicit);
    assert_eq!(visual.nodes.len(), 2);
    assert_eq!(visual.nodes[0].id, NodeId("proxy".into()));
    assert_eq!(visual.nodes[0].label, "Reverse proxy");
    assert_eq!(visual.nodes[0].icon.as_deref(), Some("lucide/server"));
    assert_eq!(visual.nodes[0].detail.as_deref(), Some("Routes requests"));
    assert_eq!(visual.nodes[1].label, "Backend");
    assert_eq!(visual.edges.len(), 2);
    assert_eq!(visual.edges[0].from, NodeId("proxy".into()));
    assert_eq!(visual.edges[0].to, NodeId("backend".into()));
    assert_eq!(visual.edges[0].directionality, EdgeDirectionality::Directed);
    assert_eq!(
        visual.edges[1].directionality,
        EdgeDirectionality::Undirected
    );
}

#[test]
fn omitted_titles_and_direction_use_the_declared_defaults() {
    let module = compile("visual network { node proxy {} }").expect("valid visual compiles");
    let visual = &module.visuals[0];

    assert_eq!(visual.title, "network");
    assert_eq!(visual.direction, Direction::LeftToRight);
    assert!(!visual.direction_explicit);
    assert_eq!(visual.nodes[0].label, "proxy");
    assert!(visual.nodes[0].icon.is_none());
    assert!(visual.nodes[0].detail.is_none());
}

#[test]
fn reports_unknown_constructs_and_invalid_visual_content() {
    assert!(diagnostic_codes("unknown network {}").contains(&"E_UNKNOWN_CONSTRUCT".into()));
    assert!(diagnostic_codes("visual network {}").contains(&"E_EMPTY_VISUAL".into()));
    assert!(
        diagnostic_codes("visual network \" \" { node proxy {} }")
            .contains(&"E_EMPTY_VISUAL_TITLE".into())
    );
    assert!(
        diagnostic_codes("visual network { node proxy \" \" {} }")
            .contains(&"E_EMPTY_NODE_LABEL".into())
    );
}

#[test]
fn validates_visual_and_node_properties_before_lowering() {
    assert!(
        diagnostic_codes("visual network { camera: 2 node proxy {} }")
            .contains(&"E_UNKNOWN_PROPERTY".into())
    );
    assert!(
        diagnostic_codes("visual network { node proxy { icon: 2 } }")
            .contains(&"E_PROPERTY_TYPE".into())
    );
}

#[test]
fn lowers_group_layout_and_assigns_group_membership_to_its_nodes() {
    let source = r#"
visual network {
  group clients "Clients" {
    layout: grid
    columns: 2
    node user_1 "User 1" {}
    node user_2 "User 2" {}
  }
}
"#;

    let module = compile(source).expect("valid group compiles");
    let visual = &module.visuals[0];

    assert_eq!(visual.groups.len(), 1);
    assert_eq!(visual.groups[0].id, GroupId("clients".into()));
    assert_eq!(visual.groups[0].title, "Clients");
    assert_eq!(
        visual.groups[0].layout,
        GroupLayout::Grid { columns: Some(2) }
    );
    assert_eq!(visual.nodes.len(), 2);
    assert!(
        visual
            .nodes
            .iter()
            .all(|node| node.group == Some(GroupId("clients".into())))
    );
}

#[test]
fn group_layout_defaults_to_auto_and_columns_require_grid_layout() {
    let module = compile("visual network { group clients { node user_1 {} } }")
        .expect("group with default layout compiles");
    assert_eq!(module.visuals[0].groups[0].layout, GroupLayout::Auto);

    assert!(
        diagnostic_codes("visual network { group clients { columns: 2 node user_1 {} } }")
            .contains(&"E_COLUMNS_WITHOUT_GRID".into())
    );
}

#[test]
fn rejects_duplicate_visual_and_element_ids() {
    assert!(
        diagnostic_codes("visual network { node api {} }\nvisual network { node database {} }")
            .contains(&"E_DUPLICATE_VISUAL".into())
    );
    assert!(
        diagnostic_codes("visual network {\n group clients {}\n node clients {}\n}")
            .contains(&"E_DUPLICATE_VISUAL_ELEMENT".into())
    );
}

#[test]
fn rejects_links_with_unresolved_node_endpoints() {
    let diagnostics = compile("visual network {\n node proxy {}\n proxy -> missing\n}")
        .expect_err("unknown link endpoint should fail compilation");

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E_UNKNOWN_EDGE_NODE")
        .expect("edge endpoint diagnostic");
    assert_eq!(
        diagnostic.message,
        "An edge references a node that is not declared in its visual."
    );
}

#[test]
fn compiles_multiple_visual_sources_into_one_module() {
    let module = compile_sources(&[
        "visual network { node proxy {} }",
        "visual database { node store {} }",
    ])
    .expect("each source contributes visuals to the module");

    assert_eq!(module.visuals.len(), 2);
    assert_eq!(module.visuals[0].id, VisualId("network".into()));
    assert_eq!(module.visuals[1].id, VisualId("database".into()));
}
