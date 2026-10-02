//! Chat message models

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// Chat message type
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    #[default]
    Text,
    SuperChat {
        amount: String,
    },
    SuperSticker {
        amount: String,
    },
    Membership {
        milestone_months: Option<u32>,
    },
    MembershipGift {
        gift_count: u32,
    },
    /// ジュエルで送るギフト（メンバーシップギフトとは別物）
    Gift(GiftDetails),
    System,
}

/// ジュエルで送るギフトの内容。DB の messages.metadata にもこの形の JSON で保存する（08_database.md）
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GiftDetails {
    pub gift_name: String,
    pub gift_image_url: Option<String>,
    /// 本文から読めたときだけ Some（配信者本人としてログインした接続でのみ入る）
    pub jewel_count: Option<u32>,
}

impl MessageType {
    /// DB・GUI・エクスポートで使う種別名（02_chat.md MessageType）
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Text => "text",
            MessageType::SuperChat { .. } => "superchat",
            MessageType::SuperSticker { .. } => "supersticker",
            MessageType::Membership { .. } => "membership",
            MessageType::MembershipGift { .. } => "membership_gift",
            MessageType::Gift(_) => "gift",
            MessageType::System => "system",
        }
    }
}

/// Message run (text or emoji)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRun {
    Text {
        content: String,
    },
    Emoji {
        emoji_id: String,
        image_url: String,
        alt_text: String,
    },
}

/// Badge information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeInfo {
    pub badge_type: String,
    pub label: String,
    pub tooltip: Option<String>,
    pub icon_url: Option<String>,
}

/// SuperChat color scheme (per 02_chat.md spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperChatColors {
    pub header_background: String, // "#RRGGBB"
    pub header_text: String,
    pub body_background: String,
    pub body_text: String,
}

/// Message metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub amount: Option<String>,
    pub badges: Vec<String>,
    pub badge_info: Vec<BadgeInfo>,
    pub color: Option<String>,
    pub is_moderator: bool,
    pub is_verified: bool,
    pub superchat_colors: Option<SuperChatColors>,
}

/// Chat message
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub timestamp: String,
    pub timestamp_usec: String,
    pub message_type: MessageType,
    pub author: String,
    pub author_icon_url: Option<String>,
    pub channel_id: String,
    pub content: String,
    pub runs: Vec<MessageRun>,
    pub metadata: Option<MessageMetadata>,
    pub is_member: bool,
    pub is_first_time_viewer: bool,
    pub in_stream_comment_count: Option<u32>,
}

impl ChatMessage {
    /// スーパーチャットのヘッダー背景色（`#RRGGBB`）。段階の判定に使う（07_revenue.md）
    pub fn superchat_header_color(&self) -> Option<&str> {
        match self.message_type {
            MessageType::SuperChat { .. } => self
                .metadata
                .as_ref()?
                .superchat_colors
                .as_ref()
                .map(|colors| colors.header_background.as_str()),
            _ => None,
        }
    }

    /// 発言者のバッジを読める種類なら、そのバッジを持つ metadata を返す
    ///
    /// バッジを読めない種類（メンバーシップギフト・ギフト・システム）は None（不明）。
    /// 不明を「バッジ無し」と区別するため、保存・エクスポートはこれを通す（08_database.md）。
    pub fn author_badge_metadata(&self) -> Option<&MessageMetadata> {
        match self.message_type {
            MessageType::Text
            | MessageType::SuperChat { .. }
            | MessageType::SuperSticker { .. }
            | MessageType::Membership { .. } => self.metadata.as_ref(),
            MessageType::MembershipGift { .. } | MessageType::Gift(_) | MessageType::System => None,
        }
    }
}

/// ライブリアクションの 1 回の更新（02_chat.md「ライブリアクション」）
///
/// `new` でしか作れず、件数 1 以上の絵文字だけを持ち、total は常に counts の合計になる。
/// 0 件の更新は作れない（保存もイベントも出さない、という仕様を型で守る）。
#[derive(Debug, Clone, Serialize, PartialEq, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct ReactionUpdate {
    #[ts(type = "number")]
    update_time_usec: i64,
    duration_seconds: u32,
    #[ts(type = "Record<string, number>")]
    counts: BTreeMap<String, u32>,
    total: u32,
}

impl ReactionUpdate {
    /// 0 件の絵文字を除き、合計が 0 なら None を返す
    pub fn new(
        update_time_usec: i64,
        duration_seconds: u32,
        counts: BTreeMap<String, u32>,
    ) -> Option<Self> {
        let counts: BTreeMap<String, u32> =
            counts.into_iter().filter(|(_, count)| *count > 0).collect();
        let total = counts.values().sum();
        (total > 0).then_some(Self {
            update_time_usec,
            duration_seconds,
            counts,
            total,
        })
    }

