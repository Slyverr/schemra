//! Source-based rename planning for visual, node, and scenario identifiers.

use std::collections::HashSet;

use schemra_syntax::{Severity, SourceSpan};
use serde::Serialize;

use crate::{
    EditorSymbol, EditorSymbolKind, EditorSymbolRole, ProjectAnalysis, ProjectSource,
    analyze_project, validate_scenarios_for_visual,
};

#[derive(Clone, Debug, Serialize)]
pub struct RenameSelection {
    pub name: String,
    pub kind: EditorSymbolKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Serialize)]
pub struct RenameEdit {
    pub path: String,
    pub span: SourceSpan,
    pub replacement: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RenameResult {
    pub selection: RenameSelection,
    pub edits: Vec<RenameEdit>,
}

struct RenamePlan {
    symbol: EditorSymbol,
    occurrences: Vec<(String, SourceSpan)>,
}

fn build_rename_plan(
    sources: &[ProjectSource],
    path: &str,
    offset: usize,
) -> Result<RenamePlan, String> {
    let analysis = analyze_project(sources);
    if project_has_errors(&analysis) {
        return Err(
            "Resolve project errors before renaming so all references can be identified safely."
                .into(),
        );
    }

    let source = sources
        .iter()
        .find(|source| source.path == path)
        .ok_or_else(|| "The source file is unavailable.".to_owned())?;
    if offset > source.source.len() || !source.source.is_char_boundary(offset) {
        return Err("The cursor is outside a valid source position.".into());
    }

    let file = analysis
        .files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| "The source file is unavailable.".to_owned())?;
    let symbol = file
        .editor_symbols
        .iter()
        .filter(|symbol| is_renameable(symbol.kind))
        .filter(|symbol| symbol.span.start.offset <= offset && offset <= symbol.span.end.offset)
        .min_by_key(|symbol| symbol.span.end.offset - symbol.span.start.offset)
        .cloned()
        .ok_or_else(|| "No renameable symbol is at this position.".to_owned())?;

    let definitions = analysis
        .files
        .iter()
        .flat_map(|file| {
            file.editor_symbols.iter().filter_map(|candidate| {
                (candidate.role == EditorSymbolRole::Definition
                    && candidate.kind == symbol.kind
                    && candidate.target_key == symbol.target_key)
                    .then_some((file.path.as_str(), candidate))
            })
        })
        .collect::<Vec<_>>();
    if definitions.len() != 1 {
        return Err("The symbol does not have one unambiguous definition.".into());
    }

    let mut occurrences = Vec::new();
    let mut seen = HashSet::new();
    for file in &analysis.files {
        for candidate in &file.editor_symbols {
            if candidate.kind != symbol.kind || candidate.target_key != symbol.target_key {
                continue;
            }
            let source = sources
                .iter()
                .find(|source| source.path == file.path)
                .ok_or_else(|| "A referenced project source is unavailable.".to_owned())?;
            if candidate.name != symbol.name
                || source
                    .source
                    .get(candidate.span.start.offset..candidate.span.end.offset)
                    != Some(symbol.name.as_str())
            {
                return Err("Generated or overlapping references cannot be renamed safely.".into());
            }
            if seen.insert((
                file.path.clone(),
                candidate.span.start.offset,
                candidate.span.end.offset,
            )) {
                occurrences.push((file.path.clone(), candidate.span));
            }
        }
    }

    if !occurrences
        .iter()
        .any(|(candidate_path, span)| *candidate_path == path && *span == definitions[0].1.span)
    {
        return Err("The definition is not an editable source identifier.".into());
    }

    Ok(RenamePlan {
        symbol,
        occurrences,
    })
}

/// Prepares a rename at a UTF-8 byte offset and returns the selected symbol range.
pub fn prepare_project_rename(
    sources: &[ProjectSource],
    path: &str,
    offset: usize,
) -> Result<RenameSelection, String> {
    let plan = build_rename_plan(sources, path, offset)?;
    Ok(RenameSelection {
        name: plan.symbol.name,
        kind: plan.symbol.kind,
        span: plan.symbol.span,
    })
}

/// Computes and compile-checks edits without modifying the project sources.
pub fn rename_project(
    sources: &[ProjectSource],
    path: &str,
    offset: usize,
    new_name: &str,
) -> Result<RenameResult, String> {
    if !valid_identifier(new_name) {
        return Err("Use an identifier that starts with an ASCII letter or underscore and contains only ASCII letters, digits, and underscores.".into());
    }
    let plan = build_rename_plan(sources, path, offset)?;
    let selection = RenameSelection {
        name: plan.symbol.name.clone(),
        kind: plan.symbol.kind,
        span: plan.symbol.span,
    };
    let mut updated = sources.to_vec();
    let mut edits = Vec::with_capacity(plan.occurrences.len());

    for source in &mut updated {
        let mut occurrences = plan
            .occurrences
            .iter()
            .filter(|(candidate_path, _)| candidate_path == &source.path)
            .map(|(_, span)| *span)
            .collect::<Vec<_>>();
        occurrences.sort_by_key(|span| std::cmp::Reverse(span.start.offset));
        for span in occurrences {
            let old_name = source
                .source
                .get(span.start.offset..span.end.offset)
                .ok_or_else(|| "A rename occurrence no longer matches source text.".to_owned())?;
            if old_name != plan.symbol.name {
                return Err("A rename occurrence no longer matches the selected symbol.".into());
            }
            source
                .source
                .replace_range(span.start.offset..span.end.offset, new_name);
            edits.push(RenameEdit {
                path: source.path.clone(),
                span,
                replacement: new_name.to_owned(),
            });
        }
    }

    if project_has_errors(&analyze_project(&updated)) {
        return Err(
            "The new name conflicts with existing declarations or makes a reference invalid."
                .into(),
        );
    }
    edits.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.span.start.offset.cmp(&right.span.start.offset))
    });
    Ok(RenameResult { selection, edits })
}

fn is_renameable(kind: EditorSymbolKind) -> bool {
    matches!(
        kind,
        EditorSymbolKind::Visual | EditorSymbolKind::Node | EditorSymbolKind::Scenario
    )
}

fn valid_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn project_has_errors(analysis: &ProjectAnalysis) -> bool {
    analysis.files.iter().any(|file| {
        file.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }) || analysis.visuals.iter().any(|visual_analysis| {
        let Some(file) = analysis.files.get(visual_analysis.source_index) else {
            return true;
        };
        let Some(visual) = file.visual.as_ref() else {
            return true;
        };
        validate_scenarios_for_visual(&visual_analysis.scenarios, visual)
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    })
}
