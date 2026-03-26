//! auth コマンドの統合テスト (01_auth.md)
//!
//! Fallbackモード + LISCOV_APP_NAME によるテストディレクトリ分離でkeyring不要のテストを実現する。
//! CREDENTIALS_CACHE はグローバルstaticだが、各テストで save/delete/load の順序制御で対処する。

mod common;

use app_lib::commands::config::{Config, ConfigState, StorageConfig, StorageMode};
use app_lib::state::AppState;
use common::{invoke_no_args, invoke_with_args};
use serial_test::serial;
use std::fs;
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets};

// ============================================================================
// テストヘルパー
// ============================================================================

/// テスト用に Tauri app + webview を構築するヘルパー
fn build_test_app() -> tauri::App<tauri::test::MockRuntime> {
    // ConfigState をFallbackモードで初期化
    let config_state = {
        let state = ConfigState::new();
        let mut config = Config::default();
        config.storage = StorageConfig { mode: StorageMode::Fallback };
        state.set(config);
        state
    };

    mock_builder()
        .manage(AppState::new())
        .manage(config_state)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::auth::auth_load_credentials,
            app_lib::commands::auth::auth_save_raw_cookies,
            app_lib::commands::auth::auth_save_credentials,
            app_lib::commands::auth::auth_delete_credentials,
            app_lib::commands::auth::auth_validate_credentials,
            app_lib::commands::auth::auth_get_status,
        ])
        .build(mock_context(noop_assets()))
        .expect("テスト用アプリのビルドに失敗")
}

/// テスト用の app_name を設定し、テスト後にクリーンアップするガード。
/// #[serial] と組み合わせて環境変数競合と CREDENTIALS_CACHE 汚染を防ぐ。
struct AppNameGuard;

impl AppNameGuard {
    fn new() -> Self {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::set_var("LISCOV_APP_NAME", "liscov-test-auth-cmd") };
        // テスト開始前にインメモリキャッシュと credentials ファイルを削除してクリーンな状態にする
        app_lib::commands::auth::clear_credentials_cache_for_test();
        if let Ok(cred_path) = app_lib::paths::credentials_path() {
            let _ = fs::remove_file(&cred_path);
        }
        Self
    }
}

impl Drop for AppNameGuard {
    fn drop(&mut self) {
        // テスト後に credentials ファイルをクリーンアップ
        if let Ok(cred_path) = app_lib::paths::credentials_path() {
            let _ = fs::remove_file(&cred_path);
        }
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
    }
}

// ============================================================================
// auth_save_raw_cookies テスト
// ============================================================================

#[tokio::test]
#[serial]
async fn auth_save_raw_cookies_valid_cookies_saves_to_file() {
    // 仕様: 有効なraw_cookiesをFallbackモードで保存し、ファイルに永続化される
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // 保存
    let save_resp = get_ipc_response(
        &webview,
        invoke_with_args(
            "auth_save_raw_cookies",
            serde_json::json!({ "rawCookies": "SID=s; HSID=h; SSID=ss; APISID=a; SAPISID=sa" }),
        ),
    );
    assert!(save_resp.is_ok(), "auth_save_raw_cookies should succeed: {:?}", save_resp.err());

    // ファイルが作成されたことを確認
    let cred_path = app_lib::paths::credentials_path().expect("credentials_path should succeed");
    assert!(cred_path.exists(), "credentials.toml should be created");

    // auth_load_credentials で読み戻せることを確認
    let load_resp = get_ipc_response(&webview, invoke_no_args("auth_load_credentials"));
    assert!(load_resp.is_ok(), "auth_load_credentials should succeed after save");
    let loaded: bool = load_resp.unwrap().deserialize().unwrap();
    assert!(loaded, "auth_load_credentials should return true");
}

#[tokio::test]
#[serial]
async fn auth_save_raw_cookies_missing_sapisid_returns_error() {
    // 仕様: SAPISID が含まれない raw_cookies はエラー
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let resp = get_ipc_response(
        &webview,
        invoke_with_args(
            "auth_save_raw_cookies",
            serde_json::json!({ "rawCookies": "SID=s; HSID=h" }),
        ),
    );
    assert!(resp.is_err(), "SAPISID欠如の場合はエラーを返すべき");
}

#[tokio::test]
#[serial]
async fn auth_save_raw_cookies_empty_string_returns_error() {
    // 仕様: 空文字の raw_cookies はエラー
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let resp = get_ipc_response(
        &webview,
        invoke_with_args(
            "auth_save_raw_cookies",
            serde_json::json!({ "rawCookies": "" }),
        ),
    );
    assert!(resp.is_err(), "空文字の場合はエラーを返すべき");
}

// ============================================================================
// auth_save_credentials テスト
// ============================================================================

