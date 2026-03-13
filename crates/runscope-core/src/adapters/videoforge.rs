use crate::adapters::traits::{ParsedRun, RunAdapter, SourceFile};
use crate::domain::{
    AdapterWarning, ArtifactRecord, EnvironmentContext, ExecStatus, GitContext, MetricDirection,
    MetricRecord, ProjectRef, RunIdentity, RunManifestV1, RunSource, RuntimeContext, SourceKind,
    SummaryContext, WorkloadContext, RUN_SCHEMA_VERSION,
};
use crate::error::RunScopeError;
use crate::services::ingest::{generate_ulid_like, now_utc_rfc3339};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub struct VideoforgeAdapter;

impl RunAdapter for VideoforgeAdapter {
    fn name(&self) -> &'static str {
        "videoforge"
    }

    fn detect(&self, artifact_dir: &Path) -> Result<bool, RunScopeError> {
        if !artifact_dir.is_dir() {
            return Ok(false);
        }

        for marker in known_manifest_names() {
            let path = artifact_dir.join(marker);
            if path.is_file() {
                return detect_manifest_signature(&path);
            }
        }

        Ok(false)
    }

    fn parse(&self, artifact_dir: &Path) -> Result<ParsedRun, RunScopeError> {
        let manifest_value = read_candidate_manifest(artifact_dir)?;
        let mut warnings = Vec::new();

        if manifest_value.is_null() {
            warnings.push(AdapterWarning {
                code: "missing_manifest".to_string(),
                message:
                    "No recognized VideoForge manifest was found; using directory metadata only."
                        .to_string(),
            });
        }

        let suite = json_string(&manifest_value, &["identity", "suite"])
            .or_else(|| json_string(&manifest_value, &["suite"]));
        let scenario = json_string(&manifest_value, &["identity", "scenario"])
            .or_else(|| json_string(&manifest_value, &["scenario"]));
        let label = json_string(&manifest_value, &["identity", "label"])
            .or_else(|| json_string(&manifest_value, &["label"]));
        let status_value = json_string(&manifest_value, &["runtime", "exec_status"])
            .or_else(|| json_string(&manifest_value, &["exec_status"]))
            .or_else(|| json_string(&manifest_value, &["status"]));

        if suite.is_none() {
            warnings.push(AdapterWarning {
                code: "missing_suite".to_string(),
                message: "VideoForge artifact did not provide a suite; leaving it unset."
                    .to_string(),
            });
        }
        if scenario.is_none() {
            warnings.push(AdapterWarning {
                code: "missing_scenario".to_string(),
                message: "VideoForge artifact did not provide a scenario; leaving it unset."
                    .to_string(),
            });
        }

        let files_to_copy = collect_source_files(artifact_dir)?;
        let artifacts = files_to_copy
            .iter()
            .map(|file| ArtifactRecord {
                role: file.role.clone(),
                rel_path: file.target_rel_path.clone(),
                media_type: file.media_type.clone(),
                sha256: None,
                size_bytes: None,
            })
            .collect();
        let metrics = parse_metrics(&manifest_value, &mut warnings);

        Ok(ParsedRun {
            manifest: RunManifestV1 {
                schema_version: RUN_SCHEMA_VERSION.to_string(),
                run_id: generate_ulid_like(),
                project: ProjectRef {
                    slug: "videoforge".to_string(),
                    display_name: "VideoForge".to_string(),
                },
                source: RunSource {
                    adapter: self.name().to_string(),
                    source_kind: SourceKind::ArtifactDir,
                    source_path: Some(artifact_dir.display().to_string()),
                    external_run_id: json_string(&manifest_value, &["source", "external_run_id"])
                        .or_else(|| json_string(&manifest_value, &["external_run_id"]))
                        .or_else(|| json_string(&manifest_value, &["run_id"])),
                    ingested_at: now_utc_rfc3339(),
                },
                identity: RunIdentity {
                    suite,
                    scenario,
                    label,
                },
                git: optional_git_context(&manifest_value),
                runtime: RuntimeContext {
                    started_at: json_string(&manifest_value, &["runtime", "started_at"])
                        .or_else(|| json_string(&manifest_value, &["started_at"])),
                    finished_at: json_string(&manifest_value, &["runtime", "finished_at"])
                        .or_else(|| json_string(&manifest_value, &["finished_at"])),
                    duration_ms: json_u64(&manifest_value, &["runtime", "duration_ms"])
                        .or_else(|| json_u64(&manifest_value, &["duration_ms"])),
                    exit_code: json_i32(&manifest_value, &["runtime", "exit_code"])
                        .or_else(|| json_i32(&manifest_value, &["exit_code"])),
                    exec_status: parse_exec_status(status_value.as_deref()),
                },
                environment: optional_environment_context(&manifest_value),
                workload: optional_workload_context(&manifest_value),
                summary: SummaryContext {
                    error_count: json_u64(&manifest_value, &["summary", "error_count"])
                        .or_else(|| json_u64(&manifest_value, &["error_count"]))
                        .unwrap_or(0) as u32,
                    warning_count: warnings.len() as u32,
                },
                metrics,
                artifacts,
                adapter_payload: adapter_payload(manifest_value.as_object()),
            },
            files_to_copy,
            warnings,
        })
    }
}

