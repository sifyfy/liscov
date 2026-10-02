//! Application state management

use crate::connection::StreamConnection;
use crate::core::api::WebSocketServer;
use crate::core::models::ChatMessage;
use crate::database::Database;
use crate::errors::CommandError;
use crate::tts::{TtsManager, TtsProcessManager};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;

/// メモリに保持するメッセージの最大数
pub const MAX_MESSAGES: usize = 1000;

/// Application state shared across commands
pub struct AppState {
    /// WebSocket server for external app integration
    pub websocket_server: Arc<RwLock<Option<WebSocketServer>>>,
    /// Chat messages buffer（全接続のメッセージを統合するグローバルバッファ）
    pub messages: Arc<RwLock<VecDeque<ChatMessage>>>,
    /// Database connection
    pub database: Arc<RwLock<Option<Database>>>,
    /// データベースの初期化に失敗した理由（成功したら None）
    ///
    /// AppState はロガーの準備より先に作られるため、失敗をその場でログに残せない。
    /// 理由を持っておき、ロガーの準備後に書き出す。
    pub database_init_error: Option<String>,
    /// TTS manager
    pub tts_manager: Arc<TtsManager>,
    /// TTS process manager
    pub tts_process_manager: Arc<TtsProcessManager>,
    /// 次の接続IDを生成するためのカウンター
    pub next_connection_id: Arc<AtomicU64>,
    /// アクティブな接続のマップ（connection_id -> StreamConnection）
    pub connections: Arc<RwLock<HashMap<u64, StreamConnection>>>,
}

impl AppState {
    pub fn new() -> Self {
        // データベースを初期化
        let (database, database_init_error) = match Database::new() {
            Ok(db) => (Some(db), None),
            Err(e) => (None, Some(e.to_string())),
        };

        // TTS マネージャーをデフォルト設定で初期化
        let tts_manager = TtsManager::default();

        // TTS プロセスマネージャーを初期化
        let tts_process_manager = TtsProcessManager::new();

        Self {
            websocket_server: Arc::new(RwLock::new(None)),
            messages: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_MESSAGES))),
            database: Arc::new(RwLock::new(database)),
            database_init_error,
            tts_manager: Arc::new(tts_manager),
            tts_process_manager: Arc::new(tts_process_manager),
            next_connection_id: Arc::new(AtomicU64::new(0)),
            connections: Arc::new(RwLock::new(HashMap::new())),
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

    /// メッセージバッファにメッセージを追加する
    pub async fn add_message(&self, message: ChatMessage) {
        let mut messages = self.messages.write().await;
        if messages.len() >= MAX_MESSAGES {
            messages.pop_front();
        }
        messages.push_back(message);
    }

    /// 最近のメッセージを取得する
    pub async fn get_messages(&self, limit: usize) -> Vec<ChatMessage> {
        let messages = self.messages.read().await;
        messages.iter().rev().take(limit).cloned().collect()
    }

    /// 全メッセージをクリアする
    pub async fn clear_messages(&self) {
        let mut messages = self.messages.write().await;
        messages.clear();
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