#[tokio::test]
#[serial]
async fn auth_save_credentials_valid_saves() {
    // 仕様: 5つのCookie値を個別に渡し、Fallbackモードで保存成功
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let save_resp = get_ipc_response(
        &webview,
        invoke_with_args(
            "auth_save_credentials",
            serde_json::json!({
                "sid": "test_sid",
                "hsid": "test_hsid",
                "ssid": "test_ssid",
                "apisid": "test_apisid",
                "sapisid": "test_sapisid"
            }),
        ),
    );
    assert!(save_resp.is_ok(), "auth_save_credentials should succeed: {:?}", save_resp.err());

    // auth_validate_credentials で検証
    let validate_resp = get_ipc_response(&webview, invoke_no_args("auth_validate_credentials"));
    assert!(validate_resp.is_ok(), "auth_validate_credentials should succeed");
    let is_valid: bool = validate_resp.unwrap().deserialize().unwrap();
    assert!(is_valid, "保存後は validate が true を返すべき");
}

// ============================================================================
// auth_load_credentials テスト
// ============================================================================

#[tokio::test]
#[serial]
async fn auth_load_credentials_no_file_returns_error() {
    // 仕様: credentials.toml が存在しない場合はエラー（Fallbackモード）
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // ファイルが存在しないことを確認（ガードのコンストラクタで削除済み）
    let cred_path = app_lib::paths::credentials_path().expect("credentials_path should succeed");
    assert!(!cred_path.exists(), "テスト開始時はファイルが存在しないはず");

    let resp = get_ipc_response(&webview, invoke_no_args("auth_load_credentials"));
    // ファイルなし → AuthRequired エラー
    assert!(resp.is_err(), "ファイルなしの場合はエラーを返すべき");
}

#[tokio::test]
#[serial]
async fn auth_load_credentials_with_saved_credentials_returns_true() {
    // 仕様: 保存済みのcredentialsをロードすると true を返す
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // 事前に保存
    let save_resp = get_ipc_response(
        &webview,
        invoke_with_args(
            "auth_save_raw_cookies",
            serde_json::json!({ "rawCookies": "SID=s; HSID=h; SSID=ss; APISID=a; SAPISID=sa" }),
        ),
    );
    assert!(save_resp.is_ok(), "事前保存に失敗: {:?}", save_resp.err());

    // ロード
    let load_resp = get_ipc_response(&webview, invoke_no_args("auth_load_credentials"));
    assert!(load_resp.is_ok(), "auth_load_credentials should succeed");
    let loaded: bool = load_resp.unwrap().deserialize().unwrap();
    assert!(loaded, "保存後は load が true を返すべき");
}

// ============================================================================
// auth_delete_credentials テスト
// ============================================================================

#[tokio::test]
#[serial]
async fn auth_delete_credentials_removes_file() {
    // 仕様: save → delete → load でファイルが削除されている
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // 保存
    let save_resp = get_ipc_response(
        &webview,
        invoke_with_args(
            "auth_save_raw_cookies",
            serde_json::json!({ "rawCookies": "SID=s; HSID=h; SSID=ss; APISID=a; SAPISID=sa" }),
        ),
    );
    assert!(save_resp.is_ok(), "保存に失敗: {:?}", save_resp.err());

    // 削除
    let delete_resp = get_ipc_response(&webview, invoke_no_args("auth_delete_credentials"));
    assert!(delete_resp.is_ok(), "auth_delete_credentials should succeed: {:?}", delete_resp.err());

    // ファイルが削除されたことを確認
    let cred_path = app_lib::paths::credentials_path().expect("credentials_path should succeed");
    assert!(!cred_path.exists(), "削除後はファイルが存在しないはず");

    // ロードでエラーになることを確認（CREDENTIALS_CACHE もクリアされているはず）
    let load_resp = get_ipc_response(&webview, invoke_no_args("auth_load_credentials"));
    assert!(load_resp.is_err(), "削除後はロードがエラーを返すべき");
}

// ============================================================================
// auth_validate_credentials テスト
// ============================================================================

#[tokio::test]
#[serial]
async fn auth_validate_credentials_without_credentials_returns_error() {
    // 仕様: credentials なしで validate を呼ぶとエラー
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let resp = get_ipc_response(&webview, invoke_no_args("auth_validate_credentials"));
    assert!(resp.is_err(), "ファイルなしの場合はエラーを返すべき");
}

#[tokio::test]
#[serial]
async fn auth_validate_credentials_with_valid_credentials_returns_true() {
    // 仕様: 有効なcredentials保存後に validate は true を返す
    let _guard = AppNameGuard::new();

    let app = build_test_app();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // 保存
    let save_resp = get_ipc_response(
        &webview,
        invoke_with_args(
            "auth_save_credentials",
            serde_json::json!({
                "sid": "s",
                "hsid": "h",
                "ssid": "ss",
                "apisid": "a",
                "sapisid": "sa"
            }),
        ),
    );
    assert!(save_resp.is_ok(), "保存に失敗: {:?}", save_resp.err());

    // バリデーション
    let validate_resp = get_ipc_response(&webview, invoke_no_args("auth_validate_credentials"));
    assert!(validate_resp.is_ok(), "auth_validate_credentials should succeed");
    let is_valid: bool = validate_resp.unwrap().deserialize().unwrap();
    assert!(is_valid, "保存後は validate が true を返すべき");
}
