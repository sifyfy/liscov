//! WebSocket API commands for external app integration
//!
//! The WebSocket server starts automatically when the application launches.
//! Manual start/stop is not required.

use crate::AppState;
use crate::core::api::{ClientEvent, WebSocketServer};
use crate::errors::CommandError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use ts_rs::TS;

/// WebSocket server status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
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

/// 本番の開始ポート (8765〜8774 を試す)
const DEFAULT_WEBSOCKET_PORT: u16 = 8765;

/// 開始ポートを上書きする環境変数 (E2E テスト用。0 で OS に選ばせる)
const WEBSOCKET_PORT_ENV: &str = "LISCOV_WEBSOCKET_PORT";

/// `LISCOV_WEBSOCKET_PORT` の値から開始ポートを決める。未設定・不正なら本番の既定
pub fn websocket_start_port(value: Option<&str>) -> u16 {
    let Some(value) = value else {
        return DEFAULT_WEBSOCKET_PORT;
    };
    value.parse().unwrap_or_else(|_| {
        tracing::warn!(
            "{WEBSOCKET_PORT_ENV}={value:?} はポート番号として読めないため、既定の {DEFAULT_WEBSOCKET_PORT} から試します"
        );
        DEFAULT_WEBSOCKET_PORT
    })
}

/// Start the WebSocket server automatically on app launch
///
/// This function is called from the setup hook, not exposed as a Tauri command.
pub async fn start_websocket_server_auto(
    app: AppHandle,
    websocket_server: Arc<RwLock<Option<WebSocketServer>>>,
) {
    let preferred_port = websocket_start_port(std::env::var(WEBSOCKET_PORT_ENV).ok().as_deref());

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
                            let _ = app_handle.emit(
                                "websocket-client-connected",
                                ClientEventPayload { client_id },
                            );
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

            tracing::info!(
                "WebSocket server started automatically on port {}",
                actual_port
            );
        }
        Err(e) => {
            // Log error but don't fail app startup
            tracing::error!(
                "Failed to start WebSocket server: {}. App will continue without WebSocket functionality.",
                e
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    // 仕様: 03_websocket.md「開始ポートの上書き（E2E テスト用）」の表

    #[test]
    fn unset_uses_default_port() {
        assert_eq!(websocket_start_port(None), 8765);
    }

    #[test]
    fn zero_lets_os_choose() {
        assert_eq!(websocket_start_port(Some("0")), 0);
    }

    #[test]
    fn valid_port_is_used_as_start() {
        assert_eq!(websocket_start_port(Some("9000")), 9000);
        assert_eq!(websocket_start_port(Some("65530")), 65530);
    }

    #[test]
    fn invalid_values_fall_back_to_default_port() {
        for value in ["abc", "-1", "65536", ""] {
            assert_eq!(websocket_start_port(Some(value)), 8765, "value: {value:?}");
        }
    }
}
