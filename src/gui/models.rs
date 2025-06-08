use serde::{Deserialize, Serialize};

/// GUI用のチャットメッセージ構造体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuiChatMessage {
    pub timestamp: String,
    pub message_type: MessageType,
    pub author: String,
    pub channel_id: String,
    pub content: String,
    pub metadata: Option<MessageMetadata>,
    pub is_member: bool, // メンバーかどうかの判定フラグ
}

/// メッセージタイプ列挙型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Text,
    SuperChat { amount: String },
    SuperSticker { amount: String },
    Membership,
    System,
}

impl MessageType {
    pub fn as_string(&self) -> String {
        match self {
            MessageType::Text => "text".to_string(),
            MessageType::SuperChat { .. } => "super-chat".to_string(),
            MessageType::SuperSticker { .. } => "super-sticker".to_string(),
            MessageType::Membership => "membership".to_string(),
            MessageType::System => "system".to_string(),
        }
    }
}

/// メッセージメタデータ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageMetadata {
    pub amount: Option<String>,
    pub badges: Vec<String>,
    pub color: Option<String>,
    pub is_moderator: bool, // モデレーターかどうか
    pub is_verified: bool,  // 認証済みかどうか
}

/// 既存のliscovライブラリからGUI用メッセージへの変換
impl From<crate::get_live_chat::ChatItem> for GuiChatMessage {
    fn from(item: crate::get_live_chat::ChatItem) -> Self {
        match item {
            crate::get_live_chat::ChatItem::TextMessage { renderer } => {
                let message_parts: Vec<String> = renderer
                    .message
                    .runs
                    .iter()
                    .filter_map(|run| {
                        if let Some(text) = run.get_text() {
                            Some(text.to_string())
                        } else {
                            run.get_emoji().map(|emoji| format!(":{}:", emoji.emoji_id))
                        }
                    })
                    .collect();

                let (badges, is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                Self {
                    timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                    message_type: MessageType::Text,
                    author: renderer.author_name.simple_text.clone(),
                    channel_id: renderer.author_external_channel_id.clone(),
                    content: message_parts.join(""),
                    metadata: Some(MessageMetadata {
                        amount: None,
                        badges,
                        color: None,
                        is_moderator,
                        is_verified,
                    }),
                    is_member,
                }
            }
            crate::get_live_chat::ChatItem::PaidMessage { renderer } => {
                let (badges, is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                Self {
                    timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                    message_type: MessageType::SuperChat {
                        amount: renderer.purchase_amount_text.simple_text.clone(),
                    },
                    author: renderer.author_name.simple_text.clone(),
                    channel_id: renderer.author_external_channel_id.clone(),
                    content: renderer
                        .message
                        .as_ref()
                        .map(|msg| {
                            msg.runs
                                .iter()
                                .filter_map(|run| run.get_text().map(|t| t.to_string()))
                                .collect::<Vec<_>>()
                                .join("")
                        })
                        .unwrap_or_default(),
                    metadata: Some(MessageMetadata {
                        amount: Some(renderer.purchase_amount_text.simple_text.clone()),
                        badges,
                        color: None,
                        is_moderator,
                        is_verified,
                    }),
                    is_member,
                }
            }
            crate::get_live_chat::ChatItem::PaidSticker { renderer } => {
                let (badges, is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                Self {
                    timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                    message_type: MessageType::SuperSticker {
                        amount: renderer.purchase_amount_text.simple_text.clone(),
                    },
                    author: renderer.author_name.simple_text.clone(),
                    channel_id: renderer.author_external_channel_id.clone(),
                    content: format!(
                        "Super Sticker ({})",
                        renderer.purchase_amount_text.simple_text
                    ),
                    metadata: Some(MessageMetadata {
                        amount: Some(renderer.purchase_amount_text.simple_text.clone()),
                        badges,
                        color: None,
                        is_moderator,
                        is_verified,
                    }),
                    is_member,
                }
            }
            crate::get_live_chat::ChatItem::MembershipItem { renderer } => {
                let (badges, _is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                Self {
                    timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                    message_type: MessageType::Membership,
                    author: renderer.author_name.simple_text.clone(),
                    channel_id: renderer.author_external_channel_id.clone(),
                    content: "New member!".to_string(),
                    metadata: Some(MessageMetadata {
                        amount: None,
                        badges,
                        color: None,
                        is_moderator,
                        is_verified,
                    }),
                    is_member: true, // メンバーシップアイテムは常にメンバー
                }
            }
            _ => Self {
                timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                message_type: MessageType::System,
                author: "System".to_string(),
                channel_id: "".to_string(),
                content: "Unknown message type".to_string(),
                metadata: None,
                is_member: false,
            },
        }
    }
}

/// バッジ情報からメンバーシップ・モデレーター・認証情報を抽出
fn extract_badge_info(
    author_badges: &[crate::get_live_chat::AuthorBadge],
) -> (Vec<String>, bool, bool, bool) {
    let mut badges = Vec::new();
    let mut is_member = false;
    let mut is_moderator = false;
    let mut is_verified = false;

    for badge in author_badges {
        let tooltip = &badge.renderer.tooltip;
        let accessibility_label = &badge.renderer.accessibility.accessibility_data.label;

        badges.push(tooltip.clone());

        // メンバーシップバッジの判定（複数パターン）
        if tooltip.contains("メンバー")
            || tooltip.contains("Member")
            || accessibility_label.contains("メンバー")
            || accessibility_label.contains("Member")
            || tooltip.contains("新規メンバー")
            || tooltip.contains("New member")
        {
            is_member = true;
        }

        // モデレーターバッジの判定
        if tooltip.contains("モデレーター")
            || tooltip.contains("Moderator")
            || accessibility_label.contains("モデレーター")
            || accessibility_label.contains("Moderator")
        {
            is_moderator = true;
        }

        // 認証済みバッジの判定
        if tooltip.contains("認証")
            || tooltip.contains("Verified")
            || accessibility_label.contains("認証")
            || accessibility_label.contains("Verified")
        {
            is_verified = true;
        }
    }

    (badges, is_member, is_moderator, is_verified)
}

/// アプリケーション状態
#[derive(Debug, Clone)]
pub struct AppState {
    pub url: String,
    pub output_file: String,
    pub auto_save_enabled: bool, // 自動保存のオン・オフ
    pub is_connected: bool,
    pub message_count: usize,
    pub request_count: usize,
    pub messages: Vec<GuiChatMessage>,
    pub active_tab: ActiveTab,

    // 新しい保存設定
    pub save_raw_responses: bool,
    pub raw_response_file: String,
    pub max_raw_file_size_mb: u64,
    pub enable_file_rotation: bool,

    // ウィンドウ設定
    pub window: crate::gui::config_manager::WindowConfig,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            url: "https://youtube.com/watch?v=".to_string(),
            output_file: "live_chat.ndjson".to_string(),
            auto_save_enabled: false,
            is_connected: false,
            message_count: 0,
            request_count: 0,
            messages: Vec::new(),
            active_tab: ActiveTab::default(),
            save_raw_responses: false,
            raw_response_file: "raw_responses.ndjson".to_string(),
            max_raw_file_size_mb: 100,
            enable_file_rotation: true,
            window: crate::gui::config_manager::WindowConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveTab {
    ChatMonitor,
    RevenueAnalytics,
    EngagementAnalytics,
    DataExport,
    Settings,
}

impl Default for ActiveTab {
    fn default() -> Self {
        Self::ChatMonitor
    }
}

impl ActiveTab {
    pub fn to_string(&self) -> &'static str {
        match self {
            ActiveTab::ChatMonitor => "Chat Monitor",
            ActiveTab::RevenueAnalytics => "Revenue Analytics",
            ActiveTab::EngagementAnalytics => "Engagement Analytics",
            ActiveTab::DataExport => "Data Export",
            ActiveTab::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ActiveTab::ChatMonitor => "💬",
            ActiveTab::RevenueAnalytics => "💰",
            ActiveTab::EngagementAnalytics => "📊",
            ActiveTab::DataExport => "📥",
            ActiveTab::Settings => "⚙️",
        }
    }
}
