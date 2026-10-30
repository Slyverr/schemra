use schemra_renderer::{SvgRenderError, render_svg};
use schemra_scene::{
    FlowDirection, Rect, Scene, SceneFont, SceneIcon, SceneIconPack, SceneImage, SceneImageFit,
    SceneNode, SceneNodeLayout, SceneNodeShape, SceneStyle, SceneText, SceneTextAlign, SceneTheme,
};

fn scene() -> Scene {
    Scene {
        title: "Annotations".into(),
        direction: FlowDirection::LeftToRight,
        theme: SceneTheme::default(),
        width: 320.0,
        height: 180.0,
        groups: Vec::new(),
        nodes: vec![node()],
        texts: Vec::new(),
        images: Vec::new(),
        icons: Vec::new(),
        edges: Vec::new(),
    }
}

fn node() -> SceneNode {
    SceneNode {
        id: "service".into(),
        label: "Service".into(),
        icon: None,
        detail: None,
        layout: SceneNodeLayout::Horizontal,
        text_align: SceneTextAlign::Left,
        shape: SceneNodeShape::Round,
        style: SceneStyle::default(),
        bounds: Rect {
            x: 180.0,
            y: 40.0,
            width: 120.0,
            height: 64.0,
        },
        group: None,
    }
}

fn bounds(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn renders_styled_wrapped_text_annotations_with_accessible_labels() {
    let mut visual = scene();
    visual.texts.push(SceneText {
        id: "caption".into(),
        text: "Service notes".into(),
        align: SceneTextAlign::Center,
        style: SceneStyle {
            font: Some(SceneFont::Serif),
            weight: Some(600),
            text_color: Some("#123456".into()),
            opacity: Some(0.8),
            ..SceneStyle::default()
        },
        bounds: bounds(20.0, 20.0, 100.0, 50.0),
        group: None,
    });

    let svg = render_svg(&visual).unwrap();

    assert!(
        svg.contains("<g data-text-id=\"caption\" aria-label=\"Service notes\" opacity=\"0.8\">")
    );
    assert!(svg.contains("text-anchor=\"middle\" font-family=\"'Noto Serif', serif\" font-size=\"14\" font-weight=\"600\" fill=\"#123456\">"));
}

#[test]
fn renders_safe_image_annotations_with_fit_and_alt_text() {
    let mut visual = scene();
    visual.images.push(SceneImage {
        id: "logo".into(),
        src: "assets/logo.png".into(),
        alt: Some("Service logo".into()),
        fit: SceneImageFit::Cover,
        style: SceneStyle::default(),
        bounds: bounds(20.0, 20.0, 48.0, 32.0),
        group: None,
    });

    let svg = render_svg(&visual).unwrap();

    assert!(svg.contains("<image data-image-id=\"logo\" x=\"20\" y=\"20\" width=\"48\" height=\"32\" href=\"assets/logo.png\" preserveAspectRatio=\"xMidYMid slice\" role=\"img\" aria-label=\"Service logo\"/>"));
}

#[test]
fn skips_unsafe_image_annotation_sources() {
    let mut visual = scene();
    visual.images.push(SceneImage {
        id: "remote".into(),
        src: "https://example.test/logo.png".into(),
        alt: Some("Remote logo".into()),
        fit: SceneImageFit::Contain,
        style: SceneStyle::default(),
        bounds: bounds(20.0, 20.0, 48.0, 32.0),
        group: None,
    });

    let svg = render_svg(&visual).unwrap();

    assert!(!svg.contains("data-image-id=\"remote\""));
}

#[test]
fn rejects_invalid_annotation_bounds_before_serializing_svg() {
    let mut visual = scene();
    visual.texts.push(SceneText {
        id: "caption".into(),
        text: "Caption".into(),
        align: SceneTextAlign::Left,
        style: SceneStyle::default(),
        bounds: bounds(20.0, 20.0, f32::INFINITY, 24.0),
        group: None,
    });

    assert_eq!(
        render_svg(&visual),
        Err(SvgRenderError::InvalidAnnotationBounds)
    );
}

#[test]
fn rejects_invalid_annotation_opacity_before_serializing_svg() {
    let mut visual = scene();
    visual.images.push(SceneImage {
        id: "logo".into(),
        src: "assets/logo.png".into(),
        alt: None,
        fit: SceneImageFit::Contain,
        style: SceneStyle {
            opacity: Some(f32::NAN),
            ..SceneStyle::default()
        },
        bounds: bounds(20.0, 20.0, 48.0, 32.0),
        group: None,
    });

    assert_eq!(
        render_svg(&visual),
        Err(SvgRenderError::InvalidAnnotationStyle)
    );
}

#[test]
fn renders_icon_annotations_using_the_scene_icon_pack() {
    let mut visual = scene();
    visual.theme.icon_pack = SceneIconPack::Solid;
    visual.icons.push(SceneIcon {
        id: "security".into(),
        icon: "lucide/shield".into(),
        bounds: bounds(24.0, 24.0, 30.0, 30.0),
        group: None,
    });

    let svg = render_svg(&visual).unwrap();

    assert!(svg.contains("<g data-icon-id=\"security\" role=\"img\" aria-label=\"security\" transform=\"translate(24 24) scale(1 1)\">"));
    assert!(svg.contains("data-icon-pack=\"solid\""));
    assert!(svg.contains(
        "<path d=\"M16 3 27 7v8c0 6.6-4.4 11-11 14C9.4 26 5 21.6 5 15V7l11-4Z\" fill=\"#ffffff\"/>"
    ));
}

#[test]
fn paints_text_image_and_icon_layers_in_order_before_nodes() {
    let mut visual = scene();
    visual.texts.push(SceneText {
        id: "note".into(),
        text: "Note".into(),
        align: SceneTextAlign::Left,
        style: SceneStyle::default(),
        bounds: bounds(20.0, 20.0, 80.0, 24.0),
        group: None,
    });
    visual.images.push(SceneImage {
        id: "logo".into(),
        src: "assets/logo.png".into(),
        alt: None,
        fit: SceneImageFit::Contain,
        style: SceneStyle::default(),
        bounds: bounds(110.0, 20.0, 32.0, 32.0),
        group: None,
    });
    visual.icons.push(SceneIcon {
        id: "shield".into(),
        icon: "shield".into(),
        bounds: bounds(145.0, 20.0, 30.0, 30.0),
        group: None,
    });

    let svg = render_svg(&visual).unwrap();
    let text = svg.find("data-text-id=\"note\"").unwrap();
    let image = svg.find("data-image-id=\"logo\"").unwrap();
    let icon = svg.find("data-icon-id=\"shield\"").unwrap();
    let node = svg.find("data-node-id=\"service\"").unwrap();

    assert!(text < image && image < icon && icon < node);
}

#[test]
fn escapes_annotation_text_ids_image_attributes_and_icon_ids() {
    let mut visual = scene();
    visual.texts.push(SceneText {
        id: "note&<one>".into(),
        text: "A & <B> \"C\"".into(),
        align: SceneTextAlign::Left,
        style: SceneStyle::default(),
        bounds: bounds(20.0, 20.0, 120.0, 24.0),
        group: None,
    });
    visual.images.push(SceneImage {
        id: "logo&mark".into(),
        src: "assets/logo&mark.png".into(),
        alt: Some("A & B".into()),
        fit: SceneImageFit::Fill,
        style: SceneStyle::default(),
        bounds: bounds(20.0, 50.0, 32.0, 32.0),
        group: None,
    });
    visual.icons.push(SceneIcon {
        id: "icon&<id>".into(),
        icon: "server".into(),
        bounds: bounds(60.0, 50.0, 30.0, 30.0),
        group: None,
    });

    let svg = render_svg(&visual).unwrap();

    assert!(svg.contains(
        "data-text-id=\"note&amp;&lt;one&gt;\" aria-label=\"A &amp; &lt;B&gt; &quot;C&quot;\""
    ));
    assert!(svg.contains("href=\"assets/logo&amp;mark.png\" preserveAspectRatio=\"none\" role=\"img\" aria-label=\"A &amp; B\""));
    assert!(svg.contains(
        "data-icon-id=\"icon&amp;&lt;id&gt;\" role=\"img\" aria-label=\"icon&amp;&lt;id&gt;\""
    ));
    assert!(!svg.contains("<B>"));
}
