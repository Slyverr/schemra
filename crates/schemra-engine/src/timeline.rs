use schemra_ir::{
    ActionIr, HighlightFrame, ScenarioIr, SceneTimelineFrame, SourceSpan, Timeline, TimelineError,
    TimelineEvent, TimelineEventKind, TimelineFrame, TokenFrame, TokenPosition,
};
use schemra_scene::Scene;

const DEFAULT_MOVE_DURATION_MS: u64 = 1_000;
const DEFAULT_HIGHLIGHT_DURATION_MS: u64 = 1_000;
const MAX_DURATION_MS: u64 = 120_000;
const MAX_EVENTS: usize = 500;

/// Sampling operations for an immutable timeline.
pub trait TimelineSampling {
    /// Returns the active movements and node highlights at `time_ms`.
    fn sample(&self, time_ms: u64) -> TimelineFrame;

    /// Resolves active markers and highlights against immutable scene geometry.
    fn sample_scene(&self, scene: &Scene, time_ms: u64) -> SceneTimelineFrame {
        super::scene_timeline::sample_scene(self.sample(time_ms), scene)
    }
}

impl TimelineSampling for Timeline {
    fn sample(&self, time_ms: u64) -> TimelineFrame {
        let mut tokens = Vec::new();
        let mut highlighted_nodes = Vec::new();
        let mut highlights = Vec::new();

        for (instance_id, event) in self.events.iter().enumerate() {
            if !(event.start_ms <= time_ms && time_ms < event.end_ms) {
                continue;
            }
            match &event.kind {
                TimelineEventKind::Move { token, from, to } => tokens.push(TokenFrame {
                    instance_id,
                    token: token.clone(),
                    position: TokenPosition::Moving {
                        from: from.clone(),
                        to: to.clone(),
                        progress_micros: normalized_progress_micros(
                            time_ms - event.start_ms,
                            event.end_ms - event.start_ms,
                        ),
                    },
                }),
                TimelineEventKind::Highlight {
                    node,
                    effect,
                    origin,
                } => {
                    highlighted_nodes.push(node.clone());
                    let presentation = super::effects::sample(
                        *effect,
                        normalized_progress_micros(
                            time_ms - event.start_ms,
                            event.end_ms - event.start_ms,
                        ),
                    );
                    highlights.push(HighlightFrame {
                        instance_id,
                        node: node.clone(),
                        effect: *effect,
                        progress_micros: presentation.progress_micros,
                        pulse_opacity: presentation.pulse_opacity,
                        origin: origin.clone(),
                    });
                }
                TimelineEventKind::Wait => {}
            }
        }

        tokens.sort_by_key(|token| {
            let event = &self.events[token.instance_id];
            (event.start_ms, token.instance_id)
        });
        highlights.sort_by_key(|highlight| {
            let event = &self.events[highlight.instance_id];
            (event.start_ms, highlight.instance_id)
        });
        highlighted_nodes.sort_by(|left, right| left.0.cmp(&right.0));
        highlighted_nodes.dedup();

        TimelineFrame {
            time_ms,
            tokens,
            highlighted_nodes,
            highlights,
        }
    }
}

fn normalized_progress_micros(elapsed_ms: u64, duration_ms: u64) -> u32 {
    let progress = (u128::from(elapsed_ms) * 1_000_000)
        .checked_div(u128::from(duration_ms))
        .unwrap_or(1_000_000);
    progress.min(1_000_000) as u32
}

/// Compiles sequential, parallel, and finite repeated actions into a flat schedule.
///
/// Parallel branches share their start time and advance the enclosing sequence by
/// their longest duration. Durations are half-open when sampled.
pub fn compile_timeline(scenario: &ScenarioIr) -> Result<Timeline, TimelineError> {
    let mut events = Vec::new();
    let duration_ms = schedule_sequence(&scenario.actions, 0, &mut events)?;
    events.sort_by_key(|event| (event.start_ms, event.end_ms));
    Ok(Timeline {
        duration_ms,
        events,
    })
}

fn schedule_sequence(
    actions: &[ActionIr],
    start_ms: u64,
    events: &mut Vec<TimelineEvent>,
) -> Result<u64, TimelineError> {
    let mut cursor = start_ms;
    for action in actions {
        let duration_ms = schedule_action(action, cursor, events)?;
        cursor = cursor
            .checked_add(duration_ms)
            .ok_or_else(|| duration_error(action))?;
        if cursor > MAX_DURATION_MS {
            return Err(duration_error(action));
        }
    }
    Ok(cursor - start_ms)
}

