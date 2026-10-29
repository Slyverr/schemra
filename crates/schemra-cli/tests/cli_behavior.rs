use std::process::{Command, Output};

fn run_cli(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_schemra"))
        .args(arguments)
        .output()
        .expect("the Schemra CLI should start")
}

#[test]
fn no_arguments_print_help_and_require_a_subcommand() {
    let output = run_cli(&[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: schemra"));
}

#[test]
fn help_option_describes_the_cli() {
    let output = run_cli(&["--help"]);
    let help = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(help.contains("Local authoring environment"));
    assert!(help.contains("--version"));
    assert!(help.contains("check"));
    assert!(help.contains("fmt"));
    assert!(help.contains("scenario-check"));
    assert!(help.contains("render"));
}

#[test]
fn version_option_reports_the_package_version() {
    let output = run_cli(&["--version"]);
    let version = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(version.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_option_fails_with_a_usage_hint() {
    let output = run_cli(&["--not-a-real-option"]);
    let error = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(error.contains("--not-a-real-option"));
    assert!(error.contains("Usage: schemra"));
}
