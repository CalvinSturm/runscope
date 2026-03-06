pub mod connection;
pub mod migrations;

use crate::domain::{AdapterWarning, ExecStatus, MetricDirection, RunManifestV1, SourceKind};
use crate::error::RunScopeError;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

#[derive(Debug, Clone)]
pub struct ExistingRunRecord {
    pub run_id: String,
    pub project_slug: String,
    pub ingested_at: String,
}

pub fn find_existing_run_by_fingerprint(
    conn: &Connection,
    fingerprint: &str,
) -> Result<Option<ExistingRunRecord>, RunScopeError> {
    let mut statement = conn.prepare(
        "SELECT runs.id, projects.slug, runs.ingested_at
         FROM runs
         JOIN projects ON projects.id = runs.project_id
         WHERE runs.ingest_fingerprint = ?1",
    )?;
    statement
        .query_row(params![fingerprint], |row| {
            Ok(ExistingRunRecord {
                run_id: row.get(0)?,
                project_slug: row.get(1)?,
                ingested_at: row.get(2)?,
            })
        })
        .optional()
        .map_err(RunScopeError::from)
}

pub fn insert_ingested_run(
    conn: &mut Connection,
    manifest: &RunManifestV1,
    warnings: &[AdapterWarning],
    ingest_fingerprint: &str,
    source_hash: &str,
    tags: &[String],
    note: Option<&str>,
) -> Result<(), RunScopeError> {
    let tx = conn.transaction()?;
    let project_id = ensure_project(
        &tx,
        &manifest.project.slug,
        &manifest.project.display_name,
        &manifest.source.ingested_at,
    )?;
    insert_run_row(&tx, project_id, manifest, ingest_fingerprint, source_hash)?;
    insert_metrics(&tx, manifest)?;
    insert_artifacts(&tx, manifest)?;
    insert_warnings(
        &tx,
        &manifest.run_id,
        warnings,
        &manifest.source.ingested_at,
    )?;
    insert_tags(&tx, &manifest.run_id, tags, &manifest.source.ingested_at)?;
    insert_note(&tx, &manifest.run_id, note, &manifest.source.ingested_at)?;
    tx.commit()?;
    Ok(())
}

pub fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, RunScopeError> {
    let exists = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1")?
        .query_row(params![table_name], |_| Ok(()))
        .optional()?
        .is_some();
    Ok(exists)
}

fn ensure_project(
    tx: &Transaction<'_>,
    slug: &str,
    display_name: &str,
    created_at: &str,
) -> Result<i64, RunScopeError> {
    tx.execute(
        "INSERT INTO projects (slug, display_name, created_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(slug) DO UPDATE SET display_name = excluded.display_name",
        params![slug, display_name, created_at],
    )?;

    let project_id = tx.query_row(
        "SELECT id FROM projects WHERE slug = ?1",
        params![slug],
        |row| row.get(0),
    )?;
    Ok(project_id)
}

fn insert_run_row(
    tx: &Transaction<'_>,
    project_id: i64,
    manifest: &RunManifestV1,
    ingest_fingerprint: &str,
    source_hash: &str,
) -> Result<(), RunScopeError> {
    let command_json = manifest
        .workload
        .as_ref()
        .map(|workload| serde_json::to_string(&workload.command_argv))
        .transpose()?;
    let raw_manifest_rel_path = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.role == "raw_source_manifest")
        .map(|artifact| artifact.rel_path.clone());

    tx.execute(
        "INSERT INTO runs (
            id, project_id, schema_version, adapter_kind, source_kind, source_path, external_run_id,
            suite, scenario, label, exec_status, started_at, finished_at, duration_ms, exit_code,
            git_commit_sha, git_branch, git_dirty, machine_name, os, cpu, gpu, backend, model,
            precision, dataset, input_count, command_json, display_command, cwd, env_snapshot_rel_path,
            raw_manifest_rel_path, error_count, warning_count, ingest_fingerprint, source_hash, ingested_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7,
            ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24,
            ?25, ?26, ?27, ?28, ?29, ?30, ?31,
            ?32, ?33, ?34, ?35, ?36, ?37
         )",
        params![
            manifest.run_id,
            project_id,
            manifest.schema_version,
            manifest.source.adapter,
            source_kind_to_str(&manifest.source.source_kind),
            manifest.source.source_path,
            manifest.source.external_run_id,
            manifest.identity.suite,
            manifest.identity.scenario,
            manifest.identity.label,
            exec_status_to_str(&manifest.runtime.exec_status),
            manifest.runtime.started_at,
            manifest.runtime.finished_at,
            manifest.runtime.duration_ms.map(|value| value as i64),
            manifest.runtime.exit_code,
            manifest.git.as_ref().and_then(|git| git.commit_sha.clone()),
            manifest.git.as_ref().and_then(|git| git.branch.clone()),
            manifest.git.as_ref().and_then(|git| git.dirty).unwrap_or(false) as i64,
            manifest.environment.as_ref().and_then(|env| env.machine_name.clone()),
            manifest.environment.as_ref().and_then(|env| env.os.clone()),
            manifest.environment.as_ref().and_then(|env| env.cpu.clone()),
            manifest.environment.as_ref().and_then(|env| env.gpu.clone()),
            manifest.environment.as_ref().and_then(|env| env.backend.clone()),
            manifest.environment.as_ref().and_then(|env| env.model.clone()),
            manifest.environment.as_ref().and_then(|env| env.precision.clone()),
            manifest.workload.as_ref().and_then(|workload| workload.dataset.clone()),
            manifest.workload.as_ref().and_then(|workload| workload.input_count.map(|value| value as i64)),
            command_json,
            manifest.workload.as_ref().and_then(|workload| workload.display_command.clone()),
            manifest.workload.as_ref().and_then(|workload| workload.cwd.clone()),
            manifest.workload.as_ref().and_then(|workload| workload.env_snapshot_ref.clone()),
            raw_manifest_rel_path,
            manifest.summary.error_count as i64,
            manifest.summary.warning_count as i64,
            ingest_fingerprint,
            source_hash,
            manifest.source.ingested_at,
        ],
    )?;
    Ok(())
}

