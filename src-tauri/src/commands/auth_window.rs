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

/// Cookie取得先URLを返す。
/// LISCOV_YOUTUBE_BASE_URL 設定時はそのURLを使用（テスト用）。
/// 未設定時は本番YouTubeの2ドメインを使用。
fn get_cookie_urls() -> Vec<String> {
    match std::env::var("LISCOV_YOUTUBE_BASE_URL") {
        Ok(base) => {
            let url = if base.ends_with('/') { base } else { format!("{}/", base) };
            vec![url]
        }
        Err(_) => vec![
            "https://www.youtube.com/".to_string(),
            "https://youtube.com/".to_string(),
        ],
    }
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
    let cookie_urls = get_cookie_urls();

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
            // LISCOV_YOUTUBE_BASE_URL で取得先URLを切替可能（テスト時はモックサーバーURL）
            // 本番ではyoutube.comスコープのCookieのみ取得（他ドメインは混在させない）
            {
                let mut all_cookies = Vec::new();
                for cookie_url in &cookie_urls {
                    match auth_window.cookies_for_url(cookie_url.parse().unwrap()) {
                        Ok(c) => {
                            tracing::debug!("🍪 Retrieved {} cookies from {}", c.len(), cookie_url);
                            all_cookies.extend(c);
                        }
                        Err(e) => {
                            tracing::debug!("🍪 Failed to get cookies from {}: {}", cookie_url, e);
                        }
                    }
                }

                tracing::debug!(
                    "🍪 Total cookies for extraction: {}",
                    all_cookies.len()
                );

                if !all_cookies.is_empty() {
                    let cookie_names: Vec<&str> = all_cookies.iter().map(|c| c.name()).collect();
                    tracing::debug!("🍪 Cookie names: {:?}", cookie_names);

                    if all_cookies.iter().any(|c| c.name() == "SAPISID") {
                        tracing::info!("🔓 SAPISID detected in cookies");

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
            }
        }

        // 短い間隔で待機
        tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}

/// Cookie文字列をパース（テストで使用）
#[cfg(test)]
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

    // =========================================================================
    // G3: Cookie Source Isolation — ソース分離テスト
    // =========================================================================

    #[test]
    fn get_cookie_urls_returns_youtube_only_by_default() {
        // G3: デフォルトでYouTube URLのみ返す。Google URLを含まない。
        unsafe { std::env::remove_var("LISCOV_YOUTUBE_BASE_URL") };

        let urls = get_cookie_urls();
        assert_eq!(urls.len(), 2);
        assert!(urls.iter().all(|u| u.contains("youtube.com")));
        assert!(!urls.iter().any(|u| u.contains("google.com")));
    }

    #[test]
    fn get_cookie_urls_returns_env_url_when_set() {
        // G3: 環境変数でURL切替可能（E2Eテスト用）
        unsafe { std::env::set_var("LISCOV_YOUTUBE_BASE_URL", "http://localhost:3456") };

        let urls = get_cookie_urls();
        assert_eq!(urls, vec!["http://localhost:3456/"]);

        unsafe { std::env::remove_var("LISCOV_YOUTUBE_BASE_URL") };
    }

    // =========================================================================
    // G1: Cookie Pipeline Completeness — 完全性テスト
    // =========================================================================

    #[test]
    fn extract_preserves_all_cookies_in_raw() {
        // G1: 5基本Cookie + 未知のCookie3つを含むmap → to_cookie_string()に全8Cookie含まれる
        // YouTubeが将来新Cookieを追加しても、raw_cookie_stringに自動的に含まれることを保証
        let mut cookies = std::collections::HashMap::new();
        cookies.insert("SID".to_string(), "sid_val".to_string());
        cookies.insert("HSID".to_string(), "hsid_val".to_string());
        cookies.insert("SSID".to_string(), "ssid_val".to_string());
        cookies.insert("APISID".to_string(), "apisid_val".to_string());
        cookies.insert("SAPISID".to_string(), "sapisid_val".to_string());
        // 未知の将来Cookie（YouTubeが追加する可能性がある）
        cookies.insert("__Secure-4PSID".to_string(), "future_secure".to_string());
        cookies.insert("LOGIN_INFO".to_string(), "login_data".to_string());
        cookies.insert("PREF".to_string(), "tz=Asia.Tokyo".to_string());

        let result = extract_youtube_cookies_from_map(&cookies).unwrap();
        let cookie_string = result.to_cookie_string();

        // 全8Cookieが出力に含まれる
        for (name, value) in &cookies {
            assert!(
                cookie_string.contains(&format!("{}={}", name, value)),
                "Cookie {}={} not found in output: {}",
                name, value, cookie_string
            );
        }
    }

    #[test]
    fn extract_preserves_cookie_count() {
        // G1: Cookie数が入力と出力で一致
        let mut cookies = std::collections::HashMap::new();
        cookies.insert("SID".to_string(), "s".to_string());
        cookies.insert("HSID".to_string(), "h".to_string());
        cookies.insert("SSID".to_string(), "ss".to_string());
        cookies.insert("APISID".to_string(), "a".to_string());
        cookies.insert("SAPISID".to_string(), "sa".to_string());
        cookies.insert("__Secure-1PSID".to_string(), "sec1".to_string());
        cookies.insert("__Secure-3PSID".to_string(), "sec3".to_string());
        cookies.insert("YSC".to_string(), "ysc".to_string());
        cookies.insert("VISITOR_INFO1_LIVE".to_string(), "vi".to_string());
        cookies.insert("PREF".to_string(), "pref".to_string());
        let input_count = cookies.len(); // 10

        let result = extract_youtube_cookies_from_map(&cookies).unwrap();
        let raw = result.raw_cookie_string.unwrap();
        let output_count = raw.split("; ").count();

        assert_eq!(input_count, output_count,
            "Cookie count mismatch: input={}, output={}. raw={}",
            input_count, output_count, raw);
    }

    // =========================================================================
    // G2: Cookie Pipeline Losslessness — 無損失性テスト
    // =========================================================================

    #[test]
    fn extract_preserves_special_characters_in_values() {
        // G2: Cookie値にBase64（=含む）、スラッシュ、ドット等が含まれても保持される
        let mut cookies = std::collections::HashMap::new();
        cookies.insert("SID".to_string(), "abc123".to_string());
        cookies.insert("HSID".to_string(), "def456".to_string());
        cookies.insert("SSID".to_string(), "ghi789".to_string());
        cookies.insert("APISID".to_string(), "jkl012".to_string());
        cookies.insert("SAPISID".to_string(), "mno345".to_string());
        // Base64値（末尾に=）
        cookies.insert("__Secure-1PSID".to_string(), "aGVsbG8gd29ybGQ=".to_string());
        // ドットやスラッシュを含む値
        cookies.insert("VISITOR_INFO1_LIVE".to_string(), "abc.def/ghi_jkl-mno".to_string());

        let result = extract_youtube_cookies_from_map(&cookies).unwrap();
        let raw = result.raw_cookie_string.unwrap();

        assert!(raw.contains("__Secure-1PSID=aGVsbG8gd29ybGQ="),
            "Base64 value not preserved: {}", raw);
        assert!(raw.contains("VISITOR_INFO1_LIVE=abc.def/ghi_jkl-mno"),
            "Special chars not preserved: {}", raw);
    }
}
