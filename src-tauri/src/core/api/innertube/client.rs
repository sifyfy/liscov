//! HTTP クライアントのリクエスト構築・送信・cookie 管理

use crate::core::models::*;
use anyhow::{anyhow, Result};
use serde_json::Value;

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
pub fn build_request_body(video_id: &str, continuation: &str, client_version: &str) -> Value {
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

/// レスポンスから continuation トークンを抽出する
pub fn extract_continuation(data: &Value) -> Option<String> {
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

/// InnerTube `next` API 経由でウォッチページの初期データを取得する。
/// SAPISIDHASH 認証を使用し、5つの cookie で動作する。
/// ウォッチページが chat データを返さないメンバー限定配信のフォールバック。
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
        .pointer(
            "/contents/twoColumnWatchNextResults/conversationBar/liveChatRenderer",
        )
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
