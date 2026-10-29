use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use schemra_compiler::{ProjectSource, analyze_project, validate_scenario_for_visual};
use schemra_syntax::{Diagnostic, Severity};
use serde::Serialize;

const IGNORED_DIRECTORIES: [&str; 6] = [
    ".git",
    ".direnv",
    ".schemra",
    "node_modules",
    "target",
    ".svelte-kit",
];

const MAX_PROJECT_FILES: usize = 512;
const MAX_PROJECT_SOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_PROJECT_DEPTH: usize = 32;
const MAX_DIRECTORY_ENTRIES: usize = 8192;
const MAX_PATH_BYTES: usize = 4096;

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

pub(super) fn check(input: &Path, root: &Path) -> Result<CheckReport, String> {
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

    let ok = file.kind != "unknown"
        && diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error);
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

pub(super) fn scenario_check(
    input: &Path,
    visual: &Path,
    root: &Path,
) -> Result<ScenarioCheckReport, String> {
    let scenario = check(input, root)?;
    let selected_visual = check(visual, root)?;
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
    let report = check(input, root).map_err(|error| (2, error))?;
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

fn resolve_entry_path(input: &Path, root: &Path) -> Result<String, String> {
    let input = if input.is_absolute() {
        input.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("could not determine the current directory: {error}"))?
            .join(input)
    };
    let input = fs::canonicalize(&input)
        .map_err(|error| format!("could not read source file {}: {error}", input.display()))?;
    let relative = input.strip_prefix(root).map_err(|_| {
        format!(
            "{} is outside project root {}",
            input.display(),
            root.display()
        )
    })?;
    if !relative
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("schemra"))
    {
        return Err(format!("{} is not a .schemra source file", input.display()));
    }
    relative_path(relative).ok_or_else(|| {
        format!(
            "{} cannot be represented as a project-relative UTF-8 path",
            input.display()
        )
    })
}

fn read_sources(root: &Path) -> Result<Vec<ProjectSource>, String> {
    let mut sources = Vec::new();
    let mut total_bytes = 0usize;
    collect_sources(root, root, 0, &mut sources, &mut total_bytes)?;
    sources.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(sources)
}

fn collect_sources(
    root: &Path,
    directory: &Path,
    depth: usize,
    sources: &mut Vec<ProjectSource>,
    total_bytes: &mut usize,
) -> Result<(), String> {
    if depth > MAX_PROJECT_DEPTH {
        return Err(format!(
            "project directory nesting exceeds the configured depth limit of {MAX_PROJECT_DEPTH}"
        ));
    }

    let directory_entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "could not read project directory {}: {error}",
            directory.display()
        )
    })?;
    let mut entries = Vec::new();
    for entry in directory_entries {
        if entries.len() == MAX_DIRECTORY_ENTRIES {
            return Err(format!(
                "project directory {} exceeds the configured entry limit of {MAX_DIRECTORY_ENTRIES}",
                directory.display()
            ));
        }
        entries.push(entry.map_err(|error| {
            format!(
                "could not read an entry in {}: {error}",
                directory.display()
            )
        })?);
    }
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if file_type.is_dir() {
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| IGNORED_DIRECTORIES.contains(&name))
            {
                continue;
            }
            collect_sources(root, &path, depth + 1, sources, total_bytes)?;
            continue;
        }
        if !file_type.is_file()
            || !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("schemra"))
        {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .expect("project walk stays under the canonical root");
        let relative = relative_path(relative).ok_or_else(|| {
            format!(
                "{} cannot be represented as a project-relative UTF-8 path",
                path.display()
            )
        })?;
        if relative.len() > MAX_PATH_BYTES {
            return Err(format!(
                "project source path exceeds the configured {MAX_PATH_BYTES}-byte limit"
            ));
        }
        if sources.len() >= MAX_PROJECT_FILES {
            return Err(format!(
                "project contains more than the configured {MAX_PROJECT_FILES} source files"
            ));
        }

        let file = File::open(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let mut source = String::new();
        file.take(super::MAX_SOURCE_BYTES.saturating_add(1) as u64)
            .read_to_string(&mut source)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if source.len() > super::MAX_SOURCE_BYTES {
            return Err(format!(
                "source {relative} exceeds the configured per-source limit of {} bytes",
                super::MAX_SOURCE_BYTES
            ));
        }
        *total_bytes = total_bytes.checked_add(source.len()).ok_or_else(|| {
            "the total project source size overflowed the supported range".to_owned()
        })?;
        if *total_bytes > MAX_PROJECT_SOURCE_BYTES {
            return Err(format!(
                "project source text exceeds the configured {MAX_PROJECT_SOURCE_BYTES}-byte aggregate limit"
            ));
        }
        sources.push(ProjectSource {
            id: relative.clone(),
            path: relative,
            source,
        });
    }
    Ok(())
}

fn relative_path(path: &Path) -> Option<String> {
    path.to_str().map(|path| path.replace('\\', "/"))
}
