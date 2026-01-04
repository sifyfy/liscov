use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// セッションモデル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub stream_url: Option<String>,
    pub stream_title: Option<String>,
    pub total_messages: i64,
    pub total_revenue: f64,
}

/// 視聴者プロフィールモデル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerProfile {
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub message_count: i64,
    pub total_contribution: f64,
    pub membership_level: Option<String>,
    pub tags: Vec<String>,
}

/// 質問モデル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: Option<i64>,
    pub message_id: i64,
    pub session_id: String,
    pub detected_at: DateTime<Utc>,
    pub question_text: String,
    pub category: crate::chat_management::QuestionCategory,
    pub priority: crate::chat_management::Priority,
    pub confidence: f64,
    pub answered_at: Option<DateTime<Utc>>,
    pub answer_method: Option<crate::chat_management::AnswerMethod>,
    pub notes: Option<String>,
}

/// 視聴者カスタム情報モデル
/// 配信者チャンネル単位で管理される視聴者固有の情報
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ViewerCustomInfo {
    /// データベースID（新規作成時はNone）
    pub id: Option<i64>,
    /// 配信者のYouTubeチャンネルID
    pub broadcaster_channel_id: String,
    /// 視聴者のYouTubeチャンネルID
    pub viewer_channel_id: String,
    /// 視聴者名の読み仮名（ふりがな）
    pub reading: Option<String>,
    /// メモ（将来の拡張用）
    pub notes: Option<String>,
    /// カスタムデータ（JSON形式、将来の拡張用）
    pub custom_data: Option<String>,
    /// 作成日時
    pub created_at: Option<String>,
    /// 更新日時
    pub updated_at: Option<String>,
}

impl ViewerCustomInfo {
    /// 新規作成
    pub fn new(broadcaster_channel_id: String, viewer_channel_id: String) -> Self {
        Self {
            broadcaster_channel_id,
            viewer_channel_id,
            ..Default::default()
        }
    }

    /// 読み仮名を設定
    pub fn with_reading(mut self, reading: impl Into<String>) -> Self {
        self.reading = Some(reading.into());
        self
    }

    /// メモを設定
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}
