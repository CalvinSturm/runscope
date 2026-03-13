use crate::domain::{AdapterWarning, BaselineBinding, ExecStatus, MetricRecord, RunManifestV1};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default, deny_unknown_fields)]
pub struct RunListFilter {
    pub project: Option<String>,
    pub suite: Option<String>,
    pub scenario: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub precision: Option<String>,
    pub exec_status: Option<ExecStatus>,
    pub query_text: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunListItem {
    pub run_id: String,
    pub project_slug: String,
    pub adapter: String,
    pub suite: Option<String>,
    pub scenario: Option<String>,
    pub label: Option<String>,
    pub exec_status: ExecStatus,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub precision: Option<String>,
    pub warning_count: u32,
    pub primary_metrics: Vec<MetricRecord>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunListPage {
    pub items: Vec<RunListItem>,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WarningRecord {
    pub code: String,
    pub message: String,
    pub created_at: String,
}

impl From<AdapterWarning> for WarningRecord {
    fn from(value: AdapterWarning) -> Self {
        Self {
            code: value.code,
            message: value.message,
            created_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoteRecord {
    pub id: i64,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunDetail {
    pub run_root: String,
    pub manifest: RunManifestV1,
    pub warnings: Vec<WarningRecord>,
    pub notes: Vec<NoteRecord>,
    pub tags: Vec<String>,
    pub active_baselines: Vec<BaselineBinding>,
}
