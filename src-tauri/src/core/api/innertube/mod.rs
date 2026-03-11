//! InnerTube API クライアント（YouTube Live Chat）
//!
//! サブモジュール構成:
//! - `client`       : HTTP リクエスト構築・送信・cookie 管理
//! - `initial_data` : ウォッチページ HTML パース・continuation token 解析
//! - `chat_parser`  : チャットメッセージのパース・変換ロジック

mod chat_parser;
mod client;
mod initial_data;

use crate::core::models::*;
use anyhow::{anyhow, Result};
use reqwest::Client;

pub use chat_parser::parse_chat_actions;
pub use client::{get_youtube_base_url, get_innertube_api_url};

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
            http_client: Client::new(),
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
            tracing::warn!(
                "Failed to modify continuation token for mode {:?}",
                mode
            );
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

        let mut request = self
            .http_client
            .get(&page_url)
            .header(
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
                .pointer(
                    "/contents/twoColumnWatchNextResults/conversationBar/liveChatRenderer",
                )
                .is_some();
            tracing::info!("Watch page: ytInitialData found, liveChatRenderer={}", has_chat);
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
                    tracing::info!(
                        "InnerTube API fallback succeeded, continuation token obtained"
                    );
                }
                Err(e) => {
                    tracing::warn!("InnerTube API fallback failed: {}", e);
                }
            }
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
        let (messages, _) = self.fetch_messages_with_raw().await?;
        Ok(messages)
    }

    /// チャットメッセージを取得し、生のレスポンス JSON も返す
    pub async fn fetch_messages_with_raw(&mut self) -> Result<(Vec<ChatMessage>, String)> {
        let continuation = self
            .continuation
            .as_ref()
            .ok_or_else(|| anyhow!("No continuation token"))?;

        let request_body = client::build_request_body(
            &self.video_id,
            continuation,
            &self.client_version,
        );
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

        let response = request.json(&request_body).send().await?;
        let raw_json = response.text().await?;
        let data: serde_json::Value = serde_json::from_str(&raw_json)?;

        if let Some(new_continuation) = client::extract_continuation(&data) {
            self.continuation = Some(new_continuation);
        }

        let messages = chat_parser::parse_chat_actions(&data);
        Ok((messages, raw_json))
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
        use base64::{engine::general_purpose, Engine as _};

        // 有効な chattype フィールド構造を持つトークンを作成する
        // Field 16 (0x82 0x01) + length(2) + Field 1 (0x08) + value(4=TopChat)
        let inner = vec![
            0xd2, 0x87, 0xcc, 0xc8, 0x03, // YouTube ヘッダー
            0x10, 0x00, // フィールド
            0x82, 0x01, 0x02, 0x08, 0x04, // Field 16: chattype=4 (TopChat)
            0x20, 0x00, // 末尾フィールド
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let mut client = InnerTubeClient::new("test_video");
        client.continuation = Some(token);

        // AllChat に切り替える
        let result = client.set_chat_mode(ChatMode::AllChat);
        assert!(result, "有効なトークンで成功すること");
        assert_eq!(client.get_chat_mode(), ChatMode::AllChat);

        // トークンが変更されていることを確認する
        let new_token = client.continuation.as_ref().unwrap();
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(new_token).unwrap();
        assert_eq!(decoded[11], 0x01, "chattype が 1 (AllChat) になっていること");

        // TopChat に戻す
        let result = client.set_chat_mode(ChatMode::TopChat);
        assert!(result, "TopChat への切り替えが成功すること");
        assert_eq!(client.get_chat_mode(), ChatMode::TopChat);

        let new_token = client.continuation.as_ref().unwrap();
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(new_token).unwrap();
        assert_eq!(decoded[11], 0x04, "chattype が 4 (TopChat) になっていること");
    }

    #[test]
    fn test_detect_chat_mode() {
        use base64::{engine::general_purpose, Engine as _};

        // TopChat トークン
        let inner_top = vec![
            0xd2, 0x87, 0xcc, 0xc8, 0x03,
            0x10, 0x00,
            0x82, 0x01, 0x02, 0x08, 0x04, // chattype=4 (TopChat)
            0x20, 0x00,
        ];
        let top_token = general_purpose::URL_SAFE_NO_PAD.encode(&inner_top);

        let mut client = InnerTubeClient::new("test_video");
        client.continuation = Some(top_token);

        assert_eq!(client.detect_chat_mode(), Some(ChatMode::TopChat));

        // AllChat トークン
        let inner_all = vec![
            0xd2, 0x87, 0xcc, 0xc8, 0x03,
            0x10, 0x00,
            0x82, 0x01, 0x02, 0x08, 0x01, // chattype=1 (AllChat)
            0x20, 0x00,
        ];
        let all_token = general_purpose::URL_SAFE_NO_PAD.encode(&inner_all);
        client.continuation = Some(all_token);

        assert_eq!(client.detect_chat_mode(), Some(ChatMode::AllChat));
    }
}
