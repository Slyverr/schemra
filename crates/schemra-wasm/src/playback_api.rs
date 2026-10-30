use schemra_engine::{SceneGeometrySampler, TimelineSampler};
use schemra_ir::Timeline;
use schemra_scene::Scene;

use crate::compilation;

/// Reusable playback sampler compiled from one project visual and scenario.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub struct ScenarioSampler {
    timeline: TimelineSampler,
    duration_ms: u32,
    geometry: SceneGeometrySampler,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
impl ScenarioSampler {
    /// Compiles one scenario and caches its scene geometry for repeated sampling.
    #[cfg_attr(
        target_arch = "wasm32",
        wasm_bindgen::prelude::wasm_bindgen(constructor)
    )]
    pub fn new(sources_json: &str, visual_id: &str, scenario_id: &str) -> Result<Self, String> {
        let (scene, timeline) =
            compilation::compile_scenario_playback(sources_json, visual_id, scenario_id)?;
        Ok(Self::from_scene_and_timeline(scene, timeline))
    }

    /// Returns the total scenario duration in milliseconds.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = durationMs))]
    pub fn duration_ms(&self) -> u32 {
        self.duration_ms
    }

    /// Samples markers, link progress, and node highlights at the requested time.
    pub fn sample(&self, time_ms: u32) -> String {
        let frame = self.timeline.sample(u64::from(time_ms));
        serde_json::to_string(&self.geometry.sample_frame(frame))
            .unwrap_or_else(|_| "null".to_owned())
    }
}

impl ScenarioSampler {
    fn from_scene_and_timeline(scene: Scene, timeline: Timeline) -> Self {
        let duration_ms = timeline.duration_ms as u32;
        let timeline = TimelineSampler::new(timeline);
        let geometry = SceneGeometrySampler::new(&scene);
        Self {
            timeline,
            duration_ms,
            geometry,
        }
    }
}
