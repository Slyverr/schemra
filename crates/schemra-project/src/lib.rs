//! Project source discovery shared by native tools and language integrations.

use std::{
    error::Error,
    ffi::OsString,
    fmt,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde::Deserialize;

const IGNORED_DIRECTORIES: [&str; 6] = [
    ".git",
    ".direnv",
    ".schemra",
    "node_modules",
    "target",
    ".svelte-kit",
];

/// One UTF-8 `.schemra` source file addressed by its stable project-relative path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ProjectSource {
    pub id: String,
    pub path: String,
    pub source: String,
}

/// Resource bounds applied while reading a directory as a Schemra project.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectSourceLimits {
    pub max_files: usize,
    pub max_source_bytes: usize,
    pub max_total_source_bytes: usize,
    pub max_depth: usize,
    pub max_directory_entries: usize,
    pub max_path_bytes: usize,
}

impl Default for ProjectSourceLimits {
    fn default() -> Self {
        Self {
            max_files: 512,
            max_source_bytes: 2 * 1024 * 1024,
            max_total_source_bytes: 16 * 1024 * 1024,
            max_depth: 32,
            max_directory_entries: 8192,
            max_path_bytes: 4096,
        }
    }
}

/// Failure to discover or read a bounded project source tree.
#[derive(Debug)]
pub enum ProjectSourceError {
    CurrentDirectory(io::Error),
    RootIo { path: PathBuf, source: io::Error },
    InputIo { path: PathBuf, source: io::Error },
    RootNotDirectory(PathBuf),
    DirectoryIo { path: PathBuf, source: io::Error },
    EntryIo { path: PathBuf, source: io::Error },
    SourceIo { path: PathBuf, source: io::Error },
    InvalidUtf8Path(PathBuf),
    PathOutsideRoot { input: PathBuf, root: PathBuf },
    UnsupportedSourceType(PathBuf),
    TooManyFiles { max_files: usize },
    SourcePathTooLong { max_bytes: usize },
    SourceTooLarge { path: String, max_bytes: usize },
    ProjectSourcesTooLarge { max_bytes: usize },
    DirectoryTooDeep { max_depth: usize },
    TooManyDirectoryEntries { path: PathBuf, max_entries: usize },
    SizeOverflow,
}

impl fmt::Display for ProjectSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirectory(error) => {
                write!(
                    formatter,
                    "could not determine the current directory: {error}"
                )
            }
            Self::RootIo { path, source } => {
                write!(
                    formatter,
                    "could not open project root {}: {source}",
                    path.display()
                )
            }
            Self::InputIo { path, source } => {
                write!(
                    formatter,
                    "could not read source file {}: {source}",
                    path.display()
                )
            }
            Self::RootNotDirectory(path) => {
                write!(
                    formatter,
                    "project root {} is not a directory",
                    path.display()
                )
            }
            Self::DirectoryIo { path, source } => {
                write!(
                    formatter,
                    "could not read project directory {}: {source}",
                    path.display()
                )
            }
            Self::EntryIo { path, source } => {
                write!(formatter, "could not inspect {}: {source}", path.display())
            }
            Self::SourceIo { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
            Self::InvalidUtf8Path(path) => write!(
                formatter,
                "{} cannot be represented as a project-relative UTF-8 path",
                path.display()
            ),
            Self::PathOutsideRoot { input, root } => write!(
                formatter,
                "{} is outside project root {}",
                input.display(),
                root.display()
            ),
            Self::UnsupportedSourceType(path) => {
                write!(
                    formatter,
                    "{} is not a .schemra source file",
                    path.display()
                )
            }
            Self::TooManyFiles { max_files } => write!(
                formatter,
                "project contains more than the configured {max_files} source files"
            ),
            Self::SourcePathTooLong { max_bytes } => write!(
                formatter,
                "project source path exceeds the configured {max_bytes}-byte limit"
            ),
            Self::SourceTooLarge { path, max_bytes } => write!(
                formatter,
                "source {path} exceeds the configured per-source limit of {max_bytes} bytes"
            ),
            Self::ProjectSourcesTooLarge { max_bytes } => write!(
                formatter,
                "project source text exceeds the configured {max_bytes}-byte aggregate limit"
            ),
            Self::DirectoryTooDeep { max_depth } => write!(
                formatter,
                "project directory nesting exceeds the configured depth limit of {max_depth}"
            ),
            Self::TooManyDirectoryEntries { path, max_entries } => write!(
                formatter,
                "project directory {} exceeds the configured entry limit of {max_entries}",
                path.display()
            ),
            Self::SizeOverflow => {
                formatter.write_str("the total project source size overflowed the supported range")
            }
        }
    }
}

impl Error for ProjectSourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CurrentDirectory(error)
            | Self::RootIo { source: error, .. }
            | Self::InputIo { source: error, .. }
            | Self::DirectoryIo { source: error, .. }
            | Self::EntryIo { source: error, .. }
            | Self::SourceIo { source: error, .. } => Some(error),
            _ => None,
        }
    }
}

