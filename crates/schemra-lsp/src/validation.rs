use schemra_compiler::{ProjectAnalysis, validate_scenario_for_visual};
use schemra_syntax::Diagnostic;

/// Adds engine checks to compiler diagnostics for one source file.
pub(crate) fn diagnostics_for_file(analysis: &ProjectAnalysis, path: &str) -> Vec<Diagnostic> {
    let Some(file) = analysis.files.iter().find(|file| file.path == path) else {
        return Vec::new();
    };
    let mut diagnostics = file.diagnostics.clone();

    if let Some(visual) = &file.visual {
        if let Err(error) =
            schemra_engine::check_visual_limits(visual, schemra_engine::VisualLimits::default())
        {
            push_unique(
                &mut diagnostics,
                Diagnostic::error(
                    error.code,
                    error.message,
                    error.span,
                    Some("Reduce the visual elements or links that exceed the configured limit."),
                ),
            );
        }
    }

    if let Some(visual_id) = &file.resolved_visual_id {
        let visual = analysis
            .visuals
            .iter()
            .find(|visual| visual.id == *visual_id)
            .and_then(|visual| analysis.files.get(visual.source_index))
            .and_then(|visual_file| visual_file.visual.as_ref());

        if let Some(visual) = visual {
            for scenario in &file.resolved_scenarios {
                for diagnostic in validate_scenario_for_visual(scenario, visual) {
                    push_unique(&mut diagnostics, diagnostic);
                }
                if let Err(error) = schemra_engine::compile_timeline(scenario) {
                    push_unique(
                        &mut diagnostics,
                        Diagnostic::error(error.code, error.message, error.span, None::<String>),
                    );
                }
            }
        }
    }

    diagnostics.sort_by(|left, right| {
        (
            left.span.start.offset,
            left.span.end.offset,
            left.code.as_str(),
        )
            .cmp(&(
                right.span.start.offset,
                right.span.end.offset,
                right.code.as_str(),
            ))
    });
    diagnostics
}

fn push_unique(diagnostics: &mut Vec<Diagnostic>, candidate: Diagnostic) {
    let already_present = diagnostics.iter().any(|diagnostic| {
        diagnostic.code == candidate.code
            && diagnostic.span.start.offset == candidate.span.start.offset
            && diagnostic.span.end.offset == candidate.span.end.offset
    });
    if !already_present {
        diagnostics.push(candidate);
    }
}
