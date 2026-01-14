//! InnerTube API client for YouTube Live Chat

use crate::core::models::*;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::Value;

const INNERTUBE_API_URL: &str = "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat";
const DEFAULT_API_KEY: &str = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";

/// InnerTube API client
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
            api_key: DEFAULT_API_KEY.to_string(),
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

    /// Set authentication cookies
    pub fn set_auth(&mut self, cookies: YouTubeCookies) {
        self.auth_cookies = Some(cookies);
    }

    /// Set chat mode
    pub fn set_chat_mode(&mut self, mode: ChatMode) {
        self.chat_mode = mode;
    }

    /// Initialize connection and get initial data
    pub async fn initialize(&mut self) -> Result<ConnectionStatus> {
        let page_url = format!("https://www.youtube.com/watch?v={}", self.video_id);

        let response = self.http_client
            .get(&page_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;

        let html = response.text().await?;

        if let Some(data) = extract_yt_initial_data(&html) {
            self.parse_initial_data(&data)?;
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

    fn parse_initial_data(&mut self, data: &Value) -> Result<()> {
        // Extract stream title
        if let Some(title) = data.pointer("/contents/twoColumnWatchNextResults/results/results/contents/0/videoPrimaryInfoRenderer/title/runs/0/text") {
            self.stream_title = title.as_str().map(|s| s.to_string());
        }

        // Extract broadcaster info
        if let Some(owner) = data.pointer("/contents/twoColumnWatchNextResults/results/results/contents/1/videoSecondaryInfoRenderer/owner/videoOwnerRenderer") {
            if let Some(name) = owner.pointer("/title/runs/0/text") {
                self.broadcaster_name = name.as_str().map(|s| s.to_string());
            }
            if let Some(channel_id) = owner.pointer("/navigationEndpoint/browseEndpoint/browseId") {
                self.broadcaster_channel_id = channel_id.as_str().map(|s| s.to_string());
            }
        }

        // Extract continuation token
        if let Some(chat) = data.pointer("/contents/twoColumnWatchNextResults/conversationBar/liveChatRenderer") {
            if let Some(continuations) = chat.get("continuations") {
                if let Some(cont) = continuations.get(0) {
                    let token = cont.pointer("/reloadContinuationData/continuation")
                        .or_else(|| cont.pointer("/invalidationContinuationData/continuation"))
                        .or_else(|| cont.pointer("/timedContinuationData/continuation"));

                    if let Some(token) = token {
                        self.continuation = token.as_str().map(|s| s.to_string());
                    }
                }
            }
        }

        self.is_replay = data.pointer("/contents/twoColumnWatchNextResults/conversationBar/liveChatRenderer/isReplay")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(())
    }

    /// Fetch chat messages (returns messages only)
    pub async fn fetch_messages(&mut self) -> Result<Vec<ChatMessage>> {
        let (messages, _) = self.fetch_messages_with_raw().await?;
        Ok(messages)
    }

    /// Fetch chat messages and return raw response JSON as well
    pub async fn fetch_messages_with_raw(&mut self) -> Result<(Vec<ChatMessage>, String)> {
        let continuation = self.continuation.as_ref()
            .ok_or_else(|| anyhow!("No continuation token"))?;

        let request_body = build_request_body(&self.video_id, continuation, &self.client_version);
        let url = format!("{}?key={}&prettyPrint=false", INNERTUBE_API_URL, self.api_key);

        let mut request = self.http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

        if let Some(cookies) = &self.auth_cookies {
            let headers = super::auth::build_auth_headers(cookies);
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        let response = request.json(&request_body).send().await?;
        let raw_json = response.text().await?;
        let data: Value = serde_json::from_str(&raw_json)?;

        if let Some(new_continuation) = extract_continuation(&data) {
            self.continuation = Some(new_continuation);
        }

        let messages = parse_chat_actions(&data);
        Ok((messages, raw_json))
    }

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

fn extract_yt_initial_data(html: &str) -> Option<Value> {
    let start_marker = "var ytInitialData = ";
    let start = html.find(start_marker)? + start_marker.len();
    let end = html[start..].find(";</script>")? + start;
    serde_json::from_str(&html[start..end]).ok()
}

fn build_request_body(video_id: &str, continuation: &str, client_version: &str) -> Value {
    serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": client_version,
                "gl": "US",
                "hl": "en"
            }
        },
        "continuation": continuation,
        "videoId": video_id
    })
}

