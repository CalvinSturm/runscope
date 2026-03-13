mod commands;
mod state;

use state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState::new()?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::runs::list_runs,
            commands::runs::get_run,
            commands::compare::compare_runs,
            commands::baselines::set_active_baseline,
            commands::baselines::list_baselines,
            commands::regression_rules::create_regression_rule,
            commands::regression_rules::list_regression_rules,
            commands::system::open_path,
            commands::system::reveal_path
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RunScope desktop app");
}
