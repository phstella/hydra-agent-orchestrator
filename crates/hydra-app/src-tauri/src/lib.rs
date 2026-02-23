pub mod commands;
pub mod state;

use state::AppState;

/// Create and run the Tauri application.
///
/// # Errors
///
/// Returns an error if Tauri fails to initialize.
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::start_race,
            commands::get_doctor_report,
            commands::get_run_manifest,
            commands::get_run_events,
            commands::merge_dry_run,
            commands::merge_confirm,
            commands::get_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
