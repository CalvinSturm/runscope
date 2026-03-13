use crate::domain::RunManifestV1;
use crate::error::RunScopeError;
use crate::store::canonical_json_sha256;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ComparisonScope {
    pub branch: Option<String>,
    pub suite: Option<String>,
    pub scenario: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub precision: Option<String>,
    pub dataset: Option<String>,
}

impl ComparisonScope {
    pub fn from_manifest(manifest: &RunManifestV1) -> Self {
        Self {
            branch: manifest.git.as_ref().and_then(|git| git.branch.clone()),
            suite: manifest.identity.suite.clone(),
            scenario: manifest.identity.scenario.clone(),
            backend: manifest
                .environment
                .as_ref()
                .and_then(|environment| environment.backend.clone()),
            model: manifest
                .environment
                .as_ref()
                .and_then(|environment| environment.model.clone()),
            precision: manifest
                .environment
                .as_ref()
                .and_then(|environment| environment.precision.clone()),
            dataset: manifest
                .workload
                .as_ref()
                .and_then(|workload| workload.dataset.clone()),
        }
    }

    pub fn scope_hash(&self) -> Result<String, RunScopeError> {
        canonical_json_sha256(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaselineBinding {
    pub id: i64,
    pub project_slug: String,
    pub label: String,
    pub scope: ComparisonScope,
    pub scope_hash: String,
    pub run_id: String,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetBaselineRequest {
    pub run_id: String,
    pub label: String,
}
