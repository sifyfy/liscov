//! Chat message models

use serde::{Deserialize, Serialize};

/// Chat message type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
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
#[serde(tag = "type")]
pub enum MessageRun {
    Text { content: String },
    Emoji { emoji_id: String, image_url: String, alt_text: String },
}

/// Badge information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeInfo {
    pub badge_type: String,
    pub label: String,
    pub icon_url: Option<String>,
}

/// SuperChat color scheme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperChatColors {
    pub background_color: String,
    pub header_color: String,
    pub author_name_color: String,
    pub message_color: String,
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
    pub comment_count: Option<u32>,
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
            comment_count: None,
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