    pub fn update_time_usec(&self) -> i64 {
        self.update_time_usec
    }

    pub fn duration_seconds(&self) -> u32 {
        self.duration_seconds
    }

    pub fn counts(&self) -> &BTreeMap<String, u32> {
        &self.counts
    }

    pub fn total(&self) -> u32 {
        self.total
    }
}

/// 配信（video_id）単位のリアクションのまとめ（`get_connection_reactions` の戻り値）
#[derive(Debug, Clone, Default, Serialize, PartialEq, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct ReactionSummary {
    // 絵文字別の累計（ts-rs がフィールドの doc コメントを行末空白付きで出力するため // にする）
    #[ts(type = "Record<string, number>")]
    pub totals: BTreeMap<String, u64>,
    // 指定時刻以降の更新（古い順）
    pub recent: Vec<ReactionUpdate>,
}

/// Chat statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatStats {
    pub total_messages: usize,
    pub text_messages: usize,
    pub super_chats: usize,
    pub super_stickers: usize,
    pub memberships: usize,
    pub membership_gifts: usize,
    pub total_revenue: f64,
}

impl ChatStats {
    pub fn update(&mut self, message: &ChatMessage) {
        self.total_messages += 1;
        match &message.message_type {
            MessageType::Text => self.text_messages += 1,
            MessageType::SuperChat { amount } => {
                self.super_chats += 1;
                if let Some(revenue) = parse_amount(amount) {
                    self.total_revenue += revenue;
                }
            }
            MessageType::SuperSticker { amount } => {
                self.super_stickers += 1;
                if let Some(revenue) = parse_amount(amount) {
                    self.total_revenue += revenue;
                }
            }
            MessageType::Membership { .. } => self.memberships += 1,
            MessageType::MembershipGift { gift_count } => {
                self.membership_gifts += *gift_count as usize;
            }
            MessageType::Gift(_) | MessageType::System => {}
        }
    }
}

