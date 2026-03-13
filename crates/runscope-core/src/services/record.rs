use super::ingest::{generate_ulid_like, now_utc_rfc3339, AppPaths};
use crate::adapters::SourceFile;
use crate::db::{connection::open_connection, insert_recorded_run, migrations::apply_migrations};
use crate::domain::{
    EnvironmentContext, ExecStatus, GitContext, MetricDirection, MetricRecord, ProjectRef,
    RunIdentity, RunManifestV1, RunSource, RuntimeContext, SourceKind, SummaryContext,
    WorkloadContext, RUN_SCHEMA_VERSION,
};
use crate::error::RunScopeError;
use crate::store::{
    canonical_json_sha256, copied_artifacts_to_records, infer_media_type_from_path, ArtifactStore,
};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ManualRecordRequest {
    pub project_slug: String,
    pub project_display_name: Option<String>,
    pub exec_status: ExecStatus,
    pub suite: Option<String>,
    pub scenario: Option<String>,
    pub label: Option<String>,
    pub commit_sha: Option<String>,
    pub branch: Option<String>,
    pub git_dirty: Option<bool>,
    pub machine_name: Option<String>,
    pub os: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub precision: Option<String>,
    pub dataset: Option<String>,
    pub input_count: Option<u64>,
    pub command_argv: Vec<String>,
    pub display_command: Option<String>,
    pub cwd: Option<String>,
    pub env_snapshot_file: Option<PathBuf>,
    pub metrics: Vec<MetricRecord>,
    pub attachments: Vec<ManualAttachment>,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ManualAttachment {
    pub role: String,
    pub path: PathBuf,
    pub media_type: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordResult {
    pub run_id: String,
    pub project_slug: String,
    pub adapter: String,
    pub ingest_fingerprint: String,
    pub artifact_root: Option<String>,
    pub warnings: Vec<crate::domain::AdapterWarning>,
    pub duplicate: bool,
    pub dry_run: bool,
}

pub struct RecordService;

impl RecordService {
    pub fn record_manual(
        paths: &AppPaths,
        req: ManualRecordRequest,
    ) -> Result<RecordResult, RunScopeError> {
        fs::create_dir_all(&paths.data_dir)?;
        let mut conn = open_connection(&paths.db_path)?;
        apply_migrations(&conn)?;

        let run_id = generate_ulid_like();
        let ingested_at = now_utc_rfc3339();
        let project_display_name = req
            .project_display_name
            .clone()
            .unwrap_or_else(|| req.project_slug.clone());
        let source_files = build_manual_source_files(&req)?;
        let env_snapshot_ref = source_files
            .iter()
            .find(|file| file.role == "env_snapshot")
            .map(|file| file.target_rel_path.clone());
        let git = optional_git_context(&req);
        let environment = optional_environment_context(&req);
        let workload = optional_workload_context(&req, env_snapshot_ref);

        let mut manifest = RunManifestV1 {
            schema_version: RUN_SCHEMA_VERSION.to_string(),
            run_id: run_id.clone(),
            project: ProjectRef {
                slug: req.project_slug.clone(),
                display_name: project_display_name,
            },
            source: RunSource {
                adapter: "manual".to_string(),
                source_kind: SourceKind::ManualRecord,
                source_path: None,
                external_run_id: None,
                ingested_at: ingested_at.clone(),
            },
            identity: RunIdentity {
                suite: normalize_string(req.suite.clone()),
                scenario: normalize_string(req.scenario.clone()),
                label: normalize_string(req.label.clone()),
            },
            git,
            runtime: RuntimeContext {
                started_at: None,
                finished_at: None,
                duration_ms: None,
                exit_code: None,
                exec_status: req.exec_status.clone(),
            },
            environment,
            workload,
            summary: SummaryContext {
                error_count: 0,
                warning_count: 0,
            },
            metrics: req.metrics,
            artifacts: source_files
                .iter()
                .map(|file| crate::domain::ArtifactRecord {
                    role: file.role.clone(),
                    rel_path: file.target_rel_path.clone(),
                    media_type: file.media_type.clone(),
                    sha256: None,
                    size_bytes: None,
                })
                .collect(),
            adapter_payload: Default::default(),
        };
        manifest.validate()?;

        let fingerprint = compute_manual_record_fingerprint(&run_id)?;
        let store = ArtifactStore::new(&paths.data_dir);
        let layout = store.prepare_run_layout(
            &manifest.project.slug,
            &manifest.source.ingested_at,
            &manifest.run_id,
        )?;
        let copied = store.copy_source_files(&layout, &source_files)?;
        manifest.artifacts = copied_artifacts_to_records(copied);
        manifest.validate()?;
        store.write_run_manifest(&layout, &manifest)?;

        insert_recorded_run(
            &mut conn,
            &manifest,
            &[],
            &fingerprint,
            &req.tags,
            req.note.as_deref(),
        )?;

        Ok(RecordResult {
            run_id,
            project_slug: manifest.project.slug,
            adapter: manifest.source.adapter,
            ingest_fingerprint: fingerprint,
            artifact_root: Some(layout.run_root.display().to_string()),
            warnings: Vec::new(),
            duplicate: false,
            dry_run: false,
        })
    }
}

pub fn infer_metric_record(key: &str, value_num: f64) -> MetricRecord {
    MetricRecord {
        key: key.to_string(),
        group_name: String::new(),
        value_num: Some(value_num),
        value_text: None,
        unit: None,
        direction: MetricDirection::None,
        is_primary: false,
    }
}

fn build_manual_source_files(req: &ManualRecordRequest) -> Result<Vec<SourceFile>, RunScopeError> {
    let mut files = Vec::new();

    if let Some(env_snapshot_file) = &req.env_snapshot_file {
        let base_name = sanitize_file_name(env_snapshot_file, "env_snapshot");
        files.push(SourceFile {
            source_path: env_snapshot_file.clone(),
            target_rel_path: format!("attachments/env_snapshot_{base_name}"),
            role: "env_snapshot".to_string(),
            media_type: infer_media_type_from_path(env_snapshot_file).to_string(),
        });
    }

    for (index, attachment) in req.attachments.iter().enumerate() {
        if attachment.role.trim().is_empty() {
            return Err(RunScopeError::ManifestValidation(
                "manual attachment role must be non-empty".to_string(),
            ));
        }
        if attachment.media_type.trim().is_empty() {
            return Err(RunScopeError::ManifestValidation(
                "manual attachment media_type must be non-empty".to_string(),
            ));
        }

        let base_name = sanitize_file_name(&attachment.path, &format!("attachment_{index:02}"));
        files.push(SourceFile {
            source_path: attachment.path.clone(),
            target_rel_path: format!("attachments/{index:02}_{base_name}"),
            role: attachment.role.clone(),
            media_type: attachment.media_type.clone(),
        });
    }

    Ok(files)
}

fn compute_manual_record_fingerprint(run_id: &str) -> Result<String, RunScopeError> {
    #[derive(Serialize)]
    struct ManualFingerprint<'a> {
        source_kind: &'a str,
        run_id: &'a str,
    }

    canonical_json_sha256(&ManualFingerprint {
        source_kind: "manual_record",
        run_id,
    })
}

fn optional_git_context(req: &ManualRecordRequest) -> Option<GitContext> {
    let git = GitContext {
        commit_sha: normalize_string(req.commit_sha.clone()),
        branch: normalize_string(req.branch.clone()),
        dirty: req.git_dirty,
    };
    if git.commit_sha.is_none() && git.branch.is_none() && git.dirty.is_none() {
        None
    } else {
        Some(git)
    }
}

fn optional_environment_context(req: &ManualRecordRequest) -> Option<EnvironmentContext> {
    let environment = EnvironmentContext {
        machine_name: normalize_string(req.machine_name.clone()),
        os: normalize_string(req.os.clone()),
        cpu: normalize_string(req.cpu.clone()),
        gpu: normalize_string(req.gpu.clone()),
        backend: normalize_string(req.backend.clone()),
        model: normalize_string(req.model.clone()),
        precision: normalize_string(req.precision.clone()),
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

fn optional_workload_context(
    req: &ManualRecordRequest,
    env_snapshot_ref: Option<String>,
) -> Option<WorkloadContext> {
    let workload = WorkloadContext {
        dataset: normalize_string(req.dataset.clone()),
        input_count: req.input_count,
        command_argv: req.command_argv.clone(),
        display_command: normalize_string(req.display_command.clone()),
        cwd: normalize_string(req.cwd.clone()),
        env_snapshot_ref,
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

fn normalize_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn sanitize_file_name(path: &Path, fallback: &str) -> String {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(fallback);
    file_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
