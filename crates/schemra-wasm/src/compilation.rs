use std::collections::HashMap;

use schemra_compiler::{
    DeclarationInfo, EditorSymbol, ProjectFileAnalysis, ProjectSource, SyntaxToken,
    analyze_project, validate_scenarios_for_visual,
};
use schemra_engine::{
    VisualLimits, check_visual_limits, compile_timeline, layout_visual, render_svg,
};
use schemra_ir::{ScenarioIr, Timeline, TimelineEventKind, VisualIr};
use schemra_scene::Scene;
use schemra_syntax::{Diagnostic, Severity, SourceLocation, SourceSpan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResourceLimits {
    max_source_bytes: usize,
    max_project_files: usize,
    max_project_source_bytes: usize,
    max_project_json_bytes: usize,
    max_path_bytes: usize,
    max_visual_elements: usize,
    max_visual_links: usize,
    max_diagnostics: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 2 * 1024 * 1024,
            max_project_files: 512,
            max_project_source_bytes: 16 * 1024 * 1024,
            max_project_json_bytes: 64 * 1024 * 1024,
            max_path_bytes: 4096,
            max_visual_elements: 10_000,
            max_visual_links: 1_000,
            max_diagnostics: 256,
        }
    }
}

impl ResourceLimits {
    fn visual(self) -> VisualLimits {
        VisualLimits {
            max_visual_elements: self.max_visual_elements,
            max_visual_links: self.max_visual_links,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct CompileStats {
    nodes: usize,
    edges: usize,
    groups: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct CompiledTokenStyle {
    token: String,
    icon: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct CompiledScenario {
    id: String,
    title: String,
    span: schemra_ir::SourceSpan,
    duration_ms: u64,
    tokens: Vec<CompiledTokenStyle>,
    effects: Vec<ScheduledEffect>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ScheduledEffect {
    Move {
        token: String,
        from: String,
        to: String,
        duration_ms: u64,
        start_ms: u64,
        end_ms: u64,
        span: schemra_ir::SourceSpan,
    },
    Highlight {
        target_id: String,
        effect: schemra_ir::HighlightEffect,
        origin: schemra_ir::HighlightOrigin,
        start_ms: u64,
        end_ms: u64,
        span: schemra_ir::SourceSpan,
    },
    Wait {
        start_ms: u64,
        end_ms: u64,
        span: schemra_ir::SourceSpan,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct CompileOutput {
    ok: bool,
    svg: Option<String>,
    title: Option<String>,
    stats: Option<CompileStats>,
    diagnostics: Vec<Diagnostic>,
    tokens: Vec<SyntaxToken>,
    declarations: Vec<DeclarationInfo>,
    scenarios: Vec<CompiledScenario>,
    scene: Option<Scene>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ProjectFileOutput {
    id: String,
    path: String,
    kind: String,
    visual_id: Option<String>,
    declarations: Vec<DeclarationInfo>,
    editor_symbols: Vec<EditorSymbol>,
    tokens: Vec<SyntaxToken>,
    diagnostics: Vec<Diagnostic>,
    output: Option<CompileOutput>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ProjectVisualOutput {
    id: String,
    path: String,
    title: String,
    output: CompileOutput,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ProjectCompileOutput {
    error: Option<String>,
    files: Vec<ProjectFileOutput>,
    visuals: Vec<ProjectVisualOutput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectCompileRequest {
    sources: Vec<ProjectSource>,
}

struct CompiledFile {
    analysis: ProjectFileAnalysis,
    output: Option<CompileOutput>,
}

pub(super) fn compile_source_json(source: &str) -> String {
    let limits = ResourceLimits::default();
    let output = if source.len() > limits.max_source_bytes {
        input_limit_failure(format!(
            "source uses {} bytes; the configured per-source limit is {}",
            source.len(),
            limits.max_source_bytes
        ))
    } else {
        let project = compile_project_sources(
            &[ProjectSource {
                id: "standalone".into(),
                path: "visual.schemra".into(),
                source: source.into(),
            }],
            limits,
        );
        if let Some(visual) = project.visuals.into_iter().next() {
            visual.output
        } else {
            let file = project.files.first();
            CompileOutput {
                ok: false,
                svg: None,
                title: None,
                stats: None,
                diagnostics: file
                    .map(|file| file.diagnostics.clone())
                    .filter(|diagnostics| !diagnostics.is_empty())
                    .unwrap_or_else(|| {
                        vec![Diagnostic::error(
                            "E_EXPECTED_VISUAL",
                            "A standalone Schemra document needs a visual declaration.",
                            origin_span(),
                            None::<String>,
                        )]
                    }),
                tokens: file.map(|file| file.tokens.clone()).unwrap_or_default(),
                declarations: file
                    .map(|file| file.declarations.clone())
                    .unwrap_or_default(),
                scenarios: Vec::new(),
                scene: None,
            }
        }
    };

    serialize_compile_output(&output)
}

pub(super) fn compile_project_json(sources_json: &str) -> String {
    let limits = ResourceLimits::default();
    if let Err(error) = validate_project_request_size(sources_json.len()) {
        return serialize_project_output(&project_error(error.to_string()));
    }

    let request = match serde_json::from_str::<ProjectCompileRequest>(sources_json) {
        Ok(request) => request,
        Err(error) => {
            return serialize_project_output(&project_error(format!(
                "Could not read the project source list: {error}"
            )));
        }
    };

    serialize_project_output(&compile_project_sources(&request.sources, limits))
}

pub(super) fn validate_project_request_size(size: usize) -> Result<(), String> {
    let maximum = ResourceLimits::default().max_project_json_bytes;
    if size > maximum {
        return Err(format!(
            "project JSON uses {size} bytes; the configured input limit is {maximum}"
        ));
    }
    Ok(())
}

pub(super) fn validate_project_source_list(sources: &[ProjectSource]) -> Result<(), String> {
    validate_sources(sources, ResourceLimits::default())
}

pub(super) fn compile_scenario_playback(
    sources_json: &str,
    visual_id: &str,
    scenario_id: &str,
) -> Result<(Scene, Timeline), String> {
    let limits = ResourceLimits::default();
    validate_project_request_size(sources_json.len())?;
    let request: ProjectCompileRequest = serde_json::from_str(sources_json)
        .map_err(|error| format!("Could not read the project source list: {error}"))?;
    validate_sources(&request.sources, limits)?;

    let analysis = analyze_project(&request.sources);
    let visual_analysis = analysis
        .visuals
        .iter()
        .find(|visual| visual.id == visual_id)
        .ok_or_else(|| format!("The project has no visual with ID {visual_id:?}."))?;
    let visual = analysis
        .files
        .get(visual_analysis.source_index)
        .and_then(|file| file.visual.as_ref())
        .ok_or_else(|| format!("Visual {visual_id:?} is invalid or unavailable."))?;
    check_visual_limits(visual, limits.visual())
        .map_err(|error| format!("{}: {}", error.code, error.message))?;

    let scenario = visual_analysis
        .scenarios
        .iter()
        .find(|scenario| scenario.id.0 == scenario_id)
        .ok_or_else(|| format!("Visual {visual_id:?} has no scenario with ID {scenario_id:?}."))?;
    let diagnostics = validate_scenarios_for_visual(std::slice::from_ref(scenario), visual);
    if let Some(diagnostic) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
    }

    let timeline =
        compile_timeline(scenario).map_err(|error| format!("{}: {}", error.code, error.message))?;
    Ok((layout_visual(visual), timeline))
}

fn compile_project_sources(
    sources: &[ProjectSource],
    limits: ResourceLimits,
) -> ProjectCompileOutput {
    if let Err(error) = validate_sources(sources, limits) {
        return project_error(error);
    }

    let analysis = analyze_project(sources);
    let mut files = analysis
        .files
        .into_iter()
        .map(|analysis| CompiledFile {
            analysis,
            output: None,
        })
        .collect::<Vec<_>>();
    let visual_sources = analysis
        .visuals
        .iter()
        .map(|visual| (visual.id.as_str(), visual.source_index))
        .collect::<HashMap<_, _>>();

    for file_index in 0..files.len() {
        let file = &files[file_index].analysis;
        let Some(visual_id) = file.resolved_visual_id.as_deref() else {
            continue;
        };
        if file.visual.is_some() || file.resolved_scenarios.is_empty() {
            continue;
        }
        let Some(visual_index) = visual_sources.get(visual_id).copied() else {
            continue;
        };
        let Some(visual) = files
            .get(visual_index)
            .and_then(|file| file.analysis.visual.as_ref())
        else {
            continue;
        };
        let output = compile_visual(
            visual,
            &file.resolved_scenarios,
            file.declarations.clone(),
            file.tokens.clone(),
            limits,
        );
        append_unique(
            &mut files[file_index].analysis.diagnostics,
            &output.diagnostics,
        );
        files[file_index].output = Some(output);
    }

    for visual_analysis in &analysis.visuals {
        let Some(file) = files
            .get(visual_analysis.source_index)
            .map(|file| &file.analysis)
        else {
            continue;
        };
        let Some(visual) = file.visual.as_ref() else {
            continue;
        };
        let output = compile_visual(
            visual,
            &visual_analysis.scenarios,
            file.declarations.clone(),
            file.tokens.clone(),
            limits,
        );
        append_unique(
            &mut files[visual_analysis.source_index].analysis.diagnostics,
            &output.diagnostics,
        );
        files[visual_analysis.source_index].output = Some(output);
    }

    for file in &mut files {
        let Some(span) = file
            .analysis
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.span)
        else {
            continue;
        };
        if let Some(diagnostics) =
            diagnostics_over_limit(&file.analysis.diagnostics, limits.max_diagnostics, span)
        {
            file.analysis.diagnostics = diagnostics.clone();
            if let Some(output) = file.output.take() {
                file.output = Some(fail_for_diagnostic_limit(output, diagnostics));
            }
        }
    }

    let visuals = analysis
        .visuals
        .iter()
        .filter_map(|visual_analysis| {
            let file = &files[visual_analysis.source_index].analysis;
            let visual = file.visual.as_ref()?;
            let output = files[visual_analysis.source_index].output.clone()?;
            Some(ProjectVisualOutput {
                id: visual_analysis.id.clone(),
                path: file.path.clone(),
                title: visual.title.clone(),
                output,
            })
        })
        .collect();

    ProjectCompileOutput {
        error: None,
        files: files
            .into_iter()
            .map(|file| ProjectFileOutput {
                id: file.analysis.id,
                path: file.analysis.path,
                kind: file.analysis.kind,
                visual_id: file.analysis.visual_id,
                declarations: file.analysis.declarations,
                editor_symbols: file.analysis.editor_symbols,
                tokens: file.analysis.tokens,
                diagnostics: file.analysis.diagnostics,
                output: file.output,
            })
            .collect(),
        visuals,
    }
}

fn compile_visual(
    visual: &VisualIr,
    scenario_irs: &[ScenarioIr],
    declarations: Vec<DeclarationInfo>,
    tokens: Vec<SyntaxToken>,
    limits: ResourceLimits,
) -> CompileOutput {
    let stats = CompileStats {
        nodes: visual.nodes.len(),
        edges: visual.edges.len(),
        groups: visual.groups.len(),
    };
    if let Err(error) = check_visual_limits(visual, limits.visual()) {
        return CompileOutput {
            ok: false,
            svg: None,
            title: Some(visual.title.clone()),
            stats: Some(stats),
            diagnostics: vec![Diagnostic::error(
                error.code,
                error.message,
                syntax_span(error.span),
                Some("Reduce the visual elements or links that exceed the configured limit."),
            )],
            tokens,
            declarations,
            scenarios: Vec::new(),
            scene: None,
        };
    }

    let mut diagnostics = validate_scenarios_for_visual(scenario_irs, visual);
    let mut scenarios = Vec::with_capacity(scenario_irs.len());
    for scenario in scenario_irs {
        match compile_scenario(scenario) {
            Ok(compiled) => scenarios.push(compiled),
            Err(error) => diagnostics.push(Diagnostic::error(
                error.code,
                error.message,
                syntax_span(error.span),
                None::<String>,
            )),
        }
    }

    if let Some(diagnostics) = diagnostics_over_limit(
        &diagnostics,
        limits.max_diagnostics,
        syntax_span(visual.span),
    ) {
        return CompileOutput {
            ok: false,
            svg: None,
            title: Some(visual.title.clone()),
            stats: Some(stats),
            diagnostics,
            tokens,
            declarations,
            scenarios: Vec::new(),
            scene: None,
        };
    }

    let scene = layout_visual(visual);
    let svg = match render_svg(&scene) {
        Ok(svg) => Some(svg),
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "E_SCENE_RENDER",
                format!("The compiled scene cannot be rendered: {error}."),
                syntax_span(visual.span),
                None::<String>,
            ));
            None
        }
    };
    let ok = svg.is_some()
        && diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error);

    CompileOutput {
        ok,
        svg,
        title: Some(visual.title.clone()),
        stats: Some(stats),
        diagnostics,
        tokens,
        declarations,
        scenarios,
        scene: Some(scene),
    }
}

fn compile_scenario(scenario: &ScenarioIr) -> Result<CompiledScenario, schemra_ir::TimelineError> {
    let timeline = compile_timeline(scenario)?;
    let effects = timeline
        .events
        .iter()
        .map(|event| match &event.kind {
            TimelineEventKind::Move { token, from, to } => ScheduledEffect::Move {
                token: token.0.clone(),
                from: from.0.clone(),
                to: to.0.clone(),
                duration_ms: event.end_ms - event.start_ms,
                start_ms: event.start_ms,
                end_ms: event.end_ms,
                span: event.span,
            },
            TimelineEventKind::Highlight {
                node,
                effect,
                origin,
            } => ScheduledEffect::Highlight {
                target_id: node.0.clone(),
                effect: *effect,
                origin: origin.clone(),
                start_ms: event.start_ms,
                end_ms: event.end_ms,
                span: event.span,
            },
            TimelineEventKind::Wait => ScheduledEffect::Wait {
                start_ms: event.start_ms,
                end_ms: event.end_ms,
                span: event.span,
            },
        })
        .collect();

    Ok(CompiledScenario {
        id: scenario.id.0.clone(),
        title: scenario.id.0.clone(),
        span: scenario.span,
        duration_ms: timeline.duration_ms,
        tokens: scenario
            .tokens
            .iter()
            .map(|token| CompiledTokenStyle {
                token: token.id.0.clone(),
                icon: token.icon.clone(),
            })
            .collect(),
        effects,
    })
}

fn validate_sources(sources: &[ProjectSource], limits: ResourceLimits) -> Result<(), String> {
    if sources.len() > limits.max_project_files {
        return Err(format!(
            "project contains {} source files; the configured limit is {}",
            sources.len(),
            limits.max_project_files
        ));
    }

    let mut total_bytes = 0usize;
    for source in sources {
        if source.path.len() > limits.max_path_bytes || source.id.len() > limits.max_path_bytes {
            return Err(format!(
                "project source metadata exceeds the configured {}-byte path limit",
                limits.max_path_bytes
            ));
        }
        if source.source.len() > limits.max_source_bytes {
            return Err(format!(
                "source {:?} uses {} bytes; the configured per-source limit is {}",
                source.path,
                source.source.len(),
                limits.max_source_bytes
            ));
        }
        total_bytes = total_bytes
            .checked_add(source.source.len())
            .ok_or_else(|| {
                "the total project source size overflowed the supported range".to_owned()
            })?;
        if total_bytes > limits.max_project_source_bytes {
            return Err(format!(
                "project source text uses more than the configured {}-byte aggregate limit",
                limits.max_project_source_bytes
            ));
        }
    }
    Ok(())
}

fn append_unique(target: &mut Vec<Diagnostic>, candidates: &[Diagnostic]) {
    for diagnostic in candidates {
        if !target.contains(diagnostic) {
            target.push(diagnostic.clone());
        }
    }
}

fn diagnostics_over_limit(
    diagnostics: &[Diagnostic],
    maximum: usize,
    span: SourceSpan,
) -> Option<Vec<Diagnostic>> {
    let output_limit = maximum.max(1);
    if diagnostics.len() <= output_limit {
        return None;
    }
    let mut bounded = diagnostics
        .iter()
        .take(output_limit.saturating_sub(1))
        .cloned()
        .collect::<Vec<_>>();
    bounded.push(Diagnostic::error(
        "E_DIAGNOSTIC_LIMIT",
        format!(
            "Compilation exceeded the configured diagnostic limit of {maximum}; remaining diagnostics were omitted."
        ),
        span,
        Some("Fix the reported issues or raise the host's max_diagnostics limit."),
    ));
    Some(bounded)
}

fn fail_for_diagnostic_limit(output: CompileOutput, diagnostics: Vec<Diagnostic>) -> CompileOutput {
    CompileOutput {
        ok: false,
        svg: None,
        title: output.title,
        stats: output.stats,
        diagnostics,
        tokens: output.tokens,
        declarations: output.declarations,
        scenarios: Vec::new(),
        scene: None,
    }
}

fn input_limit_failure(message: String) -> CompileOutput {
    CompileOutput {
        ok: false,
        svg: None,
        title: None,
        stats: None,
        diagnostics: vec![Diagnostic::error(
            "E_INPUT_LIMIT",
            message,
            origin_span(),
            Some("Reduce the input size or raise the host's finite resource limit."),
        )],
        tokens: Vec::new(),
        declarations: Vec::new(),
        scenarios: Vec::new(),
        scene: None,
    }
}

fn origin_span() -> SourceSpan {
    SourceSpan::point(SourceLocation {
        offset: 0,
        line: 1,
        column: 1,
    })
}

fn syntax_span(span: schemra_ir::SourceSpan) -> SourceSpan {
    SourceSpan {
        start: SourceLocation {
            offset: span.start.offset,
            line: span.start.line,
            column: span.start.column,
        },
        end: SourceLocation {
            offset: span.end.offset,
            line: span.end.line,
            column: span.end.column,
        },
    }
}

fn project_error(message: String) -> ProjectCompileOutput {
    ProjectCompileOutput {
        error: Some(message),
        files: Vec::new(),
        visuals: Vec::new(),
    }
}

fn serialize_compile_output(output: &CompileOutput) -> String {
    serde_json::to_string(output).unwrap_or_else(|error| {
        serde_json::json!({
            "ok": false,
            "svg": null,
            "title": null,
            "stats": null,
            "diagnostics": [{
                "code": "E_OUTPUT_SERIALIZATION",
                "severity": "error",
                "message": format!("Could not serialize the compilation output: {error}"),
                "span": origin_span(),
                "hint": null
            }],
            "tokens": [],
            "declarations": [],
            "scenarios": [],
            "scene": null
        })
        .to_string()
    })
}

fn serialize_project_output(output: &ProjectCompileOutput) -> String {
    serde_json::to_string(output).unwrap_or_else(|error| {
        serde_json::json!({
            "error": format!("Could not serialize the project compilation output: {error}"),
            "files": [],
            "visuals": []
        })
        .to_string()
    })
}
