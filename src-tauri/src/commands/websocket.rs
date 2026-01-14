//! WebSocket API commands for external app integration

use crate::core::api::WebSocketServer;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// WebSocket server status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketStatus {
    pub is_running: bool,
    pub actual_port: Option<u16>,
    pub connected_clients: usize,
}

/// Result of starting WebSocket server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketStartResult {
    pub actual_port: u16,
}

/// Start the WebSocket server for external app integration
#[tauri::command]
pub async fn websocket_start(
    state: State<'_, AppState>,
    port: Option<u16>,
) -> Result<WebSocketStartResult, String> {
    let preferred_port = port.unwrap_or(8765);

    // Check if server is already running
    {
        let ws = state.websocket_server.read().await;
        if let Some(server) = ws.as_ref() {
            if server.is_running().await {
                if let Some(actual_port) = server.actual_port().await {
                    return Ok(WebSocketStartResult { actual_port });
                }
            }
        }
    }

    // Create and start new server
    let server = WebSocketServer::new(preferred_port);
    let actual_port = server
        .start()
        .await
        .map_err(|e| format!("Failed to start WebSocket server: {}", e))?;

    // Store server in state
    {
        let mut ws = state.websocket_server.write().await;
        *ws = Some(server);
    }

    tracing::info!("WebSocket server started on port {}", actual_port);

    Ok(WebSocketStartResult { actual_port })
}

/// Stop the WebSocket server
#[tauri::command]
pub async fn websocket_stop(state: State<'_, AppState>) -> Result<(), String> {
    let mut ws = state.websocket_server.write().await;
    if let Some(server) = ws.as_ref() {
        server.stop().await;
    }
    *ws = None;

    tracing::info!("WebSocket server stopped");

    Ok(())
}

/// Get WebSocket server status
#[tauri::command]
pub async fn websocket_get_status(state: State<'_, AppState>) -> Result<WebSocketStatus, String> {
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
