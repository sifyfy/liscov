//! チャットメッセージのパース・変換ロジック

use crate::core::models::*;
use serde_json::Value;

/// YouTube color integer（ARGB 形式）を hex 文字列（#RRGGBB）に変換する
pub fn color_int_to_hex(color: i64) -> String {
    // YouTube は符号付き i64 で色を返すが、RGB 部分のみ使用する
    // フォーマット: 0xAARRGGBB または 0xRRGGBB
    let rgb = (color & 0xFFFFFF) as u32;
    format!("#{:06X}", rgb)
}

/// YouTube API レスポンスから SuperChat の色情報をパースする
fn parse_superchat_colors(renderer: &Value) -> Option<SuperChatColors> {
    let header_bg = renderer.get("headerBackgroundColor")?.as_i64()?;
    let header_text = renderer
        .get("headerTextColor")
        .and_then(|v| v.as_i64())
        .unwrap_or(0xFFFFFF);
    let body_bg = renderer
        .get("bodyBackgroundColor")
        .and_then(|v| v.as_i64())
        .unwrap_or(header_bg);
    let body_text = renderer
        .get("bodyTextColor")
        .and_then(|v| v.as_i64())
        .unwrap_or(0xFFFFFF);

    Some(SuperChatColors {
        header_background: color_int_to_hex(header_bg),
        header_text: color_int_to_hex(header_text),
        body_background: color_int_to_hex(body_bg),
        body_text: color_int_to_hex(body_text),
    })
}

/// YouTube API レスポンスから SuperSticker の色情報をパースする。
/// SuperSticker は moneyChipBackgroundColor / moneyChipTextColor フィールドを使用する。
fn parse_supersticker_colors(renderer: &Value) -> Option<SuperChatColors> {
    let bg_color = renderer.get("moneyChipBackgroundColor")?.as_i64()?;
    let text_color = renderer
        .get("moneyChipTextColor")
        .and_then(|v| v.as_i64())
        .unwrap_or(0xFFFFFF);

    let bg_hex = color_int_to_hex(bg_color);
    let text_hex = color_int_to_hex(text_color);

    Some(SuperChatColors {
        header_background: bg_hex.clone(),
        header_text: text_hex.clone(),
        body_background: bg_hex,
        body_text: text_hex,
    })
}

/// バッジ tooltip から milestone の月数を抽出する（例: "Member (6 months)"）。
/// 新規メンバーバッジは None を返す。
pub fn extract_milestone_months_from_badge(tooltip: &str) -> Option<u32> {
    use regex::Regex;

    // "New member" バッジはスキップ
    if tooltip.to_lowercase().contains("new member") {
        return None;
    }

    // 英語フォーマット: "Member (6 months)" または "Member (1 month)"
    let en_regex = Regex::new(r"\((\d+)\s*months?\)").ok()?;
    if let Some(caps) = en_regex.captures(tooltip) {
        if let Some(m) = caps.get(1) {
            if let Ok(months) = m.as_str().parse::<u32>() {
                return Some(months);
            }
        }
    }

    // 日本語フォーマット: "メンバー（6か月）"
    let ja_regex = Regex::new(r"[（(](\d+)\s*か月[）)]").ok()?;
    if let Some(caps) = ja_regex.captures(tooltip) {
        if let Some(m) = caps.get(1) {
            if let Ok(months) = m.as_str().parse::<u32>() {
                return Some(months);
            }
        }
    }

    None
}

/// メンバーシップギフトメッセージからギフト数を抽出する。
/// サポートフォーマット:
/// - 日本語: "5人にメンバーシップをギフトしました"
/// - 英語: "Sent 5 [channel] gift memberships"
pub fn extract_gift_count(content: &str) -> Option<u32> {
    use regex::Regex;

    // 日本語フォーマット: "5人にメンバーシップをギフト"
    let ja_regex = Regex::new(r"(\d+)\s*人").ok()?;
    if let Some(caps) = ja_regex.captures(content) {
        if let Some(m) = caps.get(1) {
            if let Ok(count) = m.as_str().parse::<u32>() {
                return Some(count);
            }
        }
    }

    // 英語フォーマット: "Sent 5 [channel] gift memberships"
    let sent_regex = Regex::new(r"Sent\s+(\d+)").ok()?;
    if let Some(caps) = sent_regex.captures(content) {
        if let Some(m) = caps.get(1) {
            if let Ok(count) = m.as_str().parse::<u32>() {
                return Some(count);
            }
        }
    }

    // フォールバック: "gifted X memberships" または "X memberships"
    let en_regex = Regex::new(r"(\d+)\s+(?:gift\s+)?memberships?").ok()?;
    if let Some(caps) = en_regex.captures(content) {
        if let Some(m) = caps.get(1) {
            if let Ok(count) = m.as_str().parse::<u32>() {
                return Some(count);
            }
        }
    }

    None
}

