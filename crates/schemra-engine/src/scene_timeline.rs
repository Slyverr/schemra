use std::collections::HashMap;

use schemra_ir::{
    HighlightOrigin, HighlightSide, NodeId, SceneEdgeProgress, SceneHighlight, SceneTimelineFrame,
    SceneTokenPosition, Timeline, TimelineFrame, TokenPosition,
};
use schemra_scene::{Rect, Scene};

use crate::{
    routing::RouteGeometry,
    timeline::{TimelineSampler, TimelineSampling},
};

/// Reusable scene geometry cache for sampling multiple timelines against one scene.
pub struct SceneGeometrySampler {
    route_geometry: Vec<RouteGeometry>,
    scene_index: SceneSamplingIndex,
}

impl SceneGeometrySampler {
    /// Builds a sampler for fixed scene geometry.
    pub fn new(scene: &Scene) -> Self {
        let route_geometry = scene
            .edges
            .iter()
            .map(|edge| RouteGeometry::new(&edge.path))
            .collect();
        let scene_index = SceneSamplingIndex::new(scene);
        Self {
            route_geometry,
            scene_index,
        }
    }

    /// Samples a timeline using the cached geometry.
    pub fn sample(&self, timeline: &Timeline, time_ms: u64) -> SceneTimelineFrame {
        self.resolve_frame(timeline.sample(time_ms))
    }

    /// Resolves a previously sampled frame against the cached scene geometry.
    pub fn sample_frame(&self, frame: TimelineFrame) -> SceneTimelineFrame {
        self.resolve_frame(frame)
    }

    fn resolve_frame(&self, frame: TimelineFrame) -> SceneTimelineFrame {
        sample_scene_with_geometry(frame, &self.route_geometry, &self.scene_index)
    }
}

/// Reusable sampler that caches route lengths and scene lookups for playback.
pub struct SceneTimelineSampler<'timeline> {
    _timeline: &'timeline Timeline,
    geometry: SceneGeometrySampler,
    sampler: TimelineSampler,
}

impl<'timeline> SceneTimelineSampler<'timeline> {
    /// Builds a sampler for a fixed scene and timeline.
    pub fn new(scene: &Scene, timeline: &'timeline Timeline) -> Self {
        Self {
            _timeline: timeline,
            geometry: SceneGeometrySampler::new(scene),
            sampler: TimelineSampler::new(timeline.clone()),
        }
    }

    /// Returns the active marker positions, link progress, and highlight effects.
    pub fn sample(&self, time_ms: u64) -> SceneTimelineFrame {
        self.geometry.sample_frame(self.sampler.sample(time_ms))
    }
}

/// Resolves a sampled timeline frame against one scene without caching geometry.
pub(super) fn sample_scene(frame: TimelineFrame, scene: &Scene) -> SceneTimelineFrame {
    SceneGeometrySampler::new(scene).resolve_frame(frame)
}

#[derive(Default)]
struct SceneSamplingIndex {
    edges_by_source: HashMap<String, HashMap<String, usize>>,
    node_bounds: HashMap<String, Rect>,
}

impl SceneSamplingIndex {
    fn new(scene: &Scene) -> Self {
        let mut index = Self::default();
        for (edge_index, edge) in scene.edges.iter().enumerate() {
            index
                .edges_by_source
                .entry(edge.from.clone())
                .or_default()
                .entry(edge.to.clone())
                .or_insert(edge_index);
        }
        for node in &scene.nodes {
            index.node_bounds.insert(node.id.clone(), node.bounds);
        }
        index
    }
}

fn sample_scene_with_geometry(
    frame: TimelineFrame,
    route_geometry: &[RouteGeometry],
    scene_index: &SceneSamplingIndex,
) -> SceneTimelineFrame {
    let mut tokens = Vec::with_capacity(frame.tokens.len());
    let mut active_edges = Vec::with_capacity(frame.tokens.len());

    for token in frame.tokens {
        let TokenPosition::Moving {
            from,
            to,
            progress_micros,
        } = token.position;
        let Some((edge_index, forward)) = scene_edge_for_move(scene_index, &from, &to) else {
            continue;
        };
        let route_progress = progress_micros as f32 / 1_000_000.0;
        let path_progress = if forward {
            route_progress
        } else {
            1.0 - route_progress
        };
        let Some(point) = route_geometry
            .get(edge_index)
            .and_then(|geometry| geometry.point_at_progress(path_progress))
        else {
            continue;
        };
        tokens.push(SceneTokenPosition {
            instance_id: token.instance_id,
            token: token.token.0.clone(),
            x: point.x,
            y: point.y,
            traveling: true,
        });
        active_edges.push(SceneEdgeProgress {
            instance_id: token.instance_id,
            token: token.token.0,
            edge_index,
            progress_micros,
            traversal_direction: if forward { 1 } else { -1 },
        });
    }

    let highlights = frame
        .highlights
        .into_iter()
        .filter_map(|highlight| {
            let origin = match &highlight.origin {
                HighlightOrigin::Default => scene_index
                    .node_bounds
                    .get(&highlight.node.0)
                    .map(|bounds| [bounds.x + bounds.width / 2.0, bounds.y]),
                HighlightOrigin::Arrival { from, to } => {
                    let (edge_index, forward) = scene_edge_for_move(scene_index, from, to)?;
                    let progress = if forward { 1.0 } else { 0.0 };
                    route_geometry
                        .get(edge_index)?
                        .point_at_progress(progress)
                        .map(|point| [point.x, point.y])
                }
                HighlightOrigin::Side { side } => scene_index
                    .node_bounds
                    .get(&highlight.node.0)
                    .map(|bounds| side_origin(*side, *bounds)),
            };
            Some(SceneHighlight {
                instance_id: highlight.instance_id,
                node_id: highlight.node.0,
                effect: highlight.effect,
                progress_micros: highlight.progress_micros,
                pulse_opacity: highlight.pulse_opacity,
                origin,
            })
        })
        .collect();

    SceneTimelineFrame {
        time_ms: frame.time_ms,
        tokens,
        active_edges,
        highlights,
    }
}

fn side_origin(side: HighlightSide, bounds: Rect) -> [f32; 2] {
    match side {
        HighlightSide::Left => [bounds.x, bounds.y + bounds.height / 2.0],
        HighlightSide::Right => [bounds.x + bounds.width, bounds.y + bounds.height / 2.0],
        HighlightSide::Top => [bounds.x + bounds.width / 2.0, bounds.y],
        HighlightSide::Bottom => [bounds.x + bounds.width / 2.0, bounds.y + bounds.height],
    }
}

fn scene_edge_for_move(
    index: &SceneSamplingIndex,
    from: &NodeId,
    to: &NodeId,
) -> Option<(usize, bool)> {
    let (edge_index, forward) = index
        .edges_by_source
        .get(&from.0)
        .and_then(|targets| targets.get(&to.0))
        .map(|edge_index| (*edge_index, true))
        .or_else(|| {
            index
                .edges_by_source
                .get(&to.0)
                .and_then(|targets| targets.get(&from.0))
                .map(|edge_index| (*edge_index, false))
        })?;
    Some((edge_index, forward))
}
