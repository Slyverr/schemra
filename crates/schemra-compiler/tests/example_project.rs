use std::{fs, path::PathBuf};

use schemra_compiler::{ProjectSource, analyze_project};
use schemra_ir::{
    ActionIr, Direction, EdgeDirectionality, GroupLayout, HighlightEffect, HighlightOrigin,
    HighlightSide, NodeLayout, NodeTextAlign,
};

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

fn visit_actions(actions: &[ActionIr], visit: &mut impl FnMut(&ActionIr)) {
    for action in actions {
        visit(action);
        match action {
            ActionIr::Sequence { actions, .. } | ActionIr::Parallel { actions, .. } => {
                visit_actions(actions, visit);
            }
            ActionIr::Repeat { body, .. } => visit_actions(body, visit),
            ActionIr::Move { .. } | ActionIr::Wait { .. } | ActionIr::Highlight { .. } => {}
        }
    }
}

#[test]
fn example_project_keeps_one_visual_and_links_all_scenarios() {
    let sources = example_sources();
    let analysis = analyze_project(&sources);

    assert_eq!(analysis.visuals.len(), 1, "keep one canonical visual");
    let visual = &analysis.visuals[0];
    assert_eq!(visual.id, "traffic_overview");
    assert_eq!(visual.scenarios.len(), 6);
    assert!(
        analysis
            .files
            .iter()
            .all(|file| file.diagnostics.is_empty()),
        "example project diagnostics: {:?}",
        analysis
            .files
            .iter()
            .flat_map(|file| &file.diagnostics)
            .collect::<Vec<_>>()
    );

    let visual_ir = analysis.files[visual.source_index]
        .visual
        .as_ref()
        .expect("the canonical source declares a visual");
    assert_eq!(visual_ir.direction, Direction::LeftToRight);
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
    for index in 1..=4 {
        let user = format!("user_{index}");
        assert!(visual_ir.nodes.iter().any(|node| node.id.0 == user));
        assert!(visual_ir.edges.iter().any(|edge| {
            edge.directionality == EdgeDirectionality::Undirected
                && edge.from.0 == user
                && edge.to.0 == "reverse_proxy"
        }));
    }

    let scenario_ids = visual
        .scenarios
        .iter()
        .map(|scenario| scenario.id.0.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        scenario_ids,
        [
            "data_lifecycle",
            "ddos_test",
            "everything",
            "marker_styles",
            "request_response",
            "service_flow"
        ]
    );

    let ddos_test = visual
        .scenarios
        .iter()
        .find(|scenario| scenario.id.0 == "ddos_test")
        .expect("the sustained request example is linked to the canonical visual");
    let [ActionIr::Repeat { times, body, .. }] = ddos_test.actions.as_slice() else {
        panic!("the attack example should repeat its request wave");
    };
    assert_eq!(*times, 20);
    let [ActionIr::Parallel { actions, .. }] = body.as_slice() else {
        panic!("each request wave should run in parallel");
    };
    assert_eq!(actions.len(), 4);
    for (user, action) in ["user_1", "user_2", "user_3", "user_4"]
        .into_iter()
        .zip(actions)
    {
        assert!(matches!(
            action,
            ActionIr::Move {
                from,
                to,
                duration_ms: Some(500),
                ..
            } if from.0 == user && to.0 == "reverse_proxy"
        ));
    }
}

#[test]
fn feature_tour_visual_demonstrates_group_layouts_and_node_metadata() {
    let analysis = analyze_project(&example_sources());
    let visual = analysis.files[analysis.visuals[0].source_index]
        .visual
        .as_ref()
        .expect("the canonical source declares a visual");

    assert!(
        visual
            .groups
            .iter()
            .any(|group| { group.layout == GroupLayout::Grid { columns: Some(2) } })
    );
    assert!(
        visual
            .groups
            .iter()
            .any(|group| group.layout == GroupLayout::Auto)
    );
    assert!(visual.nodes.iter().any(|node| {
        node.icon.as_deref() == Some("database")
            && node.detail.as_deref() == Some("Stores application data")
            && node.layout == NodeLayout::Horizontal
            && node.text_align == NodeTextAlign::Left
    }));
    assert!(visual.nodes.iter().any(|node| {
        node.id.0 == "audit_worker"
            && node.layout == NodeLayout::Vertical
            && node.text_align == NodeTextAlign::Center
    }));
}

