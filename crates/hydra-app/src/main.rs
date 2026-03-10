#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hydra_core::config::HydraConfig;

fn main() {
    let config = HydraConfig::default();
    let app_state = hydra_app::AppState::new(config);
    let interactive_handle = app_state.interactive.clone();
    let file_watcher_handle = app_state.file_watcher.clone();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            hydra_app::health_check,
            hydra_app::run_preflight,
            hydra_app::list_adapters,
            hydra_app::get_working_tree_status,
            hydra_app::start_race,
            hydra_app::poll_race_events,
            hydra_app::get_race_result,
            hydra_app::get_candidate_diff,
            hydra_app::preview_merge,
            hydra_app::execute_merge,
            hydra_app::start_interactive_session,
            hydra_app::poll_interactive_events,
            hydra_app::write_interactive_input,
            hydra_app::resize_interactive_terminal,
            hydra_app::stop_interactive_session,
            hydra_app::remove_interactive_session,
            hydra_app::list_interactive_sessions,
            hydra_app::get_interactive_transport_diagnostics,
            hydra_app::list_directory,
            hydra_app::read_file_preview,
            hydra_app::start_file_watcher,
            hydra_app::poll_file_watch_events,
            hydra_app::stop_file_watcher,
        ])
        .on_window_event(move |_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Stop all file watchers
                let fw_handle = file_watcher_handle.clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(fw_handle.stop_all());
                });

                let handle = interactive_handle.clone();
                let completed = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(
                        hydra_app::shutdown_all_with_timeout(
                            &handle,
                            hydra_app::INTERACTIVE_SHUTDOWN_TIMEOUT,
                        ),
                    )
                });
                if !completed {
                    tracing::warn!(
                        timeout_secs = hydra_app::INTERACTIVE_SHUTDOWN_TIMEOUT.as_secs(),
                        "interactive shutdown timed out on window destroy"
                    );
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running hydra application");
}
