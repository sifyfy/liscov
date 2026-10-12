//! YouTube のプッシュ通知（signaler / Punctual）のクライアント（02_chat.md「プッシュ通知」、ADR-008）
//!
//! 公開の資料は無く、公式のチャット画面の通信を観察して合わせている。
//! 通知は「取りに行く」合図にだけ使い、メッセージは `get_live_chat` で取る。

use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// 通知のサーバーの既定の URL
const DEFAULT_SIGNALER_BASE_URL: &str = "https://signaler-pa.youtube.com/punctual";
/// 通知のサーバーが受け付ける参照元（無いと 403 になる）
const YOUTUBE_ORIGIN: &str = "https://www.youtube.com";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";
/// 受信以外のリクエストのタイムアウト（02_chat.md「設定値」の HTTP タイムアウトに合わせる）
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// 通知のサーバーの URL を返す
///
/// テストで YouTube の URL をモックに向けたとき（`LISCOV_YOUTUBE_BASE_URL`）は、通知もモックに向ける。
/// E2E が本物の通知のサーバーにつながらないようにするため、別の環境変数にはしない。
pub fn signaler_base_url() -> String {
    std::env::var("LISCOV_YOUTUBE_BASE_URL")
        .map(|base| format!("{base}/punctual"))
        .unwrap_or_else(|_| DEFAULT_SIGNALER_BASE_URL.to_string())
}

/// 視聴ページの HTML から通知の API キー（`LIVE_CHAT_BASE_TANGO_CONFIG.apiKey`）を取り出す
pub fn extract_signaler_api_key(html: &str) -> Option<String> {
    const MARKER: &str = r#""LIVE_CHAT_BASE_TANGO_CONFIG":{"apiKey":""#;
    let start = html.find(MARKER)? + MARKER.len();
    let len = html[start..].find('"')?;
    Some(html[start..start + len].to_string()).filter(|key| !key.is_empty())
}

/// 購読する topic（`invalidationId.topic` と同じ形）
fn topic(video_id: &str) -> String {
    format!("chat~{video_id}")
}

/// chooseServer と購読で共通の、購読の中身（JSPB）
fn subscription(video_id: &str) -> Value {
    serde_json::json!([
        null,
        null,
        null,
        [9, 5],
        null,
        [["youtube_live_chat_web"], [1], [[[topic(video_id)]]]]
    ])
}

/// chooseServer の本文
fn choose_server_body(video_id: &str) -> String {
    serde_json::json!([subscription(video_id), null, null, 0, 0]).to_string()
}

/// チャンネルを開く POST の `req0___data__`
fn subscribe_data(video_id: &str) -> String {
    let mut request = subscription(video_id);
    if let Value::Array(items) = &mut request {
        items.extend([Value::Null, Value::Null, Value::from(1)]);
    }
    serde_json::json!([[[1, request, null, 3]]]).to_string()
}

/// chooseServer の応答から gsessionid を取り出す
fn parse_gsessionid(body: &str) -> Result<String> {
    let value: Value = serde_json::from_str(body).context("chooseServer の応答が JSON でない")?;
    value
        .get(0)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("chooseServer の応答に gsessionid が無い: {body}"))
}

/// 受信で届いたもの（02_chat.md「通知のサーバーとのやりとり」）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalerEvent {
    /// 資格の更新に使うトークン（受信の最初に届く）
    CredentialToken(String),
    /// 購読の開始
    Subscribed,
    /// チャットが更新された。発行時刻（µs）を `invalidationPayloadLastPublishAtUsec` に使う
    Notified { publish_at_usec: String },
    /// 生きている印（20 秒ごとの定期のメッセージ・`noop`・知らない形）
    Alive,
    /// サーバーが閉じた・エラーを返した
    Closed,
}

/// 受信の 1 件（番号は次に受信を開くときの `AID` に使う）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedEvent {
    pub aid: u64,
    pub event: SignalerEvent,
}

/// 受信の「長さ + 改行 + JSON」の並びを切り出す
///
/// 応答は好きな位置で分かれて届くので、切れ目をまたいだ分は次の分と合わせて読む。
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
}

