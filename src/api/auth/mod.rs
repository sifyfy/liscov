//! YouTube認証モジュール
//!
//! メンバー限定配信のチャット取得に必要な認証機能を提供します。
//!
//! ## 機能
//!
//! - SAPISIDHASH生成（InnerTube API認証用）
//! - Cookie管理（保存・読み込み）
//! - WebView認証フロー

mod cookie_manager;
mod sapisidhash;
pub mod webview_auth;

pub use cookie_manager::{CookieManager, YouTubeCookies};
pub use sapisidhash::generate_sapisidhash;
pub use webview_auth::{extract_youtube_cookies_from_wry, has_sapisid, AuthStatus, YOUTUBE_AUTH_URL};

/// 認証関連のエラー型
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Cookieが見つからない
    #[error("Required cookie not found: {0}")]
    CookieNotFound(String),

    /// Cookie保存エラー
    #[error("Failed to save cookies: {0}")]
    SaveError(String),

    /// Cookie読み込みエラー
    #[error("Failed to load cookies: {0}")]
    LoadError(String),

    /// 認証期限切れ
    #[error("Authentication expired")]
    Expired,

    /// WebView認証エラー
    #[error("WebView authentication failed: {0}")]
    WebViewError(String),

    /// I/Oエラー
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML解析エラー
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOMLシリアライズエラー
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

pub type AuthResult<T> = Result<T, AuthError>;
