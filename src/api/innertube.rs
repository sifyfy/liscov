pub mod get_live_chat;

use crate::api::continuation_builder::{detect_chat_mode, modify_continuation_mode};
use crate::api::innertube::get_live_chat::GetLiveChatResponse;
use crate::api::youtube::{ApiKey, ChatContinuations, ChatMode, ClientVersion, Continuation, VideoId};
use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Not found")]
    NotFound,
    #[error("Anyhow error: {0}")]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone, derive_more::Display, Serialize, Deserialize)]
pub struct ClientId(pub String);

#[derive(Debug, Clone)]
pub struct InnerTube {
    pub video_id: VideoId,
    pub api_key: ApiKey,
    pub is_replay: bool,
    pub client_version: ClientVersion,
    pub gl: String,
    pub hl: String,
    pub continuation: Continuation,
    pub client_id: ClientId,
    pub http_client: reqwest::Client,
    /// 両方のチャットモード用のcontinuation tokens
    pub chat_continuations: Option<ChatContinuations>,
    /// 現在選択されているチャットモード
    pub chat_mode: ChatMode,
}

impl InnerTube {
    pub fn new(
        video_id: VideoId,
        api_key: ApiKey,
        client_version: ClientVersion,
        client_id: ClientId,
    ) -> Self {
        Self {
            video_id,
            api_key,
            is_replay: false,
            client_version,
            gl: "US".to_string(),
            hl: "en".to_string(),
            continuation: Continuation("".to_string()),
            client_id,
            http_client: reqwest::Client::new(),
            chat_continuations: None,
            chat_mode: ChatMode::default(),
        }
    }

    /// チャットモードを変更し、continuation tokenを更新する
    ///
    /// メインcontinuation tokenのバイナリを変更してモードを切り替える。
    /// reload tokenは使用しない（reload tokenはAPIで直接使えないため）。
    ///
    /// # Arguments
    /// * `mode` - 新しいチャットモード
    ///
    /// # Returns
    /// * `true` - モード変更成功
    /// * `false` - モード変更失敗（トークンが空または変更できない場合）
    pub fn set_chat_mode(&mut self, mode: ChatMode) -> bool {
        // 既に同じモードの場合は何もしない
        if self.chat_mode == mode {
            tracing::debug!("Chat mode already set to {:?}", mode);
            return true;
        }

        // continuation tokenが空の場合は変更不可
        if self.continuation.0.is_empty() {
            tracing::warn!("Cannot change chat mode: continuation token is empty");
            return false;
        }

        // continuation tokenをバイナリ変換してモードを変更
        if let Some(new_token) = modify_continuation_mode(&self.continuation, mode) {
            tracing::info!(
                "🔄 Chat mode changed: {:?} -> {:?} (token length: {})",
                self.chat_mode,
                mode,
                new_token.0.len()
            );
            self.continuation = new_token;
            self.chat_mode = mode;
            true
        } else {
            tracing::warn!("Failed to modify continuation token for mode {:?}", mode);
            false
        }
    }

    /// 現在のチャットモードを取得
    pub fn current_chat_mode(&self) -> ChatMode {
        self.chat_mode
    }

    /// 利用可能なチャットモードを取得
    ///
    /// continuation tokenが有効な場合、両方のモードが利用可能
    pub fn available_chat_modes(&self) -> Vec<ChatMode> {
        if self.continuation.0.is_empty() {
            vec![self.chat_mode]
        } else {
            // 有効なtokenがあれば両方のモードが利用可能
            vec![ChatMode::TopChat, ChatMode::AllChat]
        }
    }

    /// continuation tokenから現在のチャットモードを検出
    pub fn detect_current_mode(&self) -> Option<ChatMode> {
        if self.continuation.0.is_empty() {
            None
        } else {
            detect_chat_mode(&self.continuation)
        }
    }

