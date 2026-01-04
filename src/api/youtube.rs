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

/// YouTubeライブチャットのモード
///
/// - TopChat: フィルタリングされた重要なメッセージのみ表示
/// - AllChat: すべてのチャットメッセージを表示
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ChatMode {
    /// トップチャット（フィルタリングあり）
    #[default]
    TopChat,
    /// すべてのチャット（フィルタリングなし）
    AllChat,
}

impl ChatMode {
    /// 表示用の日本語名を取得
    pub fn display_name(&self) -> &'static str {
        match self {
            ChatMode::TopChat => "トップチャット",
            ChatMode::AllChat => "すべてのチャット",
        }
    }

    /// 説明文を取得
    pub fn description(&self) -> &'static str {
        match self {
            ChatMode::TopChat => "重要なメッセージのみ表示（YouTube推奨）",
            ChatMode::AllChat => "すべてのメッセージを表示",
        }
    }
}

impl std::fmt::Display for ChatMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// 両方のチャットモード用のcontinuation tokenを保持
#[derive(Debug, Clone)]
pub struct ChatContinuations {
    /// トップチャット用のcontinuation token
    pub top_chat: Option<Continuation>,
    /// すべてのチャット用のcontinuation token
    pub all_chat: Option<Continuation>,
}

impl ChatContinuations {
    /// 指定されたモードに対応するcontinuation tokenを取得
    pub fn get_for_mode(&self, mode: ChatMode) -> Option<&Continuation> {
        match mode {
            ChatMode::TopChat => self.top_chat.as_ref(),
            ChatMode::AllChat => self.all_chat.as_ref(),
        }
    }

    /// いずれかのトークンが存在するか確認
    pub fn has_any(&self) -> bool {
        self.top_chat.is_some() || self.all_chat.is_some()
    }

    /// デフォルトのトークンを取得（TopChat優先、なければAllChat）
    pub fn get_default(&self) -> Option<&Continuation> {
        self.top_chat.as_ref().or(self.all_chat.as_ref())
    }
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
    /// 現在使用中のcontinuation token
    pub continuation: Continuation,
    /// 両方のチャットモード用のcontinuation tokens
    pub chat_continuations: Option<ChatContinuations>,
    /// 現在選択されているチャットモード
    pub chat_mode: ChatMode,
}

impl InnerTube {
    /// チャットモードを変更し、対応するcontinuation tokenに切り替える
    pub fn set_chat_mode(&mut self, mode: ChatMode) -> bool {
        if let Some(ref continuations) = self.chat_continuations {
            if let Some(token) = continuations.get_for_mode(mode) {
                self.chat_mode = mode;
                self.continuation = token.clone();
                return true;
            }
        }
        false
    }

    /// 現在のチャットモードを取得
    pub fn current_chat_mode(&self) -> ChatMode {
        self.chat_mode
    }

