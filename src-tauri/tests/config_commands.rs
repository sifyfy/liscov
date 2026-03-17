//! config コマンドの統合テスト (09_config.md)
//!
//! Tauri mock ランタイムを使って IPC コマンドレイヤーをテストする。
//! 環境変数 LISCOV_APP_NAME でテスト用の設定ディレクトリを分離し、本番データを保護する。

use app_lib::commands::config::{ConfigState, Config, StorageMode, Theme};
use serial_test::serial;
use std::fs;
use tauri::test::{mock_builder, mock_context, noop_assets, get_ipc_response, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::ipc::{InvokeBody, CallbackFn};

// ============================================================================
// テストヘルパー
// ============================================================================

/// テスト用に Tauri app + webview を構築するヘルパー
fn build_test_app() -> (
    tauri::App<tauri::test::MockRuntime>,
) {
    let app = mock_builder()
        .manage(ConfigState::new())
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::config::config_load,
            app_lib::commands::config::config_save,
            app_lib::commands::config::config_get_value,
            app_lib::commands::config::config_set_value,
        ])
        .build(mock_context(noop_assets()))
        .expect("failed to build test app");
    (app,)
}

/// IPC リクエストを組み立てるヘルパー（引数なし）
fn invoke_no_args(cmd: &str) -> InvokeRequest {
    InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(),
        body: InvokeBody::default(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

/// IPC リクエストを組み立てるヘルパー（JSON 引数あり）
fn invoke_with_args(cmd: &str, args: serde_json::Value) -> InvokeRequest {
    InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(),
        body: InvokeBody::Json(args),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

/// テスト用の app_name を設定し、テスト後に環境変数を削除するガード。
/// serial() と組み合わせて使用することで、環境変数の競合を防ぐ。
struct AppNameGuard;

impl AppNameGuard {
    fn new() -> Self {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::set_var("LISCOV_APP_NAME", "liscov-test-config-cmd") };
        Self
    }
}

impl Drop for AppNameGuard {
    fn drop(&mut self) {
        // テスト後にテスト用設定ファイルをクリーンアップ
        if let Ok(config_path) = app_lib::paths::config_path() {
            let _ = fs::remove_file(&config_path);
        }
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
    }
}

// ============================================================================
// config_load テスト
// ============================================================================

#[test]
#[serial]
fn config_load_returns_defaults_when_no_file() {
    // 仕様: config.toml が存在しない場合はデフォルト値を使用
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(
        &webview,
        invoke_no_args("config_load"),
    );

    assert!(response.is_ok(), "config_load should succeed: {:?}", response.err());

    let config: Config = response
        .unwrap()
        .deserialize()
        .expect("failed to deserialize Config");

    // 仕様: デフォルト値
    assert_eq!(config.storage.mode, StorageMode::Secure);
    assert_eq!(config.chat_display.message_font_size, 13);
    assert!(config.chat_display.show_timestamps);
    assert!(config.chat_display.auto_scroll_enabled);
    assert_eq!(config.ui.theme, Theme::Dark);
}

#[test]
#[serial]
fn config_load_reads_existing_file() {
    // 仕様: config.toml が存在する場合はファイルを読み込む
    let _guard = AppNameGuard::new();

    // config ファイルを事前に書き込む
    let config_path = app_lib::paths::config_path().expect("config_path should succeed");
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).expect("failed to create config dir");
    }
    let toml = r#"
[storage]
mode = "fallback"

[chat_display]
message_font_size = 18
show_timestamps = false
auto_scroll_enabled = false

[ui]
theme = "light"
"#;
    fs::write(&config_path, toml).expect("failed to write config file");

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(
        &webview,
        invoke_no_args("config_load"),
    );

    assert!(response.is_ok());

    let config: Config = response.unwrap().deserialize().expect("deserialize failed");

    assert_eq!(config.storage.mode, StorageMode::Fallback);
    assert_eq!(config.chat_display.message_font_size, 18);
    assert!(!config.chat_display.show_timestamps);
    assert!(!config.chat_display.auto_scroll_enabled);
    assert_eq!(config.ui.theme, Theme::Light);

    // クリーンアップ
    let _ = fs::remove_file(&config_path);
}

// ============================================================================
// config_save テスト
// ============================================================================

