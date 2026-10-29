use clap::{CommandFactory, Parser};
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "schemra",
    version,
    about = "Local authoring environment for Schemra diagrams and scenarios"
)]
struct Cli {}

fn main() -> ExitCode {
    if std::env::args_os().len() == 1 {
        return match Cli::command().print_help() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }

    let _cli = Cli::parse();
    ExitCode::SUCCESS
}
