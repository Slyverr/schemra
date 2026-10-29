use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_PROJECT_ID: AtomicU64 = AtomicU64::new(0);

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "schemra-cli-format-{}-{}",
            std::process::id(),
            NEXT_PROJECT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create temporary project");
        Self(root)
    }

    fn source(&self, contents: &str) -> PathBuf {
        let path = self.0.join("visual.schemra");
        fs::write(&path, contents).expect("write source file");
        path
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_schemra"))
}

fn run(arguments: &[&str]) -> Output {
    command()
        .args(arguments)
        .output()
        .expect("the Schemra CLI should start")
}

fn run_with_stdin(arguments: &[&str], source: &[u8]) -> Output {
    let mut child = command()
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the Schemra CLI should start");
    use std::io::Write;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(source)
        .expect("write test source to stdin");
    child.wait_with_output().expect("collect command output")
}

fn unformatted_source() -> &'static str {
    "visual api \"API\" {\nnode gateway \"Gateway\" {}\n}"
}

fn formatted_source() -> &'static str {
    "visual api \"API\" {\n  node gateway \"Gateway\" {}\n}\n"
}

#[test]
fn formatter_prints_formatted_file_to_stdout_by_default() {
    let project = TempProject::new();
    let path = project.source(unformatted_source());

    let output = run(&["fmt", path.to_str().expect("UTF-8 temporary path")]);

    assert!(output.status.success());
    assert_eq!(output.stdout, formatted_source().as_bytes());
}

#[test]
fn formatter_accepts_format_alias_and_checks_without_modifying_source() {
    let project = TempProject::new();
    let path = project.source(unformatted_source());

    let output = run(&[
        "format",
        path.to_str().expect("UTF-8 temporary path"),
        "--check",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("is not formatted"));
    assert_eq!(
        fs::read_to_string(path).expect("source remains readable"),
        unformatted_source()
    );
}

#[test]
fn formatter_check_accepts_formatted_source_without_output() {
    let project = TempProject::new();
    let path = project.source(formatted_source());

    let output = run(&[
        "fmt",
        path.to_str().expect("UTF-8 temporary path"),
        "--check",
    ]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn formatter_write_replaces_the_source_and_reports_the_result() {
    let project = TempProject::new();
    let path = project.source(unformatted_source());

    let output = run(&[
        "fmt",
        path.to_str().expect("UTF-8 temporary path"),
        "--write",
    ]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Formatted"));
    assert_eq!(
        fs::read_to_string(path).expect("source remains readable"),
        formatted_source()
    );
}

#[cfg(unix)]
#[test]
fn formatter_write_preserves_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let project = TempProject::new();
    let path = project.source(unformatted_source());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("set source permissions");

    let output = run(&[
        "fmt",
        path.to_str().expect("UTF-8 temporary path"),
        "--write",
    ]);

    assert!(output.status.success());
    assert_eq!(
        fs::metadata(path)
            .expect("source metadata")
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
}

#[test]
fn formatter_reads_stdin_and_prints_the_shared_format() {
    let output = run_with_stdin(&["fmt", "-"], unformatted_source().as_bytes());

    assert!(output.status.success());
    assert_eq!(output.stdout, formatted_source().as_bytes());
}

#[test]
fn formatter_check_can_validate_stdin() {
    let output = run_with_stdin(&["fmt", "-", "--check"], formatted_source().as_bytes());

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn formatter_rejects_stdin_write_and_conflicting_modes() {
    let stdin_write = run(&["fmt", "-", "--write"]);
    let conflicting = run(&["fmt", "source.schemra", "--write", "--check"]);

    assert_eq!(stdin_write.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&stdin_write.stderr).contains("cannot use --write"));
    assert_eq!(conflicting.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&conflicting.stderr).contains("cannot be used with"));
}

#[test]
fn formatter_rejects_input_larger_than_the_source_limit() {
    let source = vec![b' '; 2 * 1024 * 1024 + 1];

    let output = run_with_stdin(&["fmt", "-"], &source);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("source limit"));
}

#[test]
fn formatter_rejects_non_utf8_source() {
    let output = run_with_stdin(&["fmt", "-"], &[0xff, 0xfe]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("could not read stdin"));
}

#[test]
fn formatted_source_is_idempotent_when_written_twice() {
    let project = TempProject::new();
    let path = project.source(unformatted_source());
    let path_text = path.to_str().expect("UTF-8 temporary path");

    assert!(run(&["fmt", path_text, "--write"]).status.success());
    let second_write = run(&["fmt", path_text, "--write"]);

    assert!(second_write.status.success());
    assert!(String::from_utf8_lossy(&second_write.stdout).contains("Already formatted"));
}

#[test]
fn formatter_rejects_missing_files() {
    let project = TempProject::new();
    let missing = project.0.join("missing.schemra");

    let output = run(&["fmt", missing.to_str().expect("UTF-8 temporary path")]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("could not read"));
}