/// Parse amount string (e.g., "¥1,000", "$10.00") to f64
fn parse_amount(amount: &str) -> Option<f64> {
    let cleaned: String = amount
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    cleaned.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(message_type: MessageType) -> ChatMessage {
        ChatMessage {
            message_type,
            ..Default::default()
        }
    }

    // spec: 02_chat.md - Text メッセージは total_messages と text_messages をインクリメントする
    #[test]
    fn update_text_message_increments_total_and_text() {
        let mut stats = ChatStats::default();
        stats.update(&make_message(MessageType::Text));
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.text_messages, 1);
        assert_eq!(stats.super_chats, 0);
        assert_eq!(stats.super_stickers, 0);
        assert_eq!(stats.memberships, 0);
        assert_eq!(stats.membership_gifts, 0);
        assert_eq!(stats.total_revenue, 0.0);
    }

    // spec: 02_chat.md - SuperChat メッセージは total_messages, super_chats をインクリメントし total_revenue に金額を加算する
    #[test]
    fn update_superchat_message_increments_total_super_chats_and_revenue() {
        let mut stats = ChatStats::default();
        stats.update(&make_message(MessageType::SuperChat {
            amount: "$10.00".to_string(),
        }));
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.super_chats, 1);
        assert_eq!(stats.text_messages, 0);
        assert_eq!(stats.super_stickers, 0);
        assert_eq!(stats.memberships, 0);
        assert_eq!(stats.membership_gifts, 0);
        assert!((stats.total_revenue - 10.0).abs() < f64::EPSILON);
    }

    // spec: 02_chat.md - SuperSticker メッセージは total_messages, super_stickers をインクリメントし total_revenue に金額を加算する
    #[test]
    fn update_supersticker_message_increments_total_super_stickers_and_revenue() {
        let mut stats = ChatStats::default();
        stats.update(&make_message(MessageType::SuperSticker {
            amount: "$5.00".to_string(),
        }));
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.super_stickers, 1);
        assert_eq!(stats.text_messages, 0);
        assert_eq!(stats.super_chats, 0);
        assert_eq!(stats.memberships, 0);
        assert_eq!(stats.membership_gifts, 0);
        assert!((stats.total_revenue - 5.0).abs() < f64::EPSILON);
    }

    // spec: 02_chat.md - Membership メッセージは total_messages と memberships をインクリメントする
    #[test]
    fn update_membership_message_increments_total_and_memberships() {
        let mut stats = ChatStats::default();
        stats.update(&make_message(MessageType::Membership {
            milestone_months: None,
        }));
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.memberships, 1);
        assert_eq!(stats.text_messages, 0);
        assert_eq!(stats.super_chats, 0);
        assert_eq!(stats.super_stickers, 0);
        assert_eq!(stats.membership_gifts, 0);
        assert_eq!(stats.total_revenue, 0.0);
    }

    // spec: 02_chat.md - MembershipGift メッセージは total_messages と membership_gifts をインクリメントする
    #[test]
    fn update_membership_gift_message_increments_total_and_membership_gifts() {
        let mut stats = ChatStats::default();
        stats.update(&make_message(MessageType::MembershipGift { gift_count: 1 }));
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.membership_gifts, 1);
        assert_eq!(stats.text_messages, 0);
        assert_eq!(stats.super_chats, 0);
        assert_eq!(stats.super_stickers, 0);
        assert_eq!(stats.memberships, 0);
        assert_eq!(stats.total_revenue, 0.0);
    }

    // spec: 02_chat.md - 複数メッセージを処理すると各フィールドが正しく累積される
    #[test]
    fn update_multiple_messages_accumulates_counts_correctly() {
        let mut stats = ChatStats::default();
        stats.update(&make_message(MessageType::Text));
        stats.update(&make_message(MessageType::SuperChat {
            amount: "$10.00".to_string(),
        }));
        stats.update(&make_message(MessageType::SuperSticker {
            amount: "$5.00".to_string(),
        }));
        assert_eq!(stats.total_messages, 3);
        assert_eq!(stats.text_messages, 1);
        assert_eq!(stats.super_chats, 1);
        assert_eq!(stats.super_stickers, 1);
        assert!((stats.total_revenue - 15.0).abs() < f64::EPSILON);
    }

    // spec: 02_chat.md - 円記号付き金額文字列をパースできる
    #[test]
    fn parse_amount_yen_with_comma_returns_correct_value() {
        assert_eq!(parse_amount("¥1,000"), Some(1000.0));
    }

    // spec: 02_chat.md - ドル記号付き小数金額文字列をパースできる
    #[test]
    fn parse_amount_dollar_with_decimal_returns_correct_value() {
        assert_eq!(parse_amount("$10.50"), Some(10.5));
    }

    // spec: 02_chat.md - 空文字列はNoneを返す
    #[test]
    fn parse_amount_empty_string_returns_none() {
        assert_eq!(parse_amount(""), None);
    }

    // spec: 02_chat.md - 数字を含まない文字列はNoneを返す
    #[test]
    fn parse_amount_non_numeric_string_returns_none() {
        assert_eq!(parse_amount("free"), None);
    }

    // spec: 03_websocket.md「ギフトの例」— 付加情報のある種別は { "種別": { ... } } の形で流れる
    #[test]
    fn gift_serializes_as_externally_tagged_for_websocket() {
        let gift = MessageType::Gift(GiftDetails {
            gift_name: "Press F".to_string(),
            gift_image_url: None,
            jewel_count: Some(10),
        });
        assert_eq!(
            serde_json::to_value(&gift).unwrap(),
            serde_json::json!({"Gift": {"gift_name": "Press F", "gift_image_url": null, "jewel_count": 10}})
        );
        assert_eq!(gift.as_str(), "gift");
    }

    fn counts(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
        pairs.iter().map(|(e, c)| (e.to_string(), *c)).collect()
    }

    // spec: 03_websocket.md「Reaction」の data の形
    #[test]
    fn reaction_update_serializes_with_total() {
        let update =
            ReactionUpdate::new(1790422357025983, 2, counts(&[("❤", 3), ("🎉", 5)])).unwrap();
        assert_eq!(
            serde_json::to_value(&update).unwrap(),
            serde_json::json!({
                "update_time_usec": 1790422357025983_i64,
                "duration_seconds": 2,
                "counts": {"❤": 3, "🎉": 5},
                "total": 8
            })
        );
    }

    // spec: 02_chat.md「ライブリアクション」— 0件の更新は捨てる
    #[test]
    fn reaction_update_with_no_reactions_is_none() {
        assert_eq!(ReactionUpdate::new(1, 1, counts(&[])), None);
        assert_eq!(ReactionUpdate::new(1, 1, counts(&[("❤", 0)])), None);
    }

    // spec: 02_chat.md「ReactionUpdate」— 0件の絵文字は含めない
    #[test]
    fn reaction_update_drops_zero_count_emoji() {
        let update = ReactionUpdate::new(1, 1, counts(&[("❤", 0), ("😄", 1)])).unwrap();
        assert_eq!(update.counts(), &counts(&[("😄", 1)]));
        assert_eq!(update.total(), 1);
    }
}
