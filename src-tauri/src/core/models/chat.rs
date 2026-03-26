//! Chat message models

use serde::{Deserialize, Serialize};

/// Chat message type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Text,
    SuperChat { amount: String },
    SuperSticker { amount: String },
    Membership { milestone_months: Option<u32> },
    MembershipGift { gift_count: u32 },
    System,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::Text
    }
}

/// Message run (text or emoji)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRun {
    Text { content: String },
    Emoji { emoji_id: String, image_url: String, alt_text: String },
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
    pub header_background: String,  // "#RRGGBB"
    pub header_text: String,
    pub body_background: String,
    pub body_text: String,
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for ChatMessage {
    fn default() -> Self {
        Self {
            id: String::new(),
            timestamp: String::new(),
            timestamp_usec: String::new(),
            message_type: MessageType::default(),
            author: String::new(),
            author_icon_url: None,
            channel_id: String::new(),
            content: String::new(),
            runs: vec![],
            metadata: None,
            is_member: false,
            is_first_time_viewer: false,
            in_stream_comment_count: None,
        }
    }
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
            MessageType::System => {}
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
        stats.update(&make_message(MessageType::SuperChat { amount: "$10.00".to_string() }));
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
        stats.update(&make_message(MessageType::SuperSticker { amount: "$5.00".to_string() }));
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
        stats.update(&make_message(MessageType::Membership { milestone_months: None }));
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
        stats.update(&make_message(MessageType::SuperChat { amount: "$10.00".to_string() }));
        stats.update(&make_message(MessageType::SuperSticker { amount: "$5.00".to_string() }));
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
}