fn known_manifest_names() -> &'static [&'static str] {
    &[
        "videoforge_run.json",
        "videoforge.run_manifest.v1.json",
        "videoforge.runtime_config_snapshot.v1.json",
        "videoforge.run_observed_metrics.v1.json",
        "videoforge_report.json",
        "videoforge_manifest.json",
        "run.json",
        "report.json",
    ]
}

fn detect_manifest_signature(path: &Path) -> Result<bool, RunScopeError> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;

    if json_string(&value, &["project", "slug"])
        .map(|value| value.eq_ignore_ascii_case("videoforge"))
        .unwrap_or(false)
    {
        return Ok(true);
    }
    if json_string(&value, &["producer"])
        .map(|value| value.eq_ignore_ascii_case("videoforge"))
        .unwrap_or(false)
    {
        return Ok(true);
    }
    if json_string_array(&value, &["command"])
        .or_else(|| json_string_array(&value, &["workload", "command_argv"]))
        .and_then(|argv| argv.first().cloned())
        .map(|value| value.eq_ignore_ascii_case("videoforge"))
        .unwrap_or(false)
    {
        return Ok(true);
    }

    let has_expected_shape = value.get("metrics").is_some()
        && (value.get("backend").is_some()
            || json_value(&value, &["environment", "backend"]).is_some()
            || value.get("pipeline").is_some());

    Ok(path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase().starts_with("videoforge_"))
        .unwrap_or(false)
        && has_expected_shape)
}

fn read_candidate_manifest(artifact_dir: &Path) -> Result<Value, RunScopeError> {
    if let Some(report_path) = [
        "videoforge_run.json",
        "videoforge_report.json",
        "videoforge_manifest.json",
        "run.json",
        "report.json",
    ]
    .iter()
    .map(|name| artifact_dir.join(name))
    .find(|path| path.is_file())
    {
        return Ok(serde_json::from_str(&fs::read_to_string(report_path)?)?);
    }

    let manifest_path = artifact_dir.join("videoforge.run_manifest.v1.json");
    let runtime_snapshot_path = artifact_dir.join("videoforge.runtime_config_snapshot.v1.json");
    let observed_metrics_path = artifact_dir.join("videoforge.run_observed_metrics.v1.json");

    if manifest_path.is_file() || runtime_snapshot_path.is_file() || observed_metrics_path.is_file()
    {
        return synthesize_manifest_from_v1_artifacts(
            manifest_path.is_file().then_some(manifest_path.as_path()),
            runtime_snapshot_path
                .is_file()
                .then_some(runtime_snapshot_path.as_path()),
            observed_metrics_path
                .is_file()
                .then_some(observed_metrics_path.as_path()),
        );
    }

    Ok(Value::Null)
}