fn extract_continuation(data: &Value) -> Option<String> {
    let paths = [
        "/continuationContents/liveChatContinuation/continuations/0/invalidationContinuationData/continuation",
        "/continuationContents/liveChatContinuation/continuations/0/timedContinuationData/continuation",
        "/continuationContents/liveChatContinuation/continuations/0/reloadContinuationData/continuation",
    ];

    for path in paths {
        if let Some(cont) = data.pointer(path) {
            if let Some(s) = cont.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn parse_chat_actions(data: &Value) -> Vec<ChatMessage> {
    let mut messages = Vec::new();

    let actions = data.pointer("/continuationContents/liveChatContinuation/actions")
        .and_then(|v| v.as_array());

    if let Some(actions) = actions {
        for action in actions {
            if let Some(msg) = parse_chat_action(action) {
                messages.push(msg);
            }
        }
    }
    messages
}

fn parse_chat_action(action: &Value) -> Option<ChatMessage> {
    let item = action.pointer("/replayChatItemAction/actions/0/addChatItemAction/item")
        .or_else(|| action.pointer("/addChatItemAction/item"))?;

    if let Some(renderer) = item.get("liveChatTextMessageRenderer") {
        return parse_text_message(renderer);
    }
    if let Some(renderer) = item.get("liveChatPaidMessageRenderer") {
        return parse_superchat_message(renderer);
    }
    if let Some(renderer) = item.get("liveChatPaidStickerRenderer") {
        return parse_supersticker_message(renderer);
    }
    if let Some(renderer) = item.get("liveChatMembershipItemRenderer") {
        return parse_membership_message(renderer);
    }
    None
}

fn parse_text_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    let author = renderer.pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = renderer.pointer("/authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let author_icon_url = renderer.pointer("/authorPhoto/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let (content, runs) = parse_message_content(renderer.get("message")?);

    let is_member = renderer.get("authorBadges")
        .and_then(|v| v.as_array())
        .map(|badges| badges.iter().any(|b| b.pointer("/liveChatAuthorBadgeRenderer/customThumbnail").is_some()))
        .unwrap_or(false);

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::Text,
        author,
        author_icon_url,
        channel_id,
        content,
        runs,
        metadata: None,
        is_member,
        comment_count: None,
    })
}

fn parse_superchat_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    let author = renderer.pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = renderer.pointer("/authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let author_icon_url = renderer.pointer("/authorPhoto/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let amount = renderer.pointer("/purchaseAmountText/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let (content, runs) = renderer.get("message")
        .map(|m| parse_message_content(m))
        .unwrap_or_default();

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::SuperChat { amount: amount.clone() },
        author,
        author_icon_url,
        channel_id,
        content,
        runs,
        metadata: Some(MessageMetadata {
            amount: Some(amount),
            badges: vec![],
            badge_info: vec![],
            color: None,
            is_moderator: false,
            is_verified: false,
            superchat_colors: None,
        }),
        is_member: false,
        comment_count: None,
    })
}

fn parse_supersticker_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    let author = renderer.pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = renderer.pointer("/authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let amount = renderer.pointer("/purchaseAmountText/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::SuperSticker { amount: amount.clone() },
        author,
        author_icon_url: None,
        channel_id,
        content: "[Sticker]".to_string(),
        runs: vec![],
        metadata: Some(MessageMetadata {
            amount: Some(amount),
            badges: vec![],
            badge_info: vec![],
            color: None,
            is_moderator: false,
            is_verified: false,
            superchat_colors: None,
        }),
        is_member: false,
        comment_count: None,
    })
}

fn parse_membership_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    let author = renderer.pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = renderer.pointer("/authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let content = renderer.pointer("/headerSubtext/simpleText")
        .or_else(|| renderer.pointer("/headerSubtext/runs/0/text"))
        .and_then(|v| v.as_str())
        .unwrap_or("New member")
        .to_string();

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::Membership { milestone_months: None },
        author,
        author_icon_url: None,
        channel_id,
        content,
        runs: vec![],
        metadata: None,
        is_member: true,
        comment_count: None,
    })
}

fn parse_message_content(message: &Value) -> (String, Vec<MessageRun>) {
    let mut content = String::new();
    let mut runs = Vec::new();

    if let Some(runs_array) = message.get("runs").and_then(|v| v.as_array()) {
        for run in runs_array {
            if let Some(text) = run.get("text").and_then(|v| v.as_str()) {
                content.push_str(text);
                runs.push(MessageRun::Text { content: text.to_string() });
            } else if let Some(emoji) = run.get("emoji") {
                let emoji_id = emoji.get("emojiId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let image_url = emoji.pointer("/image/thumbnails/0/url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let alt_text = emoji.pointer("/image/accessibility/accessibilityData/label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                content.push_str(&alt_text);
                runs.push(MessageRun::Emoji { emoji_id, image_url, alt_text });
            }
        }
    }
    (content, runs)
}

fn format_timestamp(timestamp_usec: &str) -> String {
    if let Ok(usec) = timestamp_usec.parse::<i64>() {
        let secs = usec / 1_000_000;
        let datetime = chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
        datetime.format("%H:%M:%S").to_string()
    } else {
        String::new()
    }
}
