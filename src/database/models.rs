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
