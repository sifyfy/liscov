//! 認証パネルコンポーネント
//!
//! メンバー限定配信へのアクセスに必要なYouTube認証を管理するUIコンポーネント。

use crate::api::auth::{AuthStatus, CookieManager, YouTubeCookies};
use crate::gui::auth_window::{clear_webview_cookies, open_auth_window, AuthWindowError};
use dioxus::prelude::*;

/// 認証パネルコンポーネント
///
/// メンバー限定配信用の認証状態表示と認証操作を提供します。
#[component]
pub fn AuthPanel() -> Element {
    // 認証状態
    let mut auth_status = use_signal(|| AuthStatus::InProgress);
    let mut is_authenticated = use_signal(|| false);
    let mut auth_message = use_signal(|| String::new());

    // 初期化時に保存済みCookieをチェック
    use_effect(move || {
        spawn(async move {
            match CookieManager::with_default_dir() {
                Ok(manager) => {
                    if manager.exists() {
                        match manager.load() {
                            Ok(cookies) => {
                                if cookies.is_valid() {
                                    is_authenticated.set(true);
                                    auth_message.set(format!(
                                        "認証済み（{}）",
                                        cookies.acquired_at.format("%Y-%m-%d %H:%M")
                                    ));
                                    auth_status.set(AuthStatus::Success(cookies));
                                }
                            }
                            Err(e) => {
                                tracing::debug!("No saved credentials: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize CookieManager: {}", e);
                }
            }
        });
    });

    // 認証中フラグ
    let mut is_authenticating = use_signal(|| false);

    // 認証ボタンクリック時のハンドラ
    let on_auth_click = move |_| {
        // 既に認証中の場合は何もしない
        if *is_authenticating.read() {
            return;
        }

        spawn(async move {
            is_authenticating.set(true);
            auth_message.set("YouTubeログインウィンドウを開いています...".to_string());

            // WebViewを使った認証フローを開始
            match open_auth_window().await {
                Ok(cookies) => {
                    tracing::info!("✅ Authentication completed successfully");
                    is_authenticated.set(true);
                    auth_message.set(format!(
                        "認証成功（{}）",
                        cookies.acquired_at.format("%Y-%m-%d %H:%M")
                    ));
                    auth_status.set(AuthStatus::Success(cookies));
                }
                Err(AuthWindowError::Cancelled) => {
                    tracing::info!("🚫 Authentication cancelled by user");
                    auth_message.set("認証がキャンセルされました".to_string());
                }
                Err(AuthWindowError::Timeout) => {
                    tracing::warn!("⏰ Authentication timed out");
                    auth_message.set(
                        "認証がタイムアウトしました。\n再度お試しください。".to_string(),
                    );
                }
                Err(e) => {
                    tracing::error!("❌ Authentication failed: {}", e);
                    auth_message.set(format!(
                        "認証に失敗しました: {}\n\n手動でCookieを設定する場合は、\ncredentials.tomlを編集してください。",
                        e
                    ));
                }
            }

            is_authenticating.set(false);
        });
    };

    // 認証解除ボタンクリック時のハンドラ
    let on_logout_click = move |_| {
        spawn(async move {
            match CookieManager::with_default_dir() {
                Ok(manager) => {
                    if let Err(e) = manager.delete() {
                        tracing::error!("Failed to delete credentials: {}", e);
                        auth_message.set(format!("認証情報の削除に失敗しました: {}", e));
                        return;
                    }

                    // WebViewのCookieもクリア
                    if let Err(e) = clear_webview_cookies().await {
                        tracing::warn!("Failed to clear WebView cookies: {}", e);
                        // Cookieクリア失敗は警告のみ（credentials.tomlは削除済み）
                    }

                    is_authenticated.set(false);
                    auth_status.set(AuthStatus::Cancelled);
                    auth_message.set("認証情報を削除しました".to_string());
                }
                Err(e) => {
                    tracing::error!("Failed to initialize CookieManager: {}", e);
                    auth_message.set(format!("エラー: {}", e));
                }
            }
        });
    };

    rsx! {
        div {
            class: "auth-panel",
            style: "
                padding: 16px;
                background: #f8f9fa;
                border-radius: 8px;
                border: 1px solid #dee2e6;
                margin: 8px 0;
            ",

            // ヘッダー
            div {
                style: "
                    display: flex;
                    align-items: center;
                    gap: 8px;
                    margin-bottom: 12px;
                ",
                span {
                    style: "font-size: 18px;",
                    if *is_authenticated.read() { "🔓" } else { "🔒" }
                }
                h3 {
                    style: "
                        margin: 0;
                        font-size: 16px;
                        font-weight: 600;
                    ",
                    "メンバー限定配信"
                }
            }

            // 認証状態表示
            div {
                style: "
                    padding: 12px;
                    background: white;
                    border-radius: 4px;
                    margin-bottom: 12px;
                ",

                if *is_authenticated.read() {
                    div {
                        style: "color: #28a745; font-weight: 500;",
                        "✓ {auth_message}"
                    }
                } else {
                    div {
                        style: "color: #6c757d;",
                        if auth_message.read().is_empty() {
                            "未認証 - メンバー限定配信を視聴するにはYouTubeへのログインが必要です"
                        } else {
                            "{auth_message}"
                        }
                    }
                }
            }

            // アクションボタン
            div {
                style: "display: flex; gap: 8px;",

                if *is_authenticated.read() {
                    button {
                        onclick: on_logout_click,
                        style: "
                            padding: 8px 16px;
                            background: #dc3545;
                            color: white;
                            border: none;
                            border-radius: 4px;
                            cursor: pointer;
                            font-size: 14px;
                        ",
                        "ログアウト"
                    }
                } else {
                    button {
                        onclick: on_auth_click,
                        disabled: *is_authenticating.read(),
                        style: if *is_authenticating.read() {
                            "
                                padding: 8px 16px;
                                background: #6c757d;
                                color: white;
                                border: none;
                                border-radius: 4px;
                                cursor: not-allowed;
                                font-size: 14px;
                            "
                        } else {
                            "
                                padding: 8px 16px;
                                background: #007bff;
                                color: white;
                                border: none;
                                border-radius: 4px;
                                cursor: pointer;
                                font-size: 14px;
                            "
                        },
                        if *is_authenticating.read() {
                            "ログイン中..."
                        } else {
                            "YouTubeにログイン"
                        }
                    }
                }
            }

            // 説明テキスト
            div {
                style: "
                    margin-top: 12px;
                    font-size: 12px;
                    color: #6c757d;
                    line-height: 1.5;
                ",
                p {
                    style: "margin: 0 0 4px 0;",
                    "メンバー限定配信のチャットを取得するには、YouTubeアカウントへのログインが必要です。"
                }
                p {
                    style: "margin: 0;",
                    "ログイン情報はローカルに安全に保存され、外部に送信されることはありません。"
                }
            }
        }
    }
}

/// 認証状態を管理するためのコンテキスト
#[derive(Clone)]
pub struct AuthContext {
    /// 現在の認証Cookie
    pub cookies: Option<YouTubeCookies>,
    /// 認証状態
    pub status: AuthStatus,
}

impl Default for AuthContext {
    fn default() -> Self {
        Self {
            cookies: None,
            status: AuthStatus::InProgress,
        }
    }
}

impl AuthContext {
    /// 認証済みかどうかを確認
    pub fn is_authenticated(&self) -> bool {
        self.cookies.is_some()
    }

    /// 認証情報を設定
    pub fn set_authenticated(&mut self, cookies: YouTubeCookies) {
        self.cookies = Some(cookies.clone());
        self.status = AuthStatus::Success(cookies);
    }

    /// 認証情報をクリア
    pub fn clear(&mut self) {
        self.cookies = None;
        self.status = AuthStatus::Cancelled;
    }
}
