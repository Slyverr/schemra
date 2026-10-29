use schemra_compiler::compile;
use schemra_ir::{Direction, EdgeDirectionality, NodeId, VisualId};

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
