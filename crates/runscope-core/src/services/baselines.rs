use super::{AppPaths, QueryService};
use crate::db::connection::open_connection;
use crate::db::migrations::apply_migrations;
use crate::domain::{BaselineBinding, ComparisonScope, SetBaselineRequest};
use crate::error::RunScopeError;
use rusqlite::{params, OptionalExtension};

pub struct BaselineService;

impl BaselineService {
    pub fn set_active_baseline(
        paths: &AppPaths,
        req: SetBaselineRequest,
    ) -> Result<BaselineBinding, RunScopeError> {
        let detail = QueryService::get_run(paths, &req.run_id)?;
        let scope = ComparisonScope::from_manifest(&detail.manifest);
        let scope_json = serde_json::to_string(&scope)?;
        let scope_hash = scope.scope_hash()?;
        let label = normalized_label(&req.label);

        let mut conn = open_connection(&paths.db_path)?;
        apply_migrations(&conn)?;
        let tx = conn.transaction()?;

        let project_id: i64 = tx
            .query_row(
                "SELECT projects.id
                 FROM runs
                 JOIN projects ON projects.id = runs.project_id
                 WHERE runs.id = ?1",
                params![req.run_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| RunScopeError::RunNotFound(req.run_id.clone()))?;

        tx.execute(
            "UPDATE baseline_bindings
             SET active = 0
             WHERE project_id = ?1 AND label = ?2 AND scope_hash = ?3 AND active = 1",
            params![project_id, label, scope_hash],
        )?;

        tx.execute(
            "INSERT INTO baseline_bindings (project_id, label, scope_json, scope_hash, run_id, active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
            params![
                project_id,
                label,
                scope_json,
                scope_hash,
                req.run_id,
                detail.manifest.source.ingested_at
            ],
        )?;

        let binding_id = tx.last_insert_rowid();
        tx.commit()?;

        Ok(BaselineBinding {
            id: binding_id,
            project_slug: detail.manifest.project.slug,
            label: label.to_string(),
            scope,
            scope_hash,
            run_id: req.run_id,
            active: true,
            created_at: detail.manifest.source.ingested_at,
        })
    }

    pub fn list_baselines(
        paths: &AppPaths,
        project_slug: &str,
    ) -> Result<Vec<BaselineBinding>, RunScopeError> {
        let conn = open_connection(&paths.db_path)?;
        apply_migrations(&conn)?;

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
             WHERE projects.slug = ?1 AND baseline_bindings.active = 1
             ORDER BY baseline_bindings.created_at DESC, baseline_bindings.id DESC",
        )?;

        let rows = statement.query_map(params![project_slug], |row| {
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

        let mut bindings = Vec::new();
        for row in rows {
            bindings.push(row?);
        }
        Ok(bindings)
    }
}

fn normalized_label(label: &str) -> &str {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        "default"
    } else {
        trimmed
    }
}