#[test]
fn feature_tour_scenario_demonstrates_all_action_and_marker_forms() {
    let analysis = analyze_project(&example_sources());
    let scenario = analysis.visuals[0]
        .scenarios
        .iter()
        .find(|scenario| scenario.id.0 == "everything")
        .expect("the language feature tour is linked to the canonical visual");

    assert!(
        scenario.tokens.iter().any(|token| {
            token.id.0 == "request" && token.icon.as_deref() == Some("lucide/mail")
        })
    );
    assert!(
        scenario.tokens.iter().any(|token| {
            token.id.0 == "acknowledgment" && token.char.as_deref() == Some("✓")
        })
    );
    assert!(scenario.tokens.iter().any(|token| {
        token.id.0 == "response" && token.icon.as_deref() == Some("lucide/package")
    }));

    let mut has_parallel_moves = false;
    let mut has_wait = false;
    let mut has_repeat = false;
    let mut has_default_arrival_trace = false;
    let mut has_move_override_pulse = false;
    let mut has_independent_highlight = false;
    let mut has_directional_trace = false;
    let mut has_default_duration_move = false;
    visit_actions(&scenario.actions, &mut |action| match action {
        ActionIr::Parallel { actions, .. } if actions.len() == 3 => has_parallel_moves = true,
        ActionIr::Wait {
            duration_ms: 150, ..
        } => has_wait = true,
        ActionIr::Repeat { times: 2, .. } => has_repeat = true,
        ActionIr::Highlight {
            duration_ms: Some(200),
            effect: HighlightEffect::Trace,
            origin: HighlightOrigin::Arrival { .. },
            ..
        } => has_default_arrival_trace = true,
        ActionIr::Highlight {
            duration_ms: Some(300),
            effect: HighlightEffect::Pulse,
            origin: HighlightOrigin::Arrival { from, to },
            ..
        } if from.0 == "user_1" && to.0 == "reverse_proxy" => {
            has_move_override_pulse = true;
        }
        ActionIr::Highlight {
            duration_ms: Some(300),
            effect: HighlightEffect::Pulse,
            origin: HighlightOrigin::Default,
            node,
            ..
        } if node.0 == "database" => has_independent_highlight = true,
        ActionIr::Highlight {
            duration_ms: Some(350),
            effect: HighlightEffect::Trace,
            origin: HighlightOrigin::Side {
                side: HighlightSide::Left,
            },
            node,
            ..
        } if node.0 == "reverse_proxy" => has_directional_trace = true,
        ActionIr::Move {
            token,
            from,
            to,
            duration_ms: None,
            ..
        } if token.0 == "acknowledgment" && from.0 == "reverse_proxy" && to.0 == "user_3" => {
            has_default_duration_move = true;
        }
        ActionIr::Parallel { .. }
        | ActionIr::Sequence { .. }
        | ActionIr::Move { .. }
        | ActionIr::Wait { .. }
        | ActionIr::Highlight { .. }
        | ActionIr::Repeat { .. } => {}
    });

    assert!(has_parallel_moves);
    assert!(has_wait);
    assert!(has_repeat);
    assert!(has_default_arrival_trace);
    assert!(has_move_override_pulse);
    assert!(has_independent_highlight);
    assert!(has_directional_trace);
    assert!(has_default_duration_move);

    let ActionIr::Parallel { actions, .. } = &scenario.actions[0] else {
        panic!("the feature tour begins with parallel moves");
    };
    assert!(matches!(
        actions.get(1),
        Some(ActionIr::Move {
            from,
            to,
            duration_ms: Some(600),
            ..
        }) if from.0 == "user_2" && to.0 == "reverse_proxy"
    ));
}
