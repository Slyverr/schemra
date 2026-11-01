use schemra_compiler::compile;
use schemra_engine::{layout_visual, render_svg};
use schemra_scene::{SceneNodeLayout, SceneTextAlign};

#[test]
fn node_presentation_options_flow_from_dsl_through_layout_to_svg() {
    let source = r#"
visual map {
  node horizontal "Horizontal" {
    icon: "lucide/server"
    detail: "Handles requests"
  }
  node vertical "Vertical" {
    icon: "lucide/database"
    detail: "Stores application data"
    layout: column
    text_align: center
  }
}
"#;
    let module = compile(source).expect("visual DSL compiles");
    let scene = layout_visual(&module.visuals[0]);
    let horizontal = &scene.nodes[0];
    let vertical = &scene.nodes[1];

    assert_eq!(horizontal.layout, SceneNodeLayout::Horizontal);
    assert_eq!(horizontal.text_align, SceneTextAlign::Left);
    assert_eq!(vertical.layout, SceneNodeLayout::Vertical);
    assert_eq!(vertical.text_align, SceneTextAlign::Center);
    assert!(vertical.bounds.height > horizontal.bounds.height);

    let svg = render_svg(&scene).expect("scene renders");
    assert!(svg.contains("data-node-shape=\"round\""));
    assert!(svg.contains("text-anchor=\"middle\""));
}
