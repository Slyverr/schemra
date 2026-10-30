use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_OUTPUT_ID: AtomicU64 = AtomicU64::new(0);

struct RenderedSvg(PathBuf);

impl RenderedSvg {
    fn for_example() -> Self {
        let output = std::env::temp_dir().join(format!(
            "schemra-example-render-{}-{}.svg",
            std::process::id(),
            NEXT_OUTPUT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest.join("../../examples/traffic-overview");
        let visual = project_root.join("visuals/traffic-overview.schemra");
        let result = Command::new(env!("CARGO_BIN_EXE_schemra"))
            .args([
                "render",
                visual.to_str().expect("example path is UTF-8"),
                "--out",
                output.to_str().expect("output path is UTF-8"),
                "--root",
                project_root.to_str().expect("project path is UTF-8"),
            ])
            .output()
            .expect("Schemra CLI starts");

        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Self(output)
    }

    fn read(&self) -> String {
        fs::read_to_string(&self.0).expect("CLI writes the example SVG")
    }
}

impl Drop for RenderedSvg {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn path_attribute<'a>(element: &'a str, name: &str) -> &'a str {
    let prefix = format!("{name}=\"");
    let value = element
        .split_once(&prefix)
        .expect("SVG path has the requested attribute")
        .1;
    value.split_once('"').expect("SVG attribute is closed").0
}

#[test]
fn database_connections_render_as_distinct_single_paths() {
    let svg = RenderedSvg::for_example().read();
    let paths = svg
        .split("<path class=\"schemra-edge-path\" ")
        .collect::<Vec<_>>();
    let audit_paths = paths
        .iter()
        .copied()
        .filter(|path| path.contains("data-from=\"audit_worker\" data-to=\"database\""))
        .collect::<Vec<_>>();
    let backend_paths = paths
        .iter()
        .copied()
        .filter(|path| path.contains("data-from=\"backend\" data-to=\"database\""))
        .collect::<Vec<_>>();

    assert_eq!(audit_paths.len(), 1);
    assert_eq!(backend_paths.len(), 1);
    assert_ne!(
        path_attribute(audit_paths[0], "d"),
        path_attribute(backend_paths[0], "d"),
        "separate links should not collapse into one visual route"
    );

    assert!(audit_paths[0].contains("data-arrow-start=\"false\" data-arrow-end=\"true\""));
    assert_eq!(
        path_attribute(audit_paths[0], "d").matches(" M ").count(),
        1
    );
    assert!(
        !svg.contains("class=\"schemra-edge-direction-cue\""),
        "the direction cue is part of its one SVG route element"
    );
}
