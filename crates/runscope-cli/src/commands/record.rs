use crate::cli::RecordCommand;
use anyhow::{anyhow, bail, Result};
use runscope_core::domain::ExecStatus;
use runscope_core::services::{
    infer_metric_record, AppPaths, ManualAttachment, ManualRecordRequest, RecordService,
};
use runscope_core::store::infer_media_type_from_path;
use std::path::PathBuf;

pub fn run(command: RecordCommand, paths: &AppPaths, json_output: bool) -> Result<()> {
    let result = RecordService::record_manual(paths, build_request(command)?)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "Recorded {} for project {} ({})",
            result.run_id, result.project_slug, result.adapter
        );
    }

    Ok(())
}

fn build_request(command: RecordCommand) -> Result<ManualRecordRequest> {
    let metrics = command
        .metrics
        .iter()
        .map(|metric| parse_metric(metric))
        .collect::<Result<Vec<_>>>()?;
    let attachments = command
        .artifacts
        .iter()
        .map(|artifact| parse_attachment(artifact))
        .collect::<Result<Vec<_>>>()?;

    Ok(ManualRecordRequest {
        project_slug: command.project,
        project_display_name: command.project_name,
        exec_status: parse_exec_status(&command.status)?,
        suite: command.suite,
        scenario: command.scenario,
        label: command.label,
        commit_sha: command.commit_sha,
        branch: command.branch,
        git_dirty: if command.git_dirty { Some(true) } else { None },
        machine_name: command.machine,
        os: command.os,
        cpu: command.cpu,
        gpu: command.gpu,
        backend: command.backend,
        model: command.model,
        precision: command.precision,
        dataset: command.dataset,
        input_count: command.input_count,
        command_argv: command.argv,
        display_command: command.display_command,
        cwd: command.cwd.map(|path| path.display().to_string()),
        env_snapshot_file: command.env_file,
        metrics,
        attachments,
        note: command.note,
        tags: command.tags,
    })
}

fn parse_exec_status(value: &str) -> Result<ExecStatus> {
    match value {
        "pass" => Ok(ExecStatus::Pass),
        "fail" => Ok(ExecStatus::Fail),
        "error" => Ok(ExecStatus::Error),
        "unknown" => Ok(ExecStatus::Unknown),
        _ => bail!("invalid status: {value}"),
    }
}

fn parse_metric(value: &str) -> Result<runscope_core::domain::MetricRecord> {
    let Some((key, raw_value)) = value.split_once('=') else {
        bail!("invalid metric, expected KEY=VALUE: {value}");
    };
    let parsed_value = raw_value
        .parse::<f64>()
        .map_err(|_| anyhow!("invalid metric value for {key}: {raw_value}"))?;
    Ok(infer_metric_record(key, parsed_value))
}

fn parse_attachment(value: &str) -> Result<ManualAttachment> {
    let Some((role, raw_path)) = value.split_once('=') else {
        bail!("invalid artifact, expected ROLE=PATH: {value}");
    };
    let path = PathBuf::from(raw_path);
    Ok(ManualAttachment {
        role: role.to_string(),
        path: path.clone(),
        media_type: infer_media_type_from_path(&path).to_string(),
    })
}
