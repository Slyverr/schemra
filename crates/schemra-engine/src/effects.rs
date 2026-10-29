use schemra_ir::HighlightEffect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EffectSample {
    pub progress_micros: u32,
    pub pulse_opacity: f32,
}

pub(crate) fn sample(effect: HighlightEffect, progress_micros: u32) -> EffectSample {
    match effect {
        HighlightEffect::Trace => trace::sample(progress_micros),
        HighlightEffect::Pulse => pulse::sample(progress_micros),
    }
}

mod trace {
    use super::EffectSample;

    pub(super) fn sample(progress_micros: u32) -> EffectSample {
        const TRACE_PORTION: u32 = 800_000;
        let trace_progress = (u64::from(progress_micros.min(TRACE_PORTION)) * 1_000_000
            / u64::from(TRACE_PORTION)) as u32;
        let fade_progress = (u64::from(progress_micros.saturating_sub(TRACE_PORTION)) * 1_000_000
            / u64::from(1_000_000 - TRACE_PORTION)) as u32;
        EffectSample {
            progress_micros: trace_progress,
            pulse_opacity: if progress_micros <= TRACE_PORTION {
                1.0
            } else {
                1.0 - fade_progress as f32 / 1_000_000.0
            },
        }
    }
}

mod pulse {
    use super::EffectSample;

    pub(super) fn sample(progress_micros: u32) -> EffectSample {
        let progress = progress_micros as f32 / 1_000_000.0;
        EffectSample {
            progress_micros,
            pulse_opacity: 0.35 + 0.65 * (std::f32::consts::PI * progress).sin(),
        }
    }
}