impl FrameDecoder {
    /// 届いた分を足し、読み切れた件を返す。読めない並びはエラー
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<ReceivedEvent>> {
        self.buf.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(newline) = self.buf.iter().position(|&b| b == b'\n') {
            let len: usize = std::str::from_utf8(&self.buf[..newline])
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .ok_or_else(|| anyhow!("受信の並びの長さが読めない"))?;
            let end = newline + 1 + len;
            if self.buf.len() < end {
                break;
            }
            let entries: Vec<(u64, Value)> = serde_json::from_slice(&self.buf[newline + 1..end])
                .context("受信の並びが [[番号, 中身], ...] でない")?;
            self.buf.drain(..end);
            events.extend(entries.into_iter().map(|(aid, payload)| ReceivedEvent {
                aid,
                event: classify(&payload),
            }));
        }
        Ok(events)
    }
}

/// 1 件の中身を分類する
fn classify(payload: &Value) -> SignalerEvent {
    if payload.get("__sm__").is_some() || payload.get(0).and_then(Value::as_str) == Some("close") {
        return SignalerEvent::Closed;
    }
    // [[null,null,["{トークン}"]]]
    if let Some(token) = payload.pointer("/0/2/0").and_then(Value::as_str) {
        return SignalerEvent::CredentialToken(token.to_string());
    }
    // [[[["1", 中身]]]]
    let Some(body) = payload.pointer("/0/0/0/1") else {
        return SignalerEvent::Alive;
    };
    if let Some(publish_at) = body.pointer("/1/0/0/2").and_then(Value::as_str) {
        return SignalerEvent::Notified {
            publish_at_usec: publish_at.to_string(),
        };
    }
    // 購読の開始は [["{時刻}"]]、定期のメッセージは [null,null,["{時刻}"]]
    if body.pointer("/0/0").is_some_and(Value::is_string) {
        return SignalerEvent::Subscribed;
    }
    SignalerEvent::Alive
}

/// チャンネルを開く応答（1 件の並び）から SID を取り出す
fn parse_sid(body: &str) -> Result<String> {
    let newline = body
        .find('\n')
        .ok_or_else(|| anyhow!("チャンネルの応答が読めない: {body}"))?;
    let entries: Vec<(u64, Value)> = serde_json::from_str(body[newline + 1..].trim())
        .context("チャンネルの応答が [[番号, 中身]] でない")?;
    entries
        .iter()
        .find_map(|(_, payload)| {
            (payload.get(0).and_then(Value::as_str) == Some("c"))
                .then(|| payload.get(1).and_then(Value::as_str))
                .flatten()
        })
        .map(str::to_string)
        .ok_or_else(|| anyhow!("チャンネルの応答に SID が無い: {body}"))
}

/// 通知のセッション（つなぎ直すまで使い回す）
#[derive(Debug, Clone)]
pub struct SignalerSession {
    gsessionid: String,
    sid: String,
}

/// 通知のサーバーのクライアント（接続ごとに 1 つ）
#[derive(Clone)]
pub struct SignalerClient {
    http: Client,
    base_url: String,
    api_key: String,
    video_id: String,
}

impl SignalerClient {
    pub fn new(api_key: impl Into<String>, video_id: impl Into<String>) -> Self {
        Self {
            // 受信は開いたまま待つので、全体のタイムアウトは付けない（無通信の判定は呼ぶ側が行う）
            http: Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .build()
                .expect("HTTP クライアントの初期化に失敗"),
            base_url: signaler_base_url(),
            api_key: api_key.into(),
            video_id: video_id.into(),
        }
    }

    fn post(&self, url: String) -> reqwest::RequestBuilder {
        self.http
            .post(url)
            .timeout(REQUEST_TIMEOUT)
            .header("Origin", YOUTUBE_ORIGIN)
            .header("Referer", format!("{YOUTUBE_ORIGIN}/"))
            .header("User-Agent", USER_AGENT)
    }

