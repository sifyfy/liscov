//! Liscov - YouTube Live Chat Monitor
//! Tauri backend implementation

pub mod commands;
pub mod core;
pub mod database;
pub mod state;
pub mod tts;

pub use database::Database;
pub use state::AppState;

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
    // WebSocket (spec: 03_websocket.md)
    websocket_start, websocket_stop, websocket_get_status,
    // Database (spec: 08_database.md)
    get_sessions, get_session_messages, upsert_viewer_custom_info, get_viewers_for_broadcaster,
    broadcaster_get_list, broadcaster_delete, viewer_delete, viewer_update_info,
    // Analytics (spec: 07_revenue.md)
    get_revenue_analytics, get_session_analytics, export_session_data, export_current_messages,
    // TTS (spec: 04_tts.md)
    tts_speak, tts_speak_direct, tts_update_config, tts_get_config, tts_test_connection,
    tts_start, tts_stop, tts_clear_queue, tts_get_status,
    // Viewer (spec: 06_viewer.md)
    get_viewer_profile, get_viewer_with_custom_info, search_viewers, get_top_contributors,
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
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
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
            // WebSocket (spec: 03_websocket.md)
            websocket_start,
            websocket_stop,
            websocket_get_status,
            // Database (spec: 08_database.md)
            get_sessions,
            get_session_messages,
            upsert_viewer_custom_info,
            get_viewers_for_broadcaster,
            broadcaster_get_list,
            broadcaster_delete,
            viewer_delete,
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
            get_viewer_profile,
            get_viewer_with_custom_info,
            search_viewers,
            get_top_contributors,
            // Raw Response (spec: 05_raw_response.md)
            raw_response_get_config,
            raw_response_update_config,
            raw_response_resolve_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
