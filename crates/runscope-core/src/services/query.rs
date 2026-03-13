use super::ingest::AppPaths;
use crate::db::connection::open_connection;
use crate::db::migrations::apply_migrations;
use crate::domain::{
    BaselineBinding, ComparisonScope, ExecStatus, MetricDirection, MetricRecord, NoteRecord,
    RunDetail, RunListFilter, RunListItem, RunListPage, RunManifestV1, WarningRecord,
};
use crate::error::RunScopeError;
use crate::store::managed_run_root;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use std::fs;

pub struct QueryService;

impl QueryService {
    pub fn list_runs(
        paths: &AppPaths,
        filter: RunListFilter,
    ) -> Result<RunListPage, RunScopeError> {
        let conn = open_connection(&paths.db_path)?;
        apply_migrations(&conn)?;

        let limit = normalized_limit(filter.limit);
        let offset = filter.offset;
        let (where_sql, values) = build_run_filter_clause(&filter);

        let total = query_total(&conn, &where_sql, &values)?;
        let items = query_run_items(&conn, &where_sql, values, limit, offset)?;

        Ok(RunListPage {
            items,
            total,
            limit,
            offset,
        })
    }

    pub fn get_run(paths: &AppPaths, run_id: &str) -> Result<RunDetail, RunScopeError> {
        let conn = open_connection(&paths.db_path)?;
        apply_migrations(&conn)?;

        let run_locator = conn
            .query_row(
                "SELECT projects.slug, runs.ingested_at
                 FROM runs
                 JOIN projects ON projects.id = runs.project_id
                 WHERE runs.id = ?1",
                params![run_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| RunScopeError::RunNotFound(run_id.to_string()))?;

        let run_root = managed_run_root(&paths.data_dir, &run_locator.0, &run_locator.1, run_id)?;
        let manifest_path = run_root.join("run.json");
        let manifest: RunManifestV1 = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;

        Ok(RunDetail {
            run_root: run_root.display().to_string(),
            active_baselines: load_active_baselines_for_manifest(&conn, &manifest)?,
            manifest,
            warnings: load_warnings(&conn, run_id)?,
            notes: load_notes(&conn, run_id)?,
            tags: load_tags(&conn, run_id)?,
        })
    }
}

fn query_total(conn: &Connection, where_sql: &str, values: &[Value]) -> Result<u32, RunScopeError> {
    let sql = format!(
        "SELECT COUNT(*)
         FROM runs
         JOIN projects ON projects.id = runs.project_id
         {where_sql}"
    );
    let total = conn.query_row(&sql, params_from_iter(values.iter()), |row| {
        row.get::<_, i64>(0)
    })?;
    Ok(total.max(0) as u32)
}

fn query_run_items(
    conn: &Connection,
    where_sql: &str,
    mut values: Vec<Value>,
    limit: u32,
    offset: u32,
) -> Result<Vec<RunListItem>, RunScopeError> {
    let sql = format!(
        "SELECT
            runs.id,
            projects.slug,
            runs.adapter_kind,
            runs.suite,
            runs.scenario,
            runs.label,
            runs.exec_status,
            runs.started_at,
            runs.finished_at,
            runs.duration_ms,
            runs.backend,
            runs.model,
            runs.precision,
            runs.warning_count
         FROM runs
         JOIN projects ON projects.id = runs.project_id
         {where_sql}
         ORDER BY COALESCE(runs.started_at, runs.ingested_at) DESC, runs.ingested_at DESC
         LIMIT ? OFFSET ?"
    );
    values.push(Value::Integer(limit as i64));
    values.push(Value::Integer(offset as i64));

    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            parse_exec_status(row.get::<_, String>(6)?.as_str())?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<i64>>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<String>>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, i64>(13)?,
        ))
    })?;

    let mut items = Vec::new();
    for row in rows {
        let (
            run_id,
            project_slug,
            adapter,
            suite,
            scenario,
            label,
            exec_status,
            started_at,
            finished_at,
            duration_ms,
            backend,
            model,
            precision,
            warning_count,
        ) = row?;
        items.push(RunListItem {
            primary_metrics: load_primary_metrics(conn, &run_id)?,
            tags: load_tags(conn, &run_id)?,
            run_id,
            project_slug,
            adapter,
            suite,
            scenario,
            label,
            exec_status,
            started_at,
            finished_at,
            duration_ms: duration_ms.map(|value| value.max(0) as u64),
            backend,
            model,
            precision,
            warning_count: warning_count.max(0) as u32,
        });
    }

    Ok(items)
}