fn synthesize_manifest_from_v1_artifacts(
    manifest_path: Option<&Path>,
    runtime_snapshot_path: Option<&Path>,
    observed_metrics_path: Option<&Path>,
) -> Result<Value, RunScopeError> {
    let manifest_value = read_json_opt(manifest_path)?;
    let runtime_snapshot = read_json_opt(runtime_snapshot_path)?;
    let observed_metrics = read_json_opt(observed_metrics_path)?;

    let mut report = Map::new();
    report.insert(
        "producer".to_string(),
        Value::String("videoforge".to_string()),
    );

    if let Some(run_id) = json_string(&runtime_snapshot, &["run_id"])
        .or_else(|| json_string(&manifest_value, &["job_id"]))
    {
        report.insert("run_id".to_string(), Value::String(run_id));
    }

    let suite = json_string(&runtime_snapshot, &["media_kind"]).map(|kind| match kind.as_str() {
        "video" => "video_upscale".to_string(),
        "image" => "image_upscale".to_string(),
        _ => "upscale".to_string(),
    });
    if let Some(suite) = suite {
        report.insert("suite".to_string(), Value::String(suite));
    }

    let route_id = json_string(&runtime_snapshot, &["route_id"]);
    let model = json_string(&runtime_snapshot, &["model_key"])
        .or_else(|| json_string(&manifest_value, &["model_key"]));
    let precision = json_string(&runtime_snapshot, &["precision"]);
    let scale = json_u64(&runtime_snapshot, &["scale"]);

    let mut scenario_parts = Vec::new();
    if let Some(route_id) = route_id.clone() {
        scenario_parts.push(route_id);
    }
    if let Some(model) = model.clone() {
        scenario_parts.push(model);
    }
    if let Some(precision) = precision.clone() {
        scenario_parts.push(precision);
    }
    if let Some(scale) = scale {
        scenario_parts.push(format!("x{scale}"));
    }
    if !scenario_parts.is_empty() {
        report.insert(
            "scenario".to_string(),
            Value::String(scenario_parts.join("_")),
        );
    }

    if let Some(route_id) = route_id.clone() {
        report.insert("label".to_string(), Value::String(route_id.clone()));
        report.insert("backend".to_string(), Value::String(route_id.clone()));
        report.insert("pipeline".to_string(), Value::String(route_id));
    }

    if let Some(model) = model {
        report.insert("model".to_string(), Value::String(model));
    }
    if let Some(precision) = precision {
        report.insert("precision".to_string(), Value::String(precision));
    }

    if let Some(status) = json_string(&observed_metrics, &["status"]) {
        let mapped = match status.as_str() {
            "succeeded" => "pass",
            "failed" => "fail",
            "cancelled" => "error",
            _ => "unknown",
        };
        report.insert("status".to_string(), Value::String(mapped.to_string()));
        let exit_code = match status.as_str() {
            "succeeded" => 0,
            "failed" => 1,
            "cancelled" => 130,
            _ => 2,
        };
        report.insert("exit_code".to_string(), Value::Number(exit_code.into()));
    }

    if let Some(duration_ms) = json_u64(&observed_metrics, &["total_elapsed_ms"]) {
        report.insert("duration_ms".to_string(), Value::Number(duration_ms.into()));
    }

    if let Some(engine) = json_string(&runtime_snapshot, &["engine_family"]) {
        report.insert("engine".to_string(), Value::String(engine));
    }

    if let Some(input_path) = json_string(&runtime_snapshot, &["input_path"]) {
        report.insert("input_path".to_string(), Value::String(input_path));
    }
    if let Some(output_path) = json_string(&runtime_snapshot, &["output_path"]) {
        report.insert("output_path".to_string(), Value::String(output_path));
    }

    let mut command = vec![
        Value::String("videoforge".to_string()),
        Value::String("upscale".to_string()),
    ];
    if let Some(engine_family) = json_string(&runtime_snapshot, &["engine_family"]) {
        if engine_family == "native" {
            command.push(Value::String("--native".to_string()));
        }
    }
    if let Some(executed_executor) = json_string(&runtime_snapshot, &["executed_executor"]) {
        command.push(Value::String(format!("--executor={executed_executor}")));
    }
    report.insert("command".to_string(), Value::Array(command));

    if let Ok(cwd) = std::env::current_dir() {
        report.insert("cwd".to_string(), Value::String(cwd.display().to_string()));
    }

    report.insert("runtime_snapshot".to_string(), runtime_snapshot.clone());
    if !observed_metrics.is_null() {
        report.insert("observed_metrics".to_string(), observed_metrics.clone());
    }

    let mut metrics = Map::new();
    if let Some(total_elapsed_ms) = json_u64(&observed_metrics, &["total_elapsed_ms"]) {
        metrics.insert(
            "total_elapsed_ms".to_string(),
            Value::from(total_elapsed_ms as f64),
        );
    }
    if let Some(work_units_processed) = json_u64(&observed_metrics, &["work_units_processed"]) {
        metrics.insert(
            "work_units_processed".to_string(),
            Value::from(work_units_processed as f64),
        );
        if let Some(total_elapsed_ms) = json_u64(&observed_metrics, &["total_elapsed_ms"]) {
            if total_elapsed_ms > 0 {
                let fps = work_units_processed as f64 / (total_elapsed_ms as f64 / 1000.0);
                metrics.insert("fps".to_string(), Value::from(fps));
            }
        }
    }
    for key in [
        "frames_decoded",
        "frames_preprocessed",
        "frames_inferred",
        "frames_encoded",
        "preprocess_avg_us",
        "inference_frame_avg_us",
        "inference_dispatch_avg_us",
        "postprocess_frame_avg_us",
        "postprocess_dispatch_avg_us",
        "encode_avg_us",
        "vram_current_mb",
        "vram_peak_mb",
    ] {
        if let Some(value) = json_u64(&observed_metrics, &["extensions", "native", key]) {
            metrics.insert(key.to_string(), Value::from(value as f64));
        }
    }
    if !metrics.is_empty() {
        report.insert("metrics".to_string(), Value::Object(metrics));
    }

    Ok(Value::Object(report))
}

