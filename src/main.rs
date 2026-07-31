//! `kf` binary entry point: parse, dispatch, map errors to exit codes.

use clap::Parser;

use kanbanflow_cli::cli::Cli;
use kanbanflow_cli::exit;

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match cli.run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::ExitCode::from(exit::classify(&error) as u8)
        }
    }
}
