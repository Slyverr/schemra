use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_PROJECT_ID: AtomicU64 = AtomicU64::new(0);

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "schemra-cli-render-{}-{}",
            std::process::id(),
            NEXT_PROJECT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create temporary project");
        Self(root)
    }

    fn write(&self, relative: &str, source: &str) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("test source has a parent"))
            .expect("create source directory");
        fs::write(&path, source).expect("write test source");
        path
    }

    fn root(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_schemra"))
        .args(arguments)
        .output()
        .expect("the Schemra CLI should start")
}

fn visual_source(id: &str) -> String {
    format!(
        "visual {id} \"Network\" {{\n  node client \"Client\" {{}}\n  node server \"Server\" {{}}\n  client -> server\n}}\n"
    )
}

fn scenario_source(visual: &str) -> String {
    format!(
        "scenario requests for visual {visual} {{\n  token packet {{ icon: \"mail\" }}\n  move packet from client -> server {{ duration: 100ms }}\n}}\n"
    )
}

fn write_visual_project() -> (TempProject, PathBuf) {
    let project = TempProject::new();
    let visual = project.write("visuals/network.schemra", &visual_source("network"));
    (project, visual)
}

#[test]
fn render_writes_svg_for_a_valid_visual_project() {
    let (project, visual) = write_visual_project();
    project.write("scenarios/requests.schemra", &scenario_source("network"));
    let output_path = project.root().join("exports/network.svg");
    fs::create_dir_all(output_path.parent().expect("output parent"))
        .expect("create output directory");

    let output = run(&[
        "render",
        visual.to_str().expect("UTF-8 visual path"),
        "--out",
        output_path.to_str().expect("UTF-8 output path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = fs::read_to_string(output_path).expect("rendered SVG exists");
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("Client"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Rendered"));
}

#[test]
fn render_rejects_scenario_input_without_touching_existing_output() {
    let (project, _) = write_visual_project();
    let scenario = project.write("scenarios/requests.schemra", &scenario_source("network"));
    let output_path = project.root().join("existing.svg");
    fs::write(&output_path, "keep this output").expect("prepare existing output");

    let output = run(&[
        "render",
        scenario.to_str().expect("UTF-8 scenario path"),
        "--out",
        output_path.to_str().expect("UTF-8 output path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not produce a visual"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Valid Schemra"));
    assert_eq!(
        fs::read_to_string(output_path).expect("output remains readable"),
        "keep this output"
    );
}

#[test]
fn render_rejects_invalid_linked_scenarios_before_writing() {
    let (project, visual) = write_visual_project();
    project.write(
        "scenarios/broken.schemra",
        "scenario broken for visual network {\n  token packet {}\n  move packet from client -> missing { duration: 100ms }\n}\n",
    );
    let output_path = project.root().join("existing.svg");
    fs::write(&output_path, "keep this output").expect("prepare existing output");

    let output = run(&[
        "render",
        visual.to_str().expect("UTF-8 visual path"),
        "--out",
        output_path.to_str().expect("UTF-8 output path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("E_UNKNOWN_MOVE_NODE"));
    assert_eq!(
        fs::read_to_string(output_path).expect("output remains readable"),
        "keep this output"
    );
}

#[test]
fn render_reports_output_write_failures_with_usage_status() {
    let (project, visual) = write_visual_project();
    let output_path = project.root().join("missing/export.svg");

    let output = run(&[
        "render",
        visual.to_str().expect("UTF-8 visual path"),
        "--out",
        output_path.to_str().expect("UTF-8 output path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("could not write"));
}

#[test]
fn scenario_check_accepts_matching_scenario_and_visual_files() {
    let (project, visual) = write_visual_project();
    let scenario = project.write("scenarios/requests.schemra", &scenario_source("network"));
    project.write(
        "scenarios/unrelated-broken.schemra",
        "scenario broken for visual network {\n  token packet {}\n  move packet from client -> missing { duration: 100ms }\n}\n",
    );

    let output = run(&[
        "scenario-check",
        scenario.to_str().expect("UTF-8 scenario path"),
        "--visual",
        visual.to_str().expect("UTF-8 visual path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Valid Schemra scenario"));
}

#[test]
fn scenario_check_counts_all_scenarios_in_the_selected_source() {
    let (project, visual) = write_visual_project();
    let first = scenario_source("network");
    let second = first.replace("scenario requests", "scenario responses");
    let scenario = project.write("scenarios/combined.schemra", &format!("{first}{second}"));

    let output = run(&[
        "scenario-check",
        scenario.to_str().expect("UTF-8 scenario path"),
        "--visual",
        visual.to_str().expect("UTF-8 visual path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
        "--json",
    ]);
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("scenario output should be JSON");

    assert!(output.status.success());
    assert_eq!(report["scenario_count"], 2);
}

#[test]
fn scenario_check_reports_machine_readable_selection_and_diagnostics() {
    let (project, _) = write_visual_project();
    let other = project.write("visuals/other.schemra", &visual_source("other"));
    let scenario = project.write("scenarios/requests.schemra", &scenario_source("network"));

    let output = run(&[
        "scenario-check",
        scenario.to_str().expect("UTF-8 scenario path"),
        "--visual",
        other.to_str().expect("UTF-8 visual path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
        "--json",
    ]);
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("selection output should be JSON");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report["ok"], false);
    assert_eq!(report["scenario_visual_id"], "network");
    assert_eq!(report["selected_visual_id"], "other");
    assert!(
        report["selection_error"]
            .as_str()
            .unwrap()
            .contains("targets visual")
    );
}

#[test]
fn scenario_check_rejects_the_wrong_source_kinds() {
    let (project, visual) = write_visual_project();
    let scenario = project.write("scenarios/requests.schemra", &scenario_source("network"));

    let visual_as_scenario = run(&[
        "scenario-check",
        visual.to_str().expect("UTF-8 visual path"),
        "--visual",
        visual.to_str().expect("UTF-8 visual path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
    ]);
    let scenario_as_visual = run(&[
        "scenario-check",
        scenario.to_str().expect("UTF-8 scenario path"),
        "--visual",
        scenario.to_str().expect("UTF-8 scenario path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
    ]);

    assert_eq!(visual_as_scenario.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&visual_as_scenario.stderr).contains("expects a scenario"));
    assert_eq!(scenario_as_visual.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&scenario_as_visual.stderr).contains("must point to a visual"));
}

#[test]
fn scenario_check_reports_scenario_semantic_errors() {
    let (project, visual) = write_visual_project();
    let scenario = project.write(
        "scenarios/broken.schemra",
        "scenario broken for visual network {\n  token packet {}\n  move packet from client -> missing { duration: 100ms }\n}\n",
    );

    let output = run(&[
        "scenario-check",
        scenario.to_str().expect("UTF-8 scenario path"),
        "--visual",
        visual.to_str().expect("UTF-8 visual path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
        "--json",
    ]);
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("semantic diagnostics should be JSON");

    assert_eq!(output.status.code(), Some(1));
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E_UNKNOWN_MOVE_NODE")
    );
}

#[test]
fn scenario_check_rejects_sources_outside_the_project_root() {
    let (project, _) = write_visual_project();
    let other_project = TempProject::new();
    let scenario = other_project.write("scenario.schemra", &scenario_source("network"));
    let visual = project.root().join("visuals/network.schemra");

    let output = run(&[
        "scenario-check",
        scenario.to_str().expect("UTF-8 scenario path"),
        "--visual",
        visual.to_str().expect("UTF-8 visual path"),
        "--root",
        project.root().to_str().expect("UTF-8 project path"),
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside project root"));
}
