use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// エクスポート可能なメッセージデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableData {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub message_type: String,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub emoji_count: usize,
    pub word_count: usize,
    pub is_deleted: bool,
    pub is_moderator: bool,
    pub is_member: bool,
    pub is_verified: bool,
    pub badges: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// エクスポート可能な視聴者情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableViewer {
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub total_messages: usize,
    pub total_super_chat: f64,
    pub emoji_usage_rate: f64,
    pub average_message_length: f64,
    pub is_member: bool,
    pub is_moderator: bool,
    pub tags: Vec<String>,
}

/// エクスポート可能な感情分析データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableSentiment {
    pub message_id: String,
    pub sentiment_type: String,
    pub confidence: f64,
    pub positive_score: f64,
    pub negative_score: f64,
    pub neutral_score: f64,
    pub detected_emotions: Vec<String>,
}

/// エクスポート可能な統計データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableStats {
    pub total_messages: usize,
    pub unique_viewers: usize,
    pub total_super_chat_amount: f64,
    pub total_memberships: usize,
    pub average_messages_per_minute: f64,
    pub peak_concurrent_viewers: usize,
    pub engagement_rate: f64,
    pub emoji_usage_rate: f64,
    pub top_chatters: Vec<String>,
    pub top_contributors: Vec<String>,
    pub most_used_emojis: Vec<(String, usize)>,
    pub message_type_distribution: HashMap<String, usize>,
    pub hourly_activity: Vec<(u32, usize)>, // (hour, message_count)
}

/// セッションメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: String,
    pub stream_title: Option<String>,
    pub stream_url: String,
    pub channel_name: String,
    pub channel_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub export_time: DateTime<Utc>,
    pub export_version: String,
    pub liscov_version: String,
    pub total_data_points: usize,
    pub filters_applied: Vec<String>,
}

/// エクスポート用セッションデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub metadata: SessionMetadata,
    pub messages: Vec<ExportableData>,
    pub viewers: Vec<ExportableViewer>,
    pub sentiment_analysis: Vec<ExportableSentiment>,
    pub statistics: ExportableStats,
}

impl SessionData {
    /// 新しいセッションデータを作成
    pub fn new(
        session_id: String,
        stream_url: String,
        channel_name: String,
        channel_id: String,
    ) -> Self {
        let now = Utc::now();

        Self {
            metadata: SessionMetadata {
                session_id,
                stream_title: None,
                stream_url,
                channel_name,
                channel_id,
                start_time: now,
                end_time: None,
                duration_seconds: None,
                export_time: now,
                export_version: "1.0.0".to_string(),
                liscov_version: env!("CARGO_PKG_VERSION").to_string(),
                total_data_points: 0,
                filters_applied: Vec::new(),
            },
            messages: Vec::new(),
            viewers: Vec::new(),
            sentiment_analysis: Vec::new(),
            statistics: ExportableStats {
                total_messages: 0,
                unique_viewers: 0,
                total_super_chat_amount: 0.0,
                total_memberships: 0,
                average_messages_per_minute: 0.0,
                peak_concurrent_viewers: 0,
                engagement_rate: 0.0,
                emoji_usage_rate: 0.0,
                top_chatters: Vec::new(),
                top_contributors: Vec::new(),
                most_used_emojis: Vec::new(),
                message_type_distribution: HashMap::new(),
                hourly_activity: Vec::new(),
            },
        }
    }

    /// セッションを終了
    pub fn finalize_session(&mut self) {
        let now = Utc::now();
        self.metadata.end_time = Some(now);

        let duration = now
            .signed_duration_since(self.metadata.start_time)
            .num_seconds();

        if duration >= 0 {
            self.metadata.duration_seconds = Some(duration as u64);
        }

        self.metadata.total_data_points =
            self.messages.len() + self.viewers.len() + self.sentiment_analysis.len();
        self.update_statistics();
    }

    /// 統計情報を更新
    pub fn update_statistics(&mut self) {
        self.statistics.total_messages = self.messages.len();
        self.statistics.unique_viewers = self.viewers.len();

        self.statistics.total_super_chat_amount =
            self.messages.iter().filter_map(|msg| msg.amount).sum();

        self.statistics.total_memberships = self
            .messages
            .iter()
            .filter(|msg| msg.message_type == "membership")
            .count();

        // 時間あたりのメッセージ数を計算
        if let Some(duration_seconds) = self.metadata.duration_seconds {
            let duration_minutes = duration_seconds as f64 / 60.0;
            if duration_minutes > 0.0 {
                self.statistics.average_messages_per_minute =
                    self.statistics.total_messages as f64 / duration_minutes;
            }
        }

        // 絵文字使用率を計算
        let total_emojis: usize = self.messages.iter().map(|msg| msg.emoji_count).sum();
        if self.statistics.total_messages > 0 {
            self.statistics.emoji_usage_rate =
                total_emojis as f64 / self.statistics.total_messages as f64;
        }

        // メッセージタイプ分布を計算
        self.statistics.message_type_distribution.clear();
        for message in &self.messages {
            *self
                .statistics
                .message_type_distribution
                .entry(message.message_type.clone())
                .or_insert(0) += 1;
        }

        // 時間別活動を計算
        self.calculate_hourly_activity();
    }