    /// サーバーを選び、チャンネルを開いて購読する
    pub async fn open(&self) -> Result<SignalerSession> {
        let body = self
            .post(format!(
                "{}/v1/chooseServer?key={}",
                self.base_url, self.api_key
            ))
            .header("Content-Type", "application/json+protobuf")
            .body(choose_server_body(&self.video_id))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let gsessionid = parse_gsessionid(&body)?;

        let form = [
            ("count", "1".to_string()),
            ("ofs", "0".to_string()),
            ("req0___data__", subscribe_data(&self.video_id)),
        ];
        let body = self
            .post(format!("{}/multi-watch/channel", self.base_url))
            .query(&[
                ("VER", "8"),
                ("gsessionid", &gsessionid),
                ("key", &self.api_key),
                ("RID", &(random_u128() % 100_000).to_string()),
                ("CVER", "22"),
                ("zx", &random_zx()),
                ("t", "1"),
            ])
            .header("X-WebChannel-Content-Type", "application/json+protobuf")
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let sid = parse_sid(&body)?;
        Ok(SignalerSession { gsessionid, sid })
    }

    /// 受信を開く。応答の本文は開いたまま届き続ける（約 4 分でサーバーが閉じる）
    pub async fn receive(&self, session: &SignalerSession, aid: u64) -> Result<reqwest::Response> {
        Ok(self
            .http
            .get(format!("{}/multi-watch/channel", self.base_url))
            .query(&[
                ("VER", "8"),
                ("gsessionid", session.gsessionid.as_str()),
                ("key", &self.api_key),
                ("RID", "rpc"),
                ("SID", &session.sid),
                ("AID", &aid.to_string()),
                ("CI", "0"),
                ("TYPE", "xmlhttp"),
                ("zx", &random_zx()),
                ("t", "1"),
            ])
            .header("Origin", YOUTUBE_ORIGIN)
            .header("Referer", format!("{YOUTUBE_ORIGIN}/"))
            .header("User-Agent", USER_AGENT)
            .send()
            .await?
            .error_for_status()?)
    }

    /// 資格を更新する（公式のチャット画面に合わせて約 4 分ごとに送る）
    pub async fn refresh_credentials(&self, session: &SignalerSession, token: &str) -> Result<()> {
        self.post(format!(
            "{}/v1/refreshCreds?key={}&gsessionid={}",
            self.base_url, self.api_key, session.gsessionid
        ))
        .header("Content-Type", "application/json+protobuf")
        .body(serde_json::json!([token]).to_string())
        .send()
        .await?
        .error_for_status()?;
        Ok(())
    }
}

/// 乱数（暗号の強さは要らない。依存を増やさないよう uuid の v4 から作る）
fn random_u128() -> u128 {
    uuid::Uuid::new_v4().as_u128()
}

