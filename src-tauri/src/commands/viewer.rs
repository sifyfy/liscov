//! Viewer management commands

use crate::database::{self, ContributorStats};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// GUI-friendly viewer profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiViewerProfile {
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub message_count: i64,
    pub total_contribution: f64,
    pub membership_level: Option<String>,
    pub tags: Vec<String>,
}

impl From<database::ViewerProfile> for GuiViewerProfile {
    fn from(p: database::ViewerProfile) -> Self {
        Self {
            channel_id: p.channel_id,
            display_name: p.display_name,
            first_seen: p.first_seen,
            last_seen: p.last_seen,
            message_count: p.message_count,
            total_contribution: p.total_contribution,
            membership_level: p.membership_level,
            tags: p.tags,
        }
    }
}

/// GUI-friendly viewer with custom info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiViewerWithInfo {
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

/// GUI-friendly contributor stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiContributorStats {
    pub channel_id: String,
    pub display_name: String,
    pub message_count: i64,
    pub total_contribution: f64,
}

impl From<ContributorStats> for GuiContributorStats {
    fn from(c: ContributorStats) -> Self {
        Self {
            channel_id: c.channel_id,
            display_name: c.display_name,
            message_count: c.message_count,
            total_contribution: c.total_contribution,
        }
    }
}

/// Get viewer profile by channel ID
#[tauri::command]
pub async fn get_viewer_profile(
    state: State<'_, AppState>,
    channel_id: String,
) -> Result<Option<GuiViewerProfile>, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;
    let profile = database::get_viewer_profile(&conn, &channel_id)
        .map_err(|e| format!("Failed to get viewer profile: {}", e))?;

    Ok(profile.map(GuiViewerProfile::from))
}

/// Get viewer with custom info
#[tauri::command]
pub async fn get_viewer_with_custom_info(
    state: State<'_, AppState>,
    broadcaster_id: String,
    viewer_id: String,
) -> Result<Option<GuiViewerWithInfo>, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;

    // Get viewer profile
    let profile = database::get_viewer_profile(&conn, &viewer_id)
        .map_err(|e| format!("Failed to get viewer profile: {}", e))?;

    let profile = match profile {
        Some(p) => p,
        None => return Ok(None),
    };

    // Get custom info
    let custom_info = database::get_viewer_custom_info(&conn, &broadcaster_id, &viewer_id)
        .map_err(|e| format!("Failed to get custom info: {}", e))?;

    Ok(Some(GuiViewerWithInfo {
        channel_id: profile.channel_id,
        display_name: profile.display_name,
        first_seen: profile.first_seen,
        last_seen: profile.last_seen,
        message_count: profile.message_count,
        total_contribution: profile.total_contribution,
        membership_level: profile.membership_level,
        tags: profile.tags,
        reading: custom_info.as_ref().and_then(|c| c.reading.clone()),
        notes: custom_info.as_ref().and_then(|c| c.notes.clone()),
    }))
}

/// Search viewers
#[tauri::command]
pub async fn search_viewers(
    state: State<'_, AppState>,
    broadcaster_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<GuiViewerWithInfo>, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;
    let viewers = database::get_viewers_for_broadcaster(
        &conn,
        &broadcaster_id,
        Some(&query),
        limit.unwrap_or(50),
        0,
    )
    .map_err(|e| format!("Failed to search viewers: {}", e))?;

    Ok(viewers
        .into_iter()
        .map(|v| GuiViewerWithInfo {
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
        })
        .collect())
}

/// Get top contributors for a session
#[tauri::command]
pub async fn get_top_contributors(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<usize>,
) -> Result<Vec<GuiContributorStats>, String> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conn = db.connection().await;
    let contributors = database::get_top_contributors(&conn, &session_id, limit.unwrap_or(10))
        .map_err(|e| format!("Failed to get contributors: {}", e))?;

    Ok(contributors
        .into_iter()
        .map(GuiContributorStats::from)
        .collect())
}