/// メッセージの runs（テキスト・絵文字）をパースして (content文字列, runs配列) を返す
pub fn parse_message_content(message: &Value) -> (String, Vec<MessageRun>) {
    let mut content = String::new();
    let mut runs = Vec::new();

    if let Some(runs_array) = message.get("runs").and_then(|v| v.as_array()) {
        for run in runs_array {
            if let Some(text) = run.get("text").and_then(|v| v.as_str()) {
                content.push_str(text);
                runs.push(MessageRun::Text {
                    content: text.to_string(),
                });
            } else if let Some(emoji) = run.get("emoji") {
                let emoji_id = emoji
                    .get("emojiId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let image_url = emoji
                    .pointer("/image/thumbnails/0/url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let alt_text = emoji
                    .pointer("/image/accessibility/accessibilityData/label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                content.push_str(&alt_text);
                runs.push(MessageRun::Emoji {
                    emoji_id,
                    image_url,
                    alt_text,
                });
            }
        }
    }
    (content, runs)
}

/// タイムスタンプ（マイクロ秒文字列）を RFC3339 文字列に変換する
pub fn format_timestamp(timestamp_usec: &str) -> String {
    if let Ok(usec) = timestamp_usec.parse::<i64>() {
        let secs = usec / 1_000_000;
        let datetime = chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
        // フロントエンドがローカルタイムゾーンに変換できるよう RFC3339 フォーマットで返す
        datetime.to_rfc3339()
    } else {
        String::new()
    }
}

/// テキストチャットメッセージをパースする
fn parse_text_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    let author = renderer
        .pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = renderer
        .pointer("/authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let author_icon_url = renderer
        .pointer("/authorPhoto/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let (content, runs) = parse_message_content(renderer.get("message")?);

    // メンバーバッジ（customThumbnail）の有無でメンバー判定
    let is_member = renderer
        .get("authorBadges")
        .and_then(|v| v.as_array())
        .map(|badges| {
            badges.iter().any(|b| {
                b.pointer("/liveChatAuthorBadgeRenderer/customThumbnail")
                    .is_some()
            })
        })
        .unwrap_or(false);

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::Text,
        author,
        author_icon_url,
        channel_id,
        content,
        runs,
        metadata: None,
        is_member,
        is_first_time_viewer: false,
        in_stream_comment_count: None,
    })
}

/// SuperChat メッセージをパースする
fn parse_superchat_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    let author = renderer
        .pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = renderer
        .pointer("/authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let author_icon_url = renderer
        .pointer("/authorPhoto/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let amount = renderer
        .pointer("/purchaseAmountText/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let (content, runs) = renderer
        .get("message")
        .map(parse_message_content)
        .unwrap_or_default();

    // YouTube API から SuperChat の色情報をパース
    let superchat_colors = parse_superchat_colors(renderer);

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::SuperChat {
            amount: amount.clone(),
        },
        author,
        author_icon_url,
        channel_id,
        content,
        runs,
        metadata: Some(MessageMetadata {
            amount: Some(amount),
            badges: vec![],
            badge_info: vec![],
            color: None,
            is_moderator: false,
            is_verified: false,
            superchat_colors,
        }),
        is_member: false,
        is_first_time_viewer: false,
        in_stream_comment_count: None,
    })
}

/// SuperSticker メッセージをパースする
fn parse_supersticker_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    let author = renderer
        .pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = renderer
        .pointer("/authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let amount = renderer
        .pointer("/purchaseAmountText/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // YouTube API から SuperSticker の色情報をパース
    let superchat_colors = parse_supersticker_colors(renderer);

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::SuperSticker {
            amount: amount.clone(),
        },
        author,
        author_icon_url: None,
        channel_id,
        content: "[Sticker]".to_string(),
        runs: vec![],
        metadata: Some(MessageMetadata {
            amount: Some(amount),
            badges: vec![],
            badge_info: vec![],
            color: None,
            is_moderator: false,
            is_verified: false,
            superchat_colors,
        }),
        is_member: false,
        is_first_time_viewer: false,
        in_stream_comment_count: None,
    })
}

/// メンバーシップメッセージをパースする
fn parse_membership_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    let author = renderer
        .pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = renderer
        .pointer("/authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let author_icon_url = renderer
        .pointer("/authorPhoto/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // headerSubtext は simpleText または runs フォーマットの場合がある
    let content = renderer
        .pointer("/headerSubtext/simpleText")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            // runs フォーマットの場合は全テキストを結合する
            renderer
                .pointer("/headerSubtext/runs")
                .and_then(|v| v.as_array())
                .map(|runs| {
                    runs.iter()
                        .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                        .collect::<String>()
                })
        })
        .unwrap_or_else(|| "New member".to_string());

    // バッジの tooltip から milestone の月数を抽出する（例: "Member (6 months)"）
    let badge_tooltip = renderer
        .pointer("/authorBadges/0/liveChatAuthorBadgeRenderer/tooltip")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let milestone_months = extract_milestone_months_from_badge(badge_tooltip);

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::Membership { milestone_months },
        author,
        author_icon_url,
        channel_id,
        content,
        runs: vec![],
        metadata: None,
        is_member: true,
        is_first_time_viewer: false,
        in_stream_comment_count: None,
    })
}

