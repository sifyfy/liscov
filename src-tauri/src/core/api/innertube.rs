//! InnerTube API client for YouTube Live Chat

use crate::core::models::*;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::Value;

const DEFAULT_YOUTUBE_BASE_URL: &str = "https://www.youtube.com";
const DEFAULT_API_KEY: &str = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";

/// Get YouTube base URL (can be overridden by LISCOV_YOUTUBE_BASE_URL env var for testing)
fn get_youtube_base_url() -> String {
    std::env::var("LISCOV_YOUTUBE_BASE_URL").unwrap_or_else(|_| DEFAULT_YOUTUBE_BASE_URL.to_string())
}

/// Get InnerTube API URL
fn get_innertube_api_url() -> String {
    format!("{}/youtubei/v1/live_chat/get_live_chat", get_youtube_base_url())
}

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
        let page_url = format!("{}/watch?v={}", get_youtube_base_url(), self.video_id);

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
        // Extract stream title - combine all runs (hashtags are split into separate runs)
        if let Some(runs) = data.pointer("/contents/twoColumnWatchNextResults/results/results/contents/0/videoPrimaryInfoRenderer/title/runs") {
            if let Some(runs_array) = runs.as_array() {
                let title: String = runs_array
                    .iter()
                    .filter_map(|run| run.get("text").and_then(|t| t.as_str()))
                    .collect();
                if !title.is_empty() {
                    self.stream_title = Some(title);
                }
            }
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
        let url = format!("{}?key={}&prettyPrint=false", get_innertube_api_url(), self.api_key);

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
    if let Some(renderer) = item.get("liveChatSponsorshipsGiftPurchaseAnnouncementRenderer") {
        return parse_membership_gift_message(renderer);
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

    let author_icon_url = renderer.pointer("/authorPhoto/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract header subtext - can be simpleText or runs format
    let content = renderer.pointer("/headerSubtext/simpleText")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            // Combine all runs if in runs format
            renderer.pointer("/headerSubtext/runs")
                .and_then(|v| v.as_array())
                .map(|runs| {
                    runs.iter()
                        .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                        .collect::<String>()
                })
        })
        .unwrap_or_else(|| "New member".to_string());

    // Extract milestone months from badge tooltip (e.g., "Member (6 months)")
    // This is the actual YouTube format, not from headerSubtext
    let badge_tooltip = renderer.pointer("/authorBadges/0/liveChatAuthorBadgeRenderer/tooltip")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let milestone_months = extract_milestone_months_from_badge(badge_tooltip);

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::Membership { milestone_months },
        author,
        author_icon_url,
        channel_id,
        content,
        runs: vec![],
        metadata: None,
        is_member: true,
        comment_count: None,
    })
}

/// Extract milestone months from badge tooltip (e.g., "Member (6 months)" or "Member (1 month)")
/// Returns None for "New member" badges
fn extract_milestone_months_from_badge(tooltip: &str) -> Option<u32> {
    use regex::Regex;

    // Skip "New member" badges
    if tooltip.to_lowercase().contains("new member") {
        return None;
    }

    // English format: "Member (6 months)" or "Member (1 month)"
    let en_regex = Regex::new(r"\((\d+)\s*months?\)").ok()?;
    if let Some(caps) = en_regex.captures(tooltip) {
        if let Some(m) = caps.get(1) {
            if let Ok(months) = m.as_str().parse::<u32>() {
                return Some(months);
            }
        }
    }

    // Japanese format if exists: "メンバー（6か月）"
    let ja_regex = Regex::new(r"[（(](\d+)\s*か月[）)]").ok()?;
    if let Some(caps) = ja_regex.captures(tooltip) {
        if let Some(m) = caps.get(1) {
            if let Ok(months) = m.as_str().parse::<u32>() {
                return Some(months);
            }
        }
    }

    None
}

