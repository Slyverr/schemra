use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use schemra_project::{
    ProjectSourceError, ProjectSourceLimits, read_sources, read_sources_with_limits,
    resolve_entry_path,
};

struct ProjectDirectory(PathBuf);

impl ProjectDirectory {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "schemra-project-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create temporary project directory");
        Self(path)
    }

    fn write(&self, relative: impl AsRef<Path>, source: &str) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("source path has a parent"))
            .expect("create source parent directory");
        fs::write(&path, source).expect("write project source");
        path
    }
}

impl Drop for ProjectDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn discovers_sources_in_stable_order_and_skips_generated_directories() {
    let project = ProjectDirectory::new();
    project.write("zeta.schemra", "z");
    project.write("nested/Alpha.SCHEMRA", "a");
    project.write("target/generated.schemra", "ignored");
    project.write("node_modules/dep.schemra", "ignored");
    project.write("notes.txt", "ignored");

    let sources = read_sources(&project.0).expect("read project sources");

    assert_eq!(
        sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<Vec<_>>(),
        ["nested/Alpha.SCHEMRA", "zeta.schemra"]
    );
    assert!(sources.iter().all(|source| source.id == source.path));
}

#[test]
fn resolves_only_existing_schemra_files_inside_the_project() {
    let project = ProjectDirectory::new();
    let source = project.write("visuals/traffic.schemra", "visual traffic \"Traffic\" {}");
    let outside = ProjectDirectory::new();
    let outside_source = outside.write("external.schemra", "");
    let notes = project.write("notes.txt", "text");

    assert_eq!(
        resolve_entry_path(&source, &project.0).expect("resolve project source"),
        "visuals/traffic.schemra"
    );
    assert!(matches!(
        resolve_entry_path(&outside_source, &project.0),
        Err(ProjectSourceError::PathOutsideRoot { .. })
    ));
    assert!(matches!(
        resolve_entry_path(&notes, &project.0),
        Err(ProjectSourceError::UnsupportedSourceType(_))
    ));
}

#[test]
fn applies_file_size_count_and_directory_limits() {
    let project = ProjectDirectory::new();
    project.write("one.schemra", "123");
    project.write("nested/two.schemra", "45");

    let limits = ProjectSourceLimits {
        max_source_bytes: 2,
        ..ProjectSourceLimits::default()
    };
    assert!(matches!(
        read_sources_with_limits(&project.0, limits),
        Err(ProjectSourceError::SourceTooLarge { .. })
    ));

    let limits = ProjectSourceLimits {
        max_files: 1,
        ..ProjectSourceLimits::default()
    };
    assert!(matches!(
        read_sources_with_limits(&project.0, limits),
        Err(ProjectSourceError::TooManyFiles { max_files: 1 })
    ));

    let limits = ProjectSourceLimits {
        max_total_source_bytes: 4,
        ..ProjectSourceLimits::default()
    };
    assert!(matches!(
        read_sources_with_limits(&project.0, limits),
        Err(ProjectSourceError::ProjectSourcesTooLarge { max_bytes: 4 })
    ));

    let limits = ProjectSourceLimits {
        max_depth: 0,
        ..ProjectSourceLimits::default()
    };
    assert!(matches!(
        read_sources_with_limits(&project.0, limits),
        Err(ProjectSourceError::DirectoryTooDeep { max_depth: 0 })
    ));

    let limits = ProjectSourceLimits {
        max_directory_entries: 1,
        ..ProjectSourceLimits::default()
    };
    assert!(matches!(
        read_sources_with_limits(&project.0, limits),
        Err(ProjectSourceError::TooManyDirectoryEntries { max_entries: 1, .. })
    ));

    let limits = ProjectSourceLimits {
        max_path_bytes: 8,
        ..ProjectSourceLimits::default()
    };
    assert!(matches!(
        read_sources_with_limits(&project.0, limits),
        Err(ProjectSourceError::SourcePathTooLong { max_bytes: 8 })
    ));
}
