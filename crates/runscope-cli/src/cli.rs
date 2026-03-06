use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "runscope")]
pub struct Cli {
    #[arg(long)]
    pub db: Option<PathBuf>,
    #[arg(long = "data-dir")]
    pub data_dir: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Ingest(IngestCommand),
}

#[derive(Debug, Args)]
pub struct IngestCommand {
    pub artifact_dir: PathBuf,
    #[arg(long, default_value = "auto")]
    pub adapter: String,
    #[arg(long = "project-override")]
    pub project_override: Option<String>,
    #[arg(long = "label")]
    pub label: Option<String>,
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}
