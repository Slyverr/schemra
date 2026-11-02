use std::{
    fs,
    path::{Path, PathBuf},
};

use schemra_compiler::{analyze_project, validate_scenario_for_visual};
use schemra_project::{
    ProjectSource, read_sources as discover_sources,
    resolve_entry_path as resolve_project_entry_path,
};
use schemra_syntax::{Diagnostic, Severity};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct CheckReport {
    pub ok: bool,
    pub kind: String,
    pub path: String,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip)]
    visual_id: Option<String>,
    #[serde(skip)]
    scenario_count: usize,
    #[serde(skip)]
    svg: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ScenarioCheckReport {
    pub ok: bool,
    scenario: String,
    visual: String,
    scenario_visual_id: Option<String>,
    selected_visual_id: Option<String>,
    scenario_count: usize,
    selection_error: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

impl ScenarioCheckReport {
    pub(super) fn to_json(&self) -> String {
        serde_json::to_string(self).expect("scenario check report is serializable")
    }
}

impl CheckReport {
    pub(super) fn to_json(&self) -> String {
        serde_json::to_string(self).expect("check report is serializable")
    }
}

pub(super) fn check(input: &Path, root: &Path, strict: bool) -> Result<CheckReport, String> {
    let root = canonical_root(root)?;
    let entry_path = resolve_entry_path(input, &root)?;
    let sources = read_sources(&root)?;
    let analysis = analyze_project(&sources);
    let Some(entry_index) = analysis
        .files
        .iter()
        .position(|file| file.path == entry_path)
    else {
        return Err(format!(
            "{entry_path} is not a regular Schemra source file inside the project"
        ));
    };
    let file = &analysis.files[entry_index];
    let mut diagnostics = file.diagnostics.clone();

    let visual_analysis = if file.kind == "visual" {
        analysis
            .visuals
            .iter()
            .find(|visual| visual.source_index == entry_index)
    } else {
        file.resolved_visual_id
            .as_deref()
            .and_then(|id| analysis.visuals.iter().find(|visual| visual.id == id))
    };

    let mut svg = None;
    if let Some(visual_analysis) = visual_analysis {
        let visual_file = &analysis.files[visual_analysis.source_index];
        if let Some(visual) = &visual_file.visual {
            let scenarios = if file.kind == "visual" {
                &visual_analysis.scenarios
            } else {
                &file.resolved_scenarios
            };
            validate_visual_and_scenarios(visual, scenarios, &mut diagnostics);
            let scene = schemra_engine::layout_visual(visual);
            match schemra_engine::render_svg(&scene) {
                Ok(rendered) => svg = Some(rendered),
                Err(error) => diagnostics.push(Diagnostic::error(
                    "E_SCENE_RENDER",
                    format!("The compiled scene cannot be rendered: {error}."),
                    visual.span,
                    None::<String>,
                )),
            }
        }
    }

    let ok = report_is_ok(&file.kind, &diagnostics, strict);
    Ok(CheckReport {
        ok,
        kind: file.kind.clone(),
        path: file.path.clone(),
        diagnostics,
        visual_id: file.visual_id.clone(),
        scenario_count: file.resolved_scenarios.len(),
        svg,
    })
}

fn report_is_ok(kind: &str, diagnostics: &[Diagnostic], strict: bool) -> bool {
    kind != "unknown"
        && diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error)
        && (!strict
            || diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != Severity::Warning))
}

pub(super) fn scenario_check(
    input: &Path,
    visual: &Path,
    root: &Path,
) -> Result<ScenarioCheckReport, String> {
    let scenario = check(input, root, false)?;
    let selected_visual = check(visual, root, false)?;
    let selection_error = if scenario.kind != "scenario" {
        Some(format!(
            "{} is a {} source file; scenario-check expects a scenario source file",
            scenario.path, scenario.kind
        ))
    } else if selected_visual.kind != "visual" {
        Some(format!(
            "{} is a {} source file; --visual must point to a visual",
            selected_visual.path, selected_visual.kind
        ))
    } else if scenario.visual_id != selected_visual.visual_id {
        Some(format!(
            "{} targets visual {:?}, but {} declares visual {:?}",
            scenario.path,
            scenario.visual_id.as_deref().unwrap_or("<missing>"),
            selected_visual.path,
            selected_visual.visual_id.as_deref().unwrap_or("<missing>")
        ))
    } else {
        None
    };
    let ok = selection_error.is_none() && scenario.scenario_count > 0 && scenario.ok;

    Ok(ScenarioCheckReport {
        ok,
        scenario: scenario.path,
        visual: selected_visual.path,
        scenario_visual_id: scenario.visual_id,
        selected_visual_id: selected_visual.visual_id,
        scenario_count: scenario.scenario_count,
        selection_error,
        diagnostics: scenario.diagnostics,
    })
}