    /// 時間別活動を計算
    fn calculate_hourly_activity(&mut self) {
        let mut hourly_counts: HashMap<u32, usize> = HashMap::new();

        for message in &self.messages {
            let hour = message.timestamp.hour();
            *hourly_counts.entry(hour).or_insert(0) += 1;
        }

        self.statistics.hourly_activity = hourly_counts.into_iter().collect();
        self.statistics
            .hourly_activity
            .sort_by_key(|&(hour, _)| hour);
    }

    /// フィルタリングされたメッセージを取得
    pub fn get_filtered_messages(&self, filters: &[&str]) -> Vec<&ExportableData> {
        if filters.is_empty() {
            return self.messages.iter().collect();
        }

        self.messages
            .iter()
            .filter(|msg| {
                filters.iter().any(|filter| match *filter {
                    "super-chat" => msg.message_type == "super-chat",
                    "membership" => msg.message_type == "membership",
                    "text" => msg.message_type == "text",
                    "moderator" => msg.is_moderator,
                    "member" => msg.is_member,
                    "verified" => msg.is_verified,
                    _ => false,
                })
            })
            .collect()
    }

    /// データの整合性を検証
    pub fn validate(&self) -> Result<(), String> {
        // 基本的な検証
        if self.metadata.session_id.is_empty() {
            return Err("Session ID cannot be empty".to_string());
        }

        if self.metadata.stream_url.is_empty() {
            return Err("Stream URL cannot be empty".to_string());
        }

        // メッセージIDの重複チェック
        let mut message_ids = std::collections::HashSet::new();
        for message in &self.messages {
            if !message_ids.insert(&message.id) {
                return Err(format!("Duplicate message ID: {}", message.id));
            }
        }

        // 視聴者IDの重複チェック
        let mut viewer_ids = std::collections::HashSet::new();
        for viewer in &self.viewers {
            if !viewer_ids.insert(&viewer.channel_id) {
                return Err(format!("Duplicate viewer ID: {}", viewer.channel_id));
            }
        }

        // 時系列の整合性チェック
        for i in 1..self.messages.len() {
            if self.messages[i].timestamp < self.messages[i - 1].timestamp {
                return Err("Messages are not in chronological order".to_string());
            }
        }

        Ok(())
    }

    /// データサイズを取得（バイト）
    pub fn estimated_size(&self) -> usize {
        // JSON形式でのおおよそのサイズを推定
        let messages_size = self.messages.len() * 300; // 平均300バイト/メッセージ
        let viewers_size = self.viewers.len() * 200; // 平均200バイト/視聴者
        let sentiment_size = self.sentiment_analysis.len() * 150; // 平均150バイト/感情分析
        let metadata_size = 1024; // メタデータ約1KB
        let stats_size = 2048; // 統計情報約2KB

        messages_size + viewers_size + sentiment_size + metadata_size + stats_size
    }
}

impl Default for SessionData {
    fn default() -> Self {
        Self::new(
            "default".to_string(),
            "".to_string(),
            "Unknown Channel".to_string(),
            "unknown".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_data_creation() {
        let session = SessionData::new(
            "test-session".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test Channel".to_string(),
            "test-channel-id".to_string(),
        );

        assert_eq!(session.metadata.session_id, "test-session");
        assert_eq!(session.metadata.channel_name, "Test Channel");
        assert!(session.messages.is_empty());
        assert!(session.viewers.is_empty());
    }

    #[test]
    fn test_validation() {
        let mut session = SessionData::default();
        session.metadata.session_id = "".to_string(); // 無効なセッションID

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_statistics_update() {
        let mut session = SessionData::new(
            "test".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test".to_string(),
            "test".to_string(),
        );

        // テストメッセージを追加
        session.messages.push(ExportableData {
            id: "msg1".to_string(),
            timestamp: Utc::now(),
            author: "User1".to_string(),
            author_id: "user1".to_string(),
            content: "Hello".to_string(),
            message_type: "text".to_string(),
            amount: None,
            currency: None,
            emoji_count: 1,
            word_count: 1,
            is_deleted: false,
            is_moderator: false,
            is_member: false,
            is_verified: false,
            badges: vec![],
            metadata: HashMap::new(),
        });

        session.update_statistics();
        assert_eq!(session.statistics.total_messages, 1);
        assert_eq!(session.statistics.emoji_usage_rate, 1.0);
    }
}