fn read_json_opt(path: Option<&Path>) -> Result<Value, RunScopeError> {
    match path {
        Some(path) => Ok(serde_json::from_str(&fs::read_to_string(path)?)?),
        None => Ok(Value::Null),
    }
}

fn collect_source_files(artifact_dir: &Path) -> Result<Vec<SourceFile>, RunScopeError> {
    let mut files = Vec::new();
    visit_dir(artifact_dir, artifact_dir, &mut files)?;
    files.sort_by(|left, right| left.target_rel_path.cmp(&right.target_rel_path));
    Ok(files)
}

fn visit_dir(
    root: &Path,
    current: &Path,
    files: &mut Vec<SourceFile>,
) -> Result<(), RunScopeError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_dir(root, &path, files)?;
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .map_err(|_| {
                RunScopeError::ManifestValidation("invalid VideoForge source path".to_string())
            })?
            .to_path_buf();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("artifact");
        let (prefix, role, media_type) = infer_copy_target(&relative, name);
        files.push(SourceFile {
            source_path: path,
            target_rel_path: format!("{prefix}/{}", relative.to_string_lossy().replace('\\', "/")),
            role,
            media_type,
        });
    }
    Ok(())
}

fn infer_copy_target(relative: &Path, file_name: &str) -> (&'static str, String, String) {
    let lower_name = file_name.to_ascii_lowercase();
    let extension = relative
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if lower_name.contains("stdout") {
        return ("logs", "stdout_log".to_string(), "text/plain".to_string());
    }
    if lower_name.contains("stderr") {
        return ("logs", "stderr_log".to_string(), "text/plain".to_string());
    }
    if extension == "json" {
        let role = if lower_name == "videoforge.run_manifest.v1.json" {
            "videoforge_run_manifest_v1"
        } else if lower_name == "videoforge.runtime_config_snapshot.v1.json" {
            "videoforge_runtime_config_snapshot_v1"
        } else if lower_name == "videoforge.run_observed_metrics.v1.json" {
            "videoforge_run_observed_metrics_v1"
        } else if lower_name == "videoforge_run.json" {
            "videoforge_runscope_bundle"
        } else if lower_name.contains("manifest") {
            "raw_source_manifest"
        } else {
            "report_json"
        };
        return ("raw", role.to_string(), "application/json".to_string());
    }
    if extension == "html" {
        return ("raw", "report_html".to_string(), "text/html".to_string());
    }
    if matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
        return (
            "raw",
            "screenshot".to_string(),
            match extension.as_str() {
                "png" => "image/png".to_string(),
                _ => "image/jpeg".to_string(),
            },
        );
    }
    if extension == "mp4" {
        return ("raw", "video".to_string(), "video/mp4".to_string());
    }

    (
        "raw",
        "input_manifest".to_string(),
        match extension.as_str() {
            "log" | "txt" => "text/plain".to_string(),
            _ => "application/octet-stream".to_string(),
        },
    )
}

