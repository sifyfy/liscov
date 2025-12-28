//! WebSocket API Server
//!
//! 外部アプリケーションにチャットメッセージをリアルタイムで提供するWebSocketサーバー。
//!
//! ## 使用方法
//!
//! ```ignore
//! // サーバーを起動
//! let server = WebSocketServer::new(8765);
//! server.start().await?;
//!
//! // メッセージをブロードキャスト
//! server.broadcast_message(&message).await;
//!
//! // サーバーを停止
//! server.stop().await;
//! ```
//!
//! ## WebSocket API
//!
//! クライアントは `ws://localhost:8765` に接続してメッセージを受信できる。
//! メッセージはJSON形式で送信される。

use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;

use crate::gui::models::GuiChatMessage;

/// WebSocket接続のID
type ClientId = u64;

/// サーバーからクライアントへのメッセージ
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    /// チャットメッセージ
    ChatMessage(GuiChatMessage),
    /// 接続確認
    Connected { client_id: ClientId },
    /// サーバー情報
    ServerInfo {
        version: String,
        connected_clients: usize,
    },
    /// エラー
    Error { message: String },
}

/// クライアントからサーバーへのメッセージ
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Ping
    Ping,
    /// サーバー情報をリクエスト
    GetInfo,
}

/// WebSocketサーバーの状態
#[derive(Debug, Clone, PartialEq)]
pub enum ServerState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// WebSocketサーバー
pub struct WebSocketServer {
    port: u16,
    state: Arc<RwLock<ServerState>>,
    clients: Arc<RwLock<HashMap<ClientId, tokio::sync::mpsc::UnboundedSender<Message>>>>,
    message_tx: broadcast::Sender<ServerMessage>,
    next_client_id: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
}

impl WebSocketServer {
    /// 新しいWebSocketサーバーを作成
    pub fn new(port: u16) -> Self {
        let (message_tx, _) = broadcast::channel(1024);
        Self {
            port,
            state: Arc::new(RwLock::new(ServerState::Stopped)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            next_client_id: Arc::new(AtomicU64::new(1)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// サーバーを起動
    pub async fn start(&self) -> anyhow::Result<()> {
        {
            let mut state = self.state.write().await;
            if *state != ServerState::Stopped {
                return Err(anyhow::anyhow!("Server is already running or starting"));
            }
            *state = ServerState::Starting;
        }

        self.shutdown.store(false, Ordering::SeqCst);

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("🌐 WebSocket server listening on ws://{}", addr);

        {
            let mut state = self.state.write().await;
            *state = ServerState::Running;
        }

        let clients = Arc::clone(&self.clients);
        let message_tx = self.message_tx.clone();
        let next_client_id = Arc::clone(&self.next_client_id);
        let shutdown = Arc::clone(&self.shutdown);
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            while !shutdown.load(Ordering::SeqCst) {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                let client_id = next_client_id.fetch_add(1, Ordering::SeqCst);
                                tracing::info!("📥 New WebSocket connection from {} (client_id: {})", addr, client_id);

                                let clients = Arc::clone(&clients);
                                let mut message_rx = message_tx.subscribe();

                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, addr, client_id, clients, &mut message_rx).await {
                                        tracing::warn!("WebSocket connection error for client {}: {}", client_id, e);
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        // Check shutdown flag periodically
                    }
                }
            }

            let mut state_guard = state.write().await;
            *state_guard = ServerState::Stopped;
            tracing::info!("🛑 WebSocket server stopped");
        });

        Ok(())
    }

    /// サーバーを停止
    pub async fn stop(&self) {
        tracing::info!("🛑 Stopping WebSocket server...");

        {
            let mut state = self.state.write().await;
            *state = ServerState::Stopping;
        }

        self.shutdown.store(true, Ordering::SeqCst);

        // すべてのクライアントを切断
        let mut clients = self.clients.write().await;
        clients.clear();
    }

