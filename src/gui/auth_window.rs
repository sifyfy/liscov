//! YouTube認証ウィンドウモジュール
//!
//! 別ウィンドウでYouTubeログインページを表示し、
//! ログイン完了後にCookieを取得する機能を提供します。

use crate::api::auth::{
    extract_youtube_cookies_from_wry, has_sapisid, CookieManager, YouTubeCookies, YOUTUBE_AUTH_URL,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::oneshot;

/// 認証ウィンドウのエラー型
#[derive(Error, Debug)]
pub enum AuthWindowError {
    #[error("Failed to create window: {0}")]
    WindowCreation(String),

    #[error("Failed to create WebView: {0}")]
    WebViewCreation(String),

    #[error("Cookie extraction failed: {0}")]
    CookieExtraction(String),

    #[error("Authentication timed out")]
    Timeout,

    #[error("Authentication cancelled by user")]
    Cancelled,

    #[error("Event loop error: {0}")]
    EventLoop(String),
}

/// 認証の結果
pub type AuthResult = Result<YouTubeCookies, AuthWindowError>;

/// 認証タイムアウト（秒）
const AUTH_TIMEOUT_SECS: u64 = 300; // 5分

/// Cookieポーリング間隔（ミリ秒）
const POLL_INTERVAL_MS: u64 = 1000; // 1秒

/// 認証ウィンドウを開いてYouTubeログインを行う
///
/// この関数は別スレッドでウィンドウを開き、ユーザーがログインするまで待機します。
/// ログインが完了すると（SAPISIDクッキーが検出されると）、必要なCookieを抽出して返します。
///
/// # Returns
/// - `Ok(YouTubeCookies)`: ログイン成功時のCookie
/// - `Err(AuthWindowError)`: エラー発生時
pub async fn open_auth_window() -> AuthResult {
    tracing::info!("🔐 Opening YouTube authentication window...");

    // 結果を受け取るためのチャンネル
    let (tx, rx) = oneshot::channel::<AuthResult>();

    // 別スレッドでウィンドウを作成・実行
    std::thread::spawn(move || {
        let result = run_auth_window_sync();
        let _ = tx.send(result);
    });

    // 結果を待機
    match rx.await {
        Ok(result) => result,
        Err(_) => Err(AuthWindowError::Cancelled),
    }
}

/// 同期的に認証ウィンドウを実行
fn run_auth_window_sync() -> AuthResult {
    use dioxus::desktop::tao::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        platform::run_return::EventLoopExtRunReturn,
        platform::windows::EventLoopBuilderExtWindows,
        window::WindowBuilder,
    };
    use dioxus::desktop::wry::WebViewBuilder;

    // イベントループを作成（別スレッドで実行するためany_threadを使用）
    let mut event_loop = EventLoopBuilder::new().with_any_thread(true).build();

    // ウィンドウを作成
    let window = WindowBuilder::new()
        .with_title("YouTube ログイン - liscov")
        .with_inner_size(LogicalSize::new(900.0, 700.0))
        .with_resizable(true)
        .build(&event_loop)
        .map_err(|e| AuthWindowError::WindowCreation(e.to_string()))?;

    tracing::info!("🪟 Auth window created");

    // 認証状態を共有するための変数
    let auth_result: Arc<Mutex<Option<AuthResult>>> = Arc::new(Mutex::new(None));
    let auth_result_clone = auth_result.clone();

    // WebViewを作成
    let webview = WebViewBuilder::new()
        .with_url(YOUTUBE_AUTH_URL)
        .build(&window)
        .map_err(|e| AuthWindowError::WebViewCreation(e.to_string()))?;

    tracing::info!("🌐 WebView created, navigating to YouTube...");

    let start_time = Instant::now();
    let webview = Arc::new(webview);
    let webview_clone = webview.clone();

    // Cookieポーリング用のタイマー
    let mut last_poll = Instant::now();

    // イベントループを実行（run_returnを使用して終了後も制御を戻す）
    event_loop.run_return(|event, _elwt, control_flow| {
        // デフォルトでPollモードを使用
        *control_flow = ControlFlow::Poll;

        // 既に結果が出ている場合は終了
        if auth_result_clone.lock().unwrap().is_some() {
            *control_flow = ControlFlow::Exit;
            return;
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                tracing::info!("🚪 Auth window closed by user");
                *auth_result_clone.lock().unwrap() = Some(Err(AuthWindowError::Cancelled));
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // Cookieポーリング（1秒間隔）
                if last_poll.elapsed() >= Duration::from_millis(POLL_INTERVAL_MS) {
                    last_poll = Instant::now();

                    // タイムアウトチェック
                    if start_time.elapsed() > Duration::from_secs(AUTH_TIMEOUT_SECS) {
                        tracing::warn!("⏰ Authentication timed out");
                        *auth_result_clone.lock().unwrap() = Some(Err(AuthWindowError::Timeout));
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    // Cookieをチェック
                    match check_youtube_cookies(&webview_clone) {
                        Ok(Some(cookies)) => {
                            tracing::info!("✅ Authentication successful! SAPISID detected.");

                            // Cookieを保存
                            if let Err(e) = save_cookies(&cookies) {
                                tracing::error!("Failed to save cookies: {}", e);
                            }

                            *auth_result_clone.lock().unwrap() = Some(Ok(cookies));
                            *control_flow = ControlFlow::Exit;
                        }
                        Ok(None) => {
                            // まだログインしていない
                            tracing::trace!("⏳ Waiting for login... ({:.0}s elapsed)", start_time.elapsed().as_secs_f32());
                        }
                        Err(e) => {
                            tracing::debug!("Cookie check error: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    });

    // 結果を取り出して返す
    let result = auth_result
        .lock()
        .unwrap()
        .take()
        .unwrap_or(Err(AuthWindowError::Cancelled));
    result
}

/// WebViewからYouTube Cookieをチェック
fn check_youtube_cookies(
    webview: &dioxus::desktop::wry::WebView,
) -> Result<Option<YouTubeCookies>, AuthWindowError> {
    // YouTubeドメインのCookieを取得
    let cookies = webview
        .cookies_for_url(YOUTUBE_AUTH_URL)
        .map_err(|e| AuthWindowError::CookieExtraction(e.to_string()))?;

    // SAPISIDが存在するかチェック
    if has_sapisid(&cookies) {
        // すべての必要なCookieを抽出
        let yt_cookies = extract_youtube_cookies_from_wry(&cookies)
            .map_err(|e| AuthWindowError::CookieExtraction(e.to_string()))?;

        Ok(Some(yt_cookies))
    } else {
        Ok(None)
    }
}

/// Cookieをファイルに保存
fn save_cookies(cookies: &YouTubeCookies) -> Result<(), AuthWindowError> {
    let manager = CookieManager::with_default_dir()
        .map_err(|e| AuthWindowError::CookieExtraction(e.to_string()))?;

    manager
        .save(cookies)
        .map_err(|e| AuthWindowError::CookieExtraction(e.to_string()))?;

    tracing::info!("💾 Credentials saved to file");
    Ok(())
}

/// WebViewのブラウジングデータ（Cookie含む）をクリアする
///
/// ログアウト時に呼び出して、WebViewに保存されたYouTubeのCookieを削除します。
pub async fn clear_webview_cookies() -> Result<(), AuthWindowError> {
    tracing::info!("🧹 Clearing WebView browsing data...");

    let (tx, rx) = oneshot::channel::<Result<(), AuthWindowError>>();

    std::thread::spawn(move || {
        let result = clear_webview_cookies_sync();
        let _ = tx.send(result);
    });

    match rx.await {
        Ok(result) => result,
        Err(_) => Err(AuthWindowError::CookieExtraction(
            "Failed to clear cookies".to_string(),
        )),
    }
}

/// 同期的にWebViewのCookieをクリア
fn clear_webview_cookies_sync() -> Result<(), AuthWindowError> {
    use dioxus::desktop::tao::{
        dpi::LogicalSize,
        event_loop::EventLoopBuilder,
        platform::windows::EventLoopBuilderExtWindows,
        window::WindowBuilder,
    };
    use dioxus::desktop::wry::WebViewBuilder;

    // 非表示のウィンドウを作成
    let event_loop = EventLoopBuilder::new().with_any_thread(true).build();

    let window = WindowBuilder::new()
        .with_title("Clearing cookies...")
        .with_inner_size(LogicalSize::new(1.0, 1.0))
        .with_visible(false)
        .build(&event_loop)
        .map_err(|e| AuthWindowError::WindowCreation(e.to_string()))?;

    // WebViewを作成してCookieをクリア
    let webview = WebViewBuilder::new()
        .with_url("about:blank")
        .build(&window)
        .map_err(|e| AuthWindowError::WebViewCreation(e.to_string()))?;

    // すべてのブラウジングデータをクリア
    webview
        .clear_all_browsing_data()
        .map_err(|e| AuthWindowError::CookieExtraction(e.to_string()))?;

    tracing::info!("✅ WebView browsing data cleared");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_window_error_display() {
        let err = AuthWindowError::Timeout;
        assert_eq!(err.to_string(), "Authentication timed out");

        let err = AuthWindowError::Cancelled;
        assert_eq!(err.to_string(), "Authentication cancelled by user");
    }
}
