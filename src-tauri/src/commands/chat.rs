//! Chat monitoring commands

use crate::core::api::InnerTubeClient;
use crate::core::models::{extract_video_id, ChatMessage, ChatMode, ConnectionStatus};
use crate::core::raw_response::{RawResponseSaver, SaveConfig};
use crate::database::{self, Database};
use crate::AppState;
use crate::commands::SaveConfigState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

/// Result of connecting to a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionResult {
    pub success: bool,
    pub stream_title: Option<String>,
    pub broadcaster_channel_id: Option<String>,
    pub broadcaster_name: Option<String>,
    pub is_replay: bool,
    pub error: Option<String>,
    pub session_id: Option<String>,
}

impl From<ConnectionStatus> for ConnectionResult {
    fn from(status: ConnectionStatus) -> Self {
        Self {
            success: status.is_connected,
            stream_title: status.stream_title,
            broadcaster_channel_id: status.broadcaster_channel_id,
            broadcaster_name: status.broadcaster_name,
            is_replay: status.is_replay,
            error: status.error,
            session_id: None,
        }
    }
}

/// Message run (text or emoji)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageRun {
    Text { content: String },
    Emoji { emoji_id: String, image_url: String, alt_text: String },
}

/// Badge information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeInfo {
    pub tooltip: String,
    pub image_url: Option<String>,
}

/// SuperChat color scheme from YouTube
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperChatColors {
    pub header_background: String,
    pub header_text: String,
    pub body_background: String,
    pub body_text: String,
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiMessageMetadata {
    pub amount: Option<String>,
    pub badges: Vec<String>,
    pub badge_info: Vec<BadgeInfo>,
    pub is_moderator: bool,
    pub is_verified: bool,
    pub superchat_colors: Option<SuperChatColors>,
}

/// GUI-friendly chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiChatMessage {
    pub id: String,
    pub timestamp: String,
    pub timestamp_usec: String,
    pub author: String,
    pub author_icon_url: Option<String>,
    pub channel_id: String,
    pub content: String,
    pub runs: Vec<MessageRun>,
    pub message_type: String,
    pub amount: Option<String>,
    pub is_member: bool,
    pub comment_count: Option<u32>,
    pub metadata: Option<GuiMessageMetadata>,
}

impl From<ChatMessage> for GuiChatMessage {
    fn from(msg: ChatMessage) -> Self {
        let (message_type, amount) = match &msg.message_type {
            crate::core::models::MessageType::Text => ("text".to_string(), None),
            crate::core::models::MessageType::SuperChat { amount } => {
                ("superchat".to_string(), Some(amount.clone()))
            }
            crate::core::models::MessageType::SuperSticker { amount } => {
                ("supersticker".to_string(), Some(amount.clone()))
            }
            crate::core::models::MessageType::Membership { .. } => ("membership".to_string(), None),
            crate::core::models::MessageType::MembershipGift { .. } => {
                ("membership_gift".to_string(), None)
            }
            crate::core::models::MessageType::System => ("system".to_string(), None),
        };

        // Convert runs from core models to GUI models
        let runs: Vec<MessageRun> = msg.runs.into_iter().map(|run| {
            match run {
                crate::core::models::MessageRun::Text { content } => MessageRun::Text { content },
                crate::core::models::MessageRun::Emoji { emoji_id, image_url, alt_text } => {
                    MessageRun::Emoji { emoji_id, image_url, alt_text }
                }
            }
        }).collect();

        // Convert metadata
        let metadata = msg.metadata.map(|m| {
            GuiMessageMetadata {
                amount: m.amount,
                badges: m.badges,
                badge_info: m.badge_info.into_iter().map(|b| {
                    BadgeInfo {
                        tooltip: b.label,
                        image_url: b.icon_url,
                    }
                }).collect(),
                is_moderator: m.is_moderator,
                is_verified: m.is_verified,
                superchat_colors: m.superchat_colors.map(|c| {
                    SuperChatColors {
                        header_background: c.header_color,
                        header_text: c.author_name_color,
                        body_background: c.background_color,
                        body_text: c.message_color,
                    }
                }),
            }
        });

        Self {
            id: msg.id,
            timestamp: msg.timestamp,
            timestamp_usec: msg.timestamp_usec,
            author: msg.author,
            author_icon_url: msg.author_icon_url,
            channel_id: msg.channel_id,
            content: msg.content,
            runs,
            message_type,
            amount,
            is_member: msg.is_member,
            comment_count: msg.comment_count,
            metadata,
        }
    }
}

