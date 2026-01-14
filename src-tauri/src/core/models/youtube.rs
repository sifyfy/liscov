//! YouTube-specific models

use serde::{Deserialize, Serialize};

/// Video ID wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoId(pub String);

impl From<String> for VideoId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for VideoId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// API Key wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey(pub String);

/// Client version wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientVersion(pub String);

impl Default for ClientVersion {
    fn default() -> Self {
        Self("2.20240101.00.00".to_string())
    }
}

/// Continuation token wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Continuation(pub String);

/// Chat mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChatMode {
    #[default]
    TopChat,
    AllChat,
}

/// Chat continuations for both modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContinuations {
    pub top_chat: Option<String>,
    pub all_chat: Option<String>,
}

/// YouTube cookies for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeCookies {
    pub sid: String,
    pub hsid: String,
    pub ssid: String,
    pub apisid: String,
    pub sapisid: String,
}

impl YouTubeCookies {
    /// Build cookie header string
    pub fn to_cookie_string(&self) -> String {
        format!(
            "SID={}; HSID={}; SSID={}; APISID={}; SAPISID={}",
            self.sid, self.hsid, self.ssid, self.apisid, self.sapisid
        )
    }
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub is_connected: bool,
    pub stream_title: Option<String>,
    pub broadcaster_channel_id: Option<String>,
    pub broadcaster_name: Option<String>,
    pub chat_mode: ChatMode,
    pub is_replay: bool,
    pub error: Option<String>,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            is_connected: false,
            stream_title: None,
            broadcaster_channel_id: None,
            broadcaster_name: None,
            chat_mode: ChatMode::default(),
            is_replay: false,
            error: None,
        }
    }
}

/// Extract video ID from YouTube URL
pub fn extract_video_id(url: &str) -> Option<String> {
    // Handle various YouTube URL formats
    let url = url.trim();
    
    // youtu.be/VIDEO_ID
    if url.contains("youtu.be/") {
        return url.split("youtu.be/")
            .nth(1)
            .and_then(|s| s.split(&['?', '&', '#'][..]).next())
            .map(|s| s.to_string());
    }
    
    // youtube.com/watch?v=VIDEO_ID
    if url.contains("youtube.com") {
        if let Some(query) = url.split('?').nth(1) {
            for param in query.split('&') {
                if let Some(value) = param.strip_prefix("v=") {
                    return Some(value.split(&['&', '#'][..]).next()?.to_string());
                }
            }
        }
        // youtube.com/live/VIDEO_ID
        if url.contains("/live/") {
            return url.split("/live/")
                .nth(1)
                .and_then(|s| s.split(&['?', '&', '#', '/'][..]).next())
                .map(|s| s.to_string());
        }
    }
    
    // Plain video ID (11 characters)
    if url.len() == 11 && url.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Some(url.to_string());
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/live/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }
}