fn insert_metrics(tx: &Transaction<'_>, manifest: &RunManifestV1) -> Result<(), RunScopeError> {
    let mut statement = tx.prepare(
        "INSERT INTO metrics (
            run_id, key, group_name, value_num, value_text, unit, direction, is_primary, display_order
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;

    for (index, metric) in manifest.metrics.iter().enumerate() {
        statement.execute(params![
            manifest.run_id,
            metric.key,
            metric.group_name,
            metric.value_num,
            metric.value_text,
            metric.unit,
            metric_direction_to_str(&metric.direction),
            metric.is_primary as i64,
            index as i64,
        ])?;
    }
    Ok(())
}

fn insert_artifacts(tx: &Transaction<'_>, manifest: &RunManifestV1) -> Result<(), RunScopeError> {
    let mut statement = tx.prepare(
        "INSERT INTO artifacts (run_id, role, rel_path, media_type, sha256, size_bytes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;

    for artifact in &manifest.artifacts {
        statement.execute(params![
            manifest.run_id,
            artifact.role,
            artifact.rel_path,
            artifact.media_type,
            artifact.sha256,
            artifact.size_bytes.map(|value| value as i64),
            manifest.source.ingested_at,
        ])?;
    }
    Ok(())
}

fn insert_warnings(
    tx: &Transaction<'_>,
    run_id: &str,
    warnings: &[AdapterWarning],
    created_at: &str,
) -> Result<(), RunScopeError> {
    let mut statement = tx.prepare(
        "INSERT INTO run_warnings (run_id, code, message, created_at)
         VALUES (?1, ?2, ?3, ?4)",
    )?;

    for warning in warnings {
        statement.execute(params![run_id, warning.code, warning.message, created_at])?;
    }
    Ok(())
}

fn insert_note(
    tx: &Transaction<'_>,
    run_id: &str,
    note: Option<&str>,
    created_at: &str,
) -> Result<(), RunScopeError> {
    let Some(note) = note.filter(|note| !note.trim().is_empty()) else {
        return Ok(());
    };

    tx.execute(
        "INSERT INTO notes (run_id, body, created_at) VALUES (?1, ?2, ?3)",
        params![run_id, note, created_at],
    )?;
    Ok(())
}

fn insert_tags(
    tx: &Transaction<'_>,
    run_id: &str,
    tags: &[String],
    created_at: &str,
) -> Result<(), RunScopeError> {
    for tag in tags {
        tx.execute(
            "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![tag],
        )?;
        let tag_id: i64 =
            tx.query_row("SELECT id FROM tags WHERE name = ?1", params![tag], |row| {
                row.get(0)
            })?;
        tx.execute(
            "INSERT INTO run_tags (run_id, tag_id, created_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(run_id, tag_id) DO NOTHING",
            params![run_id, tag_id, created_at],
        )?;
    }
    Ok(())
}

fn source_kind_to_str(value: &SourceKind) -> &'static str {
    match value {
        SourceKind::ArtifactDir => "artifact_dir",
        SourceKind::ManualRecord => "manual_record",
        SourceKind::ImportedManifest => "imported_manifest",
    }
}

fn exec_status_to_str(value: &ExecStatus) -> &'static str {
    match value {
        ExecStatus::Pass => "pass",
        ExecStatus::Fail => "fail",
        ExecStatus::Error => "error",
        ExecStatus::Unknown => "unknown",
    }
}

fn metric_direction_to_str(value: &MetricDirection) -> &'static str {
    match value {
        MetricDirection::HigherIsBetter => "higher_is_better",
        MetricDirection::LowerIsBetter => "lower_is_better",
        MetricDirection::Target => "target",
        MetricDirection::None => "none",
    }
}
