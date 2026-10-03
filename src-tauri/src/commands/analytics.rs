//! 収益分析・エクスポートのコマンド（07_revenue.md）
//!
//! 集計と書き出しは `core::analytics`。ここは IPC の入出力と、対象のセッションを決めるだけ。

use crate::core::analytics::{
    ExportConfig, RevenueAnalytics, SessionExportData, SessionMetadata, export_messages,
    session_export_data, sessions_analytics, sessions_started_since, write_export,
};
use crate::errors::CommandError;
use crate::state::AppState;
use chrono::Utc;
use tauri::State;

/// この起動で接続したセッションの分析（07_revenue.md「集計の対象」）
#[tauri::command]
pub async fn get_revenue_analytics(
    state: State<'_, AppState>,
) -> Result<RevenueAnalytics, CommandError> {
    let conn = state.db_connection().await?;
    let session_ids = sessions_started_since(&conn, state.started_at)?;
    sessions_analytics(&conn, &session_ids)
}

/// Get analytics for a specific session from database
#[tauri::command]
pub async fn get_session_analytics(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<RevenueAnalytics, CommandError> {
    let conn = state.db_connection().await?;
    sessions_analytics(&conn, &[session_id])
}

/// Export session data to file
#[tauri::command]
pub async fn export_session_data(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
    config: ExportConfig,
) -> Result<(), CommandError> {
    // ファイルへの書き出しの間、DB のロックを握らない
    let export_data = {
        let conn = state.db_connection().await?;
        session_export_data(&conn, &session_id, &config)?
    };
    write_export(&export_data, &config, &file_path)
}

/// Export current session messages
#[tauri::command]
pub async fn export_current_messages(
    state: State<'_, AppState>,
    file_path: String,
    config: ExportConfig,
) -> Result<(), CommandError> {
    // この起動で接続したセッションのメッセージを DB から読む（07_revenue.md「集計の対象」）。
    // ファイルへの書き出しの間、DB のロックを握らない
    let (export_messages, statistics) = {
        let conn = state.db_connection().await?;
        let session_ids = sessions_started_since(&conn, state.started_at)?;
        export_messages(&conn, &session_ids, &config)?
    };

    // 多接続モデル: 最初の接続からセッションID・配信者IDを取得（エクスポートヘッダ用）
    let (session_id, broadcaster_id) = {
        let connections = state.connections.read().await;
        let session_id = connections
            .values()
            .find_map(|c| c.session_id.clone())
            .unwrap_or_else(|| "current".to_string());
        let broadcaster_id = connections
            .values()
            .map(|c| &c.broadcaster_channel_id)
            .find(|id| !id.is_empty())
            .cloned()
            .unwrap_or_default();
        (session_id, broadcaster_id)
    };

    let export_data = SessionExportData {
        metadata: SessionMetadata {
            session_id,
            stream_title: None,
            stream_url: None,
            broadcaster_name: None,
            broadcaster_channel_id: Some(broadcaster_id).filter(|id| !id.is_empty()),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            export_time: Utc::now().to_rfc3339(),
        },
        statistics,
        messages: export_messages,
    };
    write_export(&export_data, &config, &file_path)
}
