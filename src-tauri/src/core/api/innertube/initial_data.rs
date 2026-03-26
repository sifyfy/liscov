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

    // extract_yt_initial_data: マーカーなしの場合は None を返すこと (L8: FnValue mutant)
    #[test]
    fn test_extract_yt_initial_data_no_marker() {
        let html = "<html>no data here</html>";
        assert_eq!(extract_yt_initial_data(html), None);
    }

    // extract_yt_initial_data: 正常ケースで JSON オブジェクトを返すこと (L8: FnValue, L9: +/-/*, L10: +/-/*)
    #[test]
    fn test_extract_yt_initial_data_basic() {
        let html = r#"var ytInitialData = {"key":"val"};</script>"#;
        assert_eq!(
            extract_yt_initial_data(html),
            Some(serde_json::json!({"key": "val"}))
        );
    }

    // extract_yt_initial_data: HTML に埋め込まれている場合も正しく抽出できること (L9: start_marker.len() のオフセット, L10: start オフセット)
    #[test]
    fn test_extract_yt_initial_data_embedded_in_html() {
        let html = r#"<html><script>var ytInitialData = {"test":123};</script></html>"#;
        assert_eq!(
            extract_yt_initial_data(html),
            Some(serde_json::json!({"test": 123}))
        );
    }

    // extract_yt_initial_data: JSON が不正な場合は None を返すこと (L8: FnValue mutant)
    #[test]
    fn test_extract_yt_initial_data_invalid_json() {
        let html = "var ytInitialData = invalid;</script>";
        assert_eq!(extract_yt_initial_data(html), None);
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
