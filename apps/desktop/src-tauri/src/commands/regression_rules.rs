use crate::state::AppState;
use runscope_core::domain::{CreateRegressionRuleRequest, RegressionRule};
use runscope_core::services::RegressionRuleService;
use tauri::State;

#[tauri::command]
pub fn create_regression_rule(
    req: CreateRegressionRuleRequest,
    state: State<'_, AppState>,
) -> Result<RegressionRule, String> {
    RegressionRuleService::create_rule(&state.paths, req).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_regression_rules(
    project_slug: String,
    state: State<'_, AppState>,
) -> Result<Vec<RegressionRule>, String> {
    RegressionRuleService::list_rules(&state.paths, &project_slug).map_err(|error| error.to_string())
}
