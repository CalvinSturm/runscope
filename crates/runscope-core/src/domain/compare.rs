use crate::domain::MetricDirection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareReport {
    pub left_run_id: String,
    pub right_run_id: String,
    pub metadata_diffs: Vec<FieldDiff>,
    pub metric_diffs: Vec<MetricDiff>,
    pub artifact_diffs: Vec<ArtifactDiff>,
    pub regression_flags: Vec<RegressionFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldDiff {
    pub field: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MetricDiff {
    pub key: String,
    pub group_name: String,
    pub left_num: Option<f64>,
    pub right_num: Option<f64>,
    pub left_text: Option<String>,
    pub right_text: Option<String>,
    pub unit: Option<String>,
    pub direction: MetricDirection,
    pub abs_delta: Option<f64>,
    pub pct_delta: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDiff {
    pub role: String,
    pub left_rel_path: Option<String>,
    pub right_rel_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegressionComparator {
    PctDropGt,
    PctIncreaseGt,
    AbsDeltaGt,
    AbsDeltaLt,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegressionRule {
    pub id: i64,
    pub project_slug: String,
    pub label: String,
    pub scope: crate::domain::ComparisonScope,
    pub scope_hash: String,
    pub metric_key: String,
    pub comparator: RegressionComparator,
    pub threshold_value: f64,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateRegressionRuleRequest {
    pub run_id: String,
    pub label: String,
    pub metric_key: String,
    pub comparator: RegressionComparator,
    pub threshold_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegressionFlag {
    pub metric_key: String,
    pub comparator: RegressionComparator,
    pub threshold_value: f64,
    pub baseline_run_id: String,
    pub candidate_run_id: String,
    pub actual_value: Option<f64>,
    pub status: String,
    pub label: String,
}
