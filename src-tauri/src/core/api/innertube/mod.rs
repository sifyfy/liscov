//! InnerTube API クライアント（YouTube Live Chat）
//!
//! サブモジュール構成:
//! - `client`       : HTTP リクエスト構築・送信・cookie 管理
//! - `initial_data` : ウォッチページ HTML パース・continuation token 解析
//! - `chat_parser`  : チャットメッセージのパース・変換ロジック

mod chat_parser;
mod client;
mod initial_data;
mod reaction_parser;

use crate::core::models::*;
use anyhow::{Result, anyhow};
use reqwest::Client;
use std::time::Duration;

pub use chat_parser::parse_chat_actions;
pub use client::{get_innertube_api_url, get_youtube_base_url};

/// 1 回のポーリングで取れたもの
pub struct ChatFetch {
    pub messages: Vec<ChatMessage>,
    pub reactions: Vec<ReactionUpdate>,
    /// 生のレスポンス JSON（05_raw_response.md の保存用）
    pub raw_json: String,
}

/// リクエスト全体のタイムアウト（02_chat.md「設定値」）。応答が返らない取得で監視ループが止まらないようにする
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// 接続確立のタイムアウト（02_chat.md「設定値」）
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

fn build_http_client() -> Client {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .expect("HTTP クライアントの初期化に失敗")
}

/// InnerTube API クライアント
pub struct InnerTubeClient {
    http_client: Client,
    video_id: String,
    api_key: String,
    client_version: String,
    continuation: Option<String>,
    chat_mode: ChatMode,
    auth_cookies: Option<YouTubeCookies>,
    pub broadcaster_channel_id: Option<String>,
    pub broadcaster_name: Option<String>,
    pub stream_title: Option<String>,
    pub is_replay: bool,
}

impl InnerTubeClient {
    pub fn new(video_id: impl Into<String>) -> Self {
        Self {
            http_client: build_http_client(),
            video_id: video_id.into(),
            api_key: client::DEFAULT_API_KEY.to_string(),
            client_version: "2.20240101.00.00".to_string(),
            continuation: None,
            chat_mode: ChatMode::TopChat,
            auth_cookies: None,
            broadcaster_channel_id: None,
            broadcaster_name: None,
            stream_title: None,
            is_replay: false,
        }
    }

    /// 認証 cookie を設定する
    pub fn set_auth(&mut self, cookies: YouTubeCookies) {
        self.auth_cookies = Some(cookies);
    }

    /// チャットモードを設定し、continuation token のバイナリデータを変更する。
    ///
    /// TopChat / AllChat を切り替えるために continuation token 内の
    /// バイナリフィールドを書き換える。
    ///
    /// # Returns
    /// * `true`  - モード変更成功
    /// * `false` - モード変更失敗（continuation token なし、または変更失敗）
    pub fn set_chat_mode(&mut self, mode: ChatMode) -> bool {
        // 既に同じモードの場合
        if self.chat_mode == mode {
            tracing::debug!("Chat mode already set to {:?}", mode);
            return true;
        }

        // continuation token がない場合
        let Some(ref continuation) = self.continuation else {
            tracing::warn!("Cannot change chat mode: no continuation token");
            return false;
        };

        // continuation token のバイナリを変更する
        if let Some(new_token) =
            super::continuation_builder::modify_continuation_mode(continuation, mode)
        {
            tracing::info!(
                "Chat mode changed: {:?} -> {:?} (token length: {})",
                self.chat_mode,
                mode,
                new_token.len()
            );
            self.continuation = Some(new_token);
            self.chat_mode = mode;
            true
        } else {
            tracing::warn!("Failed to modify continuation token for mode {:?}", mode);
            false
        }
    }

    /// 現在のチャットモードを返す
    pub fn get_chat_mode(&self) -> ChatMode {
        self.chat_mode
    }

    /// 現在の continuation token からチャットモードを検出する
    pub fn detect_chat_mode(&self) -> Option<ChatMode> {
        self.continuation
            .as_ref()
            .and_then(|token| super::continuation_builder::detect_chat_mode(token))
    }