fn build_run_filter_clause(filter: &RunListFilter) -> (String, Vec<Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    push_exact_filter(
        &mut clauses,
        &mut values,
        "projects.slug = ?",
        filter.project.as_deref(),
    );
    push_exact_filter(
        &mut clauses,
        &mut values,
        "runs.suite = ?",
        filter.suite.as_deref(),
    );
    push_exact_filter(
        &mut clauses,
        &mut values,
        "runs.scenario = ?",
        filter.scenario.as_deref(),
    );
    push_exact_filter(
        &mut clauses,
        &mut values,
        "runs.backend = ?",
        filter.backend.as_deref(),
    );
    push_exact_filter(
        &mut clauses,
        &mut values,
        "runs.model = ?",
        filter.model.as_deref(),
    );
    push_exact_filter(
        &mut clauses,
        &mut values,
        "runs.precision = ?",
        filter.precision.as_deref(),
    );

    if let Some(exec_status) = &filter.exec_status {
        clauses.push("runs.exec_status = ?".to_string());
        values.push(Value::Text(exec_status_to_str(exec_status).to_string()));
    }

    if let Some(query_text) = normalized_text(filter.query_text.as_deref()) {
        clauses.push(
            "(runs.id LIKE ? OR projects.slug LIKE ? OR COALESCE(runs.suite, '') LIKE ? OR COALESCE(runs.scenario, '') LIKE ? OR COALESCE(runs.label, '') LIKE ? OR COALESCE(runs.backend, '') LIKE ? OR COALESCE(runs.model, '') LIKE ?)"
                .to_string(),
        );
        let like_value = Value::Text(format!("%{query_text}%"));
        for _ in 0..7 {
            values.push(like_value.clone());
        }
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

fn push_exact_filter(
    clauses: &mut Vec<String>,
    values: &mut Vec<Value>,
    sql: &str,
    value: Option<&str>,
) {
    if let Some(value) = normalized_text(value) {
        clauses.push(sql.to_string());
        values.push(Value::Text(value.to_string()));
    }
}

fn load_primary_metrics(
    conn: &Connection,
    run_id: &str,
) -> Result<Vec<MetricRecord>, RunScopeError> {
    let mut statement = conn.prepare(
        "SELECT key, group_name, value_num, value_text, unit, direction, is_primary
         FROM metrics
         WHERE run_id = ?1 AND is_primary = 1
         ORDER BY display_order ASC, key ASC",
    )?;
    let rows = statement.query_map(params![run_id], |row| {
        Ok(MetricRecord {
            key: row.get(0)?,
            group_name: row.get(1)?,
            value_num: row.get(2)?,
            value_text: row.get(3)?,
            unit: row.get(4)?,
            direction: parse_metric_direction(row.get::<_, String>(5)?.as_str())?,
            is_primary: row.get::<_, i64>(6)? != 0,
        })
    })?;

    let mut metrics = Vec::new();
    for row in rows {
        metrics.push(row?);
    }
    Ok(metrics)
}

fn load_warnings(conn: &Connection, run_id: &str) -> Result<Vec<WarningRecord>, RunScopeError> {
    let mut statement = conn.prepare(
        "SELECT code, message, created_at
         FROM run_warnings
         WHERE run_id = ?1
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map(params![run_id], |row| {
        Ok(WarningRecord {
            code: row.get(0)?,
            message: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;

    let mut warnings = Vec::new();
    for row in rows {
        warnings.push(row?);
    }
    Ok(warnings)
}

fn load_notes(conn: &Connection, run_id: &str) -> Result<Vec<NoteRecord>, RunScopeError> {
    let mut statement = conn.prepare(
        "SELECT id, body, created_at
         FROM notes
         WHERE run_id = ?1
         ORDER BY created_at DESC, id DESC",
    )?;
    let rows = statement.query_map(params![run_id], |row| {
        Ok(NoteRecord {
            id: row.get(0)?,
            body: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;

    let mut notes = Vec::new();
    for row in rows {
        notes.push(row?);
    }
    Ok(notes)
}

fn load_tags(conn: &Connection, run_id: &str) -> Result<Vec<String>, RunScopeError> {
    let mut statement = conn.prepare(
        "SELECT tags.name
         FROM run_tags
         JOIN tags ON tags.id = run_tags.tag_id
         WHERE run_tags.run_id = ?1
         ORDER BY tags.name ASC",
    )?;
    let rows = statement.query_map(params![run_id], |row| row.get::<_, String>(0))?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row?);
    }
    Ok(tags)
}

fn load_active_baselines_for_manifest(
    conn: &Connection,
    manifest: &RunManifestV1,
) -> Result<Vec<BaselineBinding>, RunScopeError> {
    let scope = ComparisonScope::from_manifest(manifest);
    let scope_hash = scope.scope_hash()?;
    let mut statement = conn.prepare(
        "SELECT
            baseline_bindings.id,
            projects.slug,
            baseline_bindings.label,
            baseline_bindings.scope_json,
            baseline_bindings.scope_hash,
            baseline_bindings.run_id,
            baseline_bindings.active,
            baseline_bindings.created_at
         FROM baseline_bindings
         JOIN projects ON projects.id = baseline_bindings.project_id
         WHERE projects.slug = ?1 AND baseline_bindings.scope_hash = ?2 AND baseline_bindings.active = 1
         ORDER BY baseline_bindings.created_at DESC, baseline_bindings.id DESC",
    )?;
    let rows = statement.query_map(params![manifest.project.slug, scope_hash], |row| {
        let scope_json: String = row.get(3)?;
        Ok(BaselineBinding {
            id: row.get(0)?,
            project_slug: row.get(1)?,
            label: row.get(2)?,
            scope: serde_json::from_str(&scope_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?,
            scope_hash: row.get(4)?,
            run_id: row.get(5)?,
            active: row.get::<_, i64>(6)? != 0,
            created_at: row.get(7)?,
        })
    })?;

    let mut baselines = Vec::new();
    for row in rows {
        baselines.push(row?);
    }
    Ok(baselines)
}

fn normalized_text(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalized_limit(limit: u32) -> u32 {
    match limit {
        0 => 50,
        1..=200 => limit,
        _ => 200,
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

fn parse_exec_status(value: &str) -> Result<ExecStatus, rusqlite::Error> {
    match value {
        "pass" => Ok(ExecStatus::Pass),
        "fail" => Ok(ExecStatus::Fail),
        "error" => Ok(ExecStatus::Error),
        "unknown" => Ok(ExecStatus::Unknown),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid exec_status: {value}"),
            )),
        )),
    }
}

fn parse_metric_direction(value: &str) -> Result<MetricDirection, rusqlite::Error> {
    match value {
        "higher_is_better" => Ok(MetricDirection::HigherIsBetter),
        "lower_is_better" => Ok(MetricDirection::LowerIsBetter),
        "target" => Ok(MetricDirection::Target),
        "none" => Ok(MetricDirection::None),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid metric direction: {value}"),
            )),
        )),
    }
}
