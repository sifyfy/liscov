//! Database models

use serde::{Deserialize, Serialize};

/// Session record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub stream_url: Option<String>,
    pub stream_title: Option<String>,
    pub broadcaster_channel_id: Option<String>,
    pub broadcaster_name: Option<String>,
    pub total_messages: i64,
    pub total_revenue: f64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Stored message record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: i64,
    pub session_id: String,
    pub message_id: String,
    pub timestamp: String,
    pub timestamp_usec: String,
    pub author: String,
    pub author_icon_url: Option<String>,
    pub channel_id: String,
    pub content: String,
    pub message_type: String,
    pub amount: Option<String>,
    pub is_member: bool,
    pub metadata: Option<String>,
    pub created_at: Option<String>,
}

/// Viewer profile record (broadcaster-scoped)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerProfile {
    pub id: i64,
    pub broadcaster_channel_id: String,
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub message_count: i64,
    pub total_contribution: f64,
    pub membership_level: Option<String>,
    pub tags: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Viewer custom info record (extension of viewer_profiles)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ViewerCustomInfo {
    pub viewer_profile_id: i64,
    pub reading: Option<String>,
    pub notes: Option<String>,
    pub custom_data: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl ViewerCustomInfo {
    pub fn new(viewer_profile_id: i64) -> Self {
        Self {
            viewer_profile_id,
            reading: None,
            notes: None,
            custom_data: None,
            created_at: None,
            updated_at: None,
        }
    }

    pub fn with_reading(mut self, reading: impl Into<String>) -> Self {
        self.reading = Some(reading.into());
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Broadcaster profile record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcasterProfile {
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub handle: Option<String>,
    pub thumbnail_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Combined viewer with custom info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerWithCustomInfo {
    pub id: i64,
    pub broadcaster_channel_id: String,
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub message_count: i64,
    pub total_contribution: f64,
    pub membership_level: Option<String>,
    pub tags: Vec<String>,
    pub reading: Option<String>,
    pub notes: Option<String>,
    pub custom_data: Option<String>,
}

/// Contributor stats for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorStats {
    pub channel_id: String,
    pub display_name: String,
    pub message_count: i64,
    pub total_contribution: f64,
}