pub(super) fn print_scenario_report(report: &ScenarioCheckReport) {
    print_diagnostics(&report.scenario, &report.diagnostics);
    if let Some(error) = &report.selection_error {
        eprintln!("{error}");
    } else if report.ok {
        println!(
            "Valid Schemra scenario: {} for {} ({} scenario{})",
            report.scenario,
            report.visual,
            report.scenario_count,
            if report.scenario_count == 1 { "" } else { "s" }
        );
    } else if report.scenario_count == 0 && report.diagnostics.is_empty() {
        eprintln!("{} does not contain a compilable scenario", report.scenario);
    }
}

pub(super) fn render(input: &Path, output: &Path, root: &Path) -> Result<(), (u8, String)> {
    let report = check(input, root, false).map_err(|error| (2, error))?;
    if report.kind != "visual" {
        print_diagnostics(&report.path, &report.diagnostics);
        return Err((
            1,
            format!("{} does not produce a visual to render", report.path),
        ));
    }
    if !report.ok {
        print_report(&report);
        return Err((1, String::new()));
    }
    let Some(svg) = report.svg else {
        return Err((1, format!("{} compiled without SVG output", report.path)));
    };
    fs::write(output, svg)
        .map_err(|error| (2, format!("could not write {}: {error}", output.display())))?;
    println!("Rendered {}", output.display());
    Ok(())
}

pub(super) fn print_report(report: &CheckReport) {
    print_diagnostics(&report.path, &report.diagnostics);
    if report.ok {
        println!("Valid Schemra {}: {}", report.kind, report.path);
    } else if report.diagnostics.is_empty() {
        eprintln!(
            "{} does not contain a visual or a scenario linked to one",
            report.path
        );
    }
}

fn print_diagnostics(path: &str, diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        let location = diagnostic.span.start;
        eprintln!(
            "{}:{}:{}: {}: {}",
            path, location.line, location.column, diagnostic.code, diagnostic.message
        );
        if let Some(hint) = &diagnostic.hint {
            eprintln!("  hint: {hint}");
        }
    }
}

fn validate_visual_and_scenarios(
    visual: &schemra_ir::VisualIr,
    scenarios: &[schemra_ir::ScenarioIr],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Err(error) =
        schemra_engine::check_visual_limits(visual, schemra_engine::VisualLimits::default())
    {
        diagnostics.push(Diagnostic::error(
            error.code,
            error.message,
            error.span,
            Some("Reduce the visual elements or links that exceed the configured limit."),
        ));
    }

    for scenario in scenarios {
        diagnostics.extend(validate_scenario_for_visual(scenario, visual));
        if let Err(error) = schemra_engine::compile_timeline(scenario) {
            diagnostics.push(Diagnostic::error(
                error.code,
                error.message,
                error.span,
                None::<String>,
            ));
        }
    }
}

fn canonical_root(root: &Path) -> Result<PathBuf, String> {
    let root = fs::canonicalize(root)
        .map_err(|error| format!("could not open project root {}: {error}", root.display()))?;
    if !root.is_dir() {
        return Err(format!(
            "project root {} is not a directory",
            root.display()
        ));
    }
    Ok(root)
}

#[cfg(test)]
mod tests;

fn resolve_entry_path(input: &Path, root: &Path) -> Result<String, String> {
    resolve_project_entry_path(input, root).map_err(|error| error.to_string())
}

fn read_sources(root: &Path) -> Result<Vec<ProjectSource>, String> {
    discover_sources(root).map_err(|error| error.to_string())
}
