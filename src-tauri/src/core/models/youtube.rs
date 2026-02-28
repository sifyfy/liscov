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
    /// Full cookie string from browser (includes __Secure-* cookies needed for member-only streams)
    #[serde(default)]
    pub raw_cookie_string: Option<String>,
}

impl YouTubeCookies {
    /// Build cookie header string
    /// Returns full browser cookies if available, otherwise the 5 API cookies
    pub fn to_cookie_string(&self) -> String {
        if let Some(ref raw) = self.raw_cookie_string {
            raw.clone()
        } else {
            format!(
                "SID={}; HSID={}; SSID={}; APISID={}; SAPISID={}",
                self.sid, self.hsid, self.ssid, self.apisid, self.sapisid
            )
        }
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

    // Any URL with watch?v=VIDEO_ID (supports youtube.com and localhost for testing)
    if url.contains("/watch") {
        if let Some(query) = url.split('?').nth(1) {
            for param in query.split('&') {
                if let Some(value) = param.strip_prefix("v=") {
                    return Some(value.split(&['&', '#'][..]).next()?.to_string());
                }
            }
        }
    }

    // youtube.com/live/VIDEO_ID or any URL with /live/VIDEO_ID
    if url.contains("/live/") {
        return url.split("/live/")
            .nth(1)
            .and_then(|s| s.split(&['?', '&', '#', '/'][..]).next())
            .map(|s| s.to_string());
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
    fn test_to_cookie_string_returns_raw_when_present() {
        let cookies = YouTubeCookies {
            sid: "s".to_string(),
            hsid: "h".to_string(),
            ssid: "ss".to_string(),
            apisid: "a".to_string(),
            sapisid: "sa".to_string(),
            raw_cookie_string: Some("SID=s; HSID=h; __Secure-1PSID=sec1; YSC=ysc".to_string()),
        };
        assert_eq!(
            cookies.to_cookie_string(),
            "SID=s; HSID=h; __Secure-1PSID=sec1; YSC=ysc"
        );
    }

    #[test]
    fn test_to_cookie_string_falls_back_to_five_cookies() {
        let cookies = YouTubeCookies {
            sid: "sid_val".to_string(),
            hsid: "hsid_val".to_string(),
            ssid: "ssid_val".to_string(),
            apisid: "apisid_val".to_string(),
            sapisid: "sapisid_val".to_string(),
            raw_cookie_string: None,
        };
        let result = cookies.to_cookie_string();
        assert!(result.contains("SID=sid_val"));
        assert!(result.contains("HSID=hsid_val"));
        assert!(result.contains("SSID=ssid_val"));
        assert!(result.contains("APISID=apisid_val"));
        assert!(result.contains("SAPISID=sapisid_val"));
        // __Secure-* cookies are NOT included in fallback
        assert!(!result.contains("__Secure"));
    }

    #[test]
    fn test_youtube_cookies_serde_roundtrip_with_raw() {
        let cookies = YouTubeCookies {
            sid: "s".to_string(),
            hsid: "h".to_string(),
            ssid: "ss".to_string(),
            apisid: "a".to_string(),
            sapisid: "sa".to_string(),
            raw_cookie_string: Some("SID=s; __Secure-1PSID=sec1".to_string()),
        };
        let json = serde_json::to_string(&cookies).unwrap();
        let deserialized: YouTubeCookies = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.raw_cookie_string, cookies.raw_cookie_string);
        assert_eq!(deserialized.to_cookie_string(), cookies.to_cookie_string());
    }

    #[test]
    fn test_youtube_cookies_serde_backwards_compatible() {
        // raw_cookie_string が無い古い形式のJSONからもデシリアライズできる
        let json = r#"{"sid":"s","hsid":"h","ssid":"ss","apisid":"a","sapisid":"sa"}"#;
        let cookies: YouTubeCookies = serde_json::from_str(json).unwrap();
        assert!(cookies.raw_cookie_string.is_none());
        assert!(cookies.to_cookie_string().contains("SID=s"));
    }

    // G4: Cookie Usage Consistency — raw_cookie_stringが5基本Cookieより優先されることを保証
    #[test]
    fn to_cookie_string_returns_raw_not_five_basic() {
        // raw_cookie_stringがある場合、to_cookie_string()はraw（追加Cookie含む）を返す。
        // 5基本Cookieのみの文字列とは異なる出力であることを値の差異で検証。
        let cookies = YouTubeCookies {
            sid: "s".to_string(),
            hsid: "h".to_string(),
            ssid: "ss".to_string(),
            apisid: "a".to_string(),
            sapisid: "sa".to_string(),
            raw_cookie_string: Some(
                "SID=s; HSID=h; SSID=ss; APISID=a; SAPISID=sa; __Secure-1PSID=sec1; YSC=ysc".to_string()
            ),
        };

        let result = cookies.to_cookie_string();

        // raw_cookie_stringを返す（5基本のみではない）
        assert!(result.contains("__Secure-1PSID=sec1"), "raw should include __Secure-1PSID");
        assert!(result.contains("YSC=ysc"), "raw should include YSC");

        // 5基本のみの文字列とは異なることを確認
        let five_basic_only = format!(
            "SID={}; HSID={}; SSID={}; APISID={}; SAPISID={}",
            cookies.sid, cookies.hsid, cookies.ssid, cookies.apisid, cookies.sapisid
        );
        assert_ne!(result, five_basic_only, "should return raw, not 5-basic fallback");
    }

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
        // Test localhost URL for testing
        assert_eq!(
            extract_video_id("http://localhost:3456/watch?v=test_video_123"),
            Some("test_video_123".to_string())
        );
    }
}