#[test]
#[serial]
fn config_save_updates_state_and_writes_file() {
    // 仕様: config_save は State を更新し config.toml に書き込む
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // カスタム設定を保存
    let config_to_save = serde_json::json!({
        "config": {
            "storage": { "mode": "fallback" },
            "chat_display": {
                "message_font_size": 20,
                "show_timestamps": false,
                "auto_scroll_enabled": false
            },
            "ui": { "theme": "light" }
        }
    });

    let response = get_ipc_response(
        &webview,
        invoke_with_args("config_save", config_to_save),
    );

    assert!(response.is_ok(), "config_save should succeed: {:?}", response.err());

    // ファイルが書き込まれたことを確認
    let config_path = app_lib::paths::config_path().expect("config_path should succeed");
    assert!(config_path.exists(), "config file should be written");

    // ファイルの内容が正しいことを確認
    let content = fs::read_to_string(&config_path).expect("failed to read config file");
    assert!(content.contains("fallback"), "file should contain storage mode");
    assert!(content.contains("20"), "file should contain font size");
    assert!(content.contains("light"), "file should contain theme");
    // クリーンアップは AppNameGuard::Drop で行う
}

// ============================================================================
// config_get_value テスト
// ============================================================================

#[test]
#[serial]
fn config_get_value_returns_default_storage_mode() {
    // 仕様: config_get_value は State の値を返す
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("config_get_value", serde_json::json!({
            "section": "storage",
            "key": "mode"
        })),
    );

    assert!(response.is_ok(), "config_get_value should succeed");

    let value: Option<serde_json::Value> = response
        .unwrap()
        .deserialize()
        .expect("deserialize failed");

    assert_eq!(value, Some(serde_json::json!("secure")));
}

#[test]
#[serial]
fn config_get_value_returns_default_font_size() {
    // 仕様: デフォルトのフォントサイズは 13
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("config_get_value", serde_json::json!({
            "section": "chat_display",
            "key": "message_font_size"
        })),
    );

    assert!(response.is_ok());
    let value: Option<serde_json::Value> = response.unwrap().deserialize().unwrap();
    assert_eq!(value, Some(serde_json::json!(13)));
}

#[test]
#[serial]
fn config_get_value_unknown_section_returns_none() {
    // 仕様: 未知のセクション → None
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("config_get_value", serde_json::json!({
            "section": "nonexistent",
            "key": "key"
        })),
    );

    assert!(response.is_ok());
    let value: Option<serde_json::Value> = response.unwrap().deserialize().unwrap();
    assert_eq!(value, None);
}

// ============================================================================
// config_set_value テスト
// ============================================================================

#[test]
#[serial]
fn config_set_value_updates_state_and_saves() {
    // 仕様: config_set_value は State を更新し、ファイルに保存する
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // テーマを light に変更
    let set_response = get_ipc_response(
        &webview,
        invoke_with_args("config_set_value", serde_json::json!({
            "section": "ui",
            "key": "theme",
            "value": "light"
        })),
    );

    assert!(set_response.is_ok(), "config_set_value should succeed: {:?}", set_response.err());

    // State が更新されたことを config_get_value で確認
    let get_response = get_ipc_response(
        &webview,
        invoke_with_args("config_get_value", serde_json::json!({
            "section": "ui",
            "key": "theme"
        })),
    );

    assert!(get_response.is_ok());
    let value: Option<serde_json::Value> = get_response.unwrap().deserialize().unwrap();
    assert_eq!(value, Some(serde_json::json!("light")));
    // クリーンアップは AppNameGuard::Drop で行う
}

#[test]
#[serial]
fn config_set_value_font_size_valid_boundary() {
    // 仕様: フォントサイズ 10-24 が有効
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    for size in [10u32, 24] {
        let response = get_ipc_response(
            &webview,
            invoke_with_args("config_set_value", serde_json::json!({
                "section": "chat_display",
                "key": "message_font_size",
                "value": size
            })),
        );
        assert!(
            response.is_ok(),
            "font size {} should be valid, got: {:?}", size, response.err()
        );
    }
    // クリーンアップは AppNameGuard::Drop で行う
}

#[test]
#[serial]
fn config_set_value_font_size_out_of_range_returns_error() {
    // 仕様: フォントサイズ範囲外（9, 25）はエラー
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    for size in [9u32, 25] {
        let response = get_ipc_response(
            &webview,
            invoke_with_args("config_set_value", serde_json::json!({
                "section": "chat_display",
                "key": "message_font_size",
                "value": size
            })),
        );
        assert!(
            response.is_err(),
            "font size {} should be rejected", size
        );
    }
}

#[test]
#[serial]
fn config_set_value_unknown_section_returns_error() {
    // 仕様: 未知のセクションはエラー
    let _guard = AppNameGuard::new();

    let (app,) = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("config_set_value", serde_json::json!({
            "section": "nonexistent",
            "key": "key",
            "value": "value"
        })),
    );

    assert!(response.is_err(), "unknown section should return error");
}
