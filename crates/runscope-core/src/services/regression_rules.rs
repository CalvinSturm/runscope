use super::{AppPaths, QueryService};
use crate::db::connection::open_connection;
use crate::db::migrations::apply_migrations;
use crate::domain::{
    ComparisonScope, CreateRegressionRuleRequest, RegressionComparator, RegressionRule,
};
use crate::error::RunScopeError;
use rusqlite::params;

pub struct RegressionRuleService;

impl RegressionRuleService {
    pub fn create_rule(
        paths: &AppPaths,
        req: CreateRegressionRuleRequest,
    ) -> Result<RegressionRule, RunScopeError> {
        let detail = QueryService::get_run(paths, &req.run_id)?;
        let scope = ComparisonScope::from_manifest(&detail.manifest);
        let scope_json = serde_json::to_string(&scope)?;
        let scope_hash = scope.scope_hash()?;
        let label = normalized_label(&req.label);

        let mut conn = open_connection(&paths.db_path)?;
        apply_migrations(&conn)?;
        let tx = conn.transaction()?;

        let project_id: i64 = tx.query_row(
            "SELECT projects.id
             FROM runs
             JOIN projects ON projects.id = runs.project_id
             WHERE runs.id = ?1",
            params![req.run_id],
            |row| row.get(0),
        )?;

        tx.execute(
            "INSERT INTO regression_rules (
                project_id, label, scope_json, scope_hash, metric_key, comparator, threshold_value, enabled, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)",
            params![
                project_id,
                label,
                scope_json,
                scope_hash,
                req.metric_key,
                comparator_to_str(&req.comparator),
                req.threshold_value,
                detail.manifest.source.ingested_at
            ],
        )?;

        let id = tx.last_insert_rowid();
        tx.commit()?;

        Ok(RegressionRule {
            id,
            project_slug: detail.manifest.project.slug,
            label: label.to_string(),
            scope,
            scope_hash,
            metric_key: req.metric_key,
            comparator: req.comparator,
            threshold_value: req.threshold_value,
            enabled: true,
            created_at: detail.manifest.source.ingested_at,
        })
    }

    pub fn list_rules(
        paths: &AppPaths,
        project_slug: &str,
    ) -> Result<Vec<RegressionRule>, RunScopeError> {
        let conn = open_connection(&paths.db_path)?;
        apply_migrations(&conn)?;
        let mut statement = conn.prepare(
            "SELECT
                regression_rules.id,
                projects.slug,
                regression_rules.label,
                regression_rules.scope_json,
                regression_rules.scope_hash,
                regression_rules.metric_key,
                regression_rules.comparator,
                regression_rules.threshold_value,
                regression_rules.enabled,
                regression_rules.created_at
             FROM regression_rules
             JOIN projects ON projects.id = regression_rules.project_id
             WHERE projects.slug = ?1 AND regression_rules.enabled = 1
             ORDER BY regression_rules.created_at DESC, regression_rules.id DESC",
        )?;

        let rows = statement.query_map(params![project_slug], |row| {
            let scope_json: String = row.get(3)?;
            Ok(RegressionRule {
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
                metric_key: row.get(5)?,
                comparator: parse_comparator(row.get::<_, String>(6)?.as_str())?,
                threshold_value: row.get(7)?,
                enabled: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
            })
        })?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(row?);
        }
        Ok(rules)
    }
}

pub fn comparator_to_str(value: &RegressionComparator) -> &'static str {
    match value {
        RegressionComparator::PctDropGt => "pct_drop_gt",
        RegressionComparator::PctIncreaseGt => "pct_increase_gt",
        RegressionComparator::AbsDeltaGt => "abs_delta_gt",
        RegressionComparator::AbsDeltaLt => "abs_delta_lt",
    }
}

pub fn parse_comparator(value: &str) -> Result<RegressionComparator, rusqlite::Error> {
    match value {
        "pct_drop_gt" => Ok(RegressionComparator::PctDropGt),
        "pct_increase_gt" => Ok(RegressionComparator::PctIncreaseGt),
        "abs_delta_gt" => Ok(RegressionComparator::AbsDeltaGt),
        "abs_delta_lt" => Ok(RegressionComparator::AbsDeltaLt),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid regression comparator: {value}"),
            )),
        )),
    }
}

fn normalized_label(label: &str) -> &str {
    let trimmed = label.trim();
    if trimmed.is_empty() { "default" } else { trimmed }
}