    /// チャットモードを非同期で切り替える
    ///
    /// reload tokenを使ってlive_chatページを再取得し、
    /// 新しいモード用のmain continuation tokenを取得する。
    ///
    /// # Arguments
    /// * `mode` - 切り替え先のチャットモード
    ///
    /// # Returns
    /// * `Ok(true)` - 切り替え成功
    /// * `Ok(false)` - reload tokenが利用できない
    /// * `Err(_)` - ページ取得失敗
    pub async fn switch_chat_mode(&mut self, mode: ChatMode) -> Result<bool> {
        // 既に同じモードの場合は何もしない
        if self.chat_mode == mode {
            tracing::debug!("Chat mode already set to {:?}", mode);
            return Ok(true);
        }

        // reload tokenを取得
        let reload_token = if let Some(ref continuations) = self.chat_continuations {
            if let Some(token) = continuations.get_for_mode(mode) {
                token.clone()
            } else {
                tracing::warn!("No reload token available for mode {:?}", mode);
                return Ok(false);
            }
        } else {
            tracing::warn!("No chat_continuations available");
            return Ok(false);
        };

        tracing::info!(
            "🔄 Switching chat mode: {:?} -> {:?}",
            self.chat_mode,
            mode
        );

        // reload tokenを使ってlive_chatページを再取得
        let url = format!(
            "https://www.youtube.com/live_chat?continuation={}",
            urlencoding::encode(&reload_token.0)
        );

        tracing::debug!("📋 Fetching live_chat page with reload token");

        let response = self
            .http_client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(
                "❌ Page fetch failed with status: {}\nResponse: {}",
                status,
                &error_body[..200.min(error_body.len())]
            );
            return Err(anyhow::anyhow!("Page fetch failed with status: {}", status));
        }

        let html = response.text().await?;
        tracing::debug!("📄 Received HTML response: {} chars", html.len());

        // 新しいmain continuation tokenを抽出
        if let Some(new_continuation) = crate::api::youtube::extract_continuation(&html) {
            tracing::info!(
                "✅ Chat mode switched: {:?} -> {:?} (new token length: {})",
                self.chat_mode,
                mode,
                new_continuation.0.len()
            );
            self.continuation = new_continuation;
            self.chat_mode = mode;

            // 新しいreload tokensも更新
            let new_continuations = crate::api::youtube::extract_chat_continuations(&html);
            if new_continuations.has_any() {
                self.chat_continuations = Some(new_continuations);
            }

            Ok(true)
        } else {
            tracing::warn!("⚠️ No continuation token found in response");
            // フォールバック: バイナリ変換を試みる
            if self.set_chat_mode(mode) {
                tracing::info!("✅ Fallback: Chat mode switched using binary modification");
                Ok(true)
            } else {
                Err(anyhow::anyhow!("Failed to extract continuation token"))
            }
        }
    }
}

/// デフォルトのチャットモード（TopChat）でライブチャットページを取得
pub async fn fetch_live_chat_page(url: &str) -> Result<InnerTube> {
    fetch_live_chat_page_with_mode(url, ChatMode::default()).await
}

