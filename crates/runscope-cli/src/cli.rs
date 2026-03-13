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
    Record(RecordCommand),
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

#[derive(Debug, Args)]
pub struct RecordCommand {
    #[arg(long = "project")]
    pub project: String,
    #[arg(long = "status")]
    pub status: String,
    #[arg(long = "project-name")]
    pub project_name: Option<String>,
    #[arg(long)]
    pub suite: Option<String>,
    #[arg(long)]
    pub scenario: Option<String>,
    #[arg(long)]
    pub label: Option<String>,
    #[arg(long = "commit-sha")]
    pub commit_sha: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long = "git-dirty")]
    pub git_dirty: bool,
    #[arg(long = "machine")]
    pub machine: Option<String>,
    #[arg(long = "os")]
    pub os: Option<String>,
    #[arg(long = "cpu")]
    pub cpu: Option<String>,
    #[arg(long = "gpu")]
    pub gpu: Option<String>,
    #[arg(long = "backend")]
    pub backend: Option<String>,
    #[arg(long = "model")]
    pub model: Option<String>,
    #[arg(long = "precision")]
    pub precision: Option<String>,
    #[arg(long = "dataset")]
    pub dataset: Option<String>,
    #[arg(long = "input-count")]
    pub input_count: Option<u64>,
    #[arg(long = "argv")]
    pub argv: Vec<String>,
    #[arg(long = "display-command")]
    pub display_command: Option<String>,
    #[arg(long = "cwd")]
    pub cwd: Option<PathBuf>,
    #[arg(long = "env-file")]
    pub env_file: Option<PathBuf>,
    #[arg(long = "metric")]
    pub metrics: Vec<String>,
    #[arg(long = "artifact")]
    pub artifacts: Vec<String>,
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    #[arg(long)]
    pub note: Option<String>,
}
