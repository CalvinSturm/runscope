use crate::adapters::traits::{ParsedRun, RunAdapter, SourceFile};
use crate::domain::{
    AdapterWarning, ArtifactRecord, ExecStatus, GitContext, MetricDirection, MetricRecord,
    ProjectRef, RunIdentity, RunManifestV1, RunSource, RuntimeContext, SourceKind, SummaryContext,
    WorkloadContext, RUN_SCHEMA_VERSION,
};
use crate::error::RunScopeError;
use crate::services::ingest::{generate_ulid_like, now_utc_rfc3339};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub struct LocalAgentAdapter;

impl RunAdapter for LocalAgentAdapter {
    fn name(&self) -> &'static str {
        "localagent"
    }

    fn detect(&self, artifact_dir: &Path) -> Result<bool, RunScopeError> {
        if !artifact_dir.is_dir() {
            return Ok(false);
        }

        for marker in known_manifest_names() {
            if artifact_dir.join(marker).is_file() {
                return detect_manifest_signature(&artifact_dir.join(marker));
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
                    "No recognized LocalAgent manifest was found; using directory metadata only."
                        .to_string(),
            });
        }

        let suite = json_string(&manifest_value, &["identity", "suite"])
            .or_else(|| json_string(&manifest_value, &["suite"]));
        let scenario = json_string(&manifest_value, &["identity", "scenario"])
            .or_else(|| json_string(&manifest_value, &["scenario"]));
        let status_value = json_string(&manifest_value, &["runtime", "exec_status"])
            .or_else(|| json_string(&manifest_value, &["exec_status"]))
            .or_else(|| json_string(&manifest_value, &["status"]));
        let exec_status = parse_exec_status(status_value.as_deref());
        if suite.is_none() {
            warnings.push(AdapterWarning {
                code: "missing_suite".to_string(),
                message: "LocalAgent artifact did not provide a suite; leaving it unset."
                    .to_string(),
            });
        }
        if scenario.is_none() {
            warnings.push(AdapterWarning {
                code: "missing_scenario".to_string(),
                message: "LocalAgent artifact did not provide a scenario; leaving it unset."
                    .to_string(),
            });
        }
        if json_string(&manifest_value, &["runtime", "started_at"])
            .or_else(|| json_string(&manifest_value, &["started_at"]))
            .is_none()
        {
            warnings.push(AdapterWarning {
                code: "missing_started_at".to_string(),
                message: "LocalAgent artifact did not provide started_at; leaving it unset."
                    .to_string(),
            });
        }
        if json_string(&manifest_value, &["runtime", "finished_at"])
            .or_else(|| json_string(&manifest_value, &["finished_at"]))
            .is_none()
        {
            warnings.push(AdapterWarning {
                code: "missing_finished_at".to_string(),
                message: "LocalAgent artifact did not provide finished_at; leaving it unset."
                    .to_string(),
            });
        }
        if status_value.is_some() && matches!(exec_status, ExecStatus::Unknown) {
            warnings.push(AdapterWarning {
                code: "unrecognized_exec_status".to_string(),
                message:
                    "LocalAgent artifact exposed an unrecognized exec status; normalized to unknown."
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
                    slug: json_string(&manifest_value, &["project", "slug"])
                        .or_else(|| json_string(&manifest_value, &["project_slug"]))
                        .unwrap_or_else(|| "localagent".to_string()),
                    display_name: json_string(&manifest_value, &["project", "display_name"])
                        .unwrap_or_else(|| "LocalAgent".to_string()),
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
                    label: json_string(&manifest_value, &["identity", "label"])
                        .or_else(|| json_string(&manifest_value, &["label"])),
                },
                git: Some(GitContext {
                    commit_sha: json_string(&manifest_value, &["git", "commit_sha"])
                        .or_else(|| json_string(&manifest_value, &["commit_sha"])),
                    branch: json_string(&manifest_value, &["git", "branch"])
                        .or_else(|| json_string(&manifest_value, &["branch"])),
                    dirty: json_bool(&manifest_value, &["git", "dirty"])
                        .or_else(|| json_bool(&manifest_value, &["dirty"])),
                }),
                runtime: RuntimeContext {
                    started_at: json_string(&manifest_value, &["runtime", "started_at"])
                        .or_else(|| json_string(&manifest_value, &["started_at"])),
                    finished_at: json_string(&manifest_value, &["runtime", "finished_at"])
                        .or_else(|| json_string(&manifest_value, &["finished_at"])),
                    duration_ms: json_u64(&manifest_value, &["runtime", "duration_ms"])
                        .or_else(|| json_u64(&manifest_value, &["duration_ms"])),
                    exit_code: json_i32(&manifest_value, &["runtime", "exit_code"])
                        .or_else(|| json_i32(&manifest_value, &["exit_code"])),
                    exec_status,
                },
                environment: None,
                workload: Some(WorkloadContext {
                    dataset: json_string(&manifest_value, &["workload", "dataset"])
                        .or_else(|| json_string(&manifest_value, &["dataset"])),
                    input_count: json_u64(&manifest_value, &["workload", "input_count"])
                        .or_else(|| json_u64(&manifest_value, &["input_count"])),
                    command_argv: json_string_array(&manifest_value, &["workload", "command_argv"])
                        .or_else(|| json_string_array(&manifest_value, &["command"]))
                        .unwrap_or_default(),
                    display_command: json_string(&manifest_value, &["workload", "display_command"])
                        .or_else(|| json_string(&manifest_value, &["display_command"])),
                    cwd: json_string(&manifest_value, &["workload", "cwd"])
                        .or_else(|| json_string(&manifest_value, &["cwd"])),
                    env_snapshot_ref: None,
                }),
                summary: SummaryContext {
                    error_count: json_u64(&manifest_value, &["summary", "error_count"])
                        .or_else(|| json_u64(&manifest_value, &["error_count"]))
                        .unwrap_or(0) as u32,
                    warning_count: json_u64(&manifest_value, &["summary", "warning_count"])
                        .or_else(|| json_u64(&manifest_value, &["warning_count"]))
                        .unwrap_or(warnings.len() as u64) as u32,
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
        "localagent_run.json",
        "localagent_manifest.json",
        "localagent_report.json",
        "run.json",
        "report.json",
    ]
}

fn detect_manifest_signature(path: &Path) -> Result<bool, RunScopeError> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;

    if json_string(&value, &["project", "slug"])
        .map(|value| value.eq_ignore_ascii_case("localagent"))
        .unwrap_or(false)
    {
        return Ok(true);
    }
    if json_string(&value, &["project", "display_name"])
        .map(|value| value.eq_ignore_ascii_case("LocalAgent"))
        .unwrap_or(false)
    {
        return Ok(true);
    }
    if json_string_array(&value, &["command"])
        .or_else(|| json_string_array(&value, &["workload", "command_argv"]))
        .and_then(|argv| argv.first().cloned())
        .map(|value| value.eq_ignore_ascii_case("localagent"))
        .unwrap_or(false)
    {
        return Ok(true);
    }

    let has_localagent_filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase().starts_with("localagent_"))
        .unwrap_or(false);
    let has_expected_shape = value.get("metrics").is_some()
        || value.get("metric_map").is_some()
        || value.get("suite").is_some()
        || value.get("scenario").is_some()
        || value.get("command").is_some()
        || value.get("workload").is_some();

    Ok(has_localagent_filename && has_expected_shape)
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
                RunScopeError::ManifestValidation("invalid LocalAgent source path".to_string())
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
        let role = if lower_name.contains("manifest") {
            "raw_source_manifest"
        } else {
            "report_json"
        };
        return ("raw", role.to_string(), "application/json".to_string());
    }
    if extension == "html" {
        return ("raw", "report_html".to_string(), "text/html".to_string());
    }
    (
        "raw",
        "input_manifest".to_string(),
        match extension.as_str() {
            "log" | "txt" => "text/plain".to_string(),
            "png" => "image/png".to_string(),
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "mp4" => "video/mp4".to_string(),
            _ => "application/octet-stream".to_string(),
        },
    )
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
        let parsed: Vec<_> = metrics
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
        if parsed.len() != metrics.len() {
            warnings.push(AdapterWarning {
                code: "ignored_metric_rows".to_string(),
                message: "LocalAgent artifact included metric rows that could not be normalized."
                    .to_string(),
            });
        }
        return parsed;
    }

    if let Some(metric_map) = value.get("metric_map").and_then(Value::as_object) {
        return metric_map
            .iter()
            .filter_map(|(key, raw)| {
                raw.as_f64().map(|value_num| MetricRecord {
                    key: key.clone(),
                    group_name: String::new(),
                    value_num: Some(value_num),
                    value_text: None,
                    unit: None,
                    direction: MetricDirection::None,
                    is_primary: false,
                })
            })
            .collect();
    }

    warnings.push(AdapterWarning {
        code: "missing_metrics".to_string(),
        message: "LocalAgent artifact did not expose metrics in a recognized format.".to_string(),
    });
    Vec::new()
}