/// キャッシュよけの乱数（公式のチャット画面と同じ 12 文字の英小文字と数字）
fn random_zx() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut n = random_u128();
    (0..12)
        .map(|_| {
            let c = CHARS[(n % CHARS.len() as u128) as usize] as char;
            n /= CHARS.len() as u128;
            c
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 02_chat.md「通知のサーバーとのやりとり」: API キーは視聴ページの ytcfg の LIVE_CHAT_BASE_TANGO_CONFIG.apiKey
    #[test]
    fn api_key_is_read_from_tango_config() {
        let html = r#"<script>ytcfg.set({"INNERTUBE_API_KEY":"AIzaInner","LIVE_CHAT_BASE_TANGO_CONFIG":{"apiKey":"AIzaTango","channelUri":"https://client-channel.google.com/client-channel/client"}});</script>"#;
        assert_eq!(extract_signaler_api_key(html).as_deref(), Some("AIzaTango"));
    }

    #[test]
    fn api_key_missing_without_tango_config() {
        let html = r#"<script>ytcfg.set({"INNERTUBE_API_KEY":"AIzaInner"});</script>"#;
        assert_eq!(extract_signaler_api_key(html), None);
    }

    // 公式のチャット画面の chooseServer の本文（2026-10 の観察）
    #[test]
    fn choose_server_body_matches_official_page() {
        assert_eq!(
            choose_server_body("RbJVw4SFu2k"),
            r#"[[null,null,null,[9,5],null,[["youtube_live_chat_web"],[1],[[["chat~RbJVw4SFu2k"]]]]],null,null,0,0]"#
        );
    }

    // 公式のチャット画面の req0___data__（2026-10 の観察）
    #[test]
    fn subscribe_data_matches_official_page() {
        assert_eq!(
            subscribe_data("RbJVw4SFu2k"),
            r#"[[[1,[null,null,null,[9,5],null,[["youtube_live_chat_web"],[1],[[["chat~RbJVw4SFu2k"]]]],null,null,1],null,3]]]"#
        );
    }

    #[test]
    fn gsessionid_is_first_element() {
        let body = r#"["IBXqiVbiZVH2w9T3ZWPIlH0beSx3KHAJ70OWyR0pIEM",3,null,"1791788841429498","1791788841429613"]"#;
        assert_eq!(
            parse_gsessionid(body).unwrap(),
            "IBXqiVbiZVH2w9T3ZWPIlH0beSx3KHAJ70OWyR0pIEM"
        );
    }

    #[test]
    fn sid_is_read_from_channel_open() {
        let body = "51\n[[0,[\"c\",\"zHbyxMfF8I2MR0WR0ru4ew\",\"\",8,15,30000]]]\n";
        assert_eq!(parse_sid(body).unwrap(), "zHbyxMfF8I2MR0WR0ru4ew");
    }

    fn decode(chunks: &[&str]) -> Vec<ReceivedEvent> {
        let mut decoder = FrameDecoder::default();
        chunks
            .iter()
            .flat_map(|c| decoder.push(c.as_bytes()).unwrap())
            .collect()
    }

    fn frame(json: &str) -> String {
        format!("{}\n{}", json.len(), json)
    }

    // 02_chat.md「受信で届くもの」の各行
    #[test]
    fn first_frame_carries_token_and_subscription() {
        let events = decode(&[&frame(
            r#"[[1,[[null,null,["zSHiS9Wd"]]]],[2,[[[["1",[["1791788841882170"]]]]]]]]"#,
        )]);
        assert_eq!(
            events,
            vec![
                ReceivedEvent {
                    aid: 1,
                    event: SignalerEvent::CredentialToken("zSHiS9Wd".into())
                },
                ReceivedEvent {
                    aid: 2,
                    event: SignalerEvent::Subscribed
                },
            ]
        );
    }

    #[test]
    fn notification_carries_publish_time() {
        let events = decode(&[&frame(
            r#"[[3,[[[["1",[null,[[[null,"EMCexc71s5cDKMmcocv1s5cD","1791788848336704","1791788848731215",["1791788848694840"],[],0,0]]]]]]]]]]"#,
        )]);
        assert_eq!(
            events,
            vec![ReceivedEvent {
                aid: 3,
                event: SignalerEvent::Notified {
                    publish_at_usec: "1791788848336704".into()
                }
            }]
        );
    }

    #[test]
    fn periodic_message_and_noop_are_alive() {
        let events = decode(&[
            &frame(r#"[[9,[[[["1",[null,null,["1791788861876399"]]]]]]]]"#),
            &frame(r#"[[10,["noop"]]]"#),
        ]);
        assert_eq!(
            events,
            vec![
                ReceivedEvent {
                    aid: 9,
                    event: SignalerEvent::Alive
                },
                ReceivedEvent {
                    aid: 10,
                    event: SignalerEvent::Alive
                },
            ]
        );
    }

    #[test]
    fn close_and_error_are_closed() {
        let events = decode(&[&frame(
            r#"[[1,{"__sm__":{"status":[[[3,"Invalid value at 'requests[1]'"]]]}}],[2,["close"]]]"#,
        )]);
        assert_eq!(
            events,
            vec![
                ReceivedEvent {
                    aid: 1,
                    event: SignalerEvent::Closed
                },
                ReceivedEvent {
                    aid: 2,
                    event: SignalerEvent::Closed
                },
            ]
        );
    }

    // 応答は好きな位置で分かれて届く
    #[test]
    fn frames_split_across_chunks() {
        let whole = format!(
            "{}{}",
            frame(r#"[[10,["noop"]]]"#),
            frame(r#"[[11,["noop"]]]"#)
        );
        let (a, b) = whole.split_at(17);
        let (b1, b2) = b.split_at(3);
        let events = decode(&[a, b1, b2]);
        assert_eq!(
            events.iter().map(|e| e.aid).collect::<Vec<_>>(),
            vec![10, 11]
        );
    }

    #[test]
    fn unreadable_frame_is_error() {
        let mut decoder = FrameDecoder::default();
        assert!(decoder.push(b"abc\n[[1]]").is_err());
    }
}
