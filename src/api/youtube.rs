#[allow(dead_code)]
use anyhow::Result;
use regex::Regex;

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Request failed")]
    Request(#[from] reqwest::Error),
    #[error("Live chat ID not found")]
    NotFound,
    #[error("Failed to parse JSON")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Clone, derive_more::Display)]
pub struct VideoId(pub String);

#[derive(Debug, Clone, derive_more::Display)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, derive_more::Display)]
pub struct ClientVersion(String);

impl ClientVersion {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, derive_more::Display, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Continuation(pub String);

#[derive(Debug, Clone)]
pub struct InnerTube {
    pub video_id: VideoId,
    pub api_key: ApiKey,
    pub is_replay: bool,
    pub client_version: ClientVersion,
    pub gl: String,
    pub hl: String,
    pub continuation: Continuation,
}

pub fn extract_video_id(html: &str) -> Option<VideoId> {
    Regex::new(r#"<link rel="canonical" href="https:\/\/www.youtube.com\/watch\?v=(.+?)">"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| VideoId(m.as_str().to_string()))
}

pub fn extract_api_key(html: &str) -> Option<ApiKey> {
    Regex::new(r#"['"]INNERTUBE_API_KEY['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| ApiKey::new(m.as_str().to_string()))
}

pub fn extract_replay(html: &str) -> bool {
    Regex::new(r#"['"]isReplay['"]:\s*true"#)
        .unwrap()
        .is_match(html)
}

pub fn extract_client_version(html: &str) -> Option<ClientVersion> {
    Regex::new(r#"['"]INNERTUBE_CLIENT_VERSION['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| ClientVersion::new(m.as_str().to_string()))
}

pub fn extract_continuation(html: &str) -> Option<Continuation> {
    Regex::new(r#"['"]continuation['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| Continuation(m.as_str().to_string()))
}

fn extract_hl(html: &str) -> Option<String> {
    Regex::new(r#"['"]hl['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_gl(html: &str) -> Option<String> {
    Regex::new(r#"['"]gl['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

pub async fn fetch_live_chat_page(url: &str) -> Result<InnerTube> {
    let response = reqwest::get(url).await?;
    let html = response.text().await?;

    let video_id = extract_video_id(&html).ok_or_else(|| anyhow::anyhow!("video_id not found"))?;
    let api_key = extract_api_key(&html).ok_or_else(|| anyhow::anyhow!("api_key not found"))?;
    let is_replay = extract_replay(&html);
    let client_version =
        extract_client_version(&html).ok_or_else(|| anyhow::anyhow!("client_version not found"))?;
    let gl = extract_gl(&html).unwrap_or_default();
    let hl = extract_hl(&html).unwrap_or_default();
    let continuation =
        extract_continuation(&html).ok_or_else(|| anyhow::anyhow!("continuation not found"))?;

    Ok(InnerTube {
        video_id,
        api_key,
        is_replay,
        client_version,
        gl,
        hl,
        continuation,
    })
}

pub async fn fetch_live_chat_messages(inner_tube: &InnerTube) -> Result<serde_json::Value> {
    let url = format!(
        "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
        inner_tube.api_key
    );

    let post_body = serde_json::to_string(&serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": inner_tube.client_version.0.as_str(),
                "gl": inner_tube.gl.as_str(),
                "hl": inner_tube.hl.as_str(),
            }
        },
        "continuation": inner_tube.continuation.0.as_str(),
    }))?;

    let client = reqwest::Client::new();
    let res = client.post(&url).body(post_body).send().await?;
    let value: serde_json::Value = res.json().await?;

    Ok(value)
}

pub async fn fetch_live_chat_id(api_key: &str, video_id: &str) -> Result<String, FetchError> {
    let url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=liveStreamingDetails&id={}&key={}",
        video_id, api_key
    );

    let client = reqwest::Client::new();
    let res = client.get(&url).send().await?;
    let text = res.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;

    let live_chat_id = json
        .get("items")
        .and_then(|v| {
            v.as_array()?
                .first()?
                .get("liveStreamingDetails")?
                .get("activeLiveChatId")?
                .as_str()
                .map(|id| id.to_string())
        })
        .ok_or_else(|| FetchError::NotFound)?;

    Ok(live_chat_id)
}

pub async fn fetch_comments(api_key: &str, live_chat_id: &str) -> Result<String, FetchError> {
    let url = format!(
        "https://www.googleapis.com/youtube/v3/liveChat/messages?part=snippet&liveChatId={}&key={}",
        live_chat_id, api_key
    );

    let client = reqwest::Client::new();
    let res = client.get(&url).send().await?;
    let text = res.text().await?;

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_id_creation_and_display() {
        let video_id = VideoId("dQw4w9WgXcQ".to_string());
        assert_eq!(video_id.0, "dQw4w9WgXcQ");
        assert_eq!(format!("{}", video_id), "dQw4w9WgXcQ");
    }

    #[test]
    fn test_api_key_creation_and_display() {
        let api_key = ApiKey("test_api_key_123".to_string());
        assert_eq!(api_key.0, "test_api_key_123");
        assert_eq!(format!("{}", api_key), "test_api_key_123");
    }

    #[test]
    fn test_client_version_creation_and_display() {
        let client_version = ClientVersion("2.20240101.01.00".to_string());
        assert_eq!(client_version.0, "2.20240101.01.00");
        assert_eq!(format!("{}", client_version), "2.20240101.01.00");
    }

    #[test]
    fn test_continuation_creation_and_display() {
        let continuation = Continuation("test_continuation_token".to_string());
        assert_eq!(continuation.0, "test_continuation_token");
        assert_eq!(format!("{}", continuation), "test_continuation_token");
    }

    #[test]
    fn test_continuation_serialization() {
        let continuation = Continuation("test_token".to_string());
        let serialized = serde_json::to_string(&continuation).unwrap();
        assert_eq!(serialized, "\"test_token\"");

        let deserialized: Continuation = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.0, "test_token");
    }

    #[test]
    fn test_inner_tube_creation() {
        let inner_tube = InnerTube {
            video_id: VideoId("test_video".to_string()),
            api_key: ApiKey("test_key".to_string()),
            is_replay: true,
            client_version: ClientVersion("2.0".to_string()),
            gl: "JP".to_string(),
            hl: "ja".to_string(),
            continuation: Continuation("test_continuation".to_string()),
        };

        assert_eq!(inner_tube.video_id.0, "test_video");
        assert_eq!(inner_tube.api_key.0, "test_key");
        assert!(inner_tube.is_replay);
        assert_eq!(inner_tube.client_version.0, "2.0");
        assert_eq!(inner_tube.gl, "JP");
        assert_eq!(inner_tube.hl, "ja");
        assert_eq!(inner_tube.continuation.0, "test_continuation");
    }

    #[test]
    fn test_extract_video_id() {
        let html = r#"<link rel="canonical" href="https://www.youtube.com/watch?v=dQw4w9WgXcQ">"#;
        let video_id = extract_video_id(html);
        assert!(video_id.is_some());
        assert_eq!(video_id.unwrap().0, "dQw4w9WgXcQ");
    }

    #[test]
    fn test_extract_video_id_not_found() {
        let html = r#"<link rel="canonical" href="https://example.com">"#;
        let video_id = extract_video_id(html);
        assert!(video_id.is_none());
    }

    #[test]
    fn test_extract_api_key() {
        let html = r#""INNERTUBE_API_KEY": "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8""#;
        let api_key = extract_api_key(html);
        assert!(api_key.is_some());
        assert_eq!(
            api_key.unwrap().0,
            "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8"
        );
    }

    #[test]
    fn test_extract_api_key_not_found() {
        let html = r#"<html><body>No API key here</body></html>"#;
        let api_key = extract_api_key(html);
        assert!(api_key.is_none());
    }

    #[test]
    fn test_extract_replay_true() {
        let html = r#""isReplay": true"#;
        let is_replay = extract_replay(html);
        assert!(is_replay);
    }

    #[test]
    fn test_extract_replay_false() {
        let html = r#""isReplay": false"#;
        let is_replay = extract_replay(html);
        assert!(!is_replay);
    }

    #[test]
    fn test_extract_replay_not_found() {
        let html = r#"<html><body>No replay info</body></html>"#;
        let is_replay = extract_replay(html);
        assert!(!is_replay);
    }

    #[test]
    fn test_extract_client_version() {
        let html = r#""INNERTUBE_CLIENT_VERSION": "2.20240101.01.00""#;
        let client_version = extract_client_version(html);
        assert!(client_version.is_some());
        assert_eq!(client_version.unwrap().0, "2.20240101.01.00");
    }

    #[test]
    fn test_extract_continuation() {
        let html = r#""continuation": "0ofMyANBElJDaWtnNVFnPT0%3D""#;
        let continuation = extract_continuation(html);
        assert!(continuation.is_some());
        assert_eq!(continuation.unwrap().0, "0ofMyANBElJDaWtnNVFnPT0%3D");
    }

    #[test]
    fn test_extract_hl() {
        let html = r#""hl": "en""#;
        let hl = extract_hl(html);
        assert!(hl.is_some());
        assert_eq!(hl.unwrap(), "en");
    }

    #[test]
    fn test_extract_gl() {
        let html = r#""gl": "US""#;
        let gl = extract_gl(html);
        assert!(gl.is_some());
        assert_eq!(gl.unwrap(), "US");
    }

    #[test]
    fn test_fetch_error_display() {
        let error = FetchError::NotFound;
        assert_eq!(format!("{}", error), "Live chat ID not found");
    }

    #[test]
    fn test_fetch_error_from_serde_json() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let fetch_error: FetchError = json_error.into();

        match fetch_error {
            FetchError::Parse(_) => {} // Expected
            _ => panic!("Expected Parse error"),
        }
    }
}
