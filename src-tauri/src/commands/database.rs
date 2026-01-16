//! Database commands

use crate::database::{self, Session, ViewerCustomInfo};
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

/// Update viewer info (custom info + tags) by viewer_profile_id
#[tauri::command]
pub async fn viewer_update_info(
    state: State<'_, AppState>,
    viewer_profile_id: i64,
    reading: Option<String>,
    notes: Option<String>,
    custom_data: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<bool, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;

    // Update custom info (reading, notes, custom_data)
    let custom_info = ViewerCustomInfo {
        viewer_profile_id,
        reading,
        notes,
        custom_data,
        created_at: None,
        updated_at: None,
    };
    database::upsert_viewer_custom_info(&conn, &custom_info)
        .map_err(|e| format!("Failed to update custom info: {}", e))?;

    // Update tags in viewer_profiles if provided
    if let Some(tags) = tags {
        database::update_viewer_tags(&conn, viewer_profile_id, Some(tags))
            .map_err(|e| format!("Failed to update tags: {}", e))?;
    }

    Ok(true)
}
