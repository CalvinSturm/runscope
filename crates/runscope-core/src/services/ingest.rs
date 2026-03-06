use crate::adapters::select_adapter;
use crate::db::{connection::open_connection, migrations::apply_migrations};
use crate::db::{find_existing_run_by_fingerprint, insert_ingested_run};
use crate::domain::{AdapterWarning, RunManifestV1};
use crate::error::RunScopeError;
use crate::store::{
    canonical_json_sha256, copied_artifacts_to_records, managed_run_root, sha256_hex_dir,
    ArtifactStore,
};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use ulid::Ulid;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub db_path: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct IngestRequest {
    pub artifact_dir: PathBuf,
    pub adapter: Option<String>,
    pub project_override: Option<String>,
    pub label_override: Option<String>,
    pub tags: Vec<String>,
    pub note: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IngestResult {
    pub run_id: String,
    pub project_slug: String,
    pub adapter: String,
    pub ingest_fingerprint: String,
    pub artifact_root: Option<String>,
    pub warnings: Vec<AdapterWarning>,
    pub duplicate: bool,
    pub dry_run: bool,
}

pub struct IngestService;

impl IngestService {
    pub fn ingest_dir(paths: &AppPaths, req: IngestRequest) -> Result<IngestResult, RunScopeError> {
        fs::create_dir_all(&paths.data_dir)?;
        let mut conn = open_connection(&paths.db_path)?;
        apply_migrations(&conn)?;

        let adapter = select_adapter(req.adapter.as_deref(), &req.artifact_dir)?;
        let mut parsed = adapter.parse(&req.artifact_dir)?;

        apply_overrides(
            &mut parsed.manifest,
            req.project_override.as_deref(),
            req.label_override.as_deref(),
        );
        if parsed.manifest.run_id.trim().is_empty() {
            parsed.manifest.run_id = generate_ulid_like();
        }
        parsed.manifest.summary.warning_count = parsed.warnings.len() as u32;
        parsed.manifest.validate()?;

        let source_hash = sha256_hex_dir(&req.artifact_dir)?;
        let ingest_fingerprint = compute_ingest_fingerprint(&parsed.manifest, &source_hash)?;

        if let Some(existing) = find_existing_run_by_fingerprint(&conn, &ingest_fingerprint)? {
            let artifact_root = managed_run_root(
                &paths.data_dir,
                &existing.project_slug,
                &existing.ingested_at,
                &existing.run_id,
            )?;
            return Ok(IngestResult {
                run_id: existing.run_id,
                project_slug: existing.project_slug,
                adapter: parsed.manifest.source.adapter,
                ingest_fingerprint,
                artifact_root: Some(artifact_root.display().to_string()),
                warnings: parsed.warnings,
                duplicate: true,
                dry_run: req.dry_run,
            });
        }

        let artifact_root = managed_run_root(
            &paths.data_dir,
            &parsed.manifest.project.slug,
            &parsed.manifest.source.ingested_at,
            &parsed.manifest.run_id,
        )?;

        if req.dry_run {
            return Ok(IngestResult {
                run_id: parsed.manifest.run_id,
                project_slug: parsed.manifest.project.slug,
                adapter: parsed.manifest.source.adapter,
                ingest_fingerprint,
                artifact_root: Some(artifact_root.display().to_string()),
                warnings: parsed.warnings,
                duplicate: false,
                dry_run: true,
            });
        }

        let store = ArtifactStore::new(&paths.data_dir);
        let layout = store.prepare_run_layout(
            &parsed.manifest.project.slug,
            &parsed.manifest.source.ingested_at,
            &parsed.manifest.run_id,
        )?;
        let copied = store.copy_source_files(&layout, &parsed.files_to_copy)?;
        parsed.manifest.artifacts = copied_artifacts_to_records(copied);
        parsed.manifest.summary.warning_count = parsed.warnings.len() as u32;
        parsed.manifest.validate()?;
        store.write_run_manifest(&layout, &parsed.manifest)?;
        insert_ingested_run(
            &mut conn,
            &parsed.manifest,
            &parsed.warnings,
            &ingest_fingerprint,
            &source_hash,
            &req.tags,
            req.note.as_deref(),
        )?;

        Ok(IngestResult {
            run_id: parsed.manifest.run_id,
            project_slug: parsed.manifest.project.slug,
            adapter: parsed.manifest.source.adapter,
            ingest_fingerprint,
            artifact_root: Some(layout.run_root.display().to_string()),
            warnings: parsed.warnings,
            duplicate: false,
            dry_run: false,
        })
    }
}

pub fn now_utc_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("Rfc3339 formatting should succeed")
}

pub fn generate_ulid_like() -> String {
    Ulid::new().to_string()
}

fn apply_overrides(
    manifest: &mut RunManifestV1,
    project_override: Option<&str>,
    label_override: Option<&str>,
) {
    if let Some(project_override) = project_override {
        manifest.project.slug = project_override.to_string();
        manifest.project.display_name = project_override.to_string();
    }
    if let Some(label_override) = label_override {
        manifest.identity.label = Some(label_override.to_string());
    }
}

#[derive(Serialize)]
struct IngestFingerprint<'a> {
    adapter: &'a str,
    project_slug: &'a str,
    external_run_id: Option<&'a str>,
    source_hash: &'a str,
    started_at: Option<&'a str>,
    finished_at: Option<&'a str>,
    suite: Option<&'a str>,
    scenario: Option<&'a str>,
    label: Option<&'a str>,
}

fn compute_ingest_fingerprint(
    manifest: &RunManifestV1,
    source_hash: &str,
) -> Result<String, RunScopeError> {
    let data = IngestFingerprint {
        adapter: &manifest.source.adapter,
        project_slug: &manifest.project.slug,
        external_run_id: normalized_optional_str(manifest.source.external_run_id.as_deref()),
        source_hash,
        started_at: normalized_optional_str(manifest.runtime.started_at.as_deref()),
        finished_at: normalized_optional_str(manifest.runtime.finished_at.as_deref()),
        suite: normalized_optional_str(manifest.identity.suite.as_deref()),
        scenario: normalized_optional_str(manifest.identity.scenario.as_deref()),
        label: normalized_optional_str(manifest.identity.label.as_deref()),
    };
    canonical_json_sha256(&data)
}

fn normalized_optional_str(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}
