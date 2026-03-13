use crate::state::AppState;
use runscope_core::domain::{BaselineBinding, SetBaselineRequest};
use runscope_core::services::BaselineService;
use tauri::State;

#[tauri::command]
pub fn set_active_baseline(
    req: SetBaselineRequest,
    state: State<'_, AppState>,
) -> Result<BaselineBinding, String> {
    BaselineService::set_active_baseline(&state.paths, req).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_baselines(
    project_slug: String,
    state: State<'_, AppState>,
) -> Result<Vec<BaselineBinding>, String> {
    BaselineService::list_baselines(&state.paths, &project_slug).map_err(|error| error.to_string())
}