    /// 接続を初期化して初期データを取得する
    pub async fn initialize(&mut self) -> Result<ConnectionStatus> {
        tracing::info!(
            "initialize: video_id={}, has_auth={}",
            self.video_id,
            self.auth_cookies.is_some()
        );

        // Step 1: ウォッチページを取得する（公開配信は cookie なしで可能）
        let page_url = format!(
            "{}/watch?v={}",
            client::get_youtube_base_url(),
            self.video_id
        );

        let mut request = self.http_client.get(&page_url).header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        );

        // ページ取得時は Cookie ヘッダーのみ送信（SAPISIDHASH Authorization は不要）
        if let Some(cookies) = &self.auth_cookies {
            request = request.header("Cookie", cookies.to_cookie_string());
        }

        let response = request.send().await?;
        let html = response.text().await?;

        if let Some(data) = initial_data::extract_yt_initial_data(&html) {
            let has_chat = data
                .pointer("/contents/twoColumnWatchNextResults/conversationBar/liveChatRenderer")
                .is_some();
            tracing::info!(
                "Watch page: ytInitialData found, liveChatRenderer={}",
                has_chat
            );
            initial_data::parse_initial_data(
                &data,
                &mut self.broadcaster_channel_id,
                &mut self.broadcaster_name,
                &mut self.stream_title,
                &mut self.continuation,
                &mut self.is_replay,
            )?;
        } else {
            tracing::warn!("Watch page: ytInitialData NOT found in HTML");
        }

        tracing::info!(
            "After watch page: continuation={}, title={:?}",
            self.continuation.is_some(),
            self.stream_title
        );

        // Step 2: ウォッチページから continuation token が得られず、認証がある場合は
        // InnerTube API を試みる（メンバー限定配信でページ cookie が不十分な場合に必要）
        if self.continuation.is_none() && self.auth_cookies.is_some() {
            tracing::info!(
                "Watch page did not return continuation token, trying InnerTube API fallback..."
            );
            match client::fetch_initial_data_via_api(
                &self.http_client,
                &self.video_id,
                &self.api_key,
                &self.client_version,
                &self.auth_cookies,
                &mut self.broadcaster_channel_id,
                &mut self.broadcaster_name,
                &mut self.stream_title,
                &mut self.continuation,
                &mut self.is_replay,
            )
            .await
            {
                Ok(()) => {
                    tracing::info!("InnerTube API fallback succeeded, continuation token obtained");
                }
                Err(e) => {
                    tracing::warn!("InnerTube API fallback failed: {}", e);
                }
            }
        }

        // 初期 token のモードを現在のモードとして持つ
        if let Some(token) = self.continuation.take() {
            self.adopt_continuation(token);
        }

