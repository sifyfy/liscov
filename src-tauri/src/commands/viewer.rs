//! Viewer management commands

use crate::database::{self, ContributorStats, ViewerCustomInfo};
use crate::errors::CommandError;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;

/// GUI-friendly viewer profile
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GuiViewerProfile {
    /// SQLite の行ID（JS number の安全整数範囲内）
    #[ts(type = "number")]
    pub id: i64,
    pub broadcaster_channel_id: String,
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    /// 通算メッセージ数（JS number の安全整数範囲内）
    #[ts(type = "number")]
    pub message_count: i64,
    pub total_contribution: f64,
    pub membership_level: Option<String>,
    pub tags: Vec<String>,
}

impl From<database::ViewerProfile> for GuiViewerProfile {
    fn from(p: database::ViewerProfile) -> Self {
        Self {
            id: p.id,
            broadcaster_channel_id: p.broadcaster_channel_id,
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GuiViewerWithInfo {
    /// SQLite の行ID（JS number の安全整数範囲内）
    #[ts(type = "number")]
    pub id: i64,
    pub broadcaster_channel_id: String,
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    /// 通算メッセージ数（JS number の安全整数範囲内）
    #[ts(type = "number")]
    pub message_count: i64,
    pub total_contribution: f64,
    pub membership_level: Option<String>,
    pub tags: Vec<String>,
    pub reading: Option<String>,
    pub notes: Option<String>,
    pub custom_data: Option<String>,
}

impl From<database::ViewerWithCustomInfo> for GuiViewerWithInfo {
    fn from(v: database::ViewerWithCustomInfo) -> Self {
        Self {
            id: v.id,
            broadcaster_channel_id: v.broadcaster_channel_id,
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
            custom_data: v.custom_data,
        }
    }
}

/// GUI-friendly contributor stats
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GuiContributorStats {
    pub channel_id: String,
    pub display_name: String,
    /// 通算メッセージ数（JS number の安全整数範囲内）
    #[ts(type = "number")]
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

/// GUI-friendly broadcaster channel
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GuiBroadcasterChannel {
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub handle: Option<String>,
    /// ビューワー数（JS number の安全整数範囲内）
    #[ts(type = "number")]
    pub viewer_count: i64,
}

/// Get viewer profile by broadcaster ID and channel ID
#[tauri::command]
pub async fn viewer_get_profile(
    state: State<'_, AppState>,
    broadcaster_id: String,
    channel_id: String,
) -> Result<Option<GuiViewerProfile>, CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;
    let profile = database::get_viewer_profile(&conn, &broadcaster_id, &channel_id)
        .map_err(|e| CommandError::DatabaseError(format!("Failed to get viewer profile: {}", e)))?;

    Ok(profile.map(GuiViewerProfile::from))
}

/// Get viewer list for a broadcaster with optional search and pagination
#[tauri::command]
pub async fn viewer_get_list(
    state: State<'_, AppState>,
    broadcaster_id: String,
    search_query: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<GuiViewerWithInfo>, CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;
    let viewers = database::get_viewers_for_broadcaster(
        &conn,
        &broadcaster_id,
        search_query.as_deref(),
        limit.unwrap_or(50),
        offset.unwrap_or(0),
    )
    .map_err(|e| CommandError::DatabaseError(format!("Failed to get viewers: {}", e)))?;

    Ok(viewers.into_iter().map(GuiViewerWithInfo::from).collect())
}

/// Search viewers
#[tauri::command]
pub async fn viewer_search(
    state: State<'_, AppState>,
    broadcaster_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<GuiViewerWithInfo>, CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;
    let viewers = database::get_viewers_for_broadcaster(
        &conn,
        &broadcaster_id,
        Some(&query),
        limit.unwrap_or(50),
        0,
    )
    .map_err(|e| CommandError::DatabaseError(format!("Failed to search viewers: {}", e)))?;

    Ok(viewers.into_iter().map(GuiViewerWithInfo::from).collect())
}

/// Get viewer custom info by viewer_profile_id (direct DB lookup, no list scan)
#[tauri::command]
pub async fn viewer_get_custom_info(
    state: State<'_, AppState>,
    viewer_profile_id: i64,
) -> Result<Option<ViewerCustomInfo>, CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;
    database::get_viewer_custom_info(&conn, viewer_profile_id)
        .map_err(|e| CommandError::DatabaseError(format!("Failed to get viewer custom info: {}", e)))
}

/// Upsert viewer custom info
#[tauri::command]
pub async fn viewer_upsert_custom_info(
    state: State<'_, AppState>,
    viewer_profile_id: i64,
    reading: Option<String>,
    notes: Option<String>,
    custom_data: Option<String>,
) -> Result<(), CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;

    let info = ViewerCustomInfo {
        viewer_profile_id,
        reading,
        notes,
        custom_data,
        created_at: None,
        updated_at: None,
    };

    database::upsert_viewer_custom_info(&conn, &info)
        .map_err(|e| CommandError::DatabaseError(format!("Failed to upsert custom info: {}", e)))?;

    Ok(())
}

/// Delete viewer profile
#[tauri::command]
pub async fn viewer_delete(
    state: State<'_, AppState>,
    viewer_profile_id: i64,
) -> Result<bool, CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;
    let deleted = database::delete_viewer_profile(&conn, viewer_profile_id)
        .map_err(|e| CommandError::DatabaseError(format!("Failed to delete viewer: {}", e)))?;

    Ok(deleted)
}

/// Get broadcaster list with viewer counts
#[tauri::command]
pub async fn broadcaster_get_list(
    state: State<'_, AppState>,
) -> Result<Vec<GuiBroadcasterChannel>, CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;

    let broadcasters = database::get_distinct_broadcaster_channels(&conn)
        .map_err(|e| CommandError::DatabaseError(format!("Failed to get broadcasters: {}", e)))?;

    let mut result = Vec::new();
    for broadcaster in broadcasters {
        let viewer_count = database::get_viewer_count_for_broadcaster(&conn, &broadcaster.channel_id)
            .map_err(|e| CommandError::DatabaseError(format!("Failed to get viewer count: {}", e)))?;

        result.push(GuiBroadcasterChannel {
            channel_id: broadcaster.channel_id,
            channel_name: broadcaster.channel_name,
            handle: broadcaster.handle,
            viewer_count,
        });
    }

    Ok(result)
}

/// Delete broadcaster and all associated data
#[tauri::command]
pub async fn broadcaster_delete(
    state: State<'_, AppState>,
    broadcaster_id: String,
) -> Result<(bool, u32), CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;
    let (broadcaster_deleted, viewers_deleted) = database::delete_broadcaster(&conn, &broadcaster_id)
        .map_err(|e| CommandError::DatabaseError(format!("Failed to delete broadcaster: {}", e)))?;

    Ok((broadcaster_deleted, viewers_deleted))
}

/// Get top contributors for a session
#[tauri::command]
pub async fn get_top_contributors(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<usize>,
) -> Result<Vec<GuiContributorStats>, CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;
    let contributors = database::get_top_contributors(&conn, &session_id, limit.unwrap_or(10))
        .map_err(|e| CommandError::DatabaseError(format!("Failed to get contributors: {}", e)))?;

    Ok(contributors
        .into_iter()
        .map(GuiContributorStats::from)
        .collect())
}