fn optional_git_context(value: &Value) -> Option<GitContext> {
    let git = GitContext {
        commit_sha: json_string(value, &["git", "commit_sha"])
            .or_else(|| json_string(value, &["commit_sha"])),
        branch: json_string(value, &["git", "branch"]).or_else(|| json_string(value, &["branch"])),
        dirty: json_bool(value, &["git", "dirty"]).or_else(|| json_bool(value, &["dirty"])),
    };
    if git.commit_sha.is_none() && git.branch.is_none() && git.dirty.is_none() {
        None
    } else {
        Some(git)
    }
}

fn optional_environment_context(value: &Value) -> Option<EnvironmentContext> {
    let environment = EnvironmentContext {
        machine_name: json_string(value, &["environment", "machine_name"])
            .or_else(|| json_string(value, &["machine_name"]))
            .or_else(|| json_string(value, &["machine"])),
        os: json_string(value, &["environment", "os"]).or_else(|| json_string(value, &["os"])),
        cpu: json_string(value, &["environment", "cpu"]).or_else(|| json_string(value, &["cpu"])),
        gpu: json_string(value, &["environment", "gpu"]).or_else(|| json_string(value, &["gpu"])),
        backend: json_string(value, &["environment", "backend"])
            .or_else(|| json_string(value, &["backend"])),
        model: json_string(value, &["environment", "model"])
            .or_else(|| json_string(value, &["model"])),
        precision: json_string(value, &["environment", "precision"])
            .or_else(|| json_string(value, &["precision"])),
    };
    if environment.machine_name.is_none()
        && environment.os.is_none()
        && environment.cpu.is_none()
        && environment.gpu.is_none()
        && environment.backend.is_none()
        && environment.model.is_none()
        && environment.precision.is_none()
    {
        None
    } else {
        Some(environment)
    }
}

fn optional_workload_context(value: &Value) -> Option<WorkloadContext> {
    let workload = WorkloadContext {
        dataset: json_string(value, &["workload", "dataset"])
            .or_else(|| json_string(value, &["dataset"])),
        input_count: json_u64(value, &["workload", "input_count"])
            .or_else(|| json_u64(value, &["input_count"])),
        command_argv: json_string_array(value, &["workload", "command_argv"])
            .or_else(|| json_string_array(value, &["command"]))
            .unwrap_or_default(),
        display_command: json_string(value, &["workload", "display_command"])
            .or_else(|| json_string(value, &["display_command"])),
        cwd: json_string(value, &["workload", "cwd"]).or_else(|| json_string(value, &["cwd"])),
        env_snapshot_ref: None,
    };
    if workload.dataset.is_none()
        && workload.input_count.is_none()
        && workload.command_argv.is_empty()
        && workload.display_command.is_none()
        && workload.cwd.is_none()
        && workload.env_snapshot_ref.is_none()
    {
        None
    } else {
        Some(workload)
    }
}

fn parse_exec_status(value: Option<&str>) -> ExecStatus {
    match value.unwrap_or("unknown") {
        "pass" => ExecStatus::Pass,
        "fail" => ExecStatus::Fail,
        "error" => ExecStatus::Error,
        _ => ExecStatus::Unknown,
    }
}

fn parse_metrics(value: &Value, warnings: &mut Vec<AdapterWarning>) -> Vec<MetricRecord> {
    if let Some(metric_map) = value.get("metrics").and_then(Value::as_object) {
        return metric_map
            .iter()
            .filter_map(|(key, raw)| {
                raw.as_f64().map(|value_num| MetricRecord {
                    key: key.clone(),
                    group_name: String::new(),
                    value_num: Some(value_num),
                    value_text: None,
                    unit: infer_metric_unit(key),
                    direction: infer_metric_direction(key),
                    is_primary: matches!(key.as_str(), "fps" | "throughput" | "latency_p50_ms"),
                })
            })
            .collect();
    }

    warnings.push(AdapterWarning {
        code: "missing_metrics".to_string(),
        message: "VideoForge artifact did not expose metrics in a recognized format.".to_string(),
    });
    Vec::new()
}

