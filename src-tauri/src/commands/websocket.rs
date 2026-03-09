//! WebSocket API commands for external app integration
//!
//! The WebSocket server starts automatically when the application launches.
//! Manual start/stop is not required.

use crate::core::api::{ClientEvent, WebSocketServer};
use crate::errors::CommandError;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

/// WebSocket server status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketStatus {
    pub is_running: bool,
    pub actual_port: Option<u16>,
    pub connected_clients: u32,
}

/// Tauri event payload for client connection events
#[derive(Debug, Clone, Serialize)]
struct ClientEventPayload {
    client_id: u64,
}

/// Start the WebSocket server automatically on app launch
///
/// This function is called from the setup hook, not exposed as a Tauri command.
pub async fn start_websocket_server_auto(
    app: AppHandle,
    websocket_server: Arc<RwLock<Option<WebSocketServer>>>,
) {
    let preferred_port = 8765;

    // Check if server is already running
    {
        let ws = websocket_server.read().await;
        if let Some(server) = ws.as_ref() {
            if server.is_running().await {
                tracing::info!("WebSocket server already running");
                return;
            }
        }
    }

    // Create and start new server
    let server = WebSocketServer::new(preferred_port);

    // Subscribe to client events before starting
    let mut event_rx = server.subscribe_events();

    match server.start().await {
        Ok(actual_port) => {
            // Spawn task to emit Tauri events for client connections
            let app_handle = app.clone();
            tokio::spawn(async move {
                while let Ok(event) = event_rx.recv().await {
                    match event {
                        ClientEvent::Connected { client_id } => {
                            let _ = app_handle
                                .emit("websocket-client-connected", ClientEventPayload { client_id });
                        }
                        ClientEvent::Disconnected { client_id } => {
                            let _ = app_handle.emit(
                                "websocket-client-disconnected",
                                ClientEventPayload { client_id },
                            );
                        }
                    }
                }
            });

            // Store server in state
            {
                let mut ws = websocket_server.write().await;
                *ws = Some(server);
            }

            tracing::info!("WebSocket server started automatically on port {}", actual_port);
        }
        Err(e) => {
            // Log error but don't fail app startup
            tracing::error!("Failed to start WebSocket server: {}. App will continue without WebSocket functionality.", e);
        }
    }
}

/// Get WebSocket server status
#[tauri::command]
pub async fn websocket_get_status(
    state: tauri::State<'_, AppState>,
) -> Result<WebSocketStatus, CommandError> {
    let ws = state.websocket_server.read().await;

    if let Some(server) = ws.as_ref() {
        Ok(WebSocketStatus {
            is_running: server.is_running().await,
            actual_port: server.actual_port().await,
            connected_clients: server.connected_clients().await,
        })
    } else {
        Ok(WebSocketStatus {
            is_running: false,
            actual_port: None,
            connected_clients: 0,
        })
    }
}
