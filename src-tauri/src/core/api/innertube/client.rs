//! HTTP クライアントのリクエスト構築・送信・cookie 管理

use crate::core::models::*;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::time::Duration;

use super::initial_data::parse_initial_data;

/// デフォルトの YouTube ベース URL
pub const DEFAULT_YOUTUBE_BASE_URL: &str = "https://www.youtube.com";
/// InnerTube API キー
pub const DEFAULT_API_KEY: &str = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";

/// YouTube ベース URL を取得する（テスト用環境変数でオーバーライド可能）
pub fn get_youtube_base_url() -> String {
    std::env::var("LISCOV_YOUTUBE_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_YOUTUBE_BASE_URL.to_string())
}

/// InnerTube API エンドポイント URL を取得する
pub fn get_innertube_api_url() -> String {
    format!(
        "{}/youtubei/v1/live_chat/get_live_chat",
        get_youtube_base_url()
    )
}

/// InnerTube API リクエストボディを構築する
///
/// `publish_at` は最後に受けたプッシュ通知の発行時刻。受けていれば付ける（02_chat.md「リクエスト形式」）
pub fn build_request_body(
    video_id: &str,
    continuation: &str,
    client_version: &str,
    publish_at: Option<&str>,
) -> Value {
    let mut body = serde_json::json!({
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
    });
    if let Some(publish_at) = publish_at {
        body["invalidationPayloadLastPublishAtUsec"] = Value::from(publish_at);
    }
    body
}

/// 応答から採用した continuation（02_chat.md「Continuation Token」）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Continuation {
    pub token: String,
    /// そのデータの `timeoutMs`。次の取得までの待ちの上限に使う（無い応答もある）
    pub timeout: Option<Duration>,
}

/// レスポンスから continuation を抽出する（優先度の高いデータから）
pub fn extract_continuation(data: &Value) -> Option<Continuation> {
    const KINDS: [&str; 3] = [
        "invalidationContinuationData",
        "timedContinuationData",
        "reloadContinuationData",
    ];
    let entry = data.pointer("/continuationContents/liveChatContinuation/continuations/0")?;
    KINDS.iter().find_map(|kind| {
        let cont = entry.get(kind)?;
        Some(Continuation {
            token: cont.get("continuation")?.as_str()?.to_string(),
            timeout: cont
                .get("timeoutMs")
                .and_then(Value::as_u64)
                .map(Duration::from_millis),
        })
    })
}

/// InnerTube `next` API 経由でウォッチページの初期データを取得する。
/// SAPISIDHASH 認証を使用し、5つの cookie で動作する。
/// ウォッチページが chat データを返さないメンバー限定配信のフォールバック。
#[allow(clippy::too_many_arguments)]
pub async fn fetch_initial_data_via_api(
    http_client: &reqwest::Client,
    video_id: &str,
    api_key: &str,
    client_version: &str,
    auth_cookies: &Option<YouTubeCookies>,
    broadcaster_channel_id: &mut Option<String>,
    broadcaster_name: &mut Option<String>,
    stream_title: &mut Option<String>,
    continuation: &mut Option<String>,
    is_replay: &mut bool,
) -> Result<()> {
    let url = format!(
        "{}/youtubei/v1/next?key={}&prettyPrint=false",
        get_youtube_base_url(),
        api_key
    );

    let request_body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": client_version,
                "gl": "US",
                "hl": "en"
            }
        },
        "videoId": video_id
    });

    let mut request = http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        );

    if let Some(cookies) = auth_cookies {
        let headers = crate::core::api::auth::build_auth_headers(cookies);
        for (key, value) in headers {
            request = request.header(&key, &value);
        }
    }

    let response = request.json(&request_body).send().await?;
    let status = response.status();
    tracing::info!("InnerTube next API response status: {}", status);

    if !status.is_success() {
        return Err(anyhow!("InnerTube next API returned {}", status));
    }

    let raw_json = response.text().await?;
    let data: Value = serde_json::from_str(&raw_json)?;

    let has_live_chat = data
        .pointer("/contents/twoColumnWatchNextResults/conversationBar/liveChatRenderer")
        .is_some();
    tracing::info!("InnerTube next API: liveChatRenderer={}", has_live_chat);

    parse_initial_data(
        &data,
        broadcaster_channel_id,
        broadcaster_name,
        stream_title,
        continuation,
        is_replay,
    )?;

    if continuation.is_none() {
        return Err(anyhow!(
            "InnerTube next API response did not contain continuation token"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_with(continuation: Value) -> Value {
        serde_json::json!({
            "continuationContents": {"liveChatContinuation": {"continuations": [continuation]}}
        })
    }

    // 02_chat.md「リクエスト形式」: 通知を受けていれば最後の発行時刻を付け、受けていなければ付けない
    #[test]
    fn request_body_carries_publish_time_only_after_push() {
        let pushed = build_request_body("vid", "cont", "2.0", Some("1791788848336704"));
        assert_eq!(
            pushed["invalidationPayloadLastPublishAtUsec"],
            "1791788848336704"
        );
        let polled = build_request_body("vid", "cont", "2.0", None);
        assert!(polled.get("invalidationPayloadLastPublishAtUsec").is_none());
    }

    // 02_chat.md「Continuation Token」: 採用したデータの timeoutMs を待ちの上限に使う
    #[test]
    fn continuation_carries_its_timeout() {
        let data = response_with(serde_json::json!({
            "invalidationContinuationData": {"continuation": "cont_a", "timeoutMs": 10000}
        }));
        let continuation = extract_continuation(&data).unwrap();
        assert_eq!(continuation.token, "cont_a");
        assert_eq!(continuation.timeout, Some(Duration::from_millis(10000)));
    }

    // 02_chat.md「取得の間隔」: timeoutMs が無い応答（reloadContinuationData など）
    #[test]
    fn continuation_without_timeout() {
        let data = response_with(serde_json::json!({
            "reloadContinuationData": {"continuation": "cont_b"}
        }));
        let continuation = extract_continuation(&data).unwrap();
        assert_eq!(continuation.token, "cont_b");
        assert_eq!(continuation.timeout, None);
    }

    #[test]
    fn no_continuation() {
        assert!(extract_continuation(&serde_json::json!({})).is_none());
    }
}
