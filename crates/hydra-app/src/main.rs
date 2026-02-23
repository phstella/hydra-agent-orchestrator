#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hydra_core::config::HydraConfig;

fn main() {
    let config = HydraConfig::default();
    let app_state = hydra_app::AppState::new(config);

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            hydra_app::health_check,
            hydra_app::run_preflight,
            hydra_app::list_adapters,
            hydra_app::start_race,
            hydra_app::poll_race_events,
            hydra_app::get_race_result,
        ])
        .run(tauri::generate_context!())
        .expect("error while running hydra application");
}