        Ok(ConnectionStatus {
            is_connected: self.continuation.is_some(),
            stream_title: self.stream_title.clone(),
            broadcaster_channel_id: self.broadcaster_channel_id.clone(),
            broadcaster_name: self.broadcaster_name.clone(),
            chat_mode: self.chat_mode,
            is_replay: self.is_replay,
            error: if self.continuation.is_none() {
                Some("Failed to get continuation token".to_string())
            } else {
                None
            },
        })
    }

    /// チャットメッセージを取得する（メッセージのみを返す）
    pub async fn fetch_messages(&mut self) -> Result<Vec<ChatMessage>> {
        Ok(self.fetch_chat().await?.messages)
    }

    /// チャットメッセージ・ライブリアクションと生のレスポンス JSON を取得する
    pub async fn fetch_chat(&mut self) -> Result<ChatFetch> {
        let continuation = self
            .continuation
            .as_ref()
            .ok_or_else(|| anyhow!("No continuation token"))?;

        let request_body =
            client::build_request_body(&self.video_id, continuation, &self.client_version);
        let url = format!(
            "{}?key={}&prettyPrint=false",
            client::get_innertube_api_url(),
            self.api_key
        );

        let mut request = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            );

        if let Some(cookies) = &self.auth_cookies {
            let headers = super::auth::build_auth_headers(cookies);
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        // 2xx 以外は失敗として数える（02_chat.md「取得に失敗したとき」）
        let response = request
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        let raw_json = response.text().await?;
        let data: serde_json::Value = serde_json::from_str(&raw_json)?;

        if let Some(new_continuation) = client::extract_continuation(&data) {
            self.adopt_continuation(new_continuation);
        }

        Ok(ChatFetch {
            messages: chat_parser::parse_chat_actions(&data),
            reactions: reaction_parser::parse_reaction_updates(&data),
            raw_json,
        })
    }

    /// 届いた continuation token を使うようにし、そのモードを現在のモードとして持つ
    ///
    /// YouTube は受け取った token のモードを保って次の token を返すが、切替の要否は
    /// 実際の token で判定したいので、読めたときはそのモードに合わせる。
    fn adopt_continuation(&mut self, token: String) {
        if let Some(mode) = super::continuation_builder::detect_chat_mode(&token) {
            self.chat_mode = mode;
        }
        self.continuation = Some(token);
    }

    /// 現在の接続状態を返す
    pub fn status(&self) -> ConnectionStatus {
        ConnectionStatus {
            is_connected: self.continuation.is_some(),
            stream_title: self.stream_title.clone(),
            broadcaster_channel_id: self.broadcaster_channel_id.clone(),
            broadcaster_name: self.broadcaster_name.clone(),
            chat_mode: self.chat_mode,
            is_replay: self.is_replay,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::api::continuation_builder::test_token;

    #[test]
    fn test_set_chat_mode_without_continuation() {
        // continuation token がない場合、set_chat_mode は false を返すこと
        let mut client = InnerTubeClient::new("test_video");
        assert_eq!(client.get_chat_mode(), ChatMode::TopChat);

        let result = client.set_chat_mode(ChatMode::AllChat);
        assert!(!result, "continuation token がない場合は失敗すること");
        assert_eq!(client.get_chat_mode(), ChatMode::TopChat);
    }

    #[test]
    fn test_set_chat_mode_same_mode() {
        // 同じモードへの切り替えは true を返すこと
        let mut client = InnerTubeClient::new("test_video");
        assert_eq!(client.get_chat_mode(), ChatMode::TopChat);

        let result = client.set_chat_mode(ChatMode::TopChat);
        assert!(result, "同じモードへの切り替えは成功すること");
        assert_eq!(client.get_chat_mode(), ChatMode::TopChat);
    }

    #[test]
    fn test_set_chat_mode_with_valid_token() {
        let mut client = InnerTubeClient::new("test_video");
        client.continuation = Some(test_token(4, true));

        assert!(client.set_chat_mode(ChatMode::AllChat));
        assert_eq!(client.get_chat_mode(), ChatMode::AllChat);
        assert_eq!(client.detect_chat_mode(), Some(ChatMode::AllChat));

        assert!(client.set_chat_mode(ChatMode::TopChat));
        assert_eq!(client.get_chat_mode(), ChatMode::TopChat);
        assert_eq!(client.detect_chat_mode(), Some(ChatMode::TopChat));
    }

    // 02_chat.md: 書き換えに失敗したら、次のポーリングで新しい token に対してやり直せる
    #[test]
    fn test_set_chat_mode_retry_after_failure() {
        let mut client = InnerTubeClient::new("test_video");
        client.continuation = Some("broken".to_string());
        assert!(!client.set_chat_mode(ChatMode::AllChat));
        assert_eq!(client.get_chat_mode(), ChatMode::TopChat);

        // 次のポーリングで届いた token
        client.adopt_continuation(test_token(4, false));
        assert!(client.set_chat_mode(ChatMode::AllChat));
        assert_eq!(client.detect_chat_mode(), Some(ChatMode::AllChat));
    }

    // 届いた token のモードをクライアントの現在のモードとして持つ（切替要否の判定を実態に合わせる）
    #[test]
    fn test_adopt_continuation_follows_token_mode() {
        let mut client = InnerTubeClient::new("test_video");
        client.adopt_continuation(test_token(1, true));
        assert_eq!(client.get_chat_mode(), ChatMode::AllChat);

        // モードを読めない token ではモードを変えない
        client.adopt_continuation("broken".to_string());
        assert_eq!(client.get_chat_mode(), ChatMode::AllChat);
    }

    #[test]
    fn test_detect_chat_mode() {
        let mut client = InnerTubeClient::new("test_video");
        client.continuation = Some(test_token(4, false));
        assert_eq!(client.detect_chat_mode(), Some(ChatMode::TopChat));

        client.continuation = Some(test_token(1, true));
        assert_eq!(client.detect_chat_mode(), Some(ChatMode::AllChat));
    }
}