/// Connect to a YouTube live stream and start monitoring chat
#[tauri::command]
pub async fn connect_to_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    save_config_state: State<'_, SaveConfigState>,
    url: String,
    chat_mode: Option<String>,
) -> Result<ConnectionResult, String> {
    // Extract video ID from URL
    let video_id = extract_video_id(&url).ok_or_else(|| "Invalid YouTube URL".to_string())?;

    // Parse chat mode
    let mode = match chat_mode.as_deref() {
        Some("all") | Some("AllChat") => ChatMode::AllChat,
        _ => ChatMode::TopChat,
    };

    // Create and initialize InnerTube client
    let mut client = InnerTubeClient::new(&video_id);
    client.set_chat_mode(mode);

    let status = client
        .initialize()
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let mut result = ConnectionResult::from(status.clone());

    if result.success {
        // Create database session
        let session_id = {
            let db_guard = state.database.read().await;
            if let Some(db) = db_guard.as_ref() {
                let conn = db.connection().await;
                match database::create_session(
                    &conn,
                    Some(&url),
                    result.stream_title.as_deref(),
                    result.broadcaster_channel_id.as_deref(),
                    result.broadcaster_name.as_deref(),
                ) {
                    Ok(id) => {
                        tracing::info!("Created session: {}", id);
                        Some(id)
                    }
                    Err(e) => {
                        tracing::error!("Failed to create session: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        };

        result.session_id = session_id.clone();

        // Store session and broadcaster info in state
        {
            let mut session = state.current_session_id.write().await;
            *session = session_id;
        }
        {
            let mut broadcaster = state.current_broadcaster_id.write().await;
            *broadcaster = result.broadcaster_channel_id.clone();
        }

        // Store client in state
        {
            let mut innertube = state.innertube_client.write().await;
            *innertube = Some(client);
        }

        // Clear old messages
        state.clear_messages().await;

        // Start monitoring task
        let is_monitoring = Arc::clone(&state.is_monitoring);
        let innertube_client = Arc::clone(&state.innertube_client);
        let messages = Arc::clone(&state.messages);
        let websocket_server = Arc::clone(&state.websocket_server);
        let database = Arc::clone(&state.database);
        let current_session_id = Arc::clone(&state.current_session_id);
        let app_handle = app.clone();

        // Get save config for raw response saving
        let save_config = save_config_state.0.lock()
            .map_err(|e| e.to_string())?
            .clone();

        {
            let mut monitoring = is_monitoring.write().await;
            *monitoring = true;
        }

        tokio::spawn(async move {
            chat_monitoring_task(
                app_handle,
                innertube_client,
                messages,
                websocket_server,
                database,
                current_session_id,
                is_monitoring,
                save_config,
            )
            .await;
        });

        // Emit connection event
        let _ = app.emit("chat:connection", &result);
    }

    Ok(result)
}

/// Chat monitoring task that polls for new messages
async fn chat_monitoring_task(
    app: AppHandle,
    innertube_client: Arc<RwLock<Option<InnerTubeClient>>>,
    messages: Arc<RwLock<std::collections::VecDeque<ChatMessage>>>,
    websocket_server: Arc<RwLock<Option<crate::core::api::WebSocketServer>>>,
    database: Arc<RwLock<Option<Database>>>,
    current_session_id: Arc<RwLock<Option<String>>>,
    is_monitoring: Arc<RwLock<bool>>,
    save_config: SaveConfig,
) {
    let poll_interval = std::time::Duration::from_millis(1500);
    let raw_response_saver = RawResponseSaver::new(save_config);

    loop {
        // Check if we should stop monitoring
        {
            let monitoring = is_monitoring.read().await;
            if !*monitoring {
                break;
            }
        }

        // Fetch new messages with raw response
        let (new_messages, raw_response) = {
            let mut client_guard = innertube_client.write().await;
            if let Some(client) = client_guard.as_mut() {
                match client.fetch_messages_with_raw().await {
                    Ok((msgs, raw)) => (msgs, Some(raw)),
                    Err(e) => {
                        tracing::warn!("Failed to fetch messages: {}", e);
                        (vec![], None)
                    }
                }
            } else {
                break;
            }
        };

        // Save raw response if enabled
        if let Some(raw_json) = raw_response {
            if let Err(e) = raw_response_saver.save_response(&raw_json).await {
                tracing::warn!("Failed to save raw response: {}", e);
            }
        }

        // Get current session ID
        let session_id = {
            let session = current_session_id.read().await;
            session.clone()
        };

        // Process new messages
        for msg in new_messages {
            // Add to buffer
            {
                let mut msgs = messages.write().await;
                if msgs.len() >= 1000 {
                    msgs.pop_front();
                }
                msgs.push_back(msg.clone());
            }

            // Save to database
            if let Some(ref session_id) = session_id {
                let db_guard = database.read().await;
                if let Some(db) = db_guard.as_ref() {
                    let conn = db.connection().await;
                    if let Err(e) = database::save_message(&conn, session_id, &msg) {
                        tracing::warn!("Failed to save message: {}", e);
                    }
                }
            }

            // Convert to GUI message
            let gui_msg = GuiChatMessage::from(msg.clone());

            // Emit to frontend
            let _ = app.emit("chat:message", &gui_msg);

            // Broadcast to WebSocket clients
            {
                let ws = websocket_server.read().await;
                if let Some(server) = ws.as_ref() {
                    server.broadcast_message(&msg).await;
                }
            }
        }

        tokio::time::sleep(poll_interval).await;
    }

    // End session
    if let Some(session_id) = current_session_id.read().await.as_ref() {
        let db_guard = database.read().await;
        if let Some(db) = db_guard.as_ref() {
            let conn = db.connection().await;
            if let Err(e) = database::end_session(&conn, session_id) {
                tracing::warn!("Failed to end session: {}", e);
            }
            if let Err(e) = database::update_session_stats(&conn, session_id) {
                tracing::warn!("Failed to update session stats: {}", e);
            }
        }
    }

    tracing::info!("Chat monitoring task stopped");
}

/// Disconnect from the current stream
#[tauri::command]
pub async fn disconnect_stream(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Stop monitoring
    {
        let mut monitoring = state.is_monitoring.write().await;
        *monitoring = false;
    }

    // Clear client
    {
        let mut client = state.innertube_client.write().await;
        *client = None;
    }

    // Clear session and broadcaster info
    {
        let mut session = state.current_session_id.write().await;
        *session = None;
    }
    {
        let mut broadcaster = state.current_broadcaster_id.write().await;
        *broadcaster = None;
    }

    // Emit disconnection event
    let _ = app.emit(
        "chat:connection",
        ConnectionResult {
            success: false,
            stream_title: None,
            broadcaster_channel_id: None,
            broadcaster_name: None,
            is_replay: false,
            error: None,
            session_id: None,
        },
    );

    Ok(())
}

/// Get recent chat messages
#[tauri::command]
pub async fn get_chat_messages(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<GuiChatMessage>, String> {
    let limit = limit.unwrap_or(100);
    let messages = state.get_messages(limit).await;
    Ok(messages.into_iter().map(GuiChatMessage::from).collect())
}

/// Set chat mode (TopChat or AllChat)
#[tauri::command]
pub async fn set_chat_mode(
    state: State<'_, AppState>,
    mode: String,
) -> Result<bool, String> {
    let chat_mode = match mode.as_str() {
        "all" | "AllChat" => ChatMode::AllChat,
        _ => ChatMode::TopChat,
    };

    let mut client_guard = state.innertube_client.write().await;
    if let Some(client) = client_guard.as_mut() {
        client.set_chat_mode(chat_mode);
        Ok(true)
    } else {
        Err("Not connected to any stream".to_string())
    }
}
