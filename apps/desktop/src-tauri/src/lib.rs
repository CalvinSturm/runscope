mod commands;
mod state;

use state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState::new(app.handle())?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::runs::list_runs,
            commands::runs::get_run
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RunScope desktop app");
}
