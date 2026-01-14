//! YouTube認証ウィンドウモジュール
//!
//! Tauriの別ウィンドウでYouTubeログインページを表示し、
//! ログイン完了後にCookieを取得する機能を提供します。

use crate::core::models::YouTubeCookies;
use std::fs;
use std::sync::Arc;
use tauri::{AppHandle, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use thiserror::Error;
use tokio::sync::Mutex;

/// YouTube認証用URL
pub const YOUTUBE_AUTH_URL: &str = "https://www.youtube.com/";

/// 認証ウィンドウのエラー型
#[derive(Error, Debug)]
pub enum AuthWindowError {
    #[error("Failed to create window: {0}")]
    WindowCreation(String),

    #[error("Cookie extraction failed: {0}")]
    CookieExtraction(String),

    #[error("Authentication timed out")]
    Timeout,

    #[error("Authentication cancelled by user")]
    Cancelled,
}

/// 認証の結果
pub type AuthResult = Result<YouTubeCookies, AuthWindowError>;

/// 認証タイムアウト（秒）
const AUTH_TIMEOUT_SECS: u64 = 300; // 5分

/// Cookieポーリング間隔（ミリ秒）
const POLL_INTERVAL_MS: u64 = 1000; // 1秒


/// 認証ウィンドウの状態
struct AuthState {
    completed: bool,
    result: Option<AuthResult>,
}

/// 認証ウィンドウを開いてYouTubeログインを行う
pub async fn open_auth_window(app: AppHandle) -> AuthResult {
    tracing::info!("🔐 Opening YouTube authentication window...");

    // 認証状態
    let state = Arc::new(Mutex::new(AuthState {
        completed: false,
        result: None,
    }));

    // 認証ウィンドウを作成
    let auth_window = WebviewWindowBuilder::new(
        &app,
        "youtube-auth",
        WebviewUrl::External(YOUTUBE_AUTH_URL.parse().unwrap()),
    )
    .title("YouTube ログイン - liscov")
    .inner_size(900.0, 700.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| AuthWindowError::WindowCreation(e.to_string()))?;

    tracing::info!("🪟 Auth window created");

    // ウィンドウ閉じイベントをハンドル
    let state_clone = state.clone();
    auth_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            tracing::info!("🚪 Auth window closed by user");
            let state = state_clone.clone();
            let _ = tauri::async_runtime::block_on(async {
                let mut s = state.lock().await;
                if !s.completed {
                    s.completed = true;
                    s.result = Some(Err(AuthWindowError::Cancelled));
                }
            });
        }
    });

    // Cookieチェック用のJavaScript
    let check_cookies_js = r#"
        (function() {
            const cookies = document.cookie.split(';').reduce((acc, c) => {
                const [key, val] = c.trim().split('=');
                acc[key] = val;
                return acc;
            }, {});

            // Check if SAPISID exists (login indicator)
            if (cookies['SAPISID']) {
                return JSON.stringify({
                    logged_in: true,
                    sid: cookies['SID'] || '',
                    hsid: cookies['HSID'] || '',
                    ssid: cookies['SSID'] || '',
                    apisid: cookies['APISID'] || '',
                    sapisid: cookies['SAPISID'] || ''
                });
            }
            return JSON.stringify({ logged_in: false });
        })()
    "#;

    // ポーリングループ
    let start_time = std::time::Instant::now();

    loop {
        // タイムアウトチェック
        if start_time.elapsed() > std::time::Duration::from_secs(AUTH_TIMEOUT_SECS) {
            tracing::warn!("⏰ Authentication timed out");
            let _ = auth_window.close();
            return Err(AuthWindowError::Timeout);
        }

        // 状態チェック
        {
            let s = state.lock().await;
            if s.completed {
                if let Some(ref result) = s.result {
                    return match result {
                        Ok(cookies) => Ok(cookies.clone()),
                        Err(e) => Err(match e {
                            AuthWindowError::Cancelled => AuthWindowError::Cancelled,
                            AuthWindowError::Timeout => AuthWindowError::Timeout,
                            AuthWindowError::WindowCreation(s) => AuthWindowError::WindowCreation(s.clone()),
                            AuthWindowError::CookieExtraction(s) => AuthWindowError::CookieExtraction(s.clone()),
                        }),
                    };
                }
            }
        }

        // ウィンドウがまだ存在するかチェック
        if app.get_webview_window("youtube-auth").is_none() {
            tracing::info!("Auth window was closed");
            return Err(AuthWindowError::Cancelled);
        }

        // JavaScriptでCookieをチェック
        match auth_window.eval(check_cookies_js) {
            Ok(_) => {
                // evalの結果を取得するためにevaluate_scriptを使用
            }
            Err(e) => {
                tracing::debug!("Failed to execute cookie check: {}", e);
            }
        }

        // URL変化を監視してログイン判定
        // YouTube Studioなどにリダイレクトされたらログイン完了と判断
        if let Ok(url) = auth_window.url() {
            let url_str = url.to_string();
            // ログイン後のCookie取得のため、JavaScriptを注入
            if url_str.contains("youtube.com") && !url_str.contains("accounts.google.com") {
                // ログインページから離れたら、再度Cookieチェック
                let js_get_cookies = format!(
                    r#"
                    (function() {{
                        const cookies = document.cookie;
                        if (cookies.includes('SAPISID')) {{
                            window.__TAURI__.event.emit('auth-cookies', cookies);
                        }}
                    }})()
                "#
                );
                let _ = auth_window.eval(&js_get_cookies);
            }
        }

        // 短い間隔で待機
        tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;

        // Cookieイベントをリッスン
        let app_clone = app.clone();
        let state_for_listener = state.clone();
        let window_to_close = auth_window.clone();

        // 一度だけリスナーを設定
        static LISTENER_SET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

        if !LISTENER_SET.swap(true, std::sync::atomic::Ordering::SeqCst) {
            app_clone.listen_any("auth-cookies", move |event| {
                // event.payload() returns a String
                let payload = event.payload();
                tracing::info!("Received cookies from WebView");

                // Cookieをパース
                let cookies = parse_cookie_string(&payload);

                if let Some(yt_cookies) = extract_youtube_cookies_from_map(&cookies) {
                    // 保存
                    if let Err(e) = save_cookies(&yt_cookies) {
                        tracing::error!("Failed to save cookies: {}", e);
                    }

                    // 状態更新
                    let state = state_for_listener.clone();
                    let window = window_to_close.clone();
                    tauri::async_runtime::spawn(async move {
                        let mut s = state.lock().await;
                        s.completed = true;
                        s.result = Some(Ok(yt_cookies));
                        let _ = window.close();
                    });
                }
            });
        }
    }
}

