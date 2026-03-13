//! Application state management

use crate::connection::StreamConnection;
use crate::core::api::WebSocketServer;
use crate::core::models::ChatMessage;
use crate::database::Database;
use crate::tts::{TtsManager, TtsProcessManager};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
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
        let database = match Database::new() {
            Ok(db) => Some(db),
            Err(e) => {
                tracing::error!("Failed to initialize database: {}", e);
                None
            }
        };

        // TTS マネージャーをデフォルト設定で初期化
        let tts_manager = TtsManager::default();

        // TTS プロセスマネージャーを初期化
        let tts_process_manager = TtsProcessManager::new();

        Self {
            websocket_server: Arc::new(RwLock::new(None)),
            messages: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_MESSAGES))),
            database: Arc::new(RwLock::new(database)),
            tts_manager: Arc::new(tts_manager),
            tts_process_manager: Arc::new(tts_process_manager),
            next_connection_id: Arc::new(AtomicU64::new(0)),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
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