    /// 利用可能なチャットモードを取得
    pub fn available_chat_modes(&self) -> Vec<ChatMode> {
        let mut modes = Vec::new();
        if let Some(ref continuations) = self.chat_continuations {
            if continuations.top_chat.is_some() {
                modes.push(ChatMode::TopChat);
            }
            if continuations.all_chat.is_some() {
                modes.push(ChatMode::AllChat);
            }
        }
        if modes.is_empty() {
            // デフォルトで現在のモードのみ
            modes.push(self.chat_mode);
        }
        modes
    }
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

/// HTMLから両方のチャットモード用continuation tokenを抽出
///
/// YouTubeのHTMLにはsubMenuItems内に2種類のモード用トークンが含まれている:
/// - "トップチャット" (selected:true) → フィルタリングされたチャット
/// - "チャット" (selected:false) → すべてのチャット
///
/// 注意: これらはreloadContinuationData内の短いトークンであり、
/// モード切替時に使用する。初回メッセージ取得にはextract_continuation()の
/// 長いトークンを使用すること。
pub fn extract_chat_continuations(html: &str) -> ChatContinuations {
    // subMenuItems形式を優先的に使用
    // 日本語: "トップチャット" / "チャット"
    // 英語: "Top chat" / "Live chat"
    let (top_chat, all_chat) = extract_chat_continuations_from_submenu(html);

    if top_chat.is_some() || all_chat.is_some() {
        return ChatContinuations { top_chat, all_chat };
    }

    // フォールバック: selected パターン
    let top_chat_pattern = Regex::new(
        r#""selected"\s*:\s*true[^}]*"continuation"\s*:\s*\{\s*"reloadContinuationData"\s*:\s*\{\s*"continuation"\s*:\s*"([^"]+)""#
    ).ok();

    let all_chat_pattern = Regex::new(
        r#""selected"\s*:\s*false[^}]*"continuation"\s*:\s*\{\s*"reloadContinuationData"\s*:\s*\{\s*"continuation"\s*:\s*"([^"]+)""#
    ).ok();

    let top_chat = top_chat_pattern
        .and_then(|re| re.captures(html))
        .and_then(|cap| cap.get(1))
        .map(|m| Continuation(m.as_str().to_string()));

    let all_chat = all_chat_pattern
        .and_then(|re| re.captures(html))
        .and_then(|cap| cap.get(1))
        .map(|m| Continuation(m.as_str().to_string()));

    ChatContinuations { top_chat, all_chat }
}

/// subMenuItems形式からチャットモード用トークンを抽出
///
/// YouTubeのsubMenuItemsは以下のネスト形式:
/// "subMenuItems":[{
///   "title":"トップのチャット",
///   "continuation":{"reloadContinuationData":{"continuation":"TOKEN"}}
/// }]
fn extract_chat_continuations_from_submenu(html: &str) -> (Option<Continuation>, Option<Continuation>) {
    // HTML全体から直接トークンを抽出（subMenuItemsセクションを別途抽出する必要なし）

    // トップチャット: ネストされた形式に対応
    // タイトル: "Top chat", "トップチャット", "トップのチャット", "トップ チャット", "上位のチャット", "上位のチャットのリプレイ"
    let top_pattern = Regex::new(
        r#""title"\s*:\s*"(?:Top chat|トップチャット|トップのチャット|トップ チャット|上位のチャット(?:のリプレイ)?)"[^}]*"continuation"\s*:\s*\{\s*"reloadContinuationData"\s*:\s*\{\s*"continuation"\s*:\s*"([^"]+)""#
    ).ok();

    // すべてのチャット: ネストされた形式に対応
    // タイトル: "Live chat", "チャット", "チャットのリプレイ"
    let all_pattern = Regex::new(
        r#""title"\s*:\s*"(?:Live chat|チャット(?:のリプレイ)?)"[^}]*"continuation"\s*:\s*\{\s*"reloadContinuationData"\s*:\s*\{\s*"continuation"\s*:\s*"([^"]+)""#
    ).ok();

    // デバッグ: subMenuItemsセクションが存在するか確認
    if html.contains("subMenuItems") {
        tracing::debug!("📋 subMenuItems found in HTML");
        // subMenuItemsセクションの内容をログに出力
        if let Some(start) = html.find("\"subMenuItems\"") {
            // UTF-8バイト境界を考慮してプレビューを取得
            let end = html[start..].char_indices()
                .take(500)
                .last()
                .map(|(i, c)| start + i + c.len_utf8())
                .unwrap_or(html.len());
            let preview = &html[start..end];
            tracing::debug!("📋 subMenuItems preview: {}", preview);
        }
    } else {
        tracing::debug!("📋 subMenuItems NOT found in HTML");
    }

    let top_chat = top_pattern
        .and_then(|re| re.captures(html))
        .and_then(|cap| cap.get(1))
        .map(|m| {
            tracing::debug!("📋 TopChat token extracted: {}...", &m.as_str()[..30.min(m.as_str().len())]);
            Continuation(m.as_str().to_string())
        });

    let all_chat = all_pattern
        .and_then(|re| re.captures(html))
        .and_then(|cap| cap.get(1))
        .map(|m| {
            tracing::debug!("📋 AllChat token extracted: {}...", &m.as_str()[..30.min(m.as_str().len())]);
            Continuation(m.as_str().to_string())
        });

    if top_chat.is_none() {
        tracing::debug!("📋 TopChat pattern did NOT match");
    }
    if all_chat.is_none() {
        tracing::debug!("📋 AllChat pattern did NOT match");
    }

    (top_chat, all_chat)
}

fn extract_hl(html: &str) -> Option<String> {
    Regex::new(r#"['"]hl['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

/// 配信者のチャンネルIDをHTMLから抽出
///
/// YouTubeのHTMLから配信者（動画の所有者）のチャンネルIDを抽出する。
/// チャンネルIDは以下の場所に存在する可能性がある:
/// - ytInitialPlayerResponse.videoDetails.channelId
/// - browseEndpoint.browseId (UC prefix)
/// - videoOwnerRenderer.navigationEndpoint
///
/// # Arguments
/// * `html` - YouTubeページのHTML
///
/// # Returns
/// * `Some(String)` - チャンネルID (UC で始まる24文字の文字列)
/// * `None` - チャンネルIDが見つからない場合
pub fn extract_broadcaster_channel_id(html: &str) -> Option<String> {
    // 優先度1: videoDetails.channelId (最も信頼性が高い)
    // パターン: "channelId":"UCxxxxxxxxxx..."
    if let Some(cap) = Regex::new(r#""channelId"\s*:\s*"(UC[a-zA-Z0-9_-]{22})""#)
        .ok()?
        .captures(html)
    {
        if let Some(m) = cap.get(1) {
            tracing::debug!("Broadcaster channel ID found via channelId: {}", m.as_str());
            return Some(m.as_str().to_string());
        }
    }

    // 優先度2: externalChannelId (ytInitialData内)
    // パターン: "externalChannelId":"UCxxxxxxxxxx..."
    if let Some(cap) = Regex::new(r#""externalChannelId"\s*:\s*"(UC[a-zA-Z0-9_-]{22})""#)
        .ok()?
        .captures(html)
    {
        if let Some(m) = cap.get(1) {
            tracing::debug!("Broadcaster channel ID found via externalChannelId: {}", m.as_str());
            return Some(m.as_str().to_string());
        }
    }

    // 優先度3: browseEndpoint.browseId (チャンネルページへのリンク)
    // パターン: "browseId":"UCxxxxxxxxxx..."
    if let Some(cap) = Regex::new(r#""browseId"\s*:\s*"(UC[a-zA-Z0-9_-]{22})""#)
        .ok()?
        .captures(html)
    {
        if let Some(m) = cap.get(1) {
            tracing::debug!("Broadcaster channel ID found via browseId: {}", m.as_str());
            return Some(m.as_str().to_string());
        }
    }

    tracing::debug!("Broadcaster channel ID not found in HTML");
    None
}

fn extract_gl(html: &str) -> Option<String> {
    Regex::new(r#"['"]gl['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

pub async fn fetch_live_chat_page(url: &str) -> Result<InnerTube> {
    fetch_live_chat_page_with_mode(url, ChatMode::default()).await
}

/// 指定したチャットモードでライブチャットページを取得
pub async fn fetch_live_chat_page_with_mode(url: &str, preferred_mode: ChatMode) -> Result<InnerTube> {
    let response = reqwest::get(url).await?;
    let html = response.text().await?;

    let video_id = extract_video_id(&html).ok_or_else(|| anyhow::anyhow!("video_id not found"))?;
    let api_key = extract_api_key(&html).ok_or_else(|| anyhow::anyhow!("api_key not found"))?;
    let is_replay = extract_replay(&html);
    let client_version =
        extract_client_version(&html).ok_or_else(|| anyhow::anyhow!("client_version not found"))?;
    let gl = extract_gl(&html).unwrap_or_default();
    let hl = extract_hl(&html).unwrap_or_default();

    // 両方のチャットモード用トークンを抽出
    let chat_continuations = extract_chat_continuations(&html);

    // 優先モードのトークンを取得、なければデフォルトまたはフォールバック
    let (continuation, chat_mode) = if let Some(token) = chat_continuations.get_for_mode(preferred_mode) {
        (token.clone(), preferred_mode)
    } else if let Some(token) = chat_continuations.get_default() {
        // 優先モードが利用できない場合はデフォルトを使用
        let actual_mode = if chat_continuations.top_chat.is_some() {
            ChatMode::TopChat
        } else {
            ChatMode::AllChat
        };
        (token.clone(), actual_mode)
    } else {
        // ChatContinuationsから取得できなかった場合は従来の方法でフォールバック
        let fallback = extract_continuation(&html)
            .ok_or_else(|| anyhow::anyhow!("continuation not found"))?;
        (fallback, ChatMode::TopChat)
    };

    let chat_continuations = if chat_continuations.has_any() {
        Some(chat_continuations)
    } else {
        None
    };

    Ok(InnerTube {
        video_id,
        api_key,
        is_replay,
        client_version,
        gl,
        hl,
        continuation,
        chat_continuations,
        chat_mode,
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
            chat_continuations: None,
            chat_mode: ChatMode::TopChat,
        };

        assert_eq!(inner_tube.video_id.0, "test_video");
        assert_eq!(inner_tube.api_key.0, "test_key");
        assert!(inner_tube.is_replay);
        assert_eq!(inner_tube.client_version.0, "2.0");
        assert_eq!(inner_tube.gl, "JP");
        assert_eq!(inner_tube.hl, "ja");
        assert_eq!(inner_tube.continuation.0, "test_continuation");
        assert_eq!(inner_tube.chat_mode, ChatMode::TopChat);
    }

    #[test]
    fn test_chat_mode_display() {
        assert_eq!(ChatMode::TopChat.display_name(), "トップチャット");
        assert_eq!(ChatMode::AllChat.display_name(), "すべてのチャット");
        assert_eq!(format!("{}", ChatMode::TopChat), "トップチャット");
        assert_eq!(format!("{}", ChatMode::AllChat), "すべてのチャット");
    }

    #[test]
    fn test_chat_mode_default() {
        assert_eq!(ChatMode::default(), ChatMode::TopChat);
    }

    #[test]
    fn test_chat_continuations() {
        let continuations = ChatContinuations {
            top_chat: Some(Continuation("top_token".to_string())),
            all_chat: Some(Continuation("all_token".to_string())),
        };

        assert!(continuations.has_any());
        assert_eq!(
            continuations.get_for_mode(ChatMode::TopChat).unwrap().0,
            "top_token"
        );
        assert_eq!(
            continuations.get_for_mode(ChatMode::AllChat).unwrap().0,
            "all_token"
        );
        assert_eq!(continuations.get_default().unwrap().0, "top_token");
    }

    #[test]
    fn test_inner_tube_set_chat_mode() {
        let mut inner_tube = InnerTube {
            video_id: VideoId("test_video".to_string()),
            api_key: ApiKey("test_key".to_string()),
            is_replay: false,
            client_version: ClientVersion("2.0".to_string()),
            gl: "JP".to_string(),
            hl: "ja".to_string(),
            continuation: Continuation("top_token".to_string()),
            chat_continuations: Some(ChatContinuations {
                top_chat: Some(Continuation("top_token".to_string())),
                all_chat: Some(Continuation("all_token".to_string())),
            }),
            chat_mode: ChatMode::TopChat,
        };

        // AllChatに切り替え
        assert!(inner_tube.set_chat_mode(ChatMode::AllChat));
        assert_eq!(inner_tube.chat_mode, ChatMode::AllChat);
        assert_eq!(inner_tube.continuation.0, "all_token");

        // TopChatに戻す
        assert!(inner_tube.set_chat_mode(ChatMode::TopChat));
        assert_eq!(inner_tube.chat_mode, ChatMode::TopChat);
        assert_eq!(inner_tube.continuation.0, "top_token");
    }

    #[test]
    fn test_inner_tube_available_chat_modes() {
        let inner_tube = InnerTube {
            video_id: VideoId("test_video".to_string()),
            api_key: ApiKey("test_key".to_string()),
            is_replay: false,
            client_version: ClientVersion("2.0".to_string()),
            gl: "JP".to_string(),
            hl: "ja".to_string(),
            continuation: Continuation("top_token".to_string()),
            chat_continuations: Some(ChatContinuations {
                top_chat: Some(Continuation("top_token".to_string())),
                all_chat: Some(Continuation("all_token".to_string())),
            }),
            chat_mode: ChatMode::TopChat,
        };

        let modes = inner_tube.available_chat_modes();
        assert_eq!(modes.len(), 2);
        assert!(modes.contains(&ChatMode::TopChat));
        assert!(modes.contains(&ChatMode::AllChat));
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

    #[test]
    fn test_extract_chat_continuations_nested_format() {
        // 実際のYouTubeのHTML構造をシミュレート
        let html = r#"
        "subMenuItems":[{"title":"上位のチャットのリプレイ","selected":true,"continuation":{"reloadContinuationData":{"continuation":"TOP_TOKEN_123","clickTrackingParams":"xxx"}},"accessibility":{"accessibilityData":{"label":"上位のチャットのリプレイ"}}},{"title":"チャットのリプレイ","selected":false,"continuation":{"reloadContinuationData":{"continuation":"ALL_TOKEN_456","clickTrackingParams":"yyy"}},"accessibility":{"accessibilityData":{"label":"チャットのリプレイ"}}}]
        "#;

        let continuations = extract_chat_continuations(html);

        assert!(continuations.top_chat.is_some(), "TopChat token should be extracted");
        assert!(continuations.all_chat.is_some(), "AllChat token should be extracted");
        assert_eq!(continuations.top_chat.unwrap().0, "TOP_TOKEN_123");
        assert_eq!(continuations.all_chat.unwrap().0, "ALL_TOKEN_456");
    }

    #[test]
    fn test_extract_chat_continuations_live_format() {
        // ライブ配信の形式（日本語）
        let html = r#"
        "subMenuItems":[{"title":"トップのチャット","selected":true,"continuation":{"reloadContinuationData":{"continuation":"LIVE_TOP","clickTrackingParams":"xxx"}}},{"title":"チャット","selected":false,"continuation":{"reloadContinuationData":{"continuation":"LIVE_ALL","clickTrackingParams":"yyy"}}}]
        "#;

        let continuations = extract_chat_continuations(html);

        assert!(continuations.top_chat.is_some(), "TopChat token should be extracted");
        assert!(continuations.all_chat.is_some(), "AllChat token should be extracted");
        assert_eq!(continuations.top_chat.unwrap().0, "LIVE_TOP");
        assert_eq!(continuations.all_chat.unwrap().0, "LIVE_ALL");
    }

    #[test]
    fn test_extract_chat_continuations_english_format() {
        // English format
        let html = r#"
        "subMenuItems":[{"title":"Top chat","selected":true,"continuation":{"reloadContinuationData":{"continuation":"EN_TOP","clickTrackingParams":"xxx"}}},{"title":"Live chat","selected":false,"continuation":{"reloadContinuationData":{"continuation":"EN_ALL","clickTrackingParams":"yyy"}}}]
        "#;

        let continuations = extract_chat_continuations(html);

        assert!(continuations.top_chat.is_some(), "TopChat token should be extracted");
        assert!(continuations.all_chat.is_some(), "AllChat token should be extracted");
        assert_eq!(continuations.top_chat.unwrap().0, "EN_TOP");
        assert_eq!(continuations.all_chat.unwrap().0, "EN_ALL");
    }

    #[test]
    fn test_extract_chat_continuations_real_youtube_format() {
        // 実際のYouTubeから取得したHTML構造
        let html = r#""subMenuItems":[{"title":"上位のチャットのリプレイ","selected":true,"continuation":{"reloadContinuationData":{"continuation":"op2w0wQOGgBAAXIICAQYACAAKAE%3D","clickTrackingParams":"CE0QxqYCIhMIh5PElP3fkQMVoNU0Bx1UbSIpygEExcfLOg=="}},"accessibility":{"accessibilityData":{"label":"上位のチャットのリプレイ"}},"subtitle":"一部のメッセージ（不適切な可能性があるものなど）を非表示にします","trackingParams":"CEwQ48AHGAAiEwiHk8SU_d-RAxWg1TQHHVRtIik="},{"title":"チャットのリプレイ","selected":false,"continuation":{"reloadContinuationData":{"continuation":"op2w0wQOGgBAAXIICAEYACAAKAE%3D","clickTrackingParams":"CEsQxqYCIhMIh5PElP3fkQMVoNU0Bx1UbSIpygEExcfLOg=="}},"accessibility":{"accessibilityData":{"label":"チャットのリプレイ"}},"subtitle":"すべてのメッセージが表示されます","trackingParams":"CEoQ48AHGAEiEwiHk8SU_d-RAxWg1TQHHVRtIik="}]"#;

        let continuations = extract_chat_continuations(html);

        assert!(continuations.top_chat.is_some(), "TopChat token should be extracted from real YouTube HTML");
        assert!(continuations.all_chat.is_some(), "AllChat token should be extracted from real YouTube HTML");
        assert_eq!(continuations.top_chat.unwrap().0, "op2w0wQOGgBAAXIICAQYACAAKAE%3D");
        assert_eq!(continuations.all_chat.unwrap().0, "op2w0wQOGgBAAXIICAEYACAAKAE%3D");
    }

    // ========================================
    // 配信者チャンネルID抽出のテスト
    // ========================================

    #[test]
    fn test_extract_broadcaster_channel_id_from_channel_id() {
        // videoDetails.channelId パターン
        let html = r#"{"videoDetails":{"videoId":"abc123","channelId":"UCabcdefghij1234567890AB","title":"Test Stream"}}"#;
        let result = extract_broadcaster_channel_id(html);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "UCabcdefghij1234567890AB");
    }

    #[test]
    fn test_extract_broadcaster_channel_id_from_external_channel_id() {
        // externalChannelId パターン (UC + 22文字 = 24文字)
        let html = r#"{"externalChannelId":"UCexternal_channel123456"}"#;
        let result = extract_broadcaster_channel_id(html);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "UCexternal_channel123456");
    }

    #[test]
    fn test_extract_broadcaster_channel_id_from_browse_id() {
        // browseId パターン (UC + 22文字 = 24文字)
        let html = r#"{"browseEndpoint":{"browseId":"UCbrowse_endpoint_id1234"}}"#;
        let result = extract_broadcaster_channel_id(html);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "UCbrowse_endpoint_id1234");
    }

    #[test]
    fn test_extract_broadcaster_channel_id_priority() {
        // 複数パターンがある場合、channelIdが優先される (UC + 22文字 = 24文字)
        let html = r#"{"channelId":"UCpriority_first_chan001","externalChannelId":"UCsecond_priority_cha02","browseId":"UCthird_priority_chan03"}"#;
        let result = extract_broadcaster_channel_id(html);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "UCpriority_first_chan001");
    }

    #[test]
    fn test_extract_broadcaster_channel_id_not_found() {
        let html = r#"<html><body>No channel ID here</body></html>"#;
        let result = extract_broadcaster_channel_id(html);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_broadcaster_channel_id_invalid_format() {
        // UCで始まらない、または長さが不正
        let html = r#"{"channelId":"NOT_UC_prefix_12345678901"}"#;
        let result = extract_broadcaster_channel_id(html);
        assert!(result.is_none());

        let html_short = r#"{"channelId":"UCtooshort"}"#;
        let result_short = extract_broadcaster_channel_id(html_short);
        assert!(result_short.is_none());
    }

    #[test]
    fn test_extract_broadcaster_channel_id_with_special_chars() {
        // ハイフンとアンダースコアを含むチャンネルID (UC + 22文字 = 24文字)
        let html = r#"{"channelId":"UC_special-chars_id-1234"}"#;
        let result = extract_broadcaster_channel_id(html);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "UC_special-chars_id-1234");
    }
}
