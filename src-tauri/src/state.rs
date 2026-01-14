//! Application state management

use crate::core::api::{InnerTubeClient, WebSocketServer};
use crate::core::models::ChatMessage;
use crate::database::Database;
use crate::tts::TtsManager;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum number of messages to keep in memory
const MAX_MESSAGES: usize = 1000;

/// Application state shared across commands
pub struct AppState {
    /// InnerTube client for YouTube API
    pub innertube_client: Arc<RwLock<Option<InnerTubeClient>>>,
    /// WebSocket server for external app integration
    pub websocket_server: Arc<RwLock<Option<WebSocketServer>>>,
    /// Chat messages buffer
    pub messages: Arc<RwLock<VecDeque<ChatMessage>>>,
    /// Whether chat monitoring is active
    pub is_monitoring: Arc<RwLock<bool>>,
    /// Database connection
    pub database: Arc<RwLock<Option<Database>>>,
    /// Current session ID
    pub current_session_id: Arc<RwLock<Option<String>>>,
    /// Current broadcaster channel ID
    pub current_broadcaster_id: Arc<RwLock<Option<String>>>,
    /// TTS manager
    pub tts_manager: Arc<TtsManager>,
}

impl AppState {
    pub fn new() -> Self {
        // Initialize database
        let database = match Database::new() {
            Ok(db) => Some(db),
            Err(e) => {
                tracing::error!("Failed to initialize database: {}", e);
                None
            }
        };

        // Initialize TTS manager with default config
        let tts_manager = TtsManager::default();

        Self {
            innertube_client: Arc::new(RwLock::new(None)),
            websocket_server: Arc::new(RwLock::new(None)),
            messages: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_MESSAGES))),
            is_monitoring: Arc::new(RwLock::new(false)),
            database: Arc::new(RwLock::new(database)),
            current_session_id: Arc::new(RwLock::new(None)),
            current_broadcaster_id: Arc::new(RwLock::new(None)),
            tts_manager: Arc::new(tts_manager),
        }
    }

    /// Add a message to the buffer
    pub async fn add_message(&self, message: ChatMessage) {
        let mut messages = self.messages.write().await;
        if messages.len() >= MAX_MESSAGES {
            messages.pop_front();
        }
        messages.push_back(message);
    }

    /// Get recent messages
    pub async fn get_messages(&self, limit: usize) -> Vec<ChatMessage> {
        let messages = self.messages.read().await;
        messages.iter().rev().take(limit).cloned().collect()
    }

    /// Clear all messages
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
