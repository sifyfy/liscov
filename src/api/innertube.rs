pub mod get_live_chat;

use crate::api::innertube::get_live_chat::GetLiveChatResponse;
use crate::api::youtube::{ApiKey, ClientVersion, Continuation, VideoId};
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
        }
    }
}

pub async fn fetch_live_chat_page(url: &str) -> Result<InnerTube> {
    tracing::info!("🌐 Fetching live chat page from URL: {}", url);

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

    let continuation = crate::api::youtube::extract_continuation(&html).ok_or_else(|| {
        tracing::error!("❌ continuation not found in HTML");
        anyhow::anyhow!("continuation not found")
    })?;
    tracing::info!(
        "🔄 Extracted continuation token: {}...",
        &continuation.to_string()[..20.min(continuation.to_string().len())]
    );

    let mut inner_tube =
        InnerTube::new(video_id, api_key, client_version, ClientId("1".to_string()));

    inner_tube.continuation = continuation;

    tracing::info!("✅ Successfully initialized InnerTube client");
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
}