/// 指定したチャットモードでライブチャットページを取得
///
/// 注意: YouTubeのチャットモード切替はreload continuation tokenを使用する。
/// 初回接続時はメインのcontinuation tokenを使用し、モード切替用のtokenは
/// chat_continuationsに保存される。
pub async fn fetch_live_chat_page_with_mode(url: &str, preferred_mode: ChatMode) -> Result<InnerTube> {
    tracing::info!("🌐 Fetching live chat page from URL: {} (mode: {})", url, preferred_mode);

    let client = reqwest::Client::new();

    let response = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to fetch URL: {}", e);
            e
        })?;

    tracing::debug!(
        "📄 Received HTTP response with status: {}",
        response.status()
    );

    let html = response.text().await.map_err(|e| {
        tracing::error!("❌ Failed to read response text: {}", e);
        e
    })?;

    tracing::debug!("📄 HTML response length: {} chars", html.len());

    let video_id = crate::api::youtube::extract_video_id(&html).ok_or_else(|| {
        tracing::error!("❌ video_id not found in HTML");
        anyhow::anyhow!("video_id not found")
    })?;
    tracing::info!("🎬 Extracted video_id: {}", video_id);

    let api_key = crate::api::youtube::extract_api_key(&html).ok_or_else(|| {
        tracing::error!("❌ api_key not found in HTML");
        anyhow::anyhow!("api_key not found")
    })?;
    tracing::info!(
        "🔑 Extracted api_key: {}...",
        &api_key.to_string()[..10.min(api_key.to_string().len())]
    );

    let client_version = crate::api::youtube::extract_client_version(&html).ok_or_else(|| {
        tracing::error!("❌ client_version not found in HTML");
        anyhow::anyhow!("client_version not found")
    })?;
    tracing::info!("📱 Extracted client_version: {}", client_version);

    // メインのcontinuation token（長い形式、メッセージ取得に使用）
    let main_continuation = crate::api::youtube::extract_continuation(&html).ok_or_else(|| {
        tracing::error!("❌ continuation not found in HTML");
        anyhow::anyhow!("continuation not found")
    })?;
    tracing::info!(
        "🔄 Extracted main continuation token (length: {}): {}...",
        main_continuation.0.len(),
        &main_continuation.to_string()[..30.min(main_continuation.to_string().len())]
    );

    // モード切替用のreload tokensを抽出（subMenuItemsから）
    let chat_continuations = crate::api::youtube::extract_chat_continuations(&html);

    let chat_continuations_option = if chat_continuations.has_any() {
        tracing::info!(
            "📋 Mode switch tokens available: TopChat={}, AllChat={}",
            chat_continuations.top_chat.is_some(),
            chat_continuations.all_chat.is_some()
        );
        Some(chat_continuations)
    } else {
        tracing::warn!("⚠️ No mode switch tokens found in HTML");
        None
    };

    let mut inner_tube =
        InnerTube::new(video_id, api_key, client_version, ClientId("1".to_string()));

    // メインcontinuation tokenを設定
    inner_tube.continuation = main_continuation;
    inner_tube.chat_continuations = chat_continuations_option;

    // トークンから現在のモードを検出
    let detected_mode = inner_tube.detect_current_mode().unwrap_or(ChatMode::TopChat);
    inner_tube.chat_mode = detected_mode;
    tracing::info!("🔍 Detected chat mode from token: {:?}", detected_mode);

    // 希望するモードと異なる場合は非同期で切り替え
    if preferred_mode != detected_mode {
        match inner_tube.switch_chat_mode(preferred_mode).await {
            Ok(true) => {
                tracing::info!("🔄 Switched chat mode to: {:?}", preferred_mode);
            }
            Ok(false) => {
                tracing::warn!("⚠️ Could not switch to preferred mode {:?}, using {:?}", preferred_mode, detected_mode);
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to switch to preferred mode {:?}: {}, using {:?}", preferred_mode, e, detected_mode);
            }
        }
    }

    tracing::info!("✅ Successfully initialized InnerTube client (mode: {:?})", inner_tube.chat_mode);
    Ok(inner_tube)
}

pub async fn fetch_live_chat_messages(inner_tube: &InnerTube) -> Result<GetLiveChatResponse> {
    let url = format!(
        "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
        inner_tube.api_key
    );

    tracing::debug!(
        "📡 Making API request to: {}",
        if tracing::level_enabled!(tracing::Level::DEBUG) {
            &url[..60.min(url.len())]
        } else {
            ""
        }
    );

    let payload = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": inner_tube.client_version.to_string()
            }
        },
        "continuation": inner_tube.continuation.to_string(),
    });

    if tracing::level_enabled!(tracing::Level::DEBUG) {
        tracing::debug!(
            "📋 Request payload size: {} bytes",
            serde_json::to_string(&payload).unwrap_or_default().len()
        );
    }

    let response = inner_tube
        .http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("❌ HTTP request failed: {}", e);
            e
        })?;

    let status = response.status();
    if tracing::level_enabled!(tracing::Level::DEBUG) {
        tracing::debug!("📡 API response status: {}", status);
    }

    if !status.is_success() {
        let error_msg = format!("HTTP request failed with status: {}", status);
        tracing::error!("❌ {}", error_msg);
        return Err(anyhow::anyhow!(error_msg));
    }

    let response_text = response.text().await.map_err(|e| {
        tracing::error!("❌ Failed to read response text: {}", e);
        e
    })?;

    if tracing::level_enabled!(tracing::Level::DEBUG) {
        tracing::debug!("📄 Response text length: {} chars", response_text.len());
    }

    let live_chat_response: GetLiveChatResponse =
        serde_json::from_str(&response_text).map_err(|e| {
            tracing::error!("❌ Failed to parse JSON response: {}", e);
            tracing::debug!(
                "🔍 Response text preview: {}",
                &response_text[..200.min(response_text.len())]
            );
            e
        })?;

    if tracing::level_enabled!(tracing::Level::DEBUG) {
        tracing::debug!("✅ Successfully parsed live chat response");
    }
    Ok(live_chat_response)
}