fn schedule_action(
    action: &ActionIr,
    start_ms: u64,
    events: &mut Vec<TimelineEvent>,
) -> Result<u64, TimelineError> {
    match action {
        ActionIr::Sequence { actions, .. } => schedule_sequence(actions, start_ms, events),
        ActionIr::Move {
            token,
            from,
            to,
            duration_ms,
            span,
            ..
        } => {
            let duration_ms = duration_ms.unwrap_or(DEFAULT_MOVE_DURATION_MS);
            push_event(
                events,
                TimelineEvent {
                    start_ms,
                    end_ms: start_ms
                        .checked_add(duration_ms)
                        .ok_or_else(|| duration_error(action))?,
                    kind: TimelineEventKind::Move {
                        token: token.clone(),
                        from: from.clone(),
                        to: to.clone(),
                    },
                    span: *span,
                },
                action,
            )?;
            Ok(duration_ms)
        }
        ActionIr::Wait { duration_ms, span } => {
            let end_ms = start_ms
                .checked_add(*duration_ms)
                .ok_or_else(|| duration_error(action))?;
            push_event(
                events,
                TimelineEvent {
                    start_ms,
                    end_ms,
                    kind: TimelineEventKind::Wait,
                    span: *span,
                },
                action,
            )?;
            Ok(*duration_ms)
        }
        ActionIr::Highlight {
            node,
            duration_ms,
            effect,
            origin,
            span,
            ..
        } => {
            let duration_ms = duration_ms.unwrap_or(DEFAULT_HIGHLIGHT_DURATION_MS);
            push_event(
                events,
                TimelineEvent {
                    start_ms,
                    end_ms: start_ms
                        .checked_add(duration_ms)
                        .ok_or_else(|| duration_error(action))?,
                    kind: TimelineEventKind::Highlight {
                        node: node.clone(),
                        effect: *effect,
                        origin: origin.clone(),
                    },
                    span: *span,
                },
                action,
            )?;
            Ok(duration_ms)
        }
        ActionIr::Parallel { actions, span } => {
            let mut longest = 0;
            for child in actions {
                longest = longest.max(schedule_sequence(
                    std::slice::from_ref(child),
                    start_ms,
                    events,
                )?);
            }
            if start_ms
                .checked_add(longest)
                .is_none_or(|end_ms| end_ms > MAX_DURATION_MS)
            {
                return Err(TimelineError {
                    code: "E_TIMELINE_DURATION_LIMIT",
                    message: "The scenario timeline exceeds the maximum duration.",
                    span: *span,
                });
            }
            Ok(longest)
        }
        ActionIr::Repeat { times, body, span } => {
            if *times == 0 {
                return Err(TimelineError {
                    code: "E_REPEAT_COUNT",
                    message: "A repeat count must be greater than zero.",
                    span: *span,
                });
            }
            let mut template = Vec::new();
            let iteration_duration = schedule_sequence(body, start_ms, &mut template)?;
            let total_duration = iteration_duration
                .checked_mul(u64::from(*times))
                .ok_or_else(|| duration_error(action))?;
            if start_ms
                .checked_add(total_duration)
                .is_none_or(|end_ms| end_ms > MAX_DURATION_MS)
            {
                return Err(duration_error(action));
            }
            let expanded_count = template
                .len()
                .checked_mul(*times as usize)
                .and_then(|count| count.checked_add(events.len()));
            if expanded_count.is_none_or(|count| count > MAX_EVENTS) {
                return Err(TimelineError {
                    code: "E_TIMELINE_EVENT_LIMIT",
                    message: "The scenario timeline contains too many events.",
                    span: *span,
                });
            }
            events.extend(template.iter().cloned());
            for iteration in 1..*times {
                let offset = iteration_duration * u64::from(iteration);
                events.extend(template.iter().cloned().map(|mut event| {
                    event.start_ms += offset;
                    event.end_ms += offset;
                    event
                }));
            }
            Ok(total_duration)
        }
    }
}

fn push_event(
    events: &mut Vec<TimelineEvent>,
    event: TimelineEvent,
    action: &ActionIr,
) -> Result<(), TimelineError> {
    if events.len() >= MAX_EVENTS {
        return Err(TimelineError {
            code: "E_TIMELINE_EVENT_LIMIT",
            message: "The scenario timeline contains too many events.",
            span: action_span(action),
        });
    }
    if event.end_ms > MAX_DURATION_MS {
        return Err(duration_error(action));
    }
    events.push(event);
    Ok(())
}

fn action_span(action: &ActionIr) -> SourceSpan {
    match action {
        ActionIr::Move { span, .. }
        | ActionIr::Sequence { span, .. }
        | ActionIr::Wait { span, .. }
        | ActionIr::Highlight { span, .. }
        | ActionIr::Parallel { span, .. }
        | ActionIr::Repeat { span, .. } => *span,
    }
}

fn duration_error(action: &ActionIr) -> TimelineError {
    TimelineError {
        code: "E_TIMELINE_DURATION_LIMIT",
        message: "The scenario timeline exceeds the maximum duration.",
        span: action_span(action),
    }
}
