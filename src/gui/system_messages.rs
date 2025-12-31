//! システムメッセージ生成機能
//!
//! 配信終了、エラー警告、統計情報などのシステムメッセージを生成

use crate::gui::models::{GuiChatMessage, MessageType};

/// システムメッセージ用のIDとタイムスタンプを生成
fn generate_system_id_and_timestamps() -> (String, String, String) {
    let now = chrono::Utc::now();
    let timestamp_usec = now.timestamp_micros().to_string();
    let display_timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    let id = format!("system_{}", timestamp_usec);
    (id, display_timestamp, timestamp_usec)
}

/// システムメッセージの種類
#[derive(Debug, Clone, PartialEq)]
pub enum SystemMessageType {
    /// 配信終了通知
    StreamEnded,
    /// エラー警告（連続エラー発生時）
    ErrorWarning,
    /// 接続状態変更
    ConnectionChanged,
    /// 一般的なシステム通知
    General,
}

/// 配信統計情報
#[derive(Debug, Clone)]
pub struct StreamStats {
    pub total_messages: usize,
    pub stream_duration_minutes: u64,
    pub consecutive_errors: u32,
    pub unique_authors: usize,
    pub superchat_count: usize,
    pub membership_count: usize,
}

impl Default for StreamStats {
    fn default() -> Self {
        Self {
            total_messages: 0,
            stream_duration_minutes: 0,
            consecutive_errors: 0,
            unique_authors: 0,
            superchat_count: 0,
            membership_count: 0,
        }
    }
}

/// システムメッセージ生成器
pub struct SystemMessageGenerator;

impl SystemMessageGenerator {
    /// 配信終了メッセージを生成
    pub fn create_stream_ended_message(stats: StreamStats) -> GuiChatMessage {
        let content = if stats.stream_duration_minutes > 0 {
            format!(
                "🔴 配信が終了しました\n\n📊 配信統計:\n• 総メッセージ数: {}件\n• 配信時間: {}分\n• ユニーク投稿者: {}人\n• スーパーチャット: {}件\n• 新規メンバー: {}件\n\n✨ 視聴ありがとうございました！",
                stats.total_messages,
                stats.stream_duration_minutes,
                stats.unique_authors,
                stats.superchat_count,
                stats.membership_count
            )
        } else {
            format!(
                "🔴 配信が終了しました\n\n📊 総メッセージ数: {}件\n\n✨ 視聴ありがとうございました！",
                stats.total_messages
            )
        };

        let (id, timestamp, timestamp_usec) = generate_system_id_and_timestamps();

        GuiChatMessage {
            id,
            timestamp,
            timestamp_usec,
            message_type: MessageType::System,
            author: "📡 System".to_string(),
            author_icon_url: None,
            channel_id: "system".to_string(),
            content,
            runs: Vec::new(),
            metadata: Some(crate::gui::models::MessageMetadata {
                amount: None,
                badges: vec!["stream-ended".to_string()],
                badge_info: Vec::new(),
                color: Some("#ed8936".to_string()),
                is_moderator: false,
                is_verified: false,
            }),
            is_member: false,
            comment_count: None,
        }
    }

    /// エラー警告メッセージを生成
    pub fn create_error_warning_message(
        consecutive_errors: u32,
        error_type: &str,
    ) -> GuiChatMessage {
        let (emoji, message) = match consecutive_errors {
            1..=2 => ("⚠️", "接続に軽微な問題が発生しています"),
            3..=4 => ("🟡", "接続に問題が発生しています（再試行中）"),
            5..=7 => ("🟠", "接続問題が継続しています（配信終了の可能性）"),
            _ => ("🔴", "重大な接続問題が発生しています"),
        };

        let content = format!(
            "{} {}\n\n🔍 詳細:\n• 連続エラー回数: {}回\n• エラータイプ: {}\n• 自動復旧を試行中...",
            emoji, message, consecutive_errors, error_type
        );

        let (id, timestamp, timestamp_usec) = generate_system_id_and_timestamps();

        GuiChatMessage {
            id,
            timestamp,
            timestamp_usec,
            message_type: MessageType::System,
            author: "⚠️ System Alert".to_string(),
            author_icon_url: None,
            channel_id: "system".to_string(),
            content,
            runs: Vec::new(),
            metadata: Some(crate::gui::models::MessageMetadata {
                amount: None,
                badges: vec!["error-warning".to_string()],
                badge_info: Vec::new(),
                color: Some("#ffc107".to_string()),
                is_moderator: false,
                is_verified: false,
            }),
            is_member: false,
            comment_count: None,
        }
    }

    /// 接続状態変更メッセージを生成
    pub fn create_connection_message(is_connected: bool, url: Option<&str>) -> GuiChatMessage {
        let (emoji, _title, content) = if is_connected {
            let base_message = "✅ 配信に接続しました\n\n🔄 ライブチャットの監視を開始します";
            let content = if let Some(url) = url {
                format!("{}\n📡 配信URL: {}", base_message, url)
            } else {
                base_message.to_string()
            };
            ("✅", "Connected", content)
        } else {
            (
                "❌",
                "Disconnected",
                "❌ 配信から切断されました\n\n🔄 必要に応じて再接続してください".to_string(),
            )
        };

        let (id, timestamp, timestamp_usec) = generate_system_id_and_timestamps();

        GuiChatMessage {
            id,
            timestamp,
            timestamp_usec,
            message_type: MessageType::System,
            author: format!("{} System", emoji),
            author_icon_url: None,
            channel_id: "system".to_string(),
            content,
            runs: Vec::new(),
            metadata: Some(crate::gui::models::MessageMetadata {
                amount: None,
                badges: vec!["connection".to_string()],
                badge_info: Vec::new(),
                color: if is_connected {
                    Some("#22c55e".to_string())
                } else {
                    Some("#ef4444".to_string())
                },
                is_moderator: false,
                is_verified: false,
            }),
            is_member: false,
            comment_count: None,
        }
    }

