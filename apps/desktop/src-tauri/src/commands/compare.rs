use crate::state::AppState;
use runscope_core::domain::CompareReport;
use runscope_core::services::CompareService;
use tauri::State;

#[tauri::command]
pub fn compare_runs(
    left_run_id: String,
    right_run_id: String,
    state: State<'_, AppState>,
) -> Result<CompareReport, String> {
    CompareService::compare_runs(&state.paths, &left_run_id, &right_run_id)
        .map_err(|error| error.to_string())
}