/// メンバーシップギフトアナウンスメッセージをパースする
fn parse_membership_gift_message(renderer: &Value) -> Option<ChatMessage> {
    let id = renderer.get("id")?.as_str()?.to_string();
    let timestamp_usec = renderer.get("timestampUsec")?.as_str()?.to_string();

    // authorExternalChannelId はルートレベルにある
    let channel_id = renderer
        .get("authorExternalChannelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // ギフトアナウンスは sponsorshipsHeaderRenderer を持つ header がある
    let header = renderer.pointer("/header/liveChatSponsorshipsHeaderRenderer")?;

    let author = header
        .pointer("/authorName/simpleText")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let author_icon_url = header
        .pointer("/authorPhoto/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // プライマリテキストを抽出（例: "Sent 5 [channel] gift memberships"）
    let primary_text = header
        .pointer("/primaryText/runs")
        .and_then(|v| v.as_array())
        .map(|runs| {
            runs.iter()
                .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                .collect::<String>()
        })
        .or_else(|| {
            header
                .pointer("/primaryText/simpleText")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    // プライマリテキストからギフト数を抽出する
    let gift_count = extract_gift_count(&primary_text).unwrap_or(1);

    Some(ChatMessage {
        id,
        timestamp: format_timestamp(&timestamp_usec),
        timestamp_usec,
        message_type: MessageType::MembershipGift { gift_count },
        author,
        author_icon_url,
        channel_id,
        content: primary_text,
        runs: vec![],
        metadata: None,
        is_member: true,
        is_first_time_viewer: false,
        in_stream_comment_count: None,
    })
}

/// 1件のチャットアクションをパースして `ChatMessage` に変換する
pub fn parse_chat_action(action: &Value) -> Option<ChatMessage> {
    let item = action
        .pointer("/replayChatItemAction/actions/0/addChatItemAction/item")
        .or_else(|| action.pointer("/addChatItemAction/item"))?;

    if let Some(renderer) = item.get("liveChatTextMessageRenderer") {
        return parse_text_message(renderer);
    }
    if let Some(renderer) = item.get("liveChatPaidMessageRenderer") {
        return parse_superchat_message(renderer);
    }
    if let Some(renderer) = item.get("liveChatPaidStickerRenderer") {
        return parse_supersticker_message(renderer);
    }
    if let Some(renderer) = item.get("liveChatMembershipItemRenderer") {
        return parse_membership_message(renderer);
    }
    if let Some(renderer) = item.get("liveChatSponsorshipsGiftPurchaseAnnouncementRenderer") {
        return parse_membership_gift_message(renderer);
    }
    None
}

/// InnerTube API レスポンスからチャットアクションをパースして `ChatMessage` 配列を返す
pub fn parse_chat_actions(data: &Value) -> Vec<ChatMessage> {
    let mut messages = Vec::new();

    let actions = data
        .pointer("/continuationContents/liveChatContinuation/actions")
        .and_then(|v| v.as_array());

    if let Some(actions) = actions {
        for action in actions {
            if let Some(msg) = parse_chat_action(action) {
                messages.push(msg);
            }
        }
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_member_badge() {
        // メンバーバッジ（customThumbnail）が正しく検出されること
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatTextMessageRenderer": {
                        "id": "test_msg_1",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "MemberUser"},
                        "authorExternalChannelId": "UC_member",
                        "message": {"runs": [{"text": "Hello"}]},
                        "authorBadges": [{
                            "liveChatAuthorBadgeRenderer": {
                                "customThumbnail": {
                                    "thumbnails": [{"url": "https://example.com/badge.png"}]
                                },
                                "tooltip": "Member"
                            }
                        }]
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "メッセージがパースされること");
        let msg = msg.unwrap();
        assert_eq!(msg.author, "MemberUser");
        assert!(msg.is_member, "メンバーバッジが検出されること");
    }

    #[test]
    fn test_parse_non_member() {
        // 非メンバーは is_member = false であること
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatTextMessageRenderer": {
                        "id": "test_msg_2",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "NonMemberUser"},
                        "authorExternalChannelId": "UC_non_member",
                        "message": {"runs": [{"text": "Hello"}]},
                        "authorBadges": []
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "メッセージがパースされること");
        let msg = msg.unwrap();
        assert_eq!(msg.author, "NonMemberUser");
        assert!(!msg.is_member, "非メンバーは is_member = false であること");
    }

    #[test]
    fn test_parse_chat_actions_from_response() {
        // フルレスポンス構造（モックサーバーが返す形式）からパースできること
        let response = serde_json::json!({
            "continuationContents": {
                "liveChatContinuation": {
                    "continuations": [{"invalidationContinuationData": {"continuation": "cont_123"}}],
                    "actions": [
                        {
                            "addChatItemAction": {
                                "item": {
                                    "liveChatTextMessageRenderer": {
                                        "id": "msg_1",
                                        "timestampUsec": "1234567890000000",
                                        "authorName": {"simpleText": "MemberUser"},
                                        "authorExternalChannelId": "UC_member",
                                        "message": {"runs": [{"text": "Member message"}]},
                                        "authorBadges": [{
                                            "liveChatAuthorBadgeRenderer": {
                                                "customThumbnail": {
                                                    "thumbnails": [{"url": "https://example.com/badge.png"}]
                                                },
                                                "tooltip": "Member"
                                            }
                                        }]
                                    }
                                }
                            }
                        }
                    ]
                }
            }
        });

        let messages = parse_chat_actions(&response);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].author, "MemberUser");
        assert!(
            messages[0].is_member,
            "フルレスポンスからメンバーバッジが検出されること"
        );
    }

    #[test]
    fn test_extract_milestone_months_from_badge_english() {
        // 英語フォーマット: "Member (6 months)"
        assert_eq!(extract_milestone_months_from_badge("Member (6 months)"), Some(6));
        assert_eq!(extract_milestone_months_from_badge("Member (1 month)"), Some(1));
        assert_eq!(extract_milestone_months_from_badge("Member (12 months)"), Some(12));
    }

    #[test]
    fn test_extract_milestone_months_from_badge_japanese() {
        // 日本語フォーマット: "メンバー（6か月）"
        assert_eq!(
            extract_milestone_months_from_badge("メンバー（6か月）"),
            Some(6)
        );
        assert_eq!(
            extract_milestone_months_from_badge("メンバー(12か月)"),
            Some(12)
        );
    }

    #[test]
    fn test_extract_milestone_months_from_badge_none() {
        // None を返すケース（新規メンバー）
        assert_eq!(extract_milestone_months_from_badge("New member"), None);
        assert_eq!(extract_milestone_months_from_badge("Member"), None);
        assert_eq!(extract_milestone_months_from_badge(""), None);
    }

    #[test]
    fn test_extract_gift_count_japanese() {
        // 日本語フォーマット: "5人にメンバーシップをギフトしました"
        assert_eq!(
            extract_gift_count("5人にメンバーシップをギフトしました"),
            Some(5)
        );
        assert_eq!(extract_gift_count("10人にギフト"), Some(10));
        assert_eq!(extract_gift_count("1人"), Some(1));
    }

    #[test]
    fn test_extract_gift_count_english() {
        // 英語フォーマット: "Sent 5 [channel] gift memberships"（実際の YouTube フォーマット）
        assert_eq!(
            extract_gift_count("Sent 5 Channel Name gift memberships"),
            Some(5)
        );
        assert_eq!(extract_gift_count("Sent 10 memberships"), Some(10));
        // フォールバックパターン
        assert_eq!(extract_gift_count("5 gift memberships"), Some(5));
        assert_eq!(extract_gift_count("1 membership"), Some(1));
    }

    #[test]
    fn test_parse_membership_milestone_message() {
        // milestone メンバーシップメッセージのパース（月数はバッジ tooltip から取得）
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatMembershipItemRenderer": {
                        "id": "milestone_msg_1",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "LongTimeMember"},
                        "authorExternalChannelId": "UC_milestone",
                        "authorPhoto": {"thumbnails": [{"url": "https://example.com/av.png"}]},
                        "headerSubtext": {"runs": [{"text": "Welcome to "}, {"text": "Channel"}, {"text": "!"}]},
                        "authorBadges": [{
                            "liveChatAuthorBadgeRenderer": {
                                "tooltip": "Member (12 months)",
                                "customThumbnail": {"thumbnails": [{"url": "https://example.com/badge.png"}]}
                            }
                        }]
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "milestone メッセージがパースされること");
        let msg = msg.unwrap();
        assert_eq!(msg.author, "LongTimeMember");
        assert!(msg.is_member);

        match msg.message_type {
            MessageType::Membership { milestone_months } => {
                assert_eq!(
                    milestone_months,
                    Some(12),
                    "バッジ tooltip から 12 か月を抽出すること"
                );
            }
            _ => panic!("Membership メッセージタイプを期待"),
        }
    }

    #[test]
    fn test_parse_membership_gift_message() {
        // メンバーシップギフトアナウンスのパース（実際の YouTube フォーマット）
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatSponsorshipsGiftPurchaseAnnouncementRenderer": {
                        "id": "gift_msg_1",
                        "timestampUsec": "1234567890000000",
                        "authorExternalChannelId": "UC_gift_giver",
                        "header": {
                            "liveChatSponsorshipsHeaderRenderer": {
                                "authorName": {"simpleText": "GiftGiver"},
                                "authorPhoto": {"thumbnails": [{"url": "https://example.com/av.png"}]},
                                "primaryText": {"runs": [
                                    {"text": "Sent ", "bold": true},
                                    {"text": "5", "bold": true},
                                    {"text": " ", "bold": true},
                                    {"text": "Channel Name", "bold": true},
                                    {"text": " gift memberships", "bold": true}
                                ]}
                            }
                        }
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "ギフトメッセージがパースされること");
        let msg = msg.unwrap();
        assert_eq!(msg.author, "GiftGiver");
        assert_eq!(msg.channel_id, "UC_gift_giver");

        match msg.message_type {
            MessageType::MembershipGift { gift_count } => {
                assert_eq!(gift_count, 5, "5ギフトを抽出すること");
            }
            _ => panic!("MembershipGift メッセージタイプを期待"),
        }
    }

    #[test]
    fn test_parse_new_member_no_milestone() {
        // 新規メンバーは milestone 月数なし（バッジ tooltip は "New member"）
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatMembershipItemRenderer": {
                        "id": "new_member_1",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "NewMember"},
                        "authorExternalChannelId": "UC_new",
                        "authorPhoto": {"thumbnails": [{"url": "https://example.com/av.png"}]},
                        "headerSubtext": {"runs": [{"text": "Welcome to "}, {"text": "Channel"}, {"text": "!"}]},
                        "authorBadges": [{
                            "liveChatAuthorBadgeRenderer": {
                                "tooltip": "New member",
                                "customThumbnail": {"thumbnails": [{"url": "https://example.com/badge.png"}]}
                            }
                        }]
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "新規メンバーメッセージがパースされること");
        let msg = msg.unwrap();

        match msg.message_type {
            MessageType::Membership { milestone_months } => {
                assert_eq!(
                    milestone_months, None,
                    "新規メンバーは milestone なし"
                );
            }
            _ => panic!("Membership メッセージタイプを期待"),
        }
    }

    #[test]
    fn test_color_int_to_hex() {
        // YouTube color integer から hex 文字列への変換
        assert_eq!(color_int_to_hex(0x1565C0), "#1565C0"); // 青ティア
        assert_eq!(color_int_to_hex(0xD00000), "#D00000"); // 赤ティア
        assert_eq!(color_int_to_hex(0x00BFA5), "#00BFA5"); // 緑ティア
        assert_eq!(color_int_to_hex(0xFFFFFF), "#FFFFFF"); // 白
        assert_eq!(color_int_to_hex(0x000000), "#000000"); // 黒
    }

    #[test]
    fn test_parse_superchat_with_colors() {
        // YouTube 指定の色情報を持つ SuperChat メッセージのパース
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatPaidMessageRenderer": {
                        "id": "sc_color_test",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "ColorDonator"},
                        "authorExternalChannelId": "UC_color_test",
                        "purchaseAmountText": {"simpleText": "¥5,000"},
                        "message": {"runs": [{"text": "Test with colors"}]},
                        "headerBackgroundColor": 0x1565C0,
                        "headerTextColor": 0xFFFFFF,
                        "bodyBackgroundColor": 0x1565C0,
                        "bodyTextColor": 0xFFFFFF
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "色情報付き SuperChat がパースされること");
        let msg = msg.unwrap();

        match &msg.message_type {
            MessageType::SuperChat { amount } => {
                assert_eq!(amount, "¥5,000");
            }
            _ => panic!("SuperChat メッセージタイプを期待"),
        }

        let metadata = msg.metadata.expect("metadata が存在すること");
        let colors = metadata.superchat_colors.expect("superchat_colors が存在すること");

        assert_eq!(colors.header_background, "#1565C0", "header_background は青");
        assert_eq!(colors.header_text, "#FFFFFF", "header_text は白");
        assert_eq!(colors.body_background, "#1565C0", "body_background は青");
        assert_eq!(colors.body_text, "#FFFFFF", "body_text は白");
    }

    // parse_message_content の直接テスト
    // 変異: 関数全体 → (String::new(), vec![]) / ("xyzzy".into(), vec![]) を検出する

    #[test]
    fn test_parse_message_content_text_run() {
        // runs 配列のテキストランが正しく content と runs に変換されること
        let msg = serde_json::json!({"runs": [{"text": "hello world"}]});
        let (content, runs) = parse_message_content(&msg);
        assert_eq!(content, "hello world");
        assert_eq!(runs.len(), 1);
    }

    #[test]
    fn test_parse_message_content_empty() {
        // runs がない場合は空の content と空の runs を返すこと
        let msg = serde_json::json!({});
        let (content, runs) = parse_message_content(&msg);
        assert_eq!(content, "");
        assert!(runs.is_empty());
    }

    // format_timestamp のテスト
    // 変異: 関数全体 → String::new() / "xyzzy", / 1_000_000 → % 1_000_000 を検出する

    #[test]
    fn test_format_timestamp_valid() {
        // マイクロ秒タイムスタンプを RFC3339 文字列に変換できること
        // 1234567890000000 usec = 1234567890 sec = 2009-02-13T...(UTC)
        let result = format_timestamp("1234567890000000");
        assert!(result.contains("2009"), "2009年のタイムスタンプであること");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_format_timestamp_invalid() {
        // 数値でない文字列は空文字列を返すこと
        let result = format_timestamp("not_a_number");
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_supersticker_with_money_chip_color() {
        // moneyChipBackgroundColor を持つ SuperSticker メッセージのパース
        // YouTube は ARGB color を i32 をオーバーフローする大きな整数で返す場合がある
        // i64 値を直接使用: 4280191205_i64 = blue, 4294967295_i64 = white
        let action = serde_json::json!({
            "addChatItemAction": {
                "item": {
                    "liveChatPaidStickerRenderer": {
                        "id": "sticker_color_test",
                        "timestampUsec": "1234567890000000",
                        "authorName": {"simpleText": "StickerUser"},
                        "authorExternalChannelId": "UC_sticker",
                        "purchaseAmountText": {"simpleText": "¥1,500"},
                        "moneyChipBackgroundColor": 4280191205_i64,
                        "moneyChipTextColor": 4294967295_i64,
                        "sticker": {"thumbnails": [{"url": "https://example.com/sticker.png"}]}
                    }
                }
            }
        });

        let msg = parse_chat_action(&action);
        assert!(msg.is_some(), "色情報付き SuperSticker がパースされること");
        let msg = msg.unwrap();

        match &msg.message_type {
            MessageType::SuperSticker { amount } => {
                assert_eq!(amount, "¥1,500");
            }
            _ => panic!("SuperSticker メッセージタイプを期待"),
        }

        let metadata = msg.metadata.expect("metadata が存在すること");
        let colors = metadata.superchat_colors.expect("superchat_colors が存在すること");

        assert_eq!(colors.header_background, "#1E88E5", "header_background は青");
        assert_eq!(colors.body_background, "#1E88E5", "body_background は青");
        assert_eq!(colors.header_text, "#FFFFFF", "header_text は白");
        assert_eq!(colors.body_text, "#FFFFFF", "body_text は白");
    }
}
