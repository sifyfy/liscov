//! TTS コマンドの統合テスト (04_tts.md)
//!
//! Tauri mock ランタイムを使って IPC コマンドレイヤーをテストする。
//! 各コマンドが TtsManager に正しく委譲することを検証する。

mod common;

use app_lib::commands::tts::{TtsConfigDto, TtsStatus};
use app_lib::state::AppState;
use app_lib::tts::{TtsBackend, TtsConfig, TtsManager, TtsProcessManager};
use app_lib::tts::backends::TtsError;
use async_trait::async_trait;
use common::{invoke_no_args, invoke_with_args};
use serial_test::serial;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets};
use tokio::sync::{Mutex, RwLock};

// ============================================================================
// MockTtsBackend（統合テスト用）
// ============================================================================

struct MockTtsBackend {
    speak_calls: Arc<Mutex<Vec<String>>>,
    connection_ok: bool,
}

impl MockTtsBackend {
    fn connected() -> Self {
        Self {
            speak_calls: Arc::new(Mutex::new(Vec::new())),
            connection_ok: true,
        }
    }
}

#[async_trait]
impl TtsBackend for MockTtsBackend {
    async fn test_connection(&self) -> Result<bool, TtsError> {
        Ok(self.connection_ok)
    }
    async fn speak(&self, text: &str) -> Result<(), TtsError> {
        self.speak_calls.lock().await.push(text.to_string());
        Ok(())
    }
    fn name(&self) -> &'static str {
        "Mock"
    }
}

// ============================================================================
// テストヘルパー
// ============================================================================

/// TTS 有効 + バックエンドなしの AppState を構築
fn app_state_tts_enabled() -> AppState {
    let tts_manager = TtsManager::new(TtsConfig {
        enabled: true,
        ..TtsConfig::default()
    });
    build_app_state(tts_manager)
}

/// モックバックエンド付きの AppState を構築
fn app_state_with_mock_backend() -> AppState {
    let tts_manager = TtsManager::with_backend(
        TtsConfig { enabled: true, ..TtsConfig::default() },
        Some(Box::new(MockTtsBackend::connected())),
    );
    build_app_state(tts_manager)
}

fn build_app_state(tts_manager: TtsManager) -> AppState {
    AppState {
        websocket_server: Arc::new(RwLock::new(None)),
        messages: Arc::new(RwLock::new(VecDeque::new())),
        database: Arc::new(RwLock::new(None)),
        tts_manager: Arc::new(tts_manager),
        tts_process_manager: Arc::new(TtsProcessManager::new()),
        next_connection_id: Arc::new(AtomicU64::new(0)),
        connections: Arc::new(RwLock::new(HashMap::new())),
    }
}

/// テスト用 Tauri app を構築
fn build_test_app(state: AppState) -> tauri::App<tauri::test::MockRuntime> {
    mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::tts::tts_speak,
            app_lib::commands::tts::tts_speak_direct,
            app_lib::commands::tts::tts_update_config,
            app_lib::commands::tts::tts_get_config,
            app_lib::commands::tts::tts_test_connection,
            app_lib::commands::tts::tts_start,
            app_lib::commands::tts::tts_stop,
            app_lib::commands::tts::tts_clear_queue,
            app_lib::commands::tts::tts_get_status,
            app_lib::commands::tts::tts_discover_exe,
            // tts_launch_backend, tts_kill_backend, tts_select_exe は
            // AppHandle を要求するため MockRuntime では使用不可
        ])
        .build(mock_context(noop_assets()))
        .expect("failed to build test app")
}

struct AppNameGuard;

impl AppNameGuard {
    fn new() -> Self {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::set_var("LISCOV_APP_NAME", "liscov-test-tts-cmd") };
        Self
    }
}

impl Drop for AppNameGuard {
    fn drop(&mut self) {
        // テスト後にテスト用設定ファイルをクリーンアップ
        if let Ok(config_path) = app_lib::paths::config_path() {
            let dir = config_path.parent().unwrap().to_path_buf();
            let _ = std::fs::remove_dir_all(&dir);
        }
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
    }
}