    /// メッセージを全クライアントにブロードキャスト
    pub async fn broadcast_message(&self, message: &GuiChatMessage) {
        let server_msg = ServerMessage::ChatMessage(message.clone());

        if let Err(e) = self.message_tx.send(server_msg.clone()) {
            tracing::trace!("No active subscribers for broadcast: {}", e);
        }

        // 直接クライアントにも送信（バックアップ）
        let json = match serde_json::to_string(&server_msg) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Failed to serialize message: {}", e);
                return;
            }
        };

        let clients = self.clients.read().await;
        for (client_id, sender) in clients.iter() {
            if sender.send(Message::Text(json.clone())).is_err() {
                tracing::debug!("Client {} disconnected", client_id);
            }
        }
    }

    /// 接続中のクライアント数を取得
    pub async fn connected_clients(&self) -> usize {
        self.clients.read().await.len()
    }

    /// サーバーの状態を取得
    pub async fn get_state(&self) -> ServerState {
        self.state.read().await.clone()
    }

    /// サーバーが実行中かどうか
    pub async fn is_running(&self) -> bool {
        *self.state.read().await == ServerState::Running
    }

    /// ポート番号を取得
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// WebSocket接続を処理
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    client_id: ClientId,
    clients: Arc<RwLock<HashMap<ClientId, tokio::sync::mpsc::UnboundedSender<Message>>>>,
    message_rx: &mut broadcast::Receiver<ServerMessage>,
) -> anyhow::Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    // クライアント用の送信チャネルを作成
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // クライアントを登録
    {
        let mut clients_guard = clients.write().await;
        clients_guard.insert(client_id, tx);
    }

    // 接続確認メッセージを送信
    let connected_msg = ServerMessage::Connected { client_id };
    let json = serde_json::to_string(&connected_msg)?;
    write.send(Message::Text(json)).await?;

    tracing::info!("✅ Client {} connected from {}", client_id, addr);

    loop {
        tokio::select! {
            // クライアントからのメッセージを処理
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::Ping => {
                                    write.send(Message::Pong(vec![])).await?;
                                }
                                ClientMessage::GetInfo => {
                                    let clients_guard = clients.read().await;
                                    let info = ServerMessage::ServerInfo {
                                        version: env!("CARGO_PKG_VERSION").to_string(),
                                        connected_clients: clients_guard.len(),
                                    };
                                    let json = serde_json::to_string(&info)?;
                                    write.send(Message::Text(json)).await?;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        write.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!("📤 Client {} disconnected", client_id);
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::warn!("WebSocket error for client {}: {}", client_id, e);
                        break;
                    }
                    _ => {}
                }
            }

            // ブロードキャストメッセージを受信
            msg = message_rx.recv() => {
                if let Ok(server_msg) = msg {
                    let json = serde_json::to_string(&server_msg)?;
                    if write.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }

            // 直接送信キューからのメッセージ
            msg = rx.recv() => {
                if let Some(message) = msg {
                    if write.send(message).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    // クライアントを削除
    {
        let mut clients_guard = clients.write().await;
        clients_guard.remove(&client_id);
    }

    Ok(())
}

// グローバルWebSocketサーバーインスタンス
static WEBSOCKET_SERVER: std::sync::OnceLock<Arc<WebSocketServer>> = std::sync::OnceLock::new();

/// グローバルWebSocketサーバーを取得または作成
pub fn get_websocket_server() -> Arc<WebSocketServer> {
    WEBSOCKET_SERVER
        .get_or_init(|| Arc::new(WebSocketServer::new(8765)))
        .clone()
}

/// カスタムポートでグローバルWebSocketサーバーを初期化
pub fn init_websocket_server(port: u16) -> Arc<WebSocketServer> {
    WEBSOCKET_SERVER
        .get_or_init(|| Arc::new(WebSocketServer::new(port)))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Connected { client_id: 1 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Connected"));
        assert!(json.contains("client_id"));
    }

    #[test]
    fn test_client_message_deserialization() {
        let json = r#"{"type":"Ping"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }

    #[tokio::test]
    async fn test_server_creation() {
        let server = WebSocketServer::new(0); // ポート0でランダムポートを使用
        assert_eq!(server.get_state().await, ServerState::Stopped);
        assert_eq!(server.connected_clients().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast_without_clients() {
        let server = WebSocketServer::new(0);
        let message = GuiChatMessage::default();
        // クライアントがいなくてもエラーにならない
        server.broadcast_message(&message).await;
    }
}
