//! YouTube認証ウィンドウモジュール
//!
//! Tauriの別ウィンドウでYouTubeログインページを表示し、
//! ログイン完了後にCookieを取得する機能を提供します。

use crate::core::models::YouTubeCookies;
use std::sync::Arc;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use thiserror::Error;
use tokio::sync::Mutex;

/// YouTube認証用URL (デフォルト)
pub const DEFAULT_YOUTUBE_AUTH_URL: &str = "https://www.youtube.com/";


/// 認証URLを取得する（テスト用に環境変数で上書き可能）
/// LISCOV_AUTH_URL 環境変数が設定されている場合はそれを使用
pub fn get_auth_url() -> String {
    std::env::var("LISCOV_AUTH_URL").unwrap_or_else(|_| DEFAULT_YOUTUBE_AUTH_URL.to_string())
}

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
    let auth_url = get_auth_url();
    tracing::info!("🔐 Opening YouTube authentication window: {}", auth_url);

    // 認証状態
    let state = Arc::new(Mutex::new(AuthState {
        completed: false,
        result: None,
    }));

    // 認証ウィンドウを作成
    let auth_window = WebviewWindowBuilder::new(
        &app,
        "youtube-auth",
        WebviewUrl::External(auth_url.parse().unwrap()),
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

    // ポーリングループ
    let start_time = std::time::Instant::now();

    loop {
        // タイムアウトチェック
        if start_time.elapsed() > std::time::Duration::from_secs(AUTH_TIMEOUT_SECS) {
            tracing::warn!("⏰ Authentication timed out");
            let _ = auth_window.close();
            return Err(AuthWindowError::Timeout);
        }

        // 状態チェック（ウィンドウが閉じられた場合）
        {
            let s = state.lock().await;
            if s.completed {
                if let Some(ref result) = s.result {
                    return match result {
                        Ok(cookies) => Ok(cookies.clone()),
                        Err(e) => Err(match e {
                            AuthWindowError::Cancelled => AuthWindowError::Cancelled,
                            AuthWindowError::Timeout => AuthWindowError::Timeout,
                            AuthWindowError::WindowCreation(s) => {
                                AuthWindowError::WindowCreation(s.clone())
                            }
                            AuthWindowError::CookieExtraction(s) => {
                                AuthWindowError::CookieExtraction(s.clone())
                            }
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

        // Googleログインページ以外でCookieをチェック
        // (モックサーバー使用時も動作するよう、youtube.com判定を削除)
        if let Ok(url) = auth_window.url() {
            let url_str = url.to_string();
            tracing::info!("📍 Current URL: {}", url_str);

            // about:blank, Googleのログインページの場合はスキップ
            if url_str == "about:blank" || url_str.contains("accounts.google.com") {
                tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
                continue;
            }

            // Cookieをチェック
            {
                // YouTube domainのCookieのみを使用
                // ブラウザがyoutube.comにリクエストする際はyoutube.comスコープのCookieのみ送信される
                // accounts.google.comやwww.google.comのCookieは同名でも値が異なる場合があり、
                // 混在させるとYouTubeが認証を認識しない（logged_in=0になる）
                let youtube_cookies = {
                    let mut cookies = Vec::new();
                    // YouTube domainのCookieのみ取得（認証に必要な全Cookieがここに含まれる）
                    for yt_url in &["https://www.youtube.com/", "https://youtube.com/"] {
                        match auth_window.cookies_for_url(yt_url.parse().unwrap()) {
                            Ok(c) => {
                                tracing::debug!("🍪 Retrieved {} cookies from {}", c.len(), yt_url);
                                cookies.extend(c);
                            }
                            Err(e) => {
                                tracing::debug!("🍪 Failed to get cookies from {}: {}", yt_url, e);
                            }
                        }
                    }
                    cookies
                };

                // SAPISIDがない場合、Google domainも含めて全ドメインからチェック
                // （モックサーバーやフォールバック用）
                let all_cookies = if youtube_cookies.iter().any(|c| c.name() == "SAPISID") {
                    youtube_cookies
                } else {
                    let mut all = youtube_cookies;
                    for google_url in &["https://accounts.google.com/", "https://www.google.com/"] {
                        match auth_window.cookies_for_url(google_url.parse().unwrap()) {
                            Ok(c) => {
                                tracing::debug!("🍪 Fallback: {} cookies from {}", c.len(), google_url);
                                all.extend(c);
                            }
                            Err(e) => {
                                tracing::debug!("🍪 Failed to get cookies from {}: {}", google_url, e);
                            }
                        }
                    }
                    all
                };

                tracing::debug!(
                    "🍪 Total cookies for extraction: {}",
                    all_cookies.len()
                );

                if !all_cookies.is_empty() {
                    let cookie_names: Vec<&str> = all_cookies.iter().map(|c| c.name()).collect();
                    tracing::debug!("🍪 Cookie names: {:?}", cookie_names);

                    if all_cookies.iter().any(|c| c.name() == "SAPISID") {
                        tracing::info!("🔓 SAPISID detected in cookies");

                        // CookieをHashMapに変換（YouTube domainのCookieが優先される）
                        let mut cookies_map = std::collections::HashMap::new();
                        for cookie in all_cookies {
                            cookies_map.insert(cookie.name().to_string(), cookie.value().to_string());
                        }

                        if let Some(yt_cookies) = extract_youtube_cookies_from_map(&cookies_map) {
                            tracing::info!("✅ Successfully extracted YouTube cookies");

                            let _ = auth_window.close();
                            return Ok(yt_cookies);
                        }
                    }
                }

                // 方法2: URLフラグメントからCookieを取得（mock server用フォールバック）
                if let Some(fragment) = url.fragment() {
                    tracing::info!("📎 URL fragment: {}", fragment);
                    if let Some(cookie_str) = fragment.strip_prefix("LISCOV_AUTH:") {
                        tracing::info!("🔓 SAPISID detected in URL fragment");

                        // URLデコード（%3B -> ;、%20 -> スペース など）
                        let decoded_cookie_str = urlencoding::decode(cookie_str)
                            .unwrap_or_else(|_| cookie_str.into());
                        tracing::info!("🍪 Decoded cookie string: {}", decoded_cookie_str);

                        let cookies = parse_cookie_string(&decoded_cookie_str);

                        if let Some(yt_cookies) = extract_youtube_cookies_from_map(&cookies) {
                            tracing::info!("✅ Successfully extracted YouTube cookies from URL");

                            let _ = auth_window.close();
                            return Ok(yt_cookies);
                        }
                    }
                }
            }
        }

        // 短い間隔で待機
        tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
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
/// 5つの必須Cookieに加え、全Cookieの文字列も保存する（member-only配信のアクセスに必要）
/// 保存先はファイルストレージへ自動フォールバックされるため、サイズ制限は問題にならない
fn extract_youtube_cookies_from_map(
    cookies: &std::collections::HashMap<String, String>,
) -> Option<YouTubeCookies> {
    let sid = cookies.get("SID")?;
    let hsid = cookies.get("HSID")?;
    let ssid = cookies.get("SSID")?;
    let apisid = cookies.get("APISID")?;
    let sapisid = cookies.get("SAPISID")?;

    // 全Cookieをraw_cookie_stringとして保存
    let raw_cookie_string: String = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");

    Some(YouTubeCookies {
        sid: sid.clone(),
        hsid: hsid.clone(),
        ssid: ssid.clone(),
        apisid: apisid.clone(),
        sapisid: sapisid.clone(),
        raw_cookie_string: Some(raw_cookie_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cookie_string_basic() {
        let cookie_str = "SID=abc123; HSID=def456; SSID=ghi789";
        let cookies = parse_cookie_string(cookie_str);

        assert_eq!(cookies.get("SID"), Some(&"abc123".to_string()));
        assert_eq!(cookies.get("HSID"), Some(&"def456".to_string()));
        assert_eq!(cookies.get("SSID"), Some(&"ghi789".to_string()));
    }

    #[test]
    fn test_parse_cookie_string_with_spaces() {
        let cookie_str = "  SID=abc123 ;  HSID=def456  ; SSID=ghi789  ";
        let cookies = parse_cookie_string(cookie_str);

        assert_eq!(cookies.get("SID"), Some(&"abc123".to_string()));
        assert_eq!(cookies.get("HSID"), Some(&"def456".to_string()));
        assert_eq!(cookies.get("SSID"), Some(&"ghi789".to_string()));
    }

    #[test]
    fn test_parse_cookie_string_with_equals_in_value() {
        let cookie_str = "SID=abc=123; HSID=def=456==";
        let cookies = parse_cookie_string(cookie_str);

        assert_eq!(cookies.get("SID"), Some(&"abc=123".to_string()));
        assert_eq!(cookies.get("HSID"), Some(&"def=456==".to_string()));
    }

    #[test]
    fn test_parse_cookie_string_empty() {
        let cookie_str = "";
        let cookies = parse_cookie_string(cookie_str);

        assert!(cookies.is_empty());
    }

    #[test]
    fn test_extract_youtube_cookies_success() {
        let mut cookies = std::collections::HashMap::new();
        cookies.insert("SID".to_string(), "sid_value".to_string());
        cookies.insert("HSID".to_string(), "hsid_value".to_string());
        cookies.insert("SSID".to_string(), "ssid_value".to_string());
        cookies.insert("APISID".to_string(), "apisid_value".to_string());
        cookies.insert("SAPISID".to_string(), "sapisid_value".to_string());

        let result = extract_youtube_cookies_from_map(&cookies);

        assert!(result.is_some());
        let yt_cookies = result.unwrap();
        assert_eq!(yt_cookies.sid, "sid_value");
        assert_eq!(yt_cookies.hsid, "hsid_value");
        assert_eq!(yt_cookies.ssid, "ssid_value");
        assert_eq!(yt_cookies.apisid, "apisid_value");
        assert_eq!(yt_cookies.sapisid, "sapisid_value");
    }

    #[test]
    fn test_extract_youtube_cookies_missing_sapisid() {
        let mut cookies = std::collections::HashMap::new();
        cookies.insert("SID".to_string(), "sid_value".to_string());
        cookies.insert("HSID".to_string(), "hsid_value".to_string());
        cookies.insert("SSID".to_string(), "ssid_value".to_string());
        cookies.insert("APISID".to_string(), "apisid_value".to_string());
        // SAPISID is missing

        let result = extract_youtube_cookies_from_map(&cookies);

        assert!(result.is_none());
    }

    #[test]
    fn test_extract_youtube_cookies_with_extra_cookies() {
        let mut cookies = std::collections::HashMap::new();
        cookies.insert("SID".to_string(), "sid_value".to_string());
        cookies.insert("HSID".to_string(), "hsid_value".to_string());
        cookies.insert("SSID".to_string(), "ssid_value".to_string());
        cookies.insert("APISID".to_string(), "apisid_value".to_string());
        cookies.insert("SAPISID".to_string(), "sapisid_value".to_string());
        cookies.insert("OTHER_COOKIE".to_string(), "other_value".to_string());

        let result = extract_youtube_cookies_from_map(&cookies);

        assert!(result.is_some());
        let yt_cookies = result.unwrap();
        assert_eq!(yt_cookies.sapisid, "sapisid_value");
    }

    #[test]
    fn test_raw_cookie_string_includes_all_cookies() {
        let mut cookies = std::collections::HashMap::new();
        cookies.insert("SID".to_string(), "s".to_string());
        cookies.insert("HSID".to_string(), "h".to_string());
        cookies.insert("SSID".to_string(), "ss".to_string());
        cookies.insert("APISID".to_string(), "a".to_string());
        cookies.insert("SAPISID".to_string(), "sa".to_string());
        cookies.insert("__Secure-1PSID".to_string(), "sec1".to_string());
        cookies.insert("VISITOR_INFO1_LIVE".to_string(), "visitor".to_string());
        cookies.insert("YSC".to_string(), "ysc_val".to_string());

        let result = extract_youtube_cookies_from_map(&cookies);
        assert!(result.is_some());

        let raw = result.unwrap().raw_cookie_string.unwrap();
        // 全Cookieが含まれる
        assert!(raw.contains("SID=s"));
        assert!(raw.contains("SAPISID=sa"));
        assert!(raw.contains("__Secure-1PSID=sec1"));
        assert!(raw.contains("VISITOR_INFO1_LIVE=visitor"));
        assert!(raw.contains("YSC=ysc_val"));
    }

    #[test]
    fn test_full_cookie_flow() {
        // Simulate the full flow from cookie string to YouTubeCookies
        let cookie_str =
            "SID=sid123; HSID=hsid456; SSID=ssid789; APISID=apisid012; SAPISID=sapisid345";
        let cookies = parse_cookie_string(cookie_str);
        let result = extract_youtube_cookies_from_map(&cookies);

        assert!(result.is_some());
        let yt_cookies = result.unwrap();
        assert_eq!(yt_cookies.sid, "sid123");
        assert_eq!(yt_cookies.hsid, "hsid456");
        assert_eq!(yt_cookies.ssid, "ssid789");
        assert_eq!(yt_cookies.apisid, "apisid012");
        assert_eq!(yt_cookies.sapisid, "sapisid345");
    }

    #[test]
    fn test_get_auth_url_default() {
        // Clear any existing env var
        // SAFETY: This test runs with --test-threads=1 to avoid race conditions
        unsafe { std::env::remove_var("LISCOV_AUTH_URL") };

        let url = get_auth_url();
        assert_eq!(url, DEFAULT_YOUTUBE_AUTH_URL);
    }

    #[test]
    fn test_get_auth_url_with_env_var() {
        // SAFETY: This test runs with --test-threads=1 to avoid race conditions
        unsafe { std::env::set_var("LISCOV_AUTH_URL", "http://localhost:3456/") };

        let url = get_auth_url();
        assert_eq!(url, "http://localhost:3456/");

        // Clean up
        unsafe { std::env::remove_var("LISCOV_AUTH_URL") };
    }
}
