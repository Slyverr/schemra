//! Renderer-independent geometry produced by the Schemra engine.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlowDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenePalette {
    #[default]
    Paper,
    Midnight,
    Lagoon,
    Sand,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneFont {
    #[default]
    Sans,
    Serif,
    Mono,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneNodeLayout {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneTextAlign {
    #[default]
    Left,
    Center,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneIconPack {
    #[default]
    Outline,
    Solid,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct SceneTheme {
    pub palette: ScenePalette,
    pub name: Option<String>,
    pub font: SceneFont,
    pub icon_pack: SceneIconPack,
    pub colors: SceneThemeColors,
    pub variants: Vec<SceneThemeVariant>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneVariant {
    #[default]
    Default,
    Muted,
    Accent,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct SceneThemeColors {
    pub canvas: Option<String>,
    pub card: Option<String>,
    pub group_fill: Option<String>,
    pub group_border: Option<String>,
    pub edge: Option<String>,
    pub text: Option<String>,
    pub muted: Option<String>,
    pub border: Option<String>,
    pub icon_background: Option<String>,
    pub icon_foreground: Option<String>,
    pub solid_icon_background: Option<String>,
    pub solid_icon_foreground: Option<String>,
    pub shadow: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SceneThemeVariant {
    pub variant: SceneVariant,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub text: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneNodeShape {
    #[default]
    Round,
    Rectangle,
    Cylinder,
}

/// Fully inherited style values ready for deterministic rendering.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct SceneStyle {
    pub variant: SceneVariant,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub text_color: Option<String>,
    pub opacity: Option<f32>,
    pub radius: Option<f32>,
    pub spacing: Option<f32>,
    pub font: Option<SceneFont>,
    pub weight: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct CubicBezier {
    pub start: Point,
    pub control1: Point,
    pub control2: Point,
    pub end: Point,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SceneGroup {
    pub name: String,
    pub title: String,
    pub bounds: Rect,
    pub style: SceneStyle,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SceneNode {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub detail: Option<String>,
    pub layout: SceneNodeLayout,
    pub text_align: SceneTextAlign,
    pub shape: SceneNodeShape,
    pub style: SceneStyle,
    pub bounds: Rect,
    pub group: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SceneText {
    pub id: String,
    pub text: String,
    pub align: SceneTextAlign,
    pub style: SceneStyle,
    pub bounds: Rect,
    pub group: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SceneImage {
    pub id: String,
    pub src: String,
    pub alt: Option<String>,
    pub fit: SceneImageFit,
    pub style: SceneStyle,
    pub bounds: Rect,
    pub group: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SceneIcon {
    pub id: String,
    pub icon: String,
    pub bounds: Rect,
    pub group: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneImageFit {
    #[default]
    Contain,
    Cover,
    Fill,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SceneEdge {
    /// Optional stable identity from the source graph.
    pub id: Option<String>,
    /// Source-level link identity shared by rendered directions of one link.
    pub source_id: Option<String>,
    /// Index of a reciprocal edge with identical presentation that shares this route.
    /// Both semantic edges remain in the scene for traversal and interaction.
    pub reciprocal_edge_index: Option<usize>,
    pub from: String,
    pub to: String,
    pub arrow_at_start: bool,
    pub arrow_at_end: bool,
    pub label: Option<String>,
    pub style: SceneStyle,
    pub group: Option<String>,
    /// Cubic segments that form the exact routed connector centerline.
    pub path: Vec<CubicBezier>,
    /// Machine-readable routing result for diagnosis and editor tooling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_diagnostic: Option<SceneRouteDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneRouteDiagnostic {
    pub status: SceneRouteStatus,
    pub source_port: String,
    pub target_port: String,
    pub candidates_considered: usize,
    pub rejected_for_obstacles: usize,
    pub rejected_for_route_conflicts: usize,
    pub obstacle_intersections: usize,
    pub route_conflicts: usize,
    pub candidate_rejections: Vec<String>,
    /// Approximate arc length of the selected cubic centerline in scene units.
    pub route_length: f32,
    /// Euclidean endpoint distance; a universal geometric lower bound.
    pub geometric_lower_bound: f32,
    /// Selected route length divided by the endpoint lower bound.
    pub detour_ratio: f32,
    /// Whether a clear, aligned facing-side candidate was available.
    pub direct_candidate_clear: bool,
    /// Routing mode used for the selected route.
    pub selected_routing: String,
    /// Short explanation of the candidate-selection decision.
    pub selection_reason: String,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SceneRouteStatus {
    Routed,
    Unroutable,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Scene {
    pub title: String,
    pub direction: FlowDirection,
    pub theme: SceneTheme,
    pub width: f32,
    pub height: f32,
    pub groups: Vec<SceneGroup>,
    pub nodes: Vec<SceneNode>,
    pub texts: Vec<SceneText>,
    pub images: Vec<SceneImage>,
    pub icons: Vec<SceneIcon>,
    pub edges: Vec<SceneEdge>,
}
