use crate::cli::IngestCommand;
use anyhow::Result;
use runscope_core::services::{AppPaths, IngestRequest, IngestService};

pub fn run(command: IngestCommand, paths: &AppPaths, json_output: bool) -> Result<()> {
    let result = IngestService::ingest_dir(
        paths,
        IngestRequest {
            artifact_dir: command.artifact_dir,
            adapter: Some(command.adapter),
            project_override: command.project_override,
            label_override: command.label,
            tags: command.tags,
            note: command.note,
            dry_run: command.dry_run,
        },
    )?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if result.duplicate {
        println!(
            "Existing run {} for project {} ({})",
            result.run_id, result.project_slug, result.adapter
        );
    } else if result.dry_run {
        println!(
            "Dry run validated {} for project {} ({})",
            result.run_id, result.project_slug, result.adapter
        );
    } else {
        println!(
            "Ingested {} for project {} ({})",
            result.run_id, result.project_slug, result.adapter
        );
    }

    Ok(())
}
