mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use runscope_core::error::RunScopeError;
use runscope_core::services::{resolve_app_paths, AppPaths};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(map_exit_code(&error))
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let resolved = resolve_app_paths(cli.data_dir.clone(), cli.db.clone());
    let paths = AppPaths {
        db_path: resolved.db_path,
        data_dir: resolved.data_dir,
    };

    match cli.command {
        Commands::Ingest(command) => commands::ingest::run(command, &paths, cli.json),
        Commands::Record(command) => commands::record::run(command, &paths, cli.json),
    }
}

fn map_exit_code(error: &anyhow::Error) -> u8 {
    match error.downcast_ref::<RunScopeError>() {
        Some(RunScopeError::AdapterNotDetected | RunScopeError::AdapterAmbiguous) => 3,
        Some(RunScopeError::ManifestValidation(_)) => 4,
        Some(RunScopeError::RunNotFound(_)) => 6,
        Some(_) => 5,
        None => 1,
    }
}