    /// 一般的なシステムメッセージを生成
    pub fn create_general_message(title: &str, content: &str) -> GuiChatMessage {
        let (id, timestamp, timestamp_usec) = generate_system_id_and_timestamps();

        GuiChatMessage {
            id,
            timestamp,
            timestamp_usec,
            message_type: MessageType::System,
            author: format!("ℹ️ {}", title),
            author_icon_url: None,
            channel_id: "system".to_string(),
            content: content.to_string(),
            runs: Vec::new(),
            metadata: Some(crate::gui::models::MessageMetadata {
                amount: None,
                badges: vec!["general".to_string()],
                badge_info: Vec::new(),
                color: Some("#3b82f6".to_string()),
                is_moderator: false,
                is_verified: false,
            }),
            is_member: false,
            comment_count: None,
        }
    }

    /// 統計情報を収集
    pub fn collect_stream_stats(
        messages: &[GuiChatMessage],
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        consecutive_errors: u32,
    ) -> StreamStats {
        let total_messages = messages.len();

        let stream_duration_minutes = if let Some(start) = start_time {
            let duration = chrono::Utc::now().signed_duration_since(start);
            (duration.num_seconds() / 60).max(0) as u64
        } else {
            0
        };

        let mut unique_authors = std::collections::HashSet::new();
        let mut superchat_count = 0;
        let mut membership_count = 0;

        for message in messages {
            // システムメッセージは統計から除外
            if matches!(message.message_type, MessageType::System) {
                continue;
            }

            unique_authors.insert(&message.author);

            match &message.message_type {
                MessageType::SuperChat { .. } | MessageType::SuperSticker { .. } => {
                    superchat_count += 1;
                }
                MessageType::Membership { .. } => {
                    membership_count += 1;
                }
                _ => {}
            }
        }

        StreamStats {
            total_messages,
            stream_duration_minutes,
            consecutive_errors,
            unique_authors: unique_authors.len(),
            superchat_count,
            membership_count,
        }
    }
}

/// システムメッセージのCSSクラス名を生成
pub fn get_system_message_css_class(message: &GuiChatMessage) -> String {
    let mut classes = vec!["chat-message", "system"];

    if let Some(metadata) = &message.metadata {
        for badge in &metadata.badges {
            match badge.as_str() {
                "stream-ended" => classes.push("stream-ended"),
                "error-warning" => classes.push("error-warning"),
                _ => {}
            }
        }
    }

    classes.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_ended_message_generation() {
        let stats = StreamStats {
            total_messages: 1500,
            stream_duration_minutes: 120,
            consecutive_errors: 0,
            unique_authors: 350,
            superchat_count: 25,
            membership_count: 8,
        };

        let message = SystemMessageGenerator::create_stream_ended_message(stats);

        assert_eq!(message.message_type, MessageType::System);
        assert_eq!(message.author, "📡 System");
        assert!(message.content.contains("配信が終了しました"));
        assert!(message.content.contains("1500件"));
        assert!(message.content.contains("120分"));

        // CSSクラステスト
        let css_class = get_system_message_css_class(&message);
        assert!(css_class.contains("system"));
        assert!(css_class.contains("stream-ended"));
    }

    #[test]
    fn test_error_warning_message_generation() {
        let message = SystemMessageGenerator::create_error_warning_message(5, "403 Forbidden");

        assert_eq!(message.message_type, MessageType::System);
        assert_eq!(message.author, "⚠️ System Alert");
        assert!(message.content.contains("5回"));
        assert!(message.content.contains("403 Forbidden"));

        // CSSクラステスト
        let css_class = get_system_message_css_class(&message);
        assert!(css_class.contains("system"));
        assert!(css_class.contains("error-warning"));
    }

    #[test]
    fn test_connection_message_generation() {
        let message = SystemMessageGenerator::create_connection_message(
            true,
            Some("https://www.youtube.com/watch?v=example"),
        );

        assert_eq!(message.message_type, MessageType::System);
        assert!(message.author.contains("✅"));
        assert!(message.content.contains("接続しました"));
        assert!(message
            .content
            .contains("https://www.youtube.com/watch?v=example"));
    }

    #[test]
    fn test_stats_collection() {
        let messages = vec![
            GuiChatMessage {
                message_type: MessageType::Text,
                author: "user1".to_string(),
                ..Default::default()
            },
            GuiChatMessage {
                message_type: MessageType::SuperChat {
                    amount: "100".to_string(),
                },
                author: "user2".to_string(),
                ..Default::default()
            },
            GuiChatMessage {
                message_type: MessageType::System,
                author: "System".to_string(),
                ..Default::default()
            },
        ];

        let start_time = chrono::Utc::now() - chrono::Duration::minutes(30);
        let stats = SystemMessageGenerator::collect_stream_stats(&messages, Some(start_time), 3);

        assert_eq!(stats.total_messages, 3);
        assert_eq!(stats.consecutive_errors, 3);
        assert_eq!(stats.unique_authors, 2); // user1, user2 (Systemは除外)
        assert_eq!(stats.superchat_count, 1);
        assert!(stats.stream_duration_minutes >= 29 && stats.stream_duration_minutes <= 31);
    }
}
