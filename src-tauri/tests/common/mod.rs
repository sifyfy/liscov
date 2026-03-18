//! 統合テスト共通ヘルパー

use tauri::test::INVOKE_KEY;
use tauri::webview::InvokeRequest;
use tauri::ipc::{InvokeBody, CallbackFn};

/// IPC リクエストを組み立てるヘルパー（引数なし）
pub fn invoke_no_args(cmd: &str) -> InvokeRequest {
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
pub fn invoke_with_args(cmd: &str, args: serde_json::Value) -> InvokeRequest {
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