/// Read all project `.schemra` sources using the default finite resource bounds.
pub fn read_sources(root: &Path) -> Result<Vec<ProjectSource>, ProjectSourceError> {
    read_sources_with_limits(root, ProjectSourceLimits::default())
}

/// Read project sources under host-selected resource bounds.
pub fn read_sources_with_limits(
    root: &Path,
    limits: ProjectSourceLimits,
) -> Result<Vec<ProjectSource>, ProjectSourceError> {
    let root = canonical_project_root(root)?;
    let mut sources = Vec::new();
    let mut total_bytes = 0usize;
    collect_sources(&root, &root, 0, &mut sources, &mut total_bytes, limits)?;
    sources.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(sources)
}

/// Resolve an input file to its normalized project-relative path.
pub fn resolve_entry_path(input: &Path, root: &Path) -> Result<String, ProjectSourceError> {
    let root = canonical_project_root(root)?;
    let input = if input.is_absolute() {
        input.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(ProjectSourceError::CurrentDirectory)?
            .join(input)
    };
    let input = fs::canonicalize(&input).map_err(|source| ProjectSourceError::InputIo {
        path: input.clone(),
        source,
    })?;
    let relative = input
        .strip_prefix(&root)
        .map_err(|_| ProjectSourceError::PathOutsideRoot {
            input: input.clone(),
            root: root.clone(),
        })?;
    if !relative
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("schemra"))
    {
        return Err(ProjectSourceError::UnsupportedSourceType(input));
    }
    normalized_relative_path(relative).ok_or(ProjectSourceError::InvalidUtf8Path(input))
}

fn canonical_project_root(root: &Path) -> Result<PathBuf, ProjectSourceError> {
    let root = fs::canonicalize(root).map_err(|source| ProjectSourceError::RootIo {
        path: root.to_path_buf(),
        source,
    })?;
    if !root.is_dir() {
        return Err(ProjectSourceError::RootNotDirectory(root));
    }
    Ok(root)
}

fn collect_sources(
    root: &Path,
    directory: &Path,
    depth: usize,
    sources: &mut Vec<ProjectSource>,
    total_bytes: &mut usize,
    limits: ProjectSourceLimits,
) -> Result<(), ProjectSourceError> {
    if depth > limits.max_depth {
        return Err(ProjectSourceError::DirectoryTooDeep {
            max_depth: limits.max_depth,
        });
    }

    let directory_entries =
        fs::read_dir(directory).map_err(|source| ProjectSourceError::DirectoryIo {
            path: directory.to_path_buf(),
            source,
        })?;
    let mut entries = Vec::new();
    for entry in directory_entries {
        if entries.len() >= limits.max_directory_entries {
            return Err(ProjectSourceError::TooManyDirectoryEntries {
                path: directory.to_path_buf(),
                max_entries: limits.max_directory_entries,
            });
        }
        entries.push(entry.map_err(|source| ProjectSourceError::EntryIo {
            path: directory.to_path_buf(),
            source,
        })?);
    }
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| ProjectSourceError::EntryIo {
                path: path.clone(),
                source,
            })?;
        if file_type.is_dir() {
            if is_ignored_directory(&entry.file_name()) {
                continue;
            }
            collect_sources(root, &path, depth + 1, sources, total_bytes, limits)?;
            continue;
        }
        if !file_type.is_file()
            || !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("schemra"))
        {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .expect("project walk stays under the canonical root");
        let relative = normalized_relative_path(relative)
            .ok_or_else(|| ProjectSourceError::InvalidUtf8Path(path.clone()))?;
        if relative.len() > limits.max_path_bytes {
            return Err(ProjectSourceError::SourcePathTooLong {
                max_bytes: limits.max_path_bytes,
            });
        }
        if sources.len() >= limits.max_files {
            return Err(ProjectSourceError::TooManyFiles {
                max_files: limits.max_files,
            });
        }

        let file = File::open(&path).map_err(|source| ProjectSourceError::SourceIo {
            path: path.clone(),
            source,
        })?;
        let mut source = String::new();
        file.take(limits.max_source_bytes.saturating_add(1) as u64)
            .read_to_string(&mut source)
            .map_err(|source| ProjectSourceError::SourceIo {
                path: path.clone(),
                source,
            })?;
        if source.len() > limits.max_source_bytes {
            return Err(ProjectSourceError::SourceTooLarge {
                path: relative,
                max_bytes: limits.max_source_bytes,
            });
        }
        *total_bytes = total_bytes
            .checked_add(source.len())
            .ok_or(ProjectSourceError::SizeOverflow)?;
        if *total_bytes > limits.max_total_source_bytes {
            return Err(ProjectSourceError::ProjectSourcesTooLarge {
                max_bytes: limits.max_total_source_bytes,
            });
        }
        sources.push(ProjectSource {
            id: relative.clone(),
            path: relative,
            source,
        });
    }
    Ok(())
}

fn is_ignored_directory(name: &OsString) -> bool {
    name.to_str()
        .is_some_and(|name| IGNORED_DIRECTORIES.contains(&name))
}

fn normalized_relative_path(path: &Path) -> Option<String> {
    path.to_str().map(|path| path.replace('\\', "/"))
}