fn infer_metric_direction(key: &str) -> MetricDirection {
    let key = key.to_ascii_lowercase();
    if key.contains("latency") || key.ends_with("_ms") {
        MetricDirection::LowerIsBetter
    } else if key.contains("fps") || key.contains("throughput") {
        MetricDirection::HigherIsBetter
    } else {
        MetricDirection::None
    }
}

fn infer_metric_unit(key: &str) -> Option<String> {
    let key = key.to_ascii_lowercase();
    if key.contains("fps") {
        Some("frames/s".to_string())
    } else if key.ends_with("_ms") || key.contains("latency") {
        Some("ms".to_string())
    } else {
        None
    }
}

fn adapter_payload(object: Option<&Map<String, Value>>) -> BTreeMap<String, Value> {
    let mut payload = BTreeMap::new();
    let Some(object) = object else {
        return payload;
    };

    let mut videoforge = Map::new();
    for key in ["engine", "pipeline"] {
        if let Some(value) = object.get(key) {
            videoforge.insert(key.to_string(), value.clone());
        }
    }
    if !videoforge.is_empty() {
        payload.insert("videoforge".to_string(), Value::Object(videoforge));
    }
    payload
}

fn json_string(root: &Value, path: &[&str]) -> Option<String> {
    json_value(root, path)?.as_str().map(ToString::to_string)
}

fn json_u64(root: &Value, path: &[&str]) -> Option<u64> {
    json_value(root, path)?.as_u64()
}

fn json_i32(root: &Value, path: &[&str]) -> Option<i32> {
    json_value(root, path)?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
}

fn json_bool(root: &Value, path: &[&str]) -> Option<bool> {
    json_value(root, path)?.as_bool()
}

fn json_string_array(root: &Value, path: &[&str]) -> Option<Vec<String>> {
    Some(
        json_value(root, path)?
            .as_array()?
            .iter()
            .filter_map(|value| value.as_str().map(ToString::to_string))
            .collect(),
    )
}

fn json_value<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_videoforge_fixture() {
        let detected = VideoforgeAdapter.detect(&fixture_dir()).unwrap();
        assert!(detected);
    }

    #[test]
    fn parse_videoforge_fixture() {
        let parsed = VideoforgeAdapter.parse(&fixture_dir()).unwrap();

        assert_eq!(parsed.manifest.project.slug, "videoforge");
        assert_eq!(
            parsed.manifest.identity.suite.as_deref(),
            Some("perf_smoke")
        );
        assert_eq!(parsed.manifest.metrics.len(), 2);
        assert!(parsed
            .manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.role == "stdout_log"));
    }

    #[test]
    fn parse_videoforge_v1_artifact_bundle_without_report_json() {
        let parsed = VideoforgeAdapter.parse(&v1_fixture_dir()).unwrap();

        assert_eq!(parsed.manifest.project.slug, "videoforge");
        assert_eq!(
            parsed.manifest.identity.suite.as_deref(),
            Some("video_upscale")
        );
        assert_eq!(
            parsed.manifest.identity.scenario.as_deref(),
            Some("native_direct_2x_SPAN_soft_fp16_x2")
        );
        assert_eq!(parsed.manifest.runtime.exec_status, ExecStatus::Pass);
        assert!(parsed
            .manifest
            .metrics
            .iter()
            .any(|metric| metric.key == "fps"));
        assert!(parsed
            .manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.role == "videoforge_run_manifest_v1"));
        assert!(parsed
            .manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.role == "videoforge_runtime_config_snapshot_v1"));
        assert!(parsed
            .manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.role == "videoforge_run_observed_metrics_v1"));
    }

    fn fixture_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("videoforge")
            .join("basic")
    }

    fn v1_fixture_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("videoforge")
            .join("v1_bundle")
    }
}
