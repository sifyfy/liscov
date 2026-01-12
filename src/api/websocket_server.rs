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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// ポート候補の開始番号
const DEFAULT_PORT_START: u16 = 8765;
/// ポート候補の終了番号
const DEFAULT_PORT_END: u16 = 8774;

/// WebSocketサーバー
pub struct WebSocketServer {
    /// 希望ポート（開始ポート）
    preferred_port: u16,
    /// 実際に使用中のポート
    actual_port: Arc<RwLock<Option<u16>>>,
    state: Arc<RwLock<ServerState>>,
    clients: Arc<RwLock<HashMap<ClientId, tokio::sync::mpsc::UnboundedSender<Message>>>>,
    message_tx: broadcast::Sender<ServerMessage>,
    next_client_id: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
}

impl WebSocketServer {
    /// 新しいWebSocketサーバーを作成
    ///
    /// `port`は希望するポート番号。サーバー起動時にこのポートが使用中の場合、
    /// 自動的に次のポート番号を試行する。
    pub fn new(port: u16) -> Self {
        let (message_tx, _) = broadcast::channel(1024);
        Self {
            preferred_port: port,
            actual_port: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(ServerState::Stopped)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            next_client_id: Arc::new(AtomicU64::new(1)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// サーバーを起動
    ///
    /// 希望ポートが使用中の場合、自動的に次のポート（最大10ポート）を試行する。
    pub async fn start(&self) -> anyhow::Result<()> {
        {
            let mut state = self.state.write().await;
            if *state != ServerState::Stopped {
                tracing::warn!("WebSocket server is already in state: {:?}", *state);
                return Err(anyhow::anyhow!("Server is already running or starting"));
            }
            *state = ServerState::Starting;
        }

        self.shutdown.store(false, Ordering::SeqCst);

        // ポートを順番に試行
        let port_range_end = self.preferred_port.saturating_add(DEFAULT_PORT_END - DEFAULT_PORT_START);
        let (listener, bound_port) = self.try_bind_ports(self.preferred_port, port_range_end).await?;

        // 実際に使用するポートを記録
        {
            let mut actual = self.actual_port.write().await;
            *actual = Some(bound_port);
        }

        let addr = format!("127.0.0.1:{}", bound_port);
        if bound_port != self.preferred_port {
            tracing::info!(
                "🌐 WebSocket server listening on ws://{} (preferred port {} was unavailable)",
                addr,
                self.preferred_port
            );
        } else {
            tracing::info!("🌐 WebSocket server listening on ws://{}", addr);
        }

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

    /// 指定範囲のポートを順番に試行してバインド
    async fn try_bind_ports(
        &self,
        start_port: u16,
        end_port: u16,
    ) -> anyhow::Result<(TcpListener, u16)> {
        let mut last_error = None;

        for port in start_port..=end_port {
            let addr = format!("127.0.0.1:{}", port);
            tracing::debug!("Attempting to bind WebSocket server to {}", addr);

            match TcpListener::bind(&addr).await {
                Ok(listener) => {
                    tracing::debug!("Successfully bound to {}", addr);
                    return Ok((listener, port));
                }
                Err(e) => {
                    tracing::debug!("Port {} unavailable: {}", port, e);
                    last_error = Some(e);
                }
            }
        }

        // すべてのポートが使用中
        let err = last_error.unwrap_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::AddrInUse, "No ports available")
        });
        tracing::error!(
            "❌ Failed to bind WebSocket server to any port in range {}-{}: {}",
            start_port,
            end_port,
            err
        );

        let mut state = self.state.write().await;
        *state = ServerState::Stopped;

        Err(anyhow::anyhow!(
            "Failed to bind to any port in range {}-{}: {}",
            start_port,
            end_port,
            err
        ))
    }

    /// サーバーを停止
    pub async fn stop(&self) {
        tracing::info!("🛑 Stopping WebSocket server...");

        {
            let mut state = self.state.write().await;
            *state = ServerState::Stopping;
        }

        self.shutdown.store(true, Ordering::SeqCst);

        // 実際に使用中のポートをクリア
        {
            let mut actual = self.actual_port.write().await;
            *actual = None;
        }

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

    /// 希望ポート番号を取得
    pub fn preferred_port(&self) -> u16 {
        self.preferred_port
    }

    /// 実際に使用中のポート番号を取得
    ///
    /// サーバーが起動していない場合はNoneを返す
    pub async fn actual_port(&self) -> Option<u16> {
        *self.actual_port.read().await
    }

    /// 後方互換性のため：実際のポートまたは希望ポートを返す
    #[deprecated(note = "Use actual_port() or preferred_port() instead")]
    pub fn port(&self) -> u16 {
        self.preferred_port
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

    /// WebSocketサーバーの起動テスト
    #[tokio::test]
    async fn test_server_start_and_stop() {
        // ランダムポートでサーバーを作成（0を指定するとOSがポートを割り当て）
        // ただし、実際にはポート0ではバインドできないため、未使用ポートを探す
        let port = find_available_port().await.expect("No available port found");
        let server = WebSocketServer::new(port);

        // サーバー起動
        let result = server.start().await;
        assert!(result.is_ok(), "Server should start successfully: {:?}", result);

        // 状態がRunningになるまで少し待つ
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert_eq!(server.get_state().await, ServerState::Running);

        // サーバー停止
        server.stop().await;

        // 停止処理が完了するまで待つ
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        assert_eq!(server.get_state().await, ServerState::Stopped);
    }

    /// WebSocketクライアント接続テスト
    #[tokio::test]
    async fn test_client_connection() {
        let port = find_available_port().await.expect("No available port found");
        let server = WebSocketServer::new(port);
        server.start().await.expect("Server should start");

        // サーバーが起動するまで待つ
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // クライアント接続
        let url = format!("ws://127.0.0.1:{}", port);
        let connect_result = tokio_tungstenite::connect_async(&url).await;

        assert!(
            connect_result.is_ok(),
            "Client should connect successfully: {:?}",
            connect_result.err()
        );

        let (ws_stream, _response) = connect_result.unwrap();
        let (mut _write, mut read) = ws_stream.split();

        // 接続確認メッセージを受信
        let msg = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            read.next()
        ).await;

        assert!(msg.is_ok(), "Should receive message within timeout");
        let msg = msg.unwrap();
        assert!(msg.is_some(), "Should receive a message");

        if let Some(Ok(Message::Text(text))) = msg {
            let server_msg: Result<ServerMessage, _> = serde_json::from_str(&text);
            assert!(server_msg.is_ok(), "Should deserialize ServerMessage");
            if let Ok(ServerMessage::Connected { client_id }) = server_msg {
                assert!(client_id > 0, "Client ID should be positive");
            } else {
                panic!("Expected Connected message, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message, got: {:?}", msg);
        }

        // クリーンアップ
        server.stop().await;
    }

    /// メッセージブロードキャストテスト
    #[tokio::test]
    async fn test_message_broadcast() {
        let port = find_available_port().await.expect("No available port found");
        let server = WebSocketServer::new(port);
        server.start().await.expect("Server should start");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // クライアント接続
        let url = format!("ws://127.0.0.1:{}", port);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("Client should connect");

        let (_write, mut read) = ws_stream.split();

        // 接続確認メッセージをスキップ
        let _ = read.next().await;

        // 接続が安定するまで少し待つ
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // テストメッセージを作成
        let test_message = GuiChatMessage {
            id: "test_123".to_string(),
            timestamp: "12:34:56".to_string(),
            timestamp_usec: "1234567890".to_string(),
            message_type: crate::gui::models::MessageType::Text,
            author: "TestUser".to_string(),
            author_icon_url: None,
            channel_id: "UC123".to_string(),
            content: "Hello, WebSocket!".to_string(),
            runs: vec![],
            metadata: None,
            is_member: false,
            comment_count: None,
        };

        // メッセージをブロードキャスト
        server.broadcast_message(&test_message).await;

        // ブロードキャストメッセージを受信
        let msg = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            read.next()
        ).await;

        assert!(msg.is_ok(), "Should receive broadcast within timeout");
        let msg = msg.unwrap();
        assert!(msg.is_some(), "Should receive a broadcast message");

        if let Some(Ok(Message::Text(text))) = msg {
            let server_msg: Result<ServerMessage, _> = serde_json::from_str(&text);
            assert!(server_msg.is_ok(), "Should deserialize ServerMessage: {}", text);
            if let Ok(ServerMessage::ChatMessage(received_msg)) = server_msg {
                assert_eq!(received_msg.id, "test_123");
                assert_eq!(received_msg.author, "TestUser");
                assert_eq!(received_msg.content, "Hello, WebSocket!");
            } else {
                panic!("Expected ChatMessage, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message, got: {:?}", msg);
        }

        server.stop().await;
    }

    /// Ping/Pongテスト
    #[tokio::test]
    async fn test_ping_pong() {
        let port = find_available_port().await.expect("No available port found");
        let server = WebSocketServer::new(port);
        server.start().await.expect("Server should start");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let url = format!("ws://127.0.0.1:{}", port);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("Client should connect");

        let (mut write, mut read) = ws_stream.split();

        // 接続確認メッセージをスキップ
        let _ = read.next().await;

        // Pingメッセージを送信
        let ping_msg = ClientMessage::Ping;
        let ping_json = serde_json::to_string(&ping_msg).unwrap();
        write.send(Message::Text(ping_json.into())).await.expect("Should send ping");

        // Pongを受信（サーバーはPongを返す）
        let msg = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            read.next()
        ).await;

        assert!(msg.is_ok(), "Should receive pong within timeout");
        let msg = msg.unwrap();
        assert!(msg.is_some(), "Should receive a pong message");

        // Pongメッセージを確認
        if let Some(Ok(Message::Pong(_))) = msg {
            // OK
        } else {
            panic!("Expected Pong message, got: {:?}", msg);
        }

        server.stop().await;
    }

    /// GetInfoリクエストテスト
    #[tokio::test]
    async fn test_get_info() {
        let port = find_available_port().await.expect("No available port found");
        let server = WebSocketServer::new(port);
        server.start().await.expect("Server should start");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let url = format!("ws://127.0.0.1:{}", port);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("Client should connect");

        let (mut write, mut read) = ws_stream.split();

        // 接続確認メッセージをスキップ
        let _ = read.next().await;

        // GetInfoリクエストを送信
        let get_info_msg = ClientMessage::GetInfo;
        let json = serde_json::to_string(&get_info_msg).unwrap();
        write.send(Message::Text(json.into())).await.expect("Should send GetInfo");

        // ServerInfoレスポンスを受信
        let msg = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            read.next()
        ).await;

        assert!(msg.is_ok(), "Should receive response within timeout");
        let msg = msg.unwrap();
        assert!(msg.is_some(), "Should receive a response");

        if let Some(Ok(Message::Text(text))) = msg {
            let server_msg: Result<ServerMessage, _> = serde_json::from_str(&text);
            assert!(server_msg.is_ok(), "Should deserialize ServerMessage");
            if let Ok(ServerMessage::ServerInfo { version, connected_clients }) = server_msg {
                assert!(!version.is_empty(), "Version should not be empty");
                assert!(connected_clients >= 1, "Should have at least 1 connected client");
            } else {
                panic!("Expected ServerInfo, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message, got: {:?}", msg);
        }

        server.stop().await;
    }

    /// 複数クライアント接続テスト
    #[tokio::test]
    async fn test_multiple_clients() {
        let port = find_available_port().await.expect("No available port found");
        let server = WebSocketServer::new(port);
        server.start().await.expect("Server should start");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let url = format!("ws://127.0.0.1:{}", port);

        // 3つのクライアントを接続
        let mut clients = Vec::new();
        for _ in 0..3 {
            let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
                .await
                .expect("Client should connect");
            clients.push(ws_stream);
        }

        // 接続数を確認
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let client_count = server.connected_clients().await;
        assert_eq!(client_count, 3, "Should have 3 connected clients");

        // クライアントを切断
        for client in clients {
            drop(client);
        }

        // 切断処理が完了するまで待つ
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let client_count = server.connected_clients().await;
        assert_eq!(client_count, 0, "All clients should be disconnected");

        server.stop().await;
    }

    /// 利用可能なポートを見つけるヘルパー関数
    async fn find_available_port() -> Option<u16> {
        for port in 49152..65535 {
            if let Ok(listener) = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                drop(listener);
                return Some(port);
            }
        }
        None
    }

    /// 自動ポート選択テスト：希望ポートが使用中の場合、次のポートを使用
    #[tokio::test]
    async fn test_auto_port_selection() {
        // まず最初のポートを占有
        let base_port = find_available_port().await.expect("No available port found");
        let _blocker = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", base_port))
            .await
            .expect("Should bind to base port");

        // 同じポートでサーバーを起動 → 自動的に次のポートを使用するはず
        let server = WebSocketServer::new(base_port);
        let result = server.start().await;
        assert!(result.is_ok(), "Server should start on alternative port");

        // サーバーが起動するまで待つ
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 実際に使用中のポートを確認
        let actual = server.actual_port().await;
        assert!(actual.is_some(), "Should have actual port set");
        let actual_port = actual.unwrap();

        // 実際のポートは希望ポートとは異なるはず（希望ポートは占有済み）
        assert_ne!(
            actual_port, base_port,
            "Should use different port than preferred"
        );
        // 実際のポートは希望ポートより大きい（次の利用可能なポートを使用）
        assert!(
            actual_port > base_port,
            "Should use a port greater than preferred: actual={}, preferred={}",
            actual_port,
            base_port
        );

        // 希望ポートは変わらない
        assert_eq!(server.preferred_port(), base_port);

        // クライアントが実際のポートに接続できることを確認
        let url = format!("ws://127.0.0.1:{}", actual_port);
        let connect_result = tokio_tungstenite::connect_async(&url).await;
        assert!(
            connect_result.is_ok(),
            "Client should connect to actual port"
        );

        server.stop().await;
    }

    /// actual_port()がサーバー停止後にNoneを返すことをテスト
    #[tokio::test]
    async fn test_actual_port_cleared_on_stop() {
        let port = find_available_port().await.expect("No available port found");
        let server = WebSocketServer::new(port);

        // 起動前はNone
        assert!(
            server.actual_port().await.is_none(),
            "actual_port should be None before start"
        );

        server.start().await.expect("Server should start");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 起動後はSome
        assert!(
            server.actual_port().await.is_some(),
            "actual_port should be Some after start"
        );

        server.stop().await;

        // 停止後はNone
        assert!(
            server.actual_port().await.is_none(),
            "actual_port should be None after stop"
        );
    }
}
