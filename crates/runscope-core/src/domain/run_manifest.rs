use crate::error::RunScopeError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub const RUN_SCHEMA_VERSION: &str = "runscope.run.v1";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecStatus {
    Pass,
    Fail,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    ArtifactDir,
    ManualRecord,
    ImportedManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
    Target,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunManifestV1 {
    pub schema_version: String,
    pub run_id: String,
    pub project: ProjectRef,
    pub source: RunSource,
    pub identity: RunIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitContext>,
    pub runtime: RuntimeContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload: Option<WorkloadContext>,
    pub summary: SummaryContext,
    #[serde(default)]
    #[schemars(required)]
    pub metrics: Vec<MetricRecord>,
    #[serde(default)]
    #[schemars(required)]
    pub artifacts: Vec<ArtifactRecord>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub adapter_payload: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectRef {
    pub slug: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunSource {
    pub adapter: String,
    pub source_kind: SourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_run_id: Option<String>,
    pub ingested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct RunIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct GitContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub exec_status: ExecStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkloadContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_argv: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_snapshot_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SummaryContext {
    pub error_count: u32,
    pub warning_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MetricRecord {
    pub key: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub group_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_num: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub direction: MetricDirection,
    #[serde(default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    pub role: String,
    pub rel_path: String,
    pub media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

impl RunManifestV1 {
    pub fn validate(&self) -> Result<(), RunScopeError> {
        if self.schema_version != RUN_SCHEMA_VERSION {
            return Err(RunScopeError::ManifestValidation(
                "schema_version must equal runscope.run.v1".to_string(),
            ));
        }
        if self.run_id.trim().is_empty() {
            return Err(RunScopeError::ManifestValidation(
                "run_id must be non-empty".to_string(),
            ));
        }
        if self.project.slug.trim().is_empty() {
            return Err(RunScopeError::ManifestValidation(
                "project.slug must be non-empty".to_string(),
            ));
        }
        if self.source.adapter.trim().is_empty() {
            return Err(RunScopeError::ManifestValidation(
                "source.adapter must be non-empty".to_string(),
            ));
        }

        validate_utc_rfc3339("source.ingested_at", &self.source.ingested_at)?;
        if let Some(started_at) = &self.runtime.started_at {
            validate_utc_rfc3339("runtime.started_at", started_at)?;
        }
        if let Some(finished_at) = &self.runtime.finished_at {
            validate_utc_rfc3339("runtime.finished_at", finished_at)?;
        }

        for metric in &self.metrics {
            if metric.key.trim().is_empty() {
                return Err(RunScopeError::ManifestValidation(
                    "metric key must be non-empty".to_string(),
                ));
            }
            if metric.value_num.is_none() && metric.value_text.is_none() {
                return Err(RunScopeError::ManifestValidation(format!(
                    "metric '{}' must include value_num or value_text",
                    metric.key
                )));
            }
        }

        for artifact in &self.artifacts {
            if artifact.rel_path.trim().is_empty() {
                return Err(RunScopeError::ManifestValidation(
                    "artifact rel_path must be non-empty".to_string(),
                ));
            }
            if is_absolute_or_non_relative(&artifact.rel_path) {
                return Err(RunScopeError::ManifestValidation(format!(
                    "artifact path must be relative: {}",
                    artifact.rel_path
                )));
            }
            if !is_lower_snake_case(&artifact.role) {
                return Err(RunScopeError::ManifestValidation(format!(
                    "artifact role must be lower snake case: {}",
                    artifact.role
                )));
            }
            if artifact.media_type.trim().is_empty() {
                return Err(RunScopeError::ManifestValidation(
                    "artifact media_type must be non-empty".to_string(),
                ));
            }
        }

        if let Some(workload) = &self.workload {
            if let Some(env_snapshot_ref) = &workload.env_snapshot_ref {
                if env_snapshot_ref.trim().is_empty() {
                    return Err(RunScopeError::ManifestValidation(
                        "env_snapshot_ref must be non-empty when present".to_string(),
                    ));
                }
                if is_absolute_or_non_relative(env_snapshot_ref) {
                    return Err(RunScopeError::ManifestValidation(format!(
                        "env_snapshot_ref must be relative: {}",
                        env_snapshot_ref
                    )));
                }
            }
        }

        if self.adapter_payload.len() > 1 {
            return Err(RunScopeError::ManifestValidation(
                "adapter_payload may contain at most one key".to_string(),
            ));
        }
        for key in self.adapter_payload.keys() {
            if !is_lower_snake_case(key) {
                return Err(RunScopeError::ManifestValidation(format!(
                    "adapter_payload key must be an adapter name: {}",
                    key
                )));
            }
            if key != &self.source.adapter {
                return Err(RunScopeError::ManifestValidation(format!(
                    "adapter_payload key must match source.adapter: {}",
                    key
                )));
            }
        }

        Ok(())
    }
}

fn validate_utc_rfc3339(field_name: &str, value: &str) -> Result<(), RunScopeError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| {
        RunScopeError::ManifestValidation(format!("{field_name} must be RFC3339 UTC"))
    })?;
    if parsed.offset().whole_seconds() != 0 {
        return Err(RunScopeError::ManifestValidation(format!(
            "{field_name} must be RFC3339 UTC"
        )));
    }
    Ok(())
}

fn is_absolute_or_non_relative(path: &str) -> bool {
    let path = Path::new(path);
    path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
}

fn is_lower_snake_case(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_'))
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> RunManifestV1 {
        RunManifestV1 {
            schema_version: RUN_SCHEMA_VERSION.to_string(),
            run_id: "01TESTULID0000000000000000".to_string(),
            project: ProjectRef {
                slug: "localagent".to_string(),
                display_name: "LocalAgent".to_string(),
            },
            source: RunSource {
                adapter: "localagent".to_string(),
                source_kind: SourceKind::ArtifactDir,
                source_path: Some("fixture/localagent".to_string()),
                external_run_id: None,
                ingested_at: "2026-03-05T17:20:31Z".to_string(),
            },
            identity: RunIdentity::default(),
            git: None,
            runtime: RuntimeContext {
                started_at: Some("2026-03-05T17:00:00Z".to_string()),
                finished_at: Some("2026-03-05T17:01:00Z".to_string()),
                duration_ms: Some(60_000),
                exit_code: Some(0),
                exec_status: ExecStatus::Pass,
            },
            environment: None,
            workload: None,
            summary: SummaryContext {
                error_count: 0,
                warning_count: 0,
            },
            metrics: vec![MetricRecord {
                key: "fps".to_string(),
                group_name: String::new(),
                value_num: Some(42.0),
                value_text: None,
                unit: Some("frames/s".to_string()),
                direction: MetricDirection::HigherIsBetter,
                is_primary: true,
            }],
            artifacts: vec![ArtifactRecord {
                role: "stdout_log".to_string(),
                rel_path: "logs/stdout.log".to_string(),
                media_type: "text/plain".to_string(),
                sha256: None,
                size_bytes: None,
            }],
            adapter_payload: BTreeMap::new(),
        }
    }

    #[test]
    fn run_manifest_validate_rejects_absolute_artifact_paths() {
        let mut manifest = sample_manifest();
        manifest.artifacts[0].rel_path = "C:/absolute/path.log".to_string();

        let error = manifest.validate().unwrap_err();

        assert!(error.to_string().contains("artifact path must be relative"));
    }

    #[test]
    fn run_manifest_validate_rejects_missing_metric_values() {
        let mut manifest = sample_manifest();
        manifest.metrics[0].value_num = None;

        let error = manifest.validate().unwrap_err();

        assert!(error.to_string().contains("value_num or value_text"));
    }

    #[test]
    fn run_manifest_validate_rejects_non_utc_timestamp() {
        let mut manifest = sample_manifest();
        manifest.source.ingested_at = "2026-03-05T17:20:31-08:00".to_string();

        let error = manifest.validate().unwrap_err();

        assert!(error
            .to_string()
            .contains("source.ingested_at must be RFC3339 UTC"));
    }

    #[test]
    fn run_manifest_validate_rejects_absolute_env_snapshot_ref() {
        let mut manifest = sample_manifest();
        manifest.workload = Some(WorkloadContext {
            env_snapshot_ref: Some("C:/secret/env.json".to_string()),
            ..WorkloadContext::default()
        });

        let error = manifest.validate().unwrap_err();

        assert!(error
            .to_string()
            .contains("env_snapshot_ref must be relative"));
    }

    #[test]
    fn run_manifest_validate_rejects_mismatched_adapter_payload_key() {
        let mut manifest = sample_manifest();
        manifest.adapter_payload.insert(
            "videoforge".to_string(),
            serde_json::json!({"engine": "v2"}),
        );

        let error = manifest.validate().unwrap_err();

        assert!(error
            .to_string()
            .contains("adapter_payload key must match source.adapter"));
    }
}