// ============================================================================
// tts_get_config（L209: replace with Ok(Default::default())）
// ============================================================================

#[test]
#[serial]
fn tts_get_config_returns_current_config() {
    // 仕様: update_config 後に get_config で変更された設定が取得できる
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_tts_enabled());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // まず設定を変更
    let dto = TtsConfigDto {
        enabled: true,
        max_text_length: 42,
        queue_size_limit: 7,
        ..TtsConfigDto::default()
    };
    let _ = get_ipc_response(
        &webview,
        invoke_with_args("tts_update_config", serde_json::json!({ "config": dto })),
    );

    // get_config で変更値を確認
    let response = get_ipc_response(&webview, invoke_no_args("tts_get_config"));
    assert!(response.is_ok(), "tts_get_config failed: {:?}", response.err());

    let config: TtsConfigDto = response.unwrap().deserialize().unwrap();
    assert_eq!(config.max_text_length, 42);
    assert_eq!(config.queue_size_limit, 7);
}

// ============================================================================
// tts_update_config（L191: replace with Ok(())）
// ============================================================================

#[test]
#[serial]
fn tts_update_config_updates_config() {
    // 仕様: update_config は設定を更新する（get_config で確認可能）
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_tts_enabled());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let dto = TtsConfigDto {
        enabled: false,
        read_author_name: false,
        ..TtsConfigDto::default()
    };
    let response = get_ipc_response(
        &webview,
        invoke_with_args("tts_update_config", serde_json::json!({ "config": dto })),
    );
    assert!(response.is_ok(), "tts_update_config failed: {:?}", response.err());

    // 変更が反映されていることを確認
    let get_response = get_ipc_response(&webview, invoke_no_args("tts_get_config"));
    let config: TtsConfigDto = get_response.unwrap().deserialize().unwrap();
    assert!(!config.enabled);
    assert!(!config.read_author_name);
}

// ============================================================================
// tts_speak（L161: replace with Ok(())）
// ============================================================================

#[test]
#[serial]
fn tts_speak_enqueues_message() {
    // 仕様: tts_speak はメッセージをキューに追加する
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_tts_enabled());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // speak で enqueue
    let response = get_ipc_response(
        &webview,
        invoke_with_args("tts_speak", serde_json::json!({
            "text": "テスト発話"
        })),
    );
    assert!(response.is_ok());

    // get_status でキューサイズを確認
    let status_response = get_ipc_response(&webview, invoke_no_args("tts_get_status"));
    let status: TtsStatus = status_response.unwrap().deserialize().unwrap();
    assert_eq!(status.queue_size, 1, "tts_speak should enqueue one message");
}

// ============================================================================
// tts_speak_direct（L178: replace with Ok(())）
// ============================================================================

#[test]
#[serial]
fn tts_speak_direct_returns_error_without_backend() {
    // 仕様: バックエンドが None の場合、speak_direct はエラーを返す
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_tts_enabled());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("tts_speak_direct", serde_json::json!({ "text": "テスト" })),
    );
    assert!(response.is_err(), "speak_direct should fail without backend");
}

// ============================================================================
// tts_test_connection（L219: replace with Ok(true) / Ok(false)）
// ============================================================================

#[test]
#[serial]
fn tts_test_connection_returns_false_without_backend() {
    // 仕様: バックエンドが None の場合、test_connection は false を返す
    // → Ok(true) mutant をkill
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_tts_enabled());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(&webview, invoke_no_args("tts_test_connection"));
    assert!(response.is_ok());
    let result: bool = response.unwrap().deserialize().unwrap();
    assert!(!result, "test_connection should return false without backend");
}

#[test]
#[serial]
fn tts_test_connection_returns_true_with_mock_backend() {
    // 仕様: モックバックエンドが接続成功を返す場合、test_connection は true を返す
    // → Ok(false) mutant をkill
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_with_mock_backend());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(&webview, invoke_no_args("tts_test_connection"));
    assert!(response.is_ok());
    let result: bool = response.unwrap().deserialize().unwrap();
    assert!(result, "test_connection should return true with connected mock backend");
}