fn parse_membership_gift_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    // authorExternalChannelId is at root level
    let channel_id = renderer.get("authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Gift announcement has header with sponsorshipsHeaderRenderer
    let header = renderer.pointer("/header/liveChatSponsorshipsHeaderRenderer")?;

    let author = header.pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let author_icon_url = header.pointer("/authorPhoto/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract primary text (e.g., "Sent 5 [channel] gift memberships")
    let primary_text = header.pointer("/primaryText/runs")
        .and_then(|v| v.as_array())
        .map(|runs| {
            runs.iter()
                .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                .collect::<String>()
        })
        .or_else(|| {
            header.pointer("/primaryText/simpleText")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    // Extract gift count from primary text
    let gift_count = extract_gift_count(&primary_text).unwrap_or(1);

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::MembershipGift { gift_count },
        author,
        author_icon_url,
        channel_id,
        content: primary_text,
        runs: vec![],
        metadata: None,
        is_member: true,
        comment_count: None,
    })
}

/// Extract gift count from membership gift message
/// Supports:
/// - Japanese: "5人にメンバーシップをギフトしました"
/// - English: "Sent 5 [channel] gift memberships"
fn extract_gift_count(content: &str) -> Option<u32> {
    use regex::Regex;

    // Japanese format: "5人にメンバーシップをギフト"
    let ja_regex = Regex::new(r"(\d+)\s*人").ok()?;
    if let Some(caps) = ja_regex.captures(content) {
        if let Some(m) = caps.get(1) {
            if let Ok(count) = m.as_str().parse::<u32>() {
                return Some(count);
            }
        }
    }

    // English format: "Sent 5 [channel] gift memberships"
    // The number comes right after "Sent "
    let sent_regex = Regex::new(r"Sent\s+(\d+)").ok()?;
    if let Some(caps) = sent_regex.captures(content) {
        if let Some(m) = caps.get(1) {
            if let Ok(count) = m.as_str().parse::<u32>() {
                return Some(count);
            }
        }
    }

    // Fallback: "gifted X memberships" or "X memberships" (directly adjacent)
    let en_regex = Regex::new(r"(\d+)\s+(?:gift\s+)?memberships?").ok()?;
    if let Some(caps) = en_regex.captures(content) {
        if let Some(m) = caps.get(1) {
            if let Ok(count) = m.as_str().parse::<u32>() {
                return Some(count);
            }
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_member_badge() {
        // Test that member badge (customThumbnail) is correctly detected
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatTextMessageRenderer": {
                        "id": "test_msg_1",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "MemberUser"},
                        "authorExternalChannelId": "UC_member",
                        "message": {"runs": [{"text": "Hello"}]},
                        "authorBadges": [{
                            "liveChatAuthorBadgeRenderer": {
                                "customThumbnail": {
                                    "thumbnails": [{"url": "https://example.com/badge.png"}]
                                },
                                "tooltip": "Member"
                            }
                        }]
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "Message should be parsed");
        let msg = msg.unwrap();
        assert_eq!(msg.author, "MemberUser");
        assert!(msg.is_member, "Member badge should be detected");
    }

    #[test]
    fn test_parse_non_member() {
        // Test that non-member message has is_member = false
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatTextMessageRenderer": {
                        "id": "test_msg_2",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "NonMemberUser"},
                        "authorExternalChannelId": "UC_non_member",
                        "message": {"runs": [{"text": "Hello"}]},
                        "authorBadges": []
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "Message should be parsed");
        let msg = msg.unwrap();
        assert_eq!(msg.author, "NonMemberUser");
        assert!(!msg.is_member, "Non-member should have is_member = false");
    }

    #[test]
    fn test_parse_chat_actions_from_response() {
        // Test parsing from full response structure (like mock server returns)
        let response = serde_json::json!({
            "continuationContents": {
                "liveChatContinuation": {
                    "continuations": [{"invalidationContinuationData": {"continuation": "cont_123"}}],
                    "actions": [
                        {
                            "addChatItemAction": {
                                "item": {
                                    "liveChatTextMessageRenderer": {
                                        "id": "msg_1",
                                        "timestampUsec": "1234567890000000",
                                        "authorName": {"simpleText": "MemberUser"},
                                        "authorExternalChannelId": "UC_member",
                                        "message": {"runs": [{"text": "Member message"}]},
                                        "authorBadges": [{
                                            "liveChatAuthorBadgeRenderer": {
                                                "customThumbnail": {
                                                    "thumbnails": [{"url": "https://example.com/badge.png"}]
                                                },
                                                "tooltip": "Member"
                                            }
                                        }]
                                    }
                                }
                            }
                        }
                    ]
                }
            }
        });

        let messages = parse_chat_actions(&response);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].author, "MemberUser");
        assert!(messages[0].is_member, "Member badge should be detected from full response");
    }

    #[test]
    fn test_parse_title_with_hashtags() {
        // Test that title with hashtags (split into multiple runs) is combined correctly
        // YouTube splits titles like "配信タイトル #hashtag1 #hashtag2" into multiple runs
        let data = serde_json::json!({
            "contents": {
                "twoColumnWatchNextResults": {
                    "results": {
                        "results": {
                            "contents": [
                                {
                                    "videoPrimaryInfoRenderer": {
                                        "title": {
                                            "runs": [
                                                {"text": "配信タイトル "},
                                                {"text": "#hashtag1"},
                                                {"text": " "},
                                                {"text": "#hashtag2"}
                                            ]
                                        }
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        });

        let mut client = InnerTubeClient::new("test_video");
        client.parse_initial_data(&data).unwrap();

        assert_eq!(
            client.stream_title,
            Some("配信タイトル #hashtag1 #hashtag2".to_string()),
            "Title with hashtags should be combined from all runs"
        );
    }

    #[test]
    fn test_parse_title_single_run() {
        // Test that simple title (single run) still works
        let data = serde_json::json!({
            "contents": {
                "twoColumnWatchNextResults": {
                    "results": {
                        "results": {
                            "contents": [
                                {
                                    "videoPrimaryInfoRenderer": {
                                        "title": {
                                            "runs": [
                                                {"text": "Simple Title Without Hashtags"}
                                            ]
                                        }
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        });

        let mut client = InnerTubeClient::new("test_video");
        client.parse_initial_data(&data).unwrap();

        assert_eq!(
            client.stream_title,
            Some("Simple Title Without Hashtags".to_string()),
            "Simple title should be parsed correctly"
        );
    }

    #[test]
    fn test_extract_milestone_months_from_badge_english() {
        // Test English format from badge tooltip: "Member (6 months)"
        assert_eq!(extract_milestone_months_from_badge("Member (6 months)"), Some(6));
        assert_eq!(extract_milestone_months_from_badge("Member (1 month)"), Some(1));
        assert_eq!(extract_milestone_months_from_badge("Member (12 months)"), Some(12));
    }

    #[test]
    fn test_extract_milestone_months_from_badge_japanese() {
        // Test Japanese format from badge tooltip if exists
        assert_eq!(extract_milestone_months_from_badge("メンバー（6か月）"), Some(6));
        assert_eq!(extract_milestone_months_from_badge("メンバー(12か月)"), Some(12));
    }

    #[test]
    fn test_extract_milestone_months_from_badge_none() {
        // Test cases that should return None (new members)
        assert_eq!(extract_milestone_months_from_badge("New member"), None);
        assert_eq!(extract_milestone_months_from_badge("Member"), None);
        assert_eq!(extract_milestone_months_from_badge(""), None);
    }

    #[test]
    fn test_extract_gift_count_japanese() {
        // Test Japanese format: "5人にメンバーシップをギフトしました"
        assert_eq!(extract_gift_count("5人にメンバーシップをギフトしました"), Some(5));
        assert_eq!(extract_gift_count("10人にギフト"), Some(10));
        assert_eq!(extract_gift_count("1人"), Some(1));
    }

    #[test]
    fn test_extract_gift_count_english() {
        // Test English format: "Sent 5 [channel] gift memberships" (actual YouTube format)
        assert_eq!(extract_gift_count("Sent 5 Channel Name gift memberships"), Some(5));
        assert_eq!(extract_gift_count("Sent 10 memberships"), Some(10));
        // Fallback patterns
        assert_eq!(extract_gift_count("5 gift memberships"), Some(5));
        assert_eq!(extract_gift_count("1 membership"), Some(1));
    }

    #[test]
    fn test_parse_membership_milestone_message() {
        // Test parsing membership message with milestone (months from badge tooltip)
        // Actual YouTube format: month count is in badge tooltip, not headerSubtext
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatMembershipItemRenderer": {
                        "id": "milestone_msg_1",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "LongTimeMember"},
                        "authorExternalChannelId": "UC_milestone",
                        "authorPhoto": {"thumbnails": [{"url": "https://example.com/av.png"}]},
                        "headerSubtext": {"runs": [{"text": "Welcome to "}, {"text": "Channel"}, {"text": "!"}]},
                        "authorBadges": [{
                            "liveChatAuthorBadgeRenderer": {
                                "tooltip": "Member (12 months)",
                                "customThumbnail": {"thumbnails": [{"url": "https://example.com/badge.png"}]}
                            }
                        }]
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "Milestone message should be parsed");
        let msg = msg.unwrap();
        assert_eq!(msg.author, "LongTimeMember");
        assert!(msg.is_member);

        match msg.message_type {
            MessageType::Membership { milestone_months } => {
                assert_eq!(milestone_months, Some(12), "Should extract 12 months from badge tooltip");
            }
            _ => panic!("Expected Membership message type"),
        }
    }

    #[test]
    fn test_parse_membership_gift_message() {
        // Test parsing membership gift announcement (actual YouTube format)
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatSponsorshipsGiftPurchaseAnnouncementRenderer": {
                        "id": "gift_msg_1",
                        "timestampUsec": "1234567890000000",
                        "authorExternalChannelId": "UC_gift_giver",
                        "header": {
                            "liveChatSponsorshipsHeaderRenderer": {
                                "authorName": {"simpleText": "GiftGiver"},
                                "authorPhoto": {"thumbnails": [{"url": "https://example.com/av.png"}]},
                                "primaryText": {"runs": [
                                    {"text": "Sent ", "bold": true},
                                    {"text": "5", "bold": true},
                                    {"text": " ", "bold": true},
                                    {"text": "Channel Name", "bold": true},
                                    {"text": " gift memberships", "bold": true}
                                ]}
                            }
                        }
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "Gift message should be parsed");
        let msg = msg.unwrap();
        assert_eq!(msg.author, "GiftGiver");
        assert_eq!(msg.channel_id, "UC_gift_giver");

        match msg.message_type {
            MessageType::MembershipGift { gift_count } => {
                assert_eq!(gift_count, 5, "Should extract 5 gifts");
            }
            _ => panic!("Expected MembershipGift message type"),
        }
    }

    #[test]
    fn test_parse_new_member_no_milestone() {
        // Test parsing new member (no milestone months)
        // Actual YouTube format: badge tooltip is "New member"
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatMembershipItemRenderer": {
                        "id": "new_member_1",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "NewMember"},
                        "authorExternalChannelId": "UC_new",
                        "authorPhoto": {"thumbnails": [{"url": "https://example.com/av.png"}]},
                        "headerSubtext": {"runs": [{"text": "Welcome to "}, {"text": "Channel"}, {"text": "!"}]},
                        "authorBadges": [{
                            "liveChatAuthorBadgeRenderer": {
                                "tooltip": "New member",
                                "customThumbnail": {"thumbnails": [{"url": "https://example.com/badge.png"}]}
                            }
                        }]
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "New member message should be parsed");
        let msg = msg.unwrap();

        match msg.message_type {
            MessageType::Membership { milestone_months } => {
                assert_eq!(milestone_months, None, "New member should have no milestone");
            }
            _ => panic!("Expected Membership message type"),
        }
    }
}
