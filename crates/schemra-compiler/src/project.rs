//! Source compilation and cross-file linking for Schemra projects.

use std::collections::{HashMap, HashSet};

use crate::compiler::compile_for_analysis;
use crate::{DeclarationInfo, EditorSymbol, SyntaxToken, editor_metadata, highlight_tokens};
use schemra_ir::{ScenarioIr, VisualIr};
pub use schemra_project::ProjectSource;
use schemra_syntax::{Diagnostic, SourceSpan};

#[derive(Clone, Debug)]
pub struct ProjectFileAnalysis {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub visual_id: Option<String>,
    pub visual: Option<VisualIr>,
    pub scenarios: Vec<ScenarioIr>,
    pub resolved_visual_id: Option<String>,
    pub resolved_scenarios: Vec<ScenarioIr>,
    pub declarations: Vec<DeclarationInfo>,
    pub editor_symbols: Vec<EditorSymbol>,
    pub tokens: Vec<SyntaxToken>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub struct ProjectVisualAnalysis {
    pub id: String,
    pub source_index: usize,
    pub scenarios: Vec<ScenarioIr>,
}

#[derive(Clone, Debug)]
pub struct ProjectAnalysis {
    pub files: Vec<ProjectFileAnalysis>,
    pub visuals: Vec<ProjectVisualAnalysis>,
}

/// Compiles files independently, then resolves visual and scenario references across the project.
pub fn analyze_project(sources: &[ProjectSource]) -> ProjectAnalysis {
    let mut files = sources.iter().map(analyze_source).collect::<Vec<_>>();
    let source_paths = files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();

    let mut visual_sources = HashMap::<String, usize>::new();
    let mut visual_order = Vec::new();
    for (file_index, file) in files.iter_mut().enumerate() {
        let Some(visual) = &file.visual else {
            continue;
        };
        if let Some(previous) = visual_sources.get(&visual.id.0) {
            file.diagnostics.push(project_error(
                "E_DUPLICATE_VISUAL_ID",
                format!(
                    "Visual ID {:?} is already used by {}.",
                    visual.id.0, source_paths[*previous]
                ),
                visual.span,
                "Give each visual a unique stable ID.",
            ));
            file.kind = "invalid".to_owned();
            file.visual_id = None;
        } else {
            visual_sources.insert(visual.id.0.clone(), file_index);
            visual_order.push(visual.id.0.clone());
        }
    }

    let mut scenario_names = HashSet::<(String, String)>::new();
    let mut scenarios_by_visual = HashMap::<String, Vec<ScenarioIr>>::new();
    for file in &mut files {
        if file.kind == "invalid" {
            continue;
        }
        if file.visual.is_none() {
            let targets = file
                .scenarios
                .iter()
                .map(|scenario| scenario.visual.0.as_str())
                .collect::<HashSet<_>>();
            if targets.len() > 1 {
                file.diagnostics.push(project_error(
                    "E_MULTIPLE_SCENARIO_TARGETS",
                    "A scenario source file must target one visual.".to_owned(),
                    file.scenarios[0].span,
                    "Keep scenarios for one visual together or split them into separate files.",
                ));
                continue;
            }
        }

        for scenario in &file.scenarios {
            let visual_id = &scenario.visual.0;
            if file
                .visual
                .as_ref()
                .is_some_and(|visual| visual.id.0 != *visual_id)
            {
                file.diagnostics.push(project_error(
                    "E_SCENARIO_VISUAL_TARGET",
                    format!(
                        "Scenario {:?} targets a different visual than the one in this file.",
                        scenario.id.0
                    ),
                    scenario.span,
                    "Keep each scenario beside its target visual or in a separate scenario source.",
                ));
                continue;
            }
            if !visual_sources.contains_key(visual_id) {
                file.diagnostics.push(project_error(
                    "E_UNKNOWN_VISUAL_TARGET",
                    format!(
                        "Scenario {:?} targets unknown visual {:?}.",
                        scenario.id.0, visual_id
                    ),
                    scenario.span,
                    "Use the stable ID from a visual declaration in this project.",
                ));
                continue;
            }
            if !scenario_names.insert((visual_id.clone(), scenario.id.0.clone())) {
                file.diagnostics.push(project_error(
                    "E_DUPLICATE_SCENARIO",
                    format!(
                        "Scenario ID {:?} is already declared for visual {:?}.",
                        scenario.id.0, visual_id
                    ),
                    scenario.span,
                    "Give each scenario for this visual a unique ID.",
                ));
                continue;
            }

            file.resolved_visual_id = Some(visual_id.clone());
            file.resolved_scenarios.push(scenario.clone());
            scenarios_by_visual
                .entry(visual_id.clone())
                .or_default()
                .push(scenario.clone());
        }
    }

    let visuals = visual_order
        .into_iter()
        .map(|id| {
            let source_index = visual_sources[&id];
            ProjectVisualAnalysis {
                scenarios: scenarios_by_visual.remove(&id).unwrap_or_default(),
                id,
                source_index,
            }
        })
        .collect();

    ProjectAnalysis { files, visuals }
}

fn analyze_source(source: &ProjectSource) -> ProjectFileAnalysis {
    let tokens = highlight_tokens(&source.source);
    let (mut module, diagnostics) = compile_for_analysis(&source.source);
    let has_multiple_visuals = module.visuals.len() > 1;
    let (declarations, editor_symbols) = editor_metadata(&module);
    let scenarios = std::mem::take(&mut module.scenarios);
    let visual = module.visuals.into_iter().next();
    let visual_id = visual
        .as_ref()
        .map(|visual| visual.id.0.clone())
        .or_else(|| {
            let first = &scenarios.first()?.visual.0;
            scenarios
                .iter()
                .all(|scenario| scenario.visual.0 == *first)
                .then(|| first.clone())
        });
    let (visual, visual_id) = if has_multiple_visuals {
        (None, None)
    } else {
        (visual, visual_id)
    };
    let kind = if has_multiple_visuals {
        "invalid"
    } else if visual.is_some() {
        "visual"
    } else if !scenarios.is_empty() {
        "scenario"
    } else {
        "unknown"
    };
    ProjectFileAnalysis {
        id: source.id.clone(),
        path: source.path.clone(),
        kind: kind.to_owned(),
        visual_id,
        visual,
        scenarios,
        resolved_visual_id: None,
        resolved_scenarios: Vec::new(),
        declarations,
        editor_symbols,
        tokens,
        diagnostics,
    }
}

fn project_error(code: &str, message: String, span: SourceSpan, hint: &str) -> Diagnostic {
    Diagnostic::error(code, message, span, Some(hint))
}
