use std::{fs, path::PathBuf};

use schemra_compiler::{ProjectSource, analyze_project};
use schemra_ir::EdgeDirectionality;

fn example_sources() -> Vec<ProjectSource> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/traffic-overview");
    let mut sources = Vec::new();

    for directory in ["visuals", "scenarios"] {
        let directory_path = root.join(directory);
        let mut paths = fs::read_dir(&directory_path)
            .expect("example source directory is available")
            .map(|entry| entry.expect("example source entry is available").path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "schemra")
            })
            .collect::<Vec<_>>();
        paths.sort();

        for path in paths {
            let relative_path = path
                .strip_prefix(&root)
                .expect("example source is inside its project")
                .to_string_lossy()
                .replace('\\', "/");
            sources.push(ProjectSource {
                id: relative_path.clone(),
                path: relative_path,
                source: fs::read_to_string(&path).expect("example source is readable"),
            });
        }
    }

    sources
}

#[test]
fn example_project_keeps_one_visual_and_links_all_scenarios() {
    let sources = example_sources();
    let analysis = analyze_project(&sources);

    assert_eq!(analysis.visuals.len(), 1, "keep one canonical visual");
    let visual = &analysis.visuals[0];
    assert_eq!(visual.id, "traffic_overview");
    assert_eq!(visual.scenarios.len(), 5);
    assert!(
        analysis
            .files
            .iter()
            .all(|file| file.diagnostics.is_empty())
    );

    let visual_ir = analysis.files[visual.source_index]
        .visual
        .as_ref()
        .expect("the canonical source declares a visual");
    assert!(
        visual_ir
            .edges
            .iter()
            .any(|edge| { edge.directionality == EdgeDirectionality::Directed })
    );
    assert!(
        visual_ir
            .edges
            .iter()
            .any(|edge| { edge.directionality == EdgeDirectionality::Undirected })
    );

    let scenario_ids = visual
        .scenarios
        .iter()
        .map(|scenario| scenario.id.0.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        scenario_ids,
        [
            "data_lifecycle",
            "everything",
            "marker_styles",
            "request_response",
            "service_flow"
        ]
    );
}
