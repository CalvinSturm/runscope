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

pub struct FaceappAdapter;

impl RunAdapter for FaceappAdapter {
    fn name(&self) -> &'static str {
        "faceapp"
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
                message: "No recognized faceapp manifest was found; using directory metadata only."
                    .to_string(),
            });
        }

        let suite = json_string(&manifest_value, &["identity", "suite"])
            .or_else(|| json_string(&manifest_value, &["suite"]))
            .or_else(|| Some("benchmark".to_string()));
        let scenario = json_string(&manifest_value, &["identity", "scenario"])
            .or_else(|| json_string(&manifest_value, &["scenario"]))
            .or_else(|| json_string(&manifest_value, &["benchmark_name"]));
        let label = json_string(&manifest_value, &["identity", "label"])
            .or_else(|| json_string(&manifest_value, &["label"]));
        if scenario.is_none() {
            warnings.push(AdapterWarning {
                code: "missing_scenario".to_string(),
                message: "faceapp artifact did not provide a scenario; leaving it unset."
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
                    slug: "faceapp".to_string(),
                    display_name: "faceapp".to_string(),
                },
                source: RunSource {
                    adapter: self.name().to_string(),
                    source_kind: SourceKind::ArtifactDir,
                    source_path: Some(artifact_dir.display().to_string()),
                    external_run_id: json_string(&manifest_value, &["source", "external_run_id"])
                        .or_else(|| json_string(&manifest_value, &["benchmark_id"]))
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
                    exec_status: parse_exec_status(
                        json_string(&manifest_value, &["runtime", "exec_status"])
                            .or_else(|| json_string(&manifest_value, &["exec_status"]))
                            .or_else(|| json_string(&manifest_value, &["status"]))
                            .as_deref(),
                    ),
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
        "faceapp_benchmark.json",
        "faceapp_run.json",
        "faceapp_manifest.json",
        "benchmark.json",
        "run.json",
    ]
}

fn detect_manifest_signature(path: &Path) -> Result<bool, RunScopeError> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;

    if json_string(&value, &["project", "slug"])
        .map(|value| value.eq_ignore_ascii_case("faceapp"))
        .unwrap_or(false)
    {
        return Ok(true);
    }
    if json_string(&value, &["producer"])
        .map(|value| value.eq_ignore_ascii_case("faceapp"))
        .unwrap_or(false)
    {
        return Ok(true);
    }
    if json_string_array(&value, &["command"])
        .or_else(|| json_string_array(&value, &["workload", "command_argv"]))
        .and_then(|argv| argv.first().cloned())
        .map(|value| value.eq_ignore_ascii_case("faceapp-bench"))
        .unwrap_or(false)
    {
        return Ok(true);
    }

    let has_expected_shape = value.get("metrics").is_some()
        && (value.get("benchmark_name").is_some()
            || value.get("benchmark_id").is_some()
            || value.get("model").is_some());
    Ok(path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase().starts_with("faceapp_"))
        .unwrap_or(false)
        && has_expected_shape)
}

fn read_candidate_manifest(artifact_dir: &Path) -> Result<Value, RunScopeError> {
    for name in known_manifest_names() {
        let path = artifact_dir.join(name);
        if path.is_file() {
            return Ok(serde_json::from_str(&fs::read_to_string(path)?)?);
        }
    }
    Ok(Value::Null)
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
                RunScopeError::ManifestValidation("invalid faceapp source path".to_string())
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
    if extension == "json" {
        let role = if lower_name.contains("manifest") {
            "raw_source_manifest"
        } else {
            "report_json"
        };
        return ("raw", role.to_string(), "application/json".to_string());
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
            .or_else(|| json_string(value, &["commit_sha"]))
            .or_else(|| json_string(value, &["git_commit"])),
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
            .or_else(|| json_string(value, &["machine_name"])),
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
    if let Some(metrics) = value.get("metrics").and_then(Value::as_array) {
        return metrics
            .iter()
            .filter_map(|metric| {
                let object = metric.as_object()?;
                Some(MetricRecord {
                    key: object.get("key")?.as_str()?.to_string(),
                    group_name: object
                        .get("group_name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    value_num: object.get("value_num").and_then(Value::as_f64),
                    value_text: object
                        .get("value_text")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    unit: object
                        .get("unit")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    direction: match object
                        .get("direction")
                        .and_then(Value::as_str)
                        .unwrap_or("none")
                    {
                        "higher_is_better" => MetricDirection::HigherIsBetter,
                        "lower_is_better" => MetricDirection::LowerIsBetter,
                        "target" => MetricDirection::Target,
                        _ => MetricDirection::None,
                    },
                    is_primary: object
                        .get("is_primary")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
            })
            .collect();
    }

    warnings.push(AdapterWarning {
        code: "missing_metrics".to_string(),
        message: "faceapp artifact did not expose metrics in a recognized format.".to_string(),
    });
    Vec::new()
}

fn adapter_payload(object: Option<&Map<String, Value>>) -> BTreeMap<String, Value> {
    let mut payload = BTreeMap::new();
    let Some(object) = object else {
        return payload;
    };

    let mut faceapp = Map::new();
    for key in ["benchmark_name", "backend_version"] {
        if let Some(value) = object.get(key) {
            faceapp.insert(key.to_string(), value.clone());
        }
    }
    if !faceapp.is_empty() {
        payload.insert("faceapp".to_string(), Value::Object(faceapp));
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
    fn detect_faceapp_fixture() {
        let detected = FaceappAdapter.detect(&fixture_dir()).unwrap();
        assert!(detected);
    }

    #[test]
    fn parse_faceapp_fixture() {
        let parsed = FaceappAdapter.parse(&fixture_dir()).unwrap();

        assert_eq!(parsed.manifest.project.slug, "faceapp");
        assert_eq!(
            parsed.manifest.identity.scenario.as_deref(),
            Some("resnet50_cuda_fp16")
        );
        assert_eq!(parsed.manifest.metrics.len(), 2);
        assert!(parsed
            .manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.role == "report_json"));
    }

    fn fixture_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("faceapp")
            .join("basic")
    }
}
