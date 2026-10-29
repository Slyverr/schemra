use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Stdout,
    Check,
    Write,
}

pub(super) struct FormatError {
    pub status: u8,
    pub message: String,
}

pub(super) fn run(input: &Path, mode: Mode) -> Result<(), FormatError> {
    if input == Path::new("-") && matches!(mode, Mode::Write) {
        return Err(usage_error(
            "schemra fmt cannot use --write when reading from stdin",
        ));
    }

    let source = read_source(input)?;
    let formatted = schemra_syntax::format_source(&source);
    match mode {
        Mode::Stdout => print!("{formatted}"),
        Mode::Check if formatted != source => {
            return Err(FormatError {
                status: 1,
                message: format!("{} is not formatted", input.display()),
            });
        }
        Mode::Check => {}
        Mode::Write if formatted == source => {
            println!("Already formatted {}", input.display());
        }
        Mode::Write => {
            write_formatted_file(input, &formatted)?;
            println!("Formatted {}", input.display());
        }
    }
    Ok(())
}

fn read_source(input: &Path) -> Result<String, FormatError> {
    let label = if input == Path::new("-") {
        "stdin".to_owned()
    } else {
        input.display().to_string()
    };
    let source = if input == Path::new("-") {
        read_bounded_source(io::stdin().lock(), &label)
    } else {
        let file = File::open(input).map_err(|error| {
            operation_error(format!("could not read {}: {error}", input.display()))
        })?;
        read_bounded_source(file, &label)
    }?;
    Ok(source)
}

fn read_bounded_source(reader: impl Read, label: &str) -> Result<String, FormatError> {
    let mut source = String::new();
    reader
        .take(super::MAX_SOURCE_BYTES.saturating_add(1) as u64)
        .read_to_string(&mut source)
        .map_err(|error| operation_error(format!("could not read {label}: {error}")))?;
    if source.len() > super::MAX_SOURCE_BYTES {
        return Err(operation_error(format!(
            "{label} exceeds the configured {}-byte source limit",
            super::MAX_SOURCE_BYTES
        )));
    }
    Ok(source)
}

fn write_formatted_file(input: &Path, contents: &str) -> Result<(), FormatError> {
    let target = fs::canonicalize(input).map_err(|error| {
        operation_error(format!("could not resolve {}: {error}", input.display()))
    })?;
    let metadata = fs::metadata(&target).map_err(|error| {
        operation_error(format!("could not inspect {}: {error}", input.display()))
    })?;
    if !metadata.is_file() {
        return Err(operation_error(format!(
            "{} is not a regular file",
            input.display()
        )));
    }
    let permissions = metadata.permissions();
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let (temporary_path, mut temporary_file) = create_temp_file(parent).map_err(|error| {
        operation_error(format!(
            "could not prepare a temporary file for {}: {error}",
            input.display()
        ))
    })?;

    let result = (|| -> io::Result<()> {
        temporary_file.write_all(contents.as_bytes())?;
        temporary_file.set_permissions(permissions)?;
        temporary_file.sync_all()?;
        drop(temporary_file);
        fs::rename(&temporary_path, &target)
    })();

    if let Err(error) = result {
        let _ = fs::remove_file(&temporary_path);
        return Err(operation_error(format!(
            "could not format {}: {error}",
            input.display()
        )));
    }

    Ok(())
}

fn create_temp_file(parent: &Path) -> io::Result<(PathBuf, File)> {
    loop {
        let id = NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".schemra-fmt-{}-{id}.tmp", process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

fn usage_error(message: &str) -> FormatError {
    FormatError {
        status: 2,
        message: message.to_owned(),
    }
}

fn operation_error(message: String) -> FormatError {
    FormatError { status: 2, message }
}
