//! WebView認証フロー
//!
//! Dioxus DesktopのWebViewを使用してYouTubeにログインし、
//! 認証Cookieを取得するためのモジュール。
//!
//! ## 認証フロー
//!
//! 1. WebViewでYouTubeにナビゲート
//! 2. ユーザーがGoogleログイン
//! 3. Cookieポーリングでログイン完了を検知（SAPISID存在確認）
//! 4. 必要なCookieを抽出して返す

use super::{AuthError, AuthResult, YouTubeCookies};
use chrono::Utc;
use dioxus::desktop::wry::cookie::Cookie;

/// YouTube認証に必要なCookie名
const REQUIRED_COOKIES: [&str; 5] = ["SID", "HSID", "SSID", "APISID", "SAPISID"];

/// YouTube認証用URL
pub const YOUTUBE_AUTH_URL: &str = "https://www.youtube.com/";

/// 認証ステータス
#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    /// 認証処理中
    InProgress,
    /// 認証成功
    Success(YouTubeCookies),
    /// 認証キャンセル
    Cancelled,
    /// 認証失敗
    Failed(String),
}

/// Wry Cookie構造体からYouTubeCookiesへ変換
///
/// # Arguments
///
/// * `cookies` - Wry WebViewから取得したCookieリスト
///
/// # Returns
///
/// すべての必須Cookieが存在する場合は`Ok(YouTubeCookies)`、
/// 不足している場合は`Err(AuthError)`
pub fn extract_youtube_cookies_from_wry(cookies: &[Cookie<'static>]) -> AuthResult<YouTubeCookies> {
    let mut sid = None;
    let mut hsid = None;
    let mut ssid = None;
    let mut apisid = None;
    let mut sapisid = None;

    for cookie in cookies {
        match cookie.name() {
            "SID" => sid = Some(cookie.value().to_string()),
            "HSID" => hsid = Some(cookie.value().to_string()),
            "SSID" => ssid = Some(cookie.value().to_string()),
            "APISID" => apisid = Some(cookie.value().to_string()),
            "SAPISID" => sapisid = Some(cookie.value().to_string()),
            _ => {}
        }
    }

    // すべての必須Cookieが存在するか確認
    let missing: Vec<&str> = REQUIRED_COOKIES
        .iter()
        .filter(|&&name| match name {
            "SID" => sid.is_none(),
            "HSID" => hsid.is_none(),
            "SSID" => ssid.is_none(),
            "APISID" => apisid.is_none(),
            "SAPISID" => sapisid.is_none(),
            _ => false,
        })
        .copied()
        .collect();

    if !missing.is_empty() {
        return Err(AuthError::CookieNotFound(missing.join(", ")));
    }

    Ok(YouTubeCookies {
        sid: sid.unwrap(),
        hsid: hsid.unwrap(),
        ssid: ssid.unwrap(),
        apisid: apisid.unwrap(),
        sapisid: sapisid.unwrap(),
        acquired_at: Utc::now(),
        raw_cookies: None,
    })
}

/// SAPISIDが存在するかチェック（ログイン完了判定用）
pub fn has_sapisid(cookies: &[Cookie<'static>]) -> bool {
    cookies.iter().any(|c| c.name() == "SAPISID")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cookie(name: &str, value: &str) -> Cookie<'static> {
        Cookie::build((name.to_string(), value.to_string()))
            .domain(".youtube.com")
            .path("/")
            .secure(true)
            .http_only(true)
            .build()
    }

    #[test]
    fn test_extract_youtube_cookies_success() {
        let cookies = vec![
            create_test_cookie("SID", "test_sid"),
            create_test_cookie("HSID", "test_hsid"),
            create_test_cookie("SSID", "test_ssid"),
            create_test_cookie("APISID", "test_apisid"),
            create_test_cookie("SAPISID", "test_sapisid"),
        ];

        let result = extract_youtube_cookies_from_wry(&cookies);
        assert!(result.is_ok());

        let yt_cookies = result.unwrap();
        assert_eq!(yt_cookies.sid, "test_sid");
        assert_eq!(yt_cookies.sapisid, "test_sapisid");
    }

    #[test]
    fn test_extract_youtube_cookies_missing() {
        let cookies = vec![
            create_test_cookie("SID", "test_sid"),
            create_test_cookie("HSID", "test_hsid"),
            // SSID, APISID, SAPISID が不足
        ];

        let result = extract_youtube_cookies_from_wry(&cookies);
        assert!(result.is_err());

        match result {
            Err(AuthError::CookieNotFound(missing)) => {
                assert!(missing.contains("SSID"));
                assert!(missing.contains("APISID"));
                assert!(missing.contains("SAPISID"));
            }
            _ => panic!("Expected CookieNotFound error"),
        }
    }

    #[test]
    fn test_has_sapisid_true() {
        let cookies = vec![
            create_test_cookie("OTHER", "value"),
            create_test_cookie("SAPISID", "test_sapisid"),
        ];

        assert!(has_sapisid(&cookies));
    }

    #[test]
    fn test_has_sapisid_false() {
        let cookies = vec![
            create_test_cookie("SID", "test_sid"),
            create_test_cookie("HSID", "test_hsid"),
        ];

        assert!(!has_sapisid(&cookies));
    }

    #[test]
    fn test_has_sapisid_empty() {
        let cookies: Vec<Cookie<'static>> = vec![];
        assert!(!has_sapisid(&cookies));
    }
}
