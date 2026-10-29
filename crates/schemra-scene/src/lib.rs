//! Renderer-neutral scene geometry and data contracts.

mod image_source;
mod model;

pub use image_source::is_safe_image_source;
pub use model::{
    CubicBezier, FlowDirection, Point, Rect, Scene, SceneEdge, SceneFont, SceneGroup, SceneIcon,
    SceneIconPack, SceneImage, SceneImageFit, SceneNode, SceneNodeLayout, SceneNodeShape,
    ScenePalette, SceneRouteDiagnostic, SceneRouteStatus, SceneStyle, SceneText, SceneTextAlign,
    SceneTheme, SceneThemeColors, SceneThemeVariant, SceneVariant,
};
