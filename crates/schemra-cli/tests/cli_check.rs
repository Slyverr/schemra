use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_PROJECT_ID: AtomicU64 = AtomicU64::new(0);

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "schemra-cli-check-{}-{}",
            std::process::id(),
            NEXT_PROJECT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create temporary project");
        Self(root)
    }

    fn write(&self, relative_path: &str, source: &str) -> PathBuf {
        let path = self.0.join(relative_path);
        fs::create_dir_all(path.parent().expect("source has a parent"))
            .expect("create source directory");
        fs::write(&path, source).expect("write project source");
        path
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_check(root: &Path, input: &Path, options: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_schemra"));
    command.arg("check").arg(input).arg("--root").arg(root);
    command.args(options);
    command.output().expect("the Schemra CLI should start")
}

fn visual_source() -> &'static str {
    "visual network {\n  node client {}\n  node server {}\n  client -> server\n}\n"
}

fn scenario_source() -> &'static str {
    "scenario requests for visual network {\n  token packet { icon: \"mail\" }\n  move packet from client -> server { duration: 100ms }\n}\n"
}

#[test]
fn check_accepts_a_visual_and_linked_scenario_project() {
    let project = TempProject::new();
    project.write("visuals/network.schemra", visual_source());
    let scenario = project.write("scenarios/requests.schemra", scenario_source());

    let output = run_check(project.root(), &scenario, &[]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Valid Schemra scenario"));
}

#[test]
fn check_reports_semantic_errors_with_a_failing_exit_status() {
    let project = TempProject::new();
    project.write("visuals/network.schemra", visual_source());
    let scenario = project.write(
        "scenarios/requests.schemra",
        "scenario requests for visual network {\n  token packet {}\n  move packet from client -> nowhere { duration: 100ms }\n}\n",
    );

    let output = run_check(project.root(), &scenario, &[]);
    let diagnostics = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(diagnostics.contains("E_UNKNOWN_MOVE_NODE"), "{diagnostics}");
}

#[test]
fn check_rejects_a_source_that_does_not_define_a_visual_or_scenario() {
    let project = TempProject::new();
    let source = project.write("notes/empty.schemra", "\n");

    let output = run_check(project.root(), &source, &[]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not contain a visual"));
}

#[test]
fn check_json_contains_machine_readable_diagnostics() {
    let project = TempProject::new();
    let visual = project.write(
        "visuals/network.schemra",
        "visual network {\n node client {}\n",
    );

    let output = run_check(project.root(), &visual, &["--json"]);
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("check output should be JSON");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report["ok"], false);
    assert_eq!(report["path"], "visuals/network.schemra");
    assert!(!report["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn check_rejects_source_files_outside_the_project_root() {
    let project = TempProject::new();
    let outside = TempProject::new();
    let source = outside.write("network.schemra", visual_source());

    let output = run_check(project.root(), &source, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside project root"));
}

#[test]
fn check_ignores_generated_source_files_and_orders_project_discovery() {
    let project = TempProject::new();
    let selected = project.write("visuals/network.schemra", visual_source());
    project.write("target/generated.schemra", "not valid Schemra");
    project.write(".schemra/cache/generated.schemra", "not valid Schemra");

    let output = run_check(project.root(), &selected, &[]);

    assert!(output.status.success());
}

#[test]
fn scenario_check_is_limited_to_the_selected_file_but_visual_check_checks_all_scenarios() {
    let project = TempProject::new();
    let visual = project.write("visuals/network.schemra", visual_source());
    let valid_scenario = project.write("scenarios/valid.schemra", scenario_source());
    project.write(
        "scenarios/broken.schemra",
        "scenario broken for visual network {\n  token packet {}\n  move packet from client -> nowhere { duration: 100ms }\n}\n",
    );

    let selected_scenario = run_check(project.root(), &valid_scenario, &[]);
    let selected_visual = run_check(project.root(), &visual, &[]);

    assert!(selected_scenario.status.success());
    assert_eq!(selected_visual.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&selected_visual.stderr).contains("E_UNKNOWN_MOVE_NODE"));
}

#[test]
fn strict_check_flag_is_accepted_for_valid_and_invalid_sources() {
    let project = TempProject::new();
    let visual = project.write("visuals/network.schemra", visual_source());

    let strict_valid = run_check(project.root(), &visual, &["--strict"]);
    assert!(strict_valid.status.success());

    let invalid = project.write(
        "visuals/invalid.schemra",
        "visual network {\n  node client {}\n  client -> nowhere\n}\n",
    );
    let strict_invalid = run_check(project.root(), &invalid, &["--strict", "--json"]);
    let report: serde_json::Value = serde_json::from_slice(&strict_invalid.stdout)
        .expect("strict check output should remain JSON");
    assert_eq!(strict_invalid.status.code(), Some(1));
    assert_eq!(report["ok"], false);
}