// ============================================================================
// tts_start（L240: replace with Ok(())）
// ============================================================================

#[test]
#[serial]
fn tts_start_sets_processing_state() {
    // 仕様: tts_start 後に get_status で is_processing=true になる
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_with_mock_backend());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // 初期状態: is_processing=false
    let status: TtsStatus = get_ipc_response(&webview, invoke_no_args("tts_get_status"))
        .unwrap()
        .deserialize()
        .unwrap();
    assert!(!status.is_processing);

    // start
    let response = get_ipc_response(&webview, invoke_no_args("tts_start"));
    assert!(response.is_ok());

    // is_processing=true
    let status: TtsStatus = get_ipc_response(&webview, invoke_no_args("tts_get_status"))
        .unwrap()
        .deserialize()
        .unwrap();
    assert!(status.is_processing, "is_processing should be true after tts_start");

    // cleanup: stop
    let _ = get_ipc_response(&webview, invoke_no_args("tts_stop"));
    std::thread::sleep(std::time::Duration::from_millis(200));
}

// ============================================================================
// tts_stop（L247: replace with Ok(())）
// ============================================================================

#[test]
#[serial]
fn tts_stop_clears_processing_state() {
    // 仕様: tts_start → tts_stop 後に is_processing=false になる
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_with_mock_backend());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // start → is_processing=true
    let _ = get_ipc_response(&webview, invoke_no_args("tts_start"));
    let status: TtsStatus = get_ipc_response(&webview, invoke_no_args("tts_get_status"))
        .unwrap()
        .deserialize()
        .unwrap();
    assert!(status.is_processing, "precondition: is_processing should be true");

    // stop
    let response = get_ipc_response(&webview, invoke_no_args("tts_stop"));
    assert!(response.is_ok());

    // 処理タスクが停止するのを待つ
    std::thread::sleep(std::time::Duration::from_millis(300));

    let status: TtsStatus = get_ipc_response(&webview, invoke_no_args("tts_get_status"))
        .unwrap()
        .deserialize()
        .unwrap();
    assert!(!status.is_processing, "is_processing should be false after tts_stop");
}

// ============================================================================
// tts_clear_queue（L254: replace with Ok(())）
// ============================================================================

#[test]
#[serial]
fn tts_clear_queue_empties_queue() {
    // 仕様: tts_clear_queue 後にキューサイズが 0 になる
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_tts_enabled());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // まずメッセージを enqueue
    for i in 0..3 {
        let _ = get_ipc_response(
            &webview,
            invoke_with_args("tts_speak", serde_json::json!({ "text": format!("msg{}", i) })),
        );
    }

    // enqueue されたことを確認
    let status: TtsStatus = get_ipc_response(&webview, invoke_no_args("tts_get_status"))
        .unwrap()
        .deserialize()
        .unwrap();
    assert_eq!(status.queue_size, 3, "precondition: queue should have 3 items");

    // clear
    let response = get_ipc_response(&webview, invoke_no_args("tts_clear_queue"));
    assert!(response.is_ok());

    // キューが空になったことを確認
    let status: TtsStatus = get_ipc_response(&webview, invoke_no_args("tts_get_status"))
        .unwrap()
        .deserialize()
        .unwrap();
    assert_eq!(status.queue_size, 0, "queue should be empty after clear");
}

// ============================================================================
// tts_discover_exe（L317: replace with Ok(None) / Ok(Some(...))）
// ============================================================================

#[test]
#[serial]
fn tts_discover_exe_returns_none_for_invalid_backend() {
    // 仕様: 無効なバックエンド文字列 → Ok(None)
    let _guard = AppNameGuard::new();
    let app = build_test_app(app_state_tts_enabled());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("tts_discover_exe", serde_json::json!({ "backend": "invalid" })),
    );
    assert!(response.is_ok());
    let result: Option<String> = response.unwrap().deserialize().unwrap();
    assert_eq!(result, None, "invalid backend should return None");
}

// tts_launch_backend / tts_kill_backend / tts_select_exe は
// AppHandle を要求するため MockRuntime ではテスト不可。
// これらのコマンドの mutant は統合テストのスコープ外とする。
