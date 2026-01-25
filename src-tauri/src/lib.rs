//! Liscov - YouTube Live Chat Monitor
//! Tauri backend implementation

pub mod commands;
pub mod core;
pub mod database;
pub mod state;
pub mod tts;

pub use database::Database;
pub use state::AppState;

use tauri::Manager;

// Re-export command functions for registration
use commands::{
    // Auth (spec: 01_auth.md)
    auth_get_status, auth_load_credentials, auth_save_raw_cookies, auth_save_credentials,
    auth_delete_credentials, auth_clear_webview_cookies, auth_validate_credentials,
    auth_open_window, auth_check_session_validity, auth_use_fallback_storage,
    // Chat (spec: 02_chat.md)
    connect_to_stream, disconnect_stream, get_chat_messages, set_chat_mode,
    // Config (spec: 09_config.md)
    config_load, config_save, config_get_value, config_set_value, ConfigState,
    // WebSocket (spec: 03_websocket.md) - auto-start, no manual start/stop
    websocket_get_status,
    websocket::start_websocket_server_auto,
    // Database (spec: 08_database.md)
    get_sessions, get_session_messages, viewer_update_info,
    // Analytics (spec: 07_revenue.md)
    get_revenue_analytics, get_session_analytics, export_session_data, export_current_messages,
    // TTS (spec: 04_tts.md)
    tts_speak, tts_speak_direct, tts_update_config, tts_get_config, tts_test_connection,
    tts_start, tts_stop, tts_clear_queue, tts_get_status,
    // Viewer (spec: 06_viewer.md)
    viewer_get_profile, viewer_get_list, viewer_search, viewer_upsert_custom_info,
    viewer_delete, broadcaster_get_list, broadcaster_delete, get_top_contributors,
    get_viewer_profile, search_viewers, // Backward compatibility aliases
    // Raw Response (spec: 05_raw_response.md)
    raw_response_get_config, raw_response_update_config, raw_response_resolve_path, SaveConfigState,
};

// Simple greet command for testing
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Liscov.", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .manage(ConfigState::default())
        .manage(SaveConfigState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Debug)
                        .build(),
                )?;
            }

            // Auto-start WebSocket server
            let app_handle = app.handle().clone();
            let state = app.state::<AppState>();
            let ws_server = state.websocket_server.clone();
            tauri::async_runtime::spawn(async move {
                start_websocket_server_auto(app_handle, ws_server).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Test
            greet,
            // Auth (spec: 01_auth.md)
            auth_get_status,
            auth_load_credentials,
            auth_save_raw_cookies,
            auth_save_credentials,
            auth_delete_credentials,
            auth_clear_webview_cookies,
            auth_validate_credentials,
            auth_open_window,
            auth_check_session_validity,
            auth_use_fallback_storage,
            // Chat (spec: 02_chat.md)
            connect_to_stream,
            disconnect_stream,
            get_chat_messages,
            set_chat_mode,
            // Config (spec: 09_config.md)
            config_load,
            config_save,
            config_get_value,
            config_set_value,
            // WebSocket (spec: 03_websocket.md) - auto-start only
            websocket_get_status,
            // Database (spec: 08_database.md)
            get_sessions,
            get_session_messages,
            viewer_update_info,
            // Analytics (spec: 07_revenue.md)
            get_revenue_analytics,
            get_session_analytics,
            export_session_data,
            export_current_messages,
            // TTS (spec: 04_tts.md)
            tts_speak,
            tts_speak_direct,
            tts_update_config,
            tts_get_config,
            tts_test_connection,
            tts_start,
            tts_stop,
            tts_clear_queue,
            tts_get_status,
            // Viewer (spec: 06_viewer.md)
            viewer_get_profile,
            viewer_get_list,
            viewer_search,
            viewer_upsert_custom_info,
            viewer_delete,
            broadcaster_get_list,
            broadcaster_delete,
            get_top_contributors,
            get_viewer_profile,   // Backward compatibility
            search_viewers,       // Backward compatibility
            // Raw Response (spec: 05_raw_response.md)
            raw_response_get_config,
            raw_response_update_config,
            raw_response_resolve_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