/// Cookie文字列をパース
fn parse_cookie_string(cookie_str: &str) -> std::collections::HashMap<String, String> {
    cookie_str
        .split(';')
        .filter_map(|c| {
            let mut parts = c.trim().splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                _ => None,
            }
        })
        .collect()
}

/// HashMapからYouTubeCookiesへ変換
fn extract_youtube_cookies_from_map(
    cookies: &std::collections::HashMap<String, String>,
) -> Option<YouTubeCookies> {
    let sid = cookies.get("SID")?;
    let hsid = cookies.get("HSID")?;
    let ssid = cookies.get("SSID")?;
    let apisid = cookies.get("APISID")?;
    let sapisid = cookies.get("SAPISID")?;

    Some(YouTubeCookies {
        sid: sid.clone(),
        hsid: hsid.clone(),
        ssid: ssid.clone(),
        apisid: apisid.clone(),
        sapisid: sapisid.clone(),
    })
}

/// Cookieをファイルに保存
fn save_cookies(cookies: &YouTubeCookies) -> Result<(), AuthWindowError> {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct CredentialsConfig {
        youtube: YouTubeCookiesConfig,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct YouTubeCookiesConfig {
        sid: String,
        hsid: String,
        ssid: String,
        apisid: String,
        sapisid: String,
    }

    let config_dir = dirs::config_dir()
        .ok_or_else(|| AuthWindowError::CookieExtraction("Failed to determine config directory".to_string()))?;
    let path = config_dir.join("liscov").join("credentials.toml");

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AuthWindowError::CookieExtraction(format!("Failed to create config directory: {}", e)))?;
    }

    let config = CredentialsConfig {
        youtube: YouTubeCookiesConfig {
            sid: cookies.sid.clone(),
            hsid: cookies.hsid.clone(),
            ssid: cookies.ssid.clone(),
            apisid: cookies.apisid.clone(),
            sapisid: cookies.sapisid.clone(),
        },
    };

    let toml_string = toml::to_string_pretty(&config)
        .map_err(|e| AuthWindowError::CookieExtraction(format!("Failed to serialize credentials: {}", e)))?;

    fs::write(&path, toml_string)
        .map_err(|e| AuthWindowError::CookieExtraction(format!("Failed to write credentials file: {}", e)))?;

    tracing::info!("💾 Credentials saved to file");
    Ok(())
}

/// WebViewのブラウジングデータ（Cookie含む）をクリアする
/// 注: Tauri v2ではWebViewのCookieクリアは直接サポートされていないため、
/// 資格情報ファイルのみを削除します
pub async fn clear_webview_cookies() -> Result<(), AuthWindowError> {
    tracing::info!("🧹 Clearing credentials...");

    let config_dir = dirs::config_dir()
        .ok_or_else(|| AuthWindowError::CookieExtraction("Failed to determine config directory".to_string()))?;
    let path = config_dir.join("liscov").join("credentials.toml");

    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AuthWindowError::CookieExtraction(format!("Failed to remove credentials file: {}", e)))?;
        tracing::info!("✅ Credentials file removed");
    }

    Ok(())
}
