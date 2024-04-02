use anyhow::anyhow;
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

#[derive(Debug, Clone, derive_more::Display)]
pub struct ClientVersion(String);

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

pub async fn fetch_live_chat_page(url: &str) -> anyhow::Result<InnerTube> {
    let response = reqwest::get(url).await?;
    let html = response.text().await?;

    let video_id = extract_video_id(&html).ok_or_else(|| anyhow!("video_id not found"))?;
    let api_key = extract_api_key(&html).ok_or_else(|| anyhow!("api_key not found"))?;
    let is_replay = extract_replay(&html);
    let client_version =
        extract_client_version(&html).ok_or_else(|| anyhow!("client_version not found"))?;
    let gl = extract_gl(&html).unwrap_or_default();
    let hl = extract_hl(&html).unwrap_or_default();
    let continuation =
        extract_continuation(&html).ok_or_else(|| anyhow!("continuation not found"))?;

    return Ok(InnerTube {
        video_id,
        api_key,
        is_replay,
        client_version,
        gl,
        hl,
        continuation,
    });
}

fn extract_video_id(html: &str) -> Option<VideoId> {
    Regex::new(r#"<link rel="canonical" href="https:\/\/www.youtube.com\/watch\?v=(.+?)">"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| VideoId(m.as_str().to_string()))
}

fn extract_api_key(html: &str) -> Option<ApiKey> {
    Regex::new(r#"['"]INNERTUBE_API_KEY['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| ApiKey(m.as_str().to_string()))
}

fn extract_replay(html: &str) -> bool {
    Regex::new(r#"['"]isReplay['"]:\s*true"#)
        .unwrap()
        .is_match(html)
}

fn extract_client_version(html: &str) -> Option<ClientVersion> {
    Regex::new(r#"['"]INNERTUBE_CLIENT_VERSION['"]:\s*['"](.+?)['"]"#)
        .unwrap()
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| ClientVersion(m.as_str().to_string()))
}

fn extract_continuation(html: &str) -> Option<Continuation> {
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

#[derive(thiserror::Error, Debug)]
pub enum GetLiveChatError {
    #[error("Request failed")]
    Request(#[from] reqwest::Error),
    #[error("Failed to parse JSON")]
    Parse(#[from] serde_json::Error),
    #[error("Failed to convert Value to GetLiveChatResponse: {0}")]
    FromValue(#[from] FromValueError),
}

pub async fn fetch_live_chat_messages(
    inner_tube: &InnerTube,
) -> Result<GetLiveChatResponse, GetLiveChatError> {
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
    let value: serde_json::Value = serde_json::from_slice(&res.bytes().await?)?;

    let response = GetLiveChatResponse::from_get_live_chat(&value)?;

    Ok(response)
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to convert value to {target}. error: {error:?}. value: {value:?}")]
pub struct FromValueError {
    pub target: String,
    pub value: serde_json::Value,
    #[source]
    pub error: Option<anyhow::Error>,
}

impl FromValueError {
    pub fn new(target: String, value: &serde_json::Value, error: Option<anyhow::Error>) -> Self {
        Self {
            target,
            value: value.clone(),
            error,
        }
    }
}

pub trait FromGetLiveChat: Sized {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetLiveChatResponse {
    // pub response_context: serde_json::Value, // ignore
    #[serde(rename = "continuationContents")]
    pub continuation_contents: ContinuationContents,
}

impl FromGetLiveChat for GetLiveChatResponse {
    fn from_get_live_chat(
        value: &serde_json::Value,
    ) -> Result<GetLiveChatResponse, FromValueError> {
        if let Some(v) = value.get("continuationContents") {
            Ok(GetLiveChatResponse {
                continuation_contents: ContinuationContents::from_get_live_chat(v)?,
            })
        } else {
            Err(FromValueError::new(
                "continuationContents".to_string(),
                value,
                None,
            ))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContinuationContents {
    pub live_chat_continuation: LiveChatContinuation,
}

impl FromGetLiveChat for ContinuationContents {
    fn from_get_live_chat(
        value: &serde_json::Value,
    ) -> Result<ContinuationContents, FromValueError> {
        if let Some(v) = value.get("liveChatContinuation") {
            Ok(ContinuationContents {
                live_chat_continuation: LiveChatContinuation::from_get_live_chat(v)?,
            })
        } else {
            Err(FromValueError::new(
                "liveChatContinuation".to_string(),
                value,
                None,
            ))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiveChatContinuation {
    pub continuation: Continuation,
    pub actions: Vec<Action>,
}

impl FromGetLiveChat for LiveChatContinuation {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError> {
        let continuation = value
            .pointer("/continuations/0")
            .and_then(|v| {
                v.get("invalidationContinuationData")
                    .or_else(|| v.get("timedContinuationData"))
                    .or_else(|| v.get("reloadContinuationData"))
            })
            .and_then(|v| {
                let c = v.get("continuation")?.as_str()?;
                Some(Continuation(c.to_string()))
            });

        let actions = value
            .get("actions")
            .and_then(|v| v.as_array())
            .map(|actions| {
                actions
                    .iter()
                    .map(|a| Action::from_get_live_chat(a))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap_or_else(|| Ok(vec![]))?;

        if let Some(continuation) = continuation {
            Ok(LiveChatContinuation {
                continuation,
                actions,
            })
        } else {
            Err(FromValueError::new(
                "LiveChatContinuation".to_string(),
                value,
                None,
            ))
        }
    }
}

#[derive(Debug, Clone, derive_more::Display, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ClientId(pub String);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Action {
    // 通常のチャットメッセージをどの様に表示するべきかという情報が入っている。
    AddChatItem(AddChatItemAction),
    // スパチャが来た時にチャット欄上部に表示されるやつ。どの様に表示するべきかという情報が入っている。
    AddLiveChatTickerItem(serde_json::Value),
    Unknown(serde_json::Value),
}

impl FromGetLiveChat for Action {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError> {
        if let Some(v) = value.get("addChatItemAction") {
            Ok(Action::AddChatItem(AddChatItemAction::from_get_live_chat(
                v,
            )?))
        } else if let Some(v) = value.get("addLiveChatTickerItemAction") {
            Ok(Action::AddLiveChatTickerItem(v.clone()))
        } else {
            Ok(Action::Unknown(value.clone()))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddChatItemAction {
    pub item: ChatItem,
    pub client_id: ClientId,
}

impl FromGetLiveChat for AddChatItemAction {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError> {
        let v = value
            .get("item")
            .ok_or_else(|| FromValueError::new("addChatItemAction".to_string(), value, None))?;
        let item = ChatItem::from_get_live_chat(&v)?;
        let client_id = value
            .get("clientId")
            .and_then(|v| Some(v.as_str()?.to_string()))
            .map(ClientId);

        if let Some(client_id) = client_id {
            Ok(AddChatItemAction { item, client_id })
        } else {
            Err(FromValueError::new(
                "AddChatItemAction".to_string(),
                value,
                None,
            ))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ChatItem {
    // 通常のチャットメッセージ。
    LiveChatTextMessage(LiveChatTextMessage),
    // スーパーチャット
    LiveChatPaidMessage(LiveChatPaidMessage),
    // スーパースティッカー
    LiveChatPaidSticker(serde_json::Value),
    // メンバーシップ加入のお知らせ。
    LiveChatMembershipItem(serde_json::Value),
    // 不明。
    LiveChatPlaceholderItem(serde_json::Value),
    // 不明。
    LiveChatSponsorshipsGiftPurchaseAnnouncement(serde_json::Value),
    // 不明。
    LiveChatSponsorshipsGiftRedemptionAnnouncement(serde_json::Value),
    // コメビュ的にはたぶん不要。
    LiveChatAutoModMessage(serde_json::Value),
    // コメビュ的にはたぶん不要。
    LiveChatModeChangeMessage(serde_json::Value),
    // チャットを開いた時に出るyoutubeからの案内メッセージっぽい. コメビュ的には無視で良い。
    LiveChatViewerEngagementMessage(serde_json::Value),
    // 想定していないオブジェクト構造のデータが来るとこれになります。
    Unknown(serde_json::Value),
}

impl FromGetLiveChat for ChatItem {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError> {
        if let Some((key, value)) = value.as_object().and_then(|o| o.iter().next()) {
            let renderer = match key.as_str() {
                "liveChatTextMessageRenderer" => {
                    ChatItem::LiveChatTextMessage(LiveChatTextMessage::from_get_live_chat(value)?)
                }
                "liveChatPaidMessageRenderer" => {
                    ChatItem::LiveChatPaidMessage(LiveChatPaidMessage::from_get_live_chat(value)?)
                }
                "liveChatPaidStickerRenderer" => {
                    // TODO
                    ChatItem::LiveChatPaidSticker(value.clone())
                }
                "liveChatMembershipItemRenderer" => {
                    // TODO
                    ChatItem::LiveChatMembershipItem(value.clone())
                }
                "liveChatPlaceholderItemRenderer" => {
                    // TODO
                    ChatItem::LiveChatPlaceholderItem(value.clone())
                }
                "liveChatSponsorshipsGiftPurchaseAnnouncementRenderer" => {
                    // TODO
                    ChatItem::LiveChatSponsorshipsGiftPurchaseAnnouncement(value.clone())
                }
                "liveChatSponsorshipsGiftRedemptionAnnouncementRenderer" => {
                    // TODO
                    ChatItem::LiveChatSponsorshipsGiftRedemptionAnnouncement(value.clone())
                }
                "liveChatAutoModMessage" => {
                    // コメビュ的にはたぶん不要
                    ChatItem::LiveChatAutoModMessage(value.clone())
                }
                "liveChatModeChangeMessage" => {
                    // コメビュ的にはたぶん不要
                    ChatItem::LiveChatModeChangeMessage(value.clone())
                }
                "liveChatViewerEngagementMessageRenderer" => {
                    ChatItem::LiveChatViewerEngagementMessage(value.clone())
                }
                _ => ChatItem::Unknown(value.clone()),
            };

            Ok(renderer)
        } else {
            Ok(ChatItem::Unknown(value.clone()))
        }
    }
}

#[derive(Debug, Clone, derive_more::Display, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ExternalChannelId(pub String);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiveChatTextMessage {
    pub id: String,
    pub runs: Vec<Run>,
    pub author_name: AuthorName,
}

impl FromGetLiveChat for LiveChatTextMessage {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError> {
        let runs = value
            .pointer("/message/runs")
            .and_then(|runs| runs.as_array())
            .map(|runs| {
                runs.iter()
                    .map(|v| Run::from_get_live_chat(v).expect("no error"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let author_name = value
            .get("authorName")
            .map(|v| AuthorName::from_get_live_chat(v))
            .unwrap_or_else(|| Ok(AuthorName::SimpleText("".to_string())))?;

        let id = value
            .get("id")
            .and_then(|v| Some(v.as_str()?.to_string()))
            .unwrap_or_default();

        Ok(LiveChatTextMessage {
            id,
            runs,
            author_name,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiveChatPaidMessage {
    pub id: String,
    pub runs: Vec<Run>,
    pub author_name: AuthorName,
}

impl FromGetLiveChat for LiveChatPaidMessage {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError> {
        let id = value
            .get("id")
            .and_then(|v| Some(v.as_str()?.to_string()))
            .unwrap_or_default();

        let runs = value
            .pointer("/message/runs")
            .and_then(|runs| runs.as_array())
            .map(|runs| {
                runs.iter()
                    .map(|v| Run::from_get_live_chat(v))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap_or_else(|| Ok(vec![]))?;

        let author_name = value
            .get("authorName")
            .map(|v| AuthorName::from_get_live_chat(v))
            .unwrap_or_else(|| Ok(AuthorName::SimpleText("不明なユーザー".to_string())))?;

        Ok(LiveChatPaidMessage {
            id,
            runs,
            author_name,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuthorName {
    SimpleText(String),
    Unknown(serde_json::Value),
}

impl FromGetLiveChat for AuthorName {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError> {
        return if let Some(simple_text) = value.get("simpleText") {
            Ok(AuthorName::SimpleText(
                simple_text
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            ))
        } else {
            Ok(AuthorName::Unknown(value.clone()))
        };
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Run {
    Text(String),
    Emoji(String),
    Unknown(serde_json::Value),
}

impl Default for Run {
    fn default() -> Self {
        Run::Text("".to_string())
    }
}

impl FromGetLiveChat for Run {
    fn from_get_live_chat(value: &serde_json::Value) -> Result<Self, FromValueError> {
        if let Some(text) = value.get("text") {
            return Ok(Run::Text(
                text.as_str().map(|s| s.to_string()).unwrap_or_default(),
            ));
        }

        if let Some(emoji) = value.get("emoji") {
            // return Run::Emoji(emoji.as_str().map(|s| s.to_string()).unwrap_or_default());
            return Ok(Run::Emoji("まだ実装してないよ".to_string()));
        }

        Ok(Run::Unknown(value.clone()))
    }
}

/*

パースエラーの扱い
- エラーにする
  - エラーだとそれ以上の処理が不可能
- Unknownでタグ付けして無視前提の値とする
- Eitherでパース成功と失敗の共存をする
  - Resultでもいいけど意味合いとしてはEitherの方が良い。

*/

// video_idはYouTubeのビデオIDで、YouTubeのURLに含まれています。
// 例えば、"https://www.youtube.com/watch?v=dQw4w9WgXcQ" のURLの場合、
// video_idは "dQw4w9WgXcQ" になります。
// このIDを使用して、YouTube APIからライブチャットIDを取得します。
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
                .get(0)?
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
