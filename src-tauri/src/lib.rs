//! Liscov - YouTube Live Chat Monitor
//! Tauri backend implementation

pub mod commands;
pub mod connection;
pub mod core;
pub mod database;
pub mod errors;
pub mod paths;
pub mod state;
pub mod tts;

pub use database::Database;
pub use state::AppState;

use tauri::Manager;
use tauri_plugin_window_state::StateFlags;

// Re-export command functions for registration
use commands::{
    ConfigState,
    auth_check_session_validity,
    auth_clear_webview_cookies,
    auth_delete_credentials,
    // Auth (spec: 01_auth.md)
    auth_get_status,
    auth_load_credentials,
    auth_open_window,
    auth_save_credentials,
    auth_save_raw_cookies,
    auth_use_fallback_storage,
    auth_validate_credentials,
    broadcaster_delete,
    broadcaster_get_list,
    config_get_value,
    // Config (spec: 09_config.md)
    config_load,
    config_save,
    config_set_value,
    // Chat (spec: 02_chat.md)
    connect_to_stream,
    disconnect_all_streams,
    disconnect_stream,
    export_current_messages,
    export_session_data,
    get_connection_reactions,
    get_connections,
    // Analytics (spec: 07_revenue.md)
    get_revenue_analytics,
    get_session_analytics,
    get_session_messages,
    // Database (spec: 08_database.md)
    get_sessions,
    get_top_contributors,
    // Raw Response (spec: 05_raw_response.md)
    raw_response_get_config,
    raw_response_resolve_path,
    raw_response_update_config,
    set_chat_mode,
    tts_clear_queue,
    tts_discover_exe,
    tts_get_config,
    tts_get_launch_status,
    tts_get_status,
    tts_kill_backend,
    tts_launch_backend,
    tts_select_exe,
    // TTS (spec: 04_tts.md)
    tts_speak,
    tts_speak_direct,
    tts_start,
    tts_stop,
    tts_test_connection,
    tts_update_config,
    viewer_delete,
    viewer_get_custom_info,
    viewer_get_list,
    // Viewer (spec: 06_viewer.md)
    viewer_get_profile,
    viewer_search,
    viewer_update_info,
    viewer_upsert_custom_info,
    websocket::start_websocket_server_auto,
    // WebSocket (spec: 03_websocket.md) - auto-start, no manual start/stop
    websocket_get_status,
};

// Simple greet command for testing
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Liscov.", name)
}

/// ログのプラグインを作る（FEATURE_SPECIFICATION.md「バックエンドのログ」）
///
/// `tracing::` のログも tracing の "log" feature で log に流れ、ここに集まる。
fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let mut targets = Vec::new();
    match paths::log_dir() {
        Ok(path) => targets.push(Target::new(TargetKind::Folder {
            path,
            file_name: Some("liscov".to_string()),
        })),
        // ログの置き場所が無くても起動はする（標準出力には出る）
        Err(e) => eprintln!(
            "ログディレクトリを特定できないため、ファイルに残さない: {}",
            e
        ),
    }
    if cfg!(debug_assertions) {
        targets.push(Target::new(TargetKind::Stdout));
    }

    tauri_plugin_log::Builder::default()
        .clear_targets()
        .targets(targets)
        .level(level)
        // HTTP 層の debug ログは量が多く、調べたいログが埋もれる
        .level_for("hyper", log::LevelFilter::Info)
        .level_for("hyper_util", log::LevelFilter::Info)
        .level_for("h2", log::LevelFilter::Info)
        .level_for("reqwest", log::LevelFilter::Info)
        .max_file_size(10 * 1024 * 1024)
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .manage(ConfigState::load_from_file())
        .plugin(log_plugin())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION)
                .build(),
        )
        .setup(|app| {
            // AppState はロガーより先に作られるので、DB 初期化の失敗はここで書き出す
            if let Some(e) = &app.state::<AppState>().database_init_error {
                log::error!(
                    "データベースの初期化に失敗したため、保存せずに動作する: {}",
                    e
                );
            }

            // Show window after state restoration (window starts hidden)
            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            window.show().expect("failed to show window");

            // Auto-start WebSocket server
            let app_handle = app.handle().clone();
            let state = app.state::<AppState>();
            let ws_server = state.websocket_server.clone();
            tauri::async_runtime::spawn(async move {
                start_websocket_server_auto(app_handle, ws_server).await;
            });

            // Auto-start TTS processing if enabled
            let tts_manager = state.tts_manager.clone();
            let tts_process_manager = state.tts_process_manager.clone();
            tauri::async_runtime::spawn(async move {
                let config = tts_manager.get_config().await;
                if config.enabled {
                    tts_manager.start_processing().await;
                }

                // Auto-launch TTS backends if enabled
                if config.bouyomichan.auto_launch {
                    let exe_path = config.bouyomichan.exe_path.as_deref();
                    if let Err(e) = tts_process_manager
                        .launch(tts::TtsBackendType::Bouyomichan, exe_path)
                        .await
                    {
                        log::error!("Failed to auto-launch Bouyomichan: {}", e);
                    }
                }
                if config.voicevox.auto_launch {
                    let exe_path = config.voicevox.exe_path.as_deref();
                    if let Err(e) = tts_process_manager
                        .launch(tts::TtsBackendType::Voicevox, exe_path)
                        .await
                    {
                        log::error!("Failed to auto-launch VOICEVOX: {}", e);
                    }
                }
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
            disconnect_all_streams,
            get_connections,
            get_connection_reactions,
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
            tts_discover_exe,
            tts_select_exe,
            tts_launch_backend,
            tts_kill_backend,
            tts_get_launch_status,
            // Viewer (spec: 06_viewer.md)
            viewer_get_profile,
            viewer_get_list,
            viewer_search,
            viewer_get_custom_info,
            viewer_upsert_custom_info,
            viewer_delete,
            broadcaster_get_list,
            broadcaster_delete,
            get_top_contributors,
            // Raw Response (spec: 05_raw_response.md)
            raw_response_get_config,
            raw_response_update_config,
            raw_response_resolve_path,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                // Kill auto-launched TTS processes on exit
                let state = app_handle.state::<AppState>();
                let tts_manager = state.tts_manager.clone();
                let tts_process_manager = state.tts_process_manager.clone();

                tauri::async_runtime::block_on(async move {
                    let config = tts_manager.get_config().await;

                    // Kill Bouyomichan if auto_close is enabled and it was launched
                    if config.bouyomichan.auto_close
                        && tts_process_manager
                            .is_launched(&tts::TtsBackendType::Bouyomichan)
                            .await
                    {
                        if let Err(e) = tts_process_manager
                            .kill(&tts::TtsBackendType::Bouyomichan)
                            .await
                        {
                            log::error!("Failed to kill Bouyomichan on exit: {}", e);
                        }
                    }

                    // Kill VOICEVOX if auto_close is enabled and it was launched
                    if config.voicevox.auto_close
                        && tts_process_manager
                            .is_launched(&tts::TtsBackendType::Voicevox)
                            .await
                    {
                        if let Err(e) = tts_process_manager
                            .kill(&tts::TtsBackendType::Voicevox)
                            .await
                        {
                            log::error!("Failed to kill VOICEVOX on exit: {}", e);
                        }
                    }
                });
            }
        });
}
