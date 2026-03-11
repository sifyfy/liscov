//! ウォッチページの HTML パース・`ytInitialData` 抽出・continuation token 解析

use anyhow::Result;
use serde_json::Value;

/// HTML から `ytInitialData` JSON を抽出する
pub fn extract_yt_initial_data(html: &str) -> Option<Value> {
    let start_marker = "var ytInitialData = ";
    let start = html.find(start_marker)? + start_marker.len();
    let end = html[start..].find(";</script>")? + start;
    serde_json::from_str(&html[start..end]).ok()
}

/// `ytInitialData` / InnerTube next API レスポンスから各種フィールドを解析して
/// `InnerTubeClient` のフィールドに書き込む。
///
/// 抽出対象:
/// - ストリームタイトル（hashtag 結合済み）
/// - ブロードキャスター名・チャンネル ID
/// - continuation token
/// - リプレイフラグ
pub fn parse_initial_data(
    data: &Value,
    broadcaster_channel_id: &mut Option<String>,
    broadcaster_name: &mut Option<String>,
    stream_title: &mut Option<String>,
    continuation: &mut Option<String>,
    is_replay: &mut bool,
) -> Result<()> {
    // ストリームタイトルを抽出（hashtag は複数の run に分割されるため結合する）
    if let Some(runs) = data.pointer("/contents/twoColumnWatchNextResults/results/results/contents/0/videoPrimaryInfoRenderer/title/runs") {
        if let Some(runs_array) = runs.as_array() {
            let title: String = runs_array
                .iter()
                .filter_map(|run| run.get("text").and_then(|t| t.as_str()))
                .collect();
            if !title.is_empty() {
                *stream_title = Some(title);
            }
        }
    }

    // ブロードキャスター情報を抽出
    if let Some(owner) = data.pointer("/contents/twoColumnWatchNextResults/results/results/contents/1/videoSecondaryInfoRenderer/owner/videoOwnerRenderer") {
        if let Some(name) = owner.pointer("/title/runs/0/text") {
            *broadcaster_name = name.as_str().map(|s| s.to_string());
        }
        if let Some(channel_id) = owner.pointer("/navigationEndpoint/browseEndpoint/browseId") {
            *broadcaster_channel_id = channel_id.as_str().map(|s| s.to_string());
        }
    }

    // continuation token を抽出
    if let Some(chat) = data.pointer("/contents/twoColumnWatchNextResults/conversationBar/liveChatRenderer") {
        if let Some(continuations) = chat.get("continuations") {
            if let Some(cont) = continuations.get(0) {
                let token = cont
                    .pointer("/reloadContinuationData/continuation")
                    .or_else(|| cont.pointer("/invalidationContinuationData/continuation"))
                    .or_else(|| cont.pointer("/timedContinuationData/continuation"));

                if let Some(token) = token {
                    *continuation = token.as_str().map(|s| s.to_string());
                }
            }
        }
    }

    // リプレイフラグを取得
    *is_replay = data
        .pointer("/contents/twoColumnWatchNextResults/conversationBar/liveChatRenderer/isReplay")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_title_with_hashtags() {
        // YouTube は hashtag を含むタイトルを複数の run に分割するため、結合を確認する
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

        let mut broadcaster_channel_id = None;
        let mut broadcaster_name = None;
        let mut stream_title = None;
        let mut continuation = None;
        let mut is_replay = false;

        parse_initial_data(
            &data,
            &mut broadcaster_channel_id,
            &mut broadcaster_name,
            &mut stream_title,
            &mut continuation,
            &mut is_replay,
        )
        .unwrap();

        assert_eq!(
            stream_title,
            Some("配信タイトル #hashtag1 #hashtag2".to_string()),
            "hashtag を含むタイトルは全 run を結合すること"
        );
    }

    #[test]
    fn test_parse_title_single_run() {
        // シンプルなタイトル（1つの run）が正しくパースされること
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

        let mut broadcaster_channel_id = None;
        let mut broadcaster_name = None;
        let mut stream_title = None;
        let mut continuation = None;
        let mut is_replay = false;

        parse_initial_data(
            &data,
            &mut broadcaster_channel_id,
            &mut broadcaster_name,
            &mut stream_title,
            &mut continuation,
            &mut is_replay,
        )
        .unwrap();

        assert_eq!(
            stream_title,
            Some("Simple Title Without Hashtags".to_string()),
            "シンプルなタイトルが正しくパースされること"
        );
    }
}
