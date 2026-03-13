use crate::state::AppState;
use runscope_core::domain::{RunDetail, RunListFilter, RunListPage};
use runscope_core::services::QueryService;
use tauri::State;

#[tauri::command]
pub fn list_runs(
    filter: Option<RunListFilter>,
    state: State<'_, AppState>,
) -> Result<RunListPage, String> {
    QueryService::list_runs(&state.paths, filter.unwrap_or_default())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_run(run_id: String, state: State<'_, AppState>) -> Result<RunDetail, String> {
    QueryService::get_run(&state.paths, &run_id).map_err(|error| error.to_string())
}