pub fn get_next_continuation(response: &GetLiveChatResponse) -> Option<String> {
    response
        .continuation_contents
        .live_chat_continuation
        .continuations
        .first()
        .and_then(|v| {
            v.get("invalidationContinuationData")
                .or_else(|| v.get("timedContinuationData"))
                .or_else(|| v.get("reloadContinuationData"))
        })
        .and_then(|v| v.get("continuation"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub author: String,
    pub message: String,
    pub timestamp: u64,
}

impl ChatMessage {
    pub fn new(id: String, author: String, message: String, timestamp: u64) -> Self {
        Self {
            id,
            author,
            message,
            timestamp,
        }
    }

    pub fn datetime(&self) -> String {
        use std::time::SystemTime;
        let datetime = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp);
        format!("{:?}", datetime)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseEntry {
    pub timestamp: u64,
    pub response: GetLiveChatResponse,
}

impl ResponseEntry {
    pub fn new(timestamp: u64, response: GetLiveChatResponse) -> Self {
        Self {
            timestamp,
            response,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inner_tube_creation() {
        let inner_tube = InnerTube::new(
            VideoId("test_video_id".to_string()),
            ApiKey::new("test_api_key".to_string()),
            ClientVersion::new("2.0".to_string()),
            ClientId("1".to_string()),
        );

        assert_eq!(inner_tube.video_id.0, "test_video_id");
        assert_eq!(inner_tube.api_key.to_string(), "test_api_key");
        assert!(!inner_tube.is_replay);
        assert_eq!(inner_tube.client_version.to_string(), "2.0");
        assert_eq!(inner_tube.gl, "US");
        assert_eq!(inner_tube.hl, "en");
        assert_eq!(inner_tube.continuation.to_string(), "");
        assert_eq!(inner_tube.client_id.0, "1");
    }

    #[test]
    fn test_chat_message_creation() {
        let message = ChatMessage::new(
            "msg_123".to_string(),
            "TestUser".to_string(),
            "Hello World!".to_string(),
            1234567890,
        );

        assert_eq!(message.id, "msg_123");
        assert_eq!(message.author, "TestUser");
        assert_eq!(message.message, "Hello World!");
        assert_eq!(message.timestamp, 1234567890);
    }

    #[test]
    fn test_response_entry_creation() {
        use crate::api::innertube::get_live_chat::*;

        let response = GetLiveChatResponse {
            continuation_contents: ContinuationContents {
                live_chat_continuation: LiveChatContinuation {
                    continuation: None,
                    actions: vec![],
                    continuations: vec![],
                },
            },
        };

        let entry = super::ResponseEntry::new(1234567890, response);
        assert_eq!(entry.timestamp, 1234567890);
    }

    #[test]
    fn test_fetch_error_display() {
        let error = FetchError::NotFound;
        assert!(format!("{}", error).contains("Not found"));
    }

    #[test]
    fn test_client_id_wrapper() {
        let client_id = ClientId("1".to_string());
        assert_eq!(client_id.0, "1");
    }

    #[test]
    fn test_fetch_error_types() {
        // FetchErrorの各バリアントをテスト
        let error = FetchError::NotFound;
        assert!(format!("{}", error).contains("Not found"));

        let anyhow_error = anyhow::anyhow!("Test error");
        let fetch_error = FetchError::from(anyhow_error);
        match fetch_error {
            FetchError::Other(_) => (), // 期待される
            _ => panic!("Expected FetchError::Other variant"),
        }
    }

    #[test]
    fn test_fetch_error_from_serde() {
        // JSON解析エラーからの変換をテスト
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let fetch_error = FetchError::from(json_error);

        match fetch_error {
            FetchError::Serialization(_) => (), // 期待される
            _ => panic!("Expected FetchError::Serialization variant"),
        }
    }

    #[test]
    fn test_fetch_error_from_anyhow() {
        // anyhowエラーからの変換をテスト
        let anyhow_error = anyhow::anyhow!("Test error");
        let fetch_error = FetchError::from(anyhow_error);

        match fetch_error {
            FetchError::Other(_) => (), // 期待される
            _ => panic!("Expected FetchError::Other variant"),
        }
    }

    #[test]
    fn test_inner_tube_with_invalid_continuation() {
        // 無効な継続トークンでのテスト
        let mut inner_tube = InnerTube::new(
            VideoId("test_video_id".to_string()),
            ApiKey::new("test_api_key".to_string()),
            ClientVersion::new("2.0".to_string()),
            ClientId("1".to_string()),
        );

        // 空の継続トークンを設定
        inner_tube.continuation = Continuation("".to_string());
        assert_eq!(inner_tube.continuation.to_string(), "");

        // 無効な継続トークンを設定
        inner_tube.continuation = Continuation("invalid_token".to_string());
        assert_eq!(inner_tube.continuation.to_string(), "invalid_token");
    }

    #[test]
    fn test_chat_message_edge_cases() {
        // 空のメッセージ
        let empty_message = ChatMessage::new(
            "msg_empty".to_string(),
            "TestUser".to_string(),
            "".to_string(),
            0,
        );
        assert_eq!(empty_message.message, "");
        assert_eq!(empty_message.timestamp, 0);

        // 極端に長いメッセージ
        let long_message = "a".repeat(1000);
        let message = ChatMessage::new(
            "msg_long".to_string(),
            "TestUser".to_string(),
            long_message.clone(),
            u64::MAX,
        );
        assert_eq!(message.message, long_message);
        assert_eq!(message.timestamp, u64::MAX);

        // 特殊文字を含むメッセージ
        let special_message = ChatMessage::new(
            "msg_special".to_string(),
            "TestUser".to_string(),
            "🎮🔥 テスト メッセージ with emojis and 日本語!".to_string(),
            1234567890,
        );
        assert!(special_message.message.contains("🎮"));
        assert!(special_message.message.contains("日本語"));
    }

    #[test]
    fn test_chat_message_datetime_formatting() {
        // 有効なタイムスタンプでの日時フォーマット
        let message = ChatMessage::new(
            "msg_time".to_string(),
            "TestUser".to_string(),
            "Time test".to_string(),
            1609459200, // 2021-01-01 00:00:00 UTC
        );

        let datetime_str = message.datetime();
        assert!(!datetime_str.is_empty());

        // タイムスタンプ0での処理
        let zero_message = ChatMessage::new(
            "msg_zero".to_string(),
            "TestUser".to_string(),
            "Zero timestamp".to_string(),
            0,
        );

        let zero_datetime = zero_message.datetime();
        assert!(!zero_datetime.is_empty());
    }

    #[test]
    fn test_get_next_continuation_edge_cases() {
        use crate::api::innertube::get_live_chat::*;
        use serde_json::json;

        // 空の継続リストの場合
        let empty_response = GetLiveChatResponse {
            continuation_contents: ContinuationContents {
                live_chat_continuation: LiveChatContinuation {
                    continuation: None,
                    actions: vec![],
                    continuations: vec![],
                },
            },
        };
        assert!(get_next_continuation(&empty_response).is_none());

        // 無効な継続データの場合
        let invalid_continuation = json!({
            "invalidKey": "invalidValue"
        });
        let invalid_response = GetLiveChatResponse {
            continuation_contents: ContinuationContents {
                live_chat_continuation: LiveChatContinuation {
                    continuation: None,
                    actions: vec![],
                    continuations: vec![invalid_continuation],
                },
            },
        };
        assert!(get_next_continuation(&invalid_response).is_none());
    }

    #[test]
    fn test_client_id_display() {
        let client_id = ClientId("test_client_123".to_string());
        let display_str = format!("{}", client_id);
        assert_eq!(display_str, "test_client_123");
    }
}
