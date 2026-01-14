//! Database commands

use crate::database::{self, Session, ViewerCustomInfo, ViewerWithCustomInfo};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// GUI-friendly session info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSession {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub stream_url: Option<String>,
    pub stream_title: Option<String>,
    pub broadcaster_name: Option<String>,
    pub total_messages: i64,
    pub total_revenue: f64,
}

impl From<Session> for GuiSession {
    fn from(s: Session) -> Self {
        Self {
            id: s.id,
            start_time: s.start_time,
            end_time: s.end_time,
            stream_url: s.stream_url,
            stream_title: s.stream_title,
            broadcaster_name: s.broadcaster_name,
            total_messages: s.total_messages,
            total_revenue: s.total_revenue,
        }
    }
}

/// GUI-friendly stored message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiStoredMessage {
    pub id: i64,
    pub message_id: String,
    pub timestamp: String,
    pub author: String,
    pub author_icon_url: Option<String>,
    pub channel_id: String,
    pub content: String,
    pub message_type: String,
    pub amount: Option<String>,
    pub is_member: bool,
}

impl From<database::StoredMessage> for GuiStoredMessage {
    fn from(m: database::StoredMessage) -> Self {
        Self {
            id: m.id,
            message_id: m.message_id,
            timestamp: m.timestamp,
            author: m.author,
            author_icon_url: m.author_icon_url,
            channel_id: m.channel_id,
            content: m.content,
            message_type: m.message_type,
            amount: m.amount,
            is_member: m.is_member,
        }
    }
}

/// GUI-friendly viewer custom info input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiViewerCustomInfo {
    pub broadcaster_channel_id: String,
    pub viewer_channel_id: String,
    pub reading: Option<String>,
    pub notes: Option<String>,
    pub custom_data: Option<String>,
}

impl From<GuiViewerCustomInfo> for ViewerCustomInfo {
    fn from(info: GuiViewerCustomInfo) -> Self {
        ViewerCustomInfo {
            id: None,
            broadcaster_channel_id: info.broadcaster_channel_id,
            viewer_channel_id: info.viewer_channel_id,
            reading: info.reading,
            notes: info.notes,
            custom_data: info.custom_data,
            created_at: None,
            updated_at: None,
        }
    }
}

/// GUI-friendly viewer with custom info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiViewerWithCustomInfo {
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
}

impl From<ViewerWithCustomInfo> for GuiViewerWithCustomInfo {
    fn from(v: ViewerWithCustomInfo) -> Self {
        Self {
            channel_id: v.channel_id,
            display_name: v.display_name,
            first_seen: v.first_seen,
            last_seen: v.last_seen,
            message_count: v.message_count,
            total_contribution: v.total_contribution,
            membership_level: v.membership_level,
            tags: v.tags,
            reading: v.reading,
            notes: v.notes,
        }
    }
}

/// Get session list
#[tauri::command]
pub async fn get_sessions(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<GuiSession>, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;
    let sessions = database::get_sessions(&conn, limit.unwrap_or(50))
        .map_err(|e| format!("Failed to get sessions: {}", e))?;

    Ok(sessions.into_iter().map(GuiSession::from).collect())
}

/// Get messages for a specific session
#[tauri::command]
pub async fn get_session_messages(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<usize>,
) -> Result<Vec<GuiStoredMessage>, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;
    let messages = database::get_session_messages(&conn, &session_id, limit.unwrap_or(100))
        .map_err(|e| format!("Failed to get messages: {}", e))?;

    Ok(messages.into_iter().map(GuiStoredMessage::from).collect())
}

/// Upsert viewer custom info
#[tauri::command]
pub async fn upsert_viewer_custom_info(
    state: State<'_, AppState>,
    info: GuiViewerCustomInfo,
) -> Result<i64, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;
    let viewer_info: ViewerCustomInfo = info.into();
    let id = database::upsert_viewer_custom_info(&conn, &viewer_info)
        .map_err(|e| format!("Failed to upsert viewer info: {}", e))?;

    Ok(id)
}

/// Get viewers for a broadcaster
#[tauri::command]
pub async fn get_viewers_for_broadcaster(
    state: State<'_, AppState>,
    broadcaster_id: String,
    search_query: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<GuiViewerWithCustomInfo>, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;
    let viewers = database::get_viewers_for_broadcaster(
        &conn,
        &broadcaster_id,
        search_query.as_deref(),
        limit.unwrap_or(50),
        offset.unwrap_or(0),
    )
    .map_err(|e| format!("Failed to get viewers: {}", e))?;

    Ok(viewers
        .into_iter()
        .map(GuiViewerWithCustomInfo::from)
        .collect())
}