fn adapter_payload(object: Option<&Map<String, Value>>) -> BTreeMap<String, Value> {
    let mut payload = BTreeMap::new();
    let Some(object) = object else {
        return payload;
    };

    let mut localagent = Map::new();
    for key in ["engine", "pipeline", "variant"] {
        if let Some(value) = object.get(key) {
            localagent.insert(key.to_string(), value.clone());
        }
    }
    if !localagent.is_empty() {
        payload.insert("localagent".to_string(), Value::Object(localagent));
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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detect_is_conservative_for_generic_run_json() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("run.json"),
            r#"{"project":{"slug":"other"},"suite":"smoke"}"#,
        )
        .unwrap();

        let detected = LocalAgentAdapter.detect(temp.path()).unwrap();

        assert!(!detected);
    }

    #[test]
    fn detect_accepts_localagent_manifest_signature() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("localagent_run.json"),
            r#"{"project":{"slug":"localagent"},"command":["localagent","eval"]}"#,
        )
        .unwrap();

        let detected = LocalAgentAdapter.detect(temp.path()).unwrap();

        assert!(detected);
    }

    #[test]
    fn parse_warns_for_ignored_metric_rows() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("localagent_run.json"),
            r#"{
                "project":{"slug":"localagent","display_name":"LocalAgent"},
                "suite":"eval",
                "scenario":"smoke",
                "status":"pass",
                "metrics":[
                    {"key":"score","value_num":1.0,"direction":"higher_is_better"},
                    {"value_num":2.0}
                ]
            }"#,
        )
        .unwrap();

        let parsed = LocalAgentAdapter.parse(temp.path()).unwrap();

        assert_eq!(parsed.manifest.metrics.len(), 1);
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "ignored_metric_rows"));
    }
}
