//! Application state management

use crate::connection::{ConnectionSlots, StreamConnection};
use crate::core::api::WebSocketServer;
use crate::database::Database;
use crate::errors::CommandError;
use crate::tts::{TtsManager, TtsProcessManager};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;

/// Application state shared across commands
pub struct AppState {
    /// WebSocket server for external app integration
    pub websocket_server: Arc<RwLock<Option<WebSocketServer>>>,
    /// Database connection
    pub database: Arc<RwLock<Option<Database>>>,
    /// TTS manager
    pub tts_manager: Arc<TtsManager>,
    /// TTS process manager
    pub tts_process_manager: Arc<TtsProcessManager>,
    /// 次の接続IDを生成するためのカウンター
    pub next_connection_id: Arc<AtomicU64>,
    /// アクティブな接続のマップ（connection_id -> StreamConnection）
    pub connections: Arc<RwLock<HashMap<u64, StreamConnection>>>,
    /// 同時接続の枠（接続済みと接続中を合わせて MAX_CONNECTIONS 件まで）
    pub connection_slots: ConnectionSlots,
    /// このアプリを起動した時刻。これ以降に始まったセッションが「現在」の分析の対象（07_revenue.md「集計の対象」）
    pub started_at: DateTime<Utc>,
}

impl AppState {
    pub fn new() -> Self {
        let started_at = Utc::now();

        // データベースを初期化
        let database = Database::new()
            .inspect_err(|e| {
                tracing::error!(
                    "データベースの初期化に失敗したため、保存せずに動作する: {:#}",
                    e
                )
            })
            .ok();

        // TTS マネージャーをデフォルト設定で初期化
        let tts_manager = TtsManager::default();

        // TTS プロセスマネージャーを初期化
        let tts_process_manager = TtsProcessManager::new();

        Self {
            websocket_server: Arc::new(RwLock::new(None)),
            database: Arc::new(RwLock::new(database)),
            tts_manager: Arc::new(tts_manager),
            tts_process_manager: Arc::new(tts_process_manager),
            next_connection_id: Arc::new(AtomicU64::new(0)),
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_slots: ConnectionSlots::default(),
            started_at,
        }
    }

    /// DB接続を取得する。データベース未初期化の場合は `DatabaseError` を返す
    ///
    /// コマンド層に散在していた「database の read ロック → 未初期化チェック →
    /// connection 取得」の定型句を集約するヘルパー。
    pub async fn db_connection(
        &self,
    ) -> Result<tokio::sync::OwnedMutexGuard<rusqlite::Connection>, CommandError> {
        let db_guard = self.database.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;
        Ok(db.connection_owned().await)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
