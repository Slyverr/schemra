mod project;
mod source_format;

const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;

use std::{path::PathBuf, process::ExitCode};

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "schemra",
    version,
    about = "Local authoring environment for Schemra diagrams and scenarios"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Check a Schemra source file together with its project.
    Check {
        /// Source file to check, relative to the current directory.
        input: PathBuf,
        /// Project root containing the source file and related Schemra files.
        #[arg(long)]
        root: Option<PathBuf>,
        /// Print diagnostics as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Format a source file or standard input.
    #[command(visible_alias = "format")]
    Fmt {
        /// Source file to format, or - to read standard input.
        input: PathBuf,
        /// Replace the source file atomically.
        #[arg(long, conflicts_with = "check")]
        write: bool,
        /// Exit with status 1 when formatting changes are needed.
        #[arg(long, conflicts_with = "write")]
        check: bool,
    },
    /// Validate a scenario against an explicitly selected visual.
    ScenarioCheck {
        /// Scenario source file.
        input: PathBuf,
        /// Visual source file the scenario must target.
        #[arg(long)]
        visual: PathBuf,
        /// Project root containing the scenario, visual, and related sources.
        #[arg(long)]
        root: Option<PathBuf>,
        /// Print diagnostics as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Render a visual source file to SVG.
    Render {
        /// Visual source file to render.
        input: PathBuf,
        /// Destination SVG file.
        #[arg(long)]
        out: PathBuf,
        /// Project root containing the visual and related sources.
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Start the language server over standard input and output.
    Lsp,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None => match Cli::command().print_help() {
            Ok(()) => {
                println!();
                ExitCode::from(2)
            }
            Err(_) => ExitCode::FAILURE,
        },
        Some(Commands::Check { input, root, json }) => {
            let root = match current_or_selected_root(root) {
                Ok(root) => root,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            match project::check(&input, &root) {
                Ok(report) => {
                    if json {
                        println!("{}", report.to_json());
                    } else {
                        project::print_report(&report);
                    }
                    if report.ok {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(2)
                }
            }
        }
        Some(Commands::Fmt {
            input,
            write,
            check,
        }) => {
            let mode = if write {
                source_format::Mode::Write
            } else if check {
                source_format::Mode::Check
            } else {
                source_format::Mode::Stdout
            };
            match source_format::run(&input, mode) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("{}", error.message);
                    ExitCode::from(error.status)
                }
            }
        }
        Some(Commands::ScenarioCheck {
            input,
            visual,
            root,
            json,
        }) => {
            let root = match current_or_selected_root(root) {
                Ok(root) => root,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            match project::scenario_check(&input, &visual, &root) {
                Ok(report) => {
                    if json {
                        println!("{}", report.to_json());
                    } else {
                        project::print_scenario_report(&report);
                    }
                    if report.ok {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(2)
                }
            }
        }
        Some(Commands::Render { input, out, root }) => {
            let root = match current_or_selected_root(root) {
                Ok(root) => root,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            match project::render(&input, &out, &root) {
                Ok(()) => ExitCode::SUCCESS,
                Err((status, error)) => {
                    if !error.is_empty() {
                        eprintln!("{error}");
                    }
                    ExitCode::from(status)
                }
            }
        }
        Some(Commands::Lsp) => match schemra_lsp::run_stdio() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Schemra language server stopped: {error}");
                ExitCode::FAILURE
            }
        },
    }
}

fn current_or_selected_root(root: Option<PathBuf>) -> Result<PathBuf, String> {
    root.map_or_else(std::env::current_dir, Ok)
        .map_err(|error| format!("could not determine the current directory: {error}"))
}
