mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use runscope_core::error::RunScopeError;
use runscope_core::services::AppPaths;
use std::env;
use std::path::PathBuf;
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
    let data_dir = cli.data_dir.clone().unwrap_or_else(default_data_dir);
    let db_path = cli
        .db
        .clone()
        .unwrap_or_else(|| data_dir.join("runscope.sqlite"));
    let paths = AppPaths { db_path, data_dir };

    match cli.command {
        Commands::Ingest(command) => commands::ingest::run(command, &paths, cli.json),
        Commands::Record(command) => commands::record::run(command, &paths, cli.json),
    }
}

fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(value) = env::var("LOCALAPPDATA") {
            return PathBuf::from(value).join("RunScope");
        }
        if let Ok(value) = env::var("APPDATA") {
            return PathBuf::from(value).join("RunScope");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("RunScope");
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg_data_home).join("runscope");
        }
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("runscope");
        }
    }

    PathBuf::from(".runscope")
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
