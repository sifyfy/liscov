//! WebSocket server for external app integration

use crate::core::models::{ChatMessage, ReactionUpdate};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

type ClientId = u64;

/// Client connection event for external notification
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Connected { client_id: u64 },
    Disconnected { client_id: u64 },
}

/// Server to client message（サーバーから送るだけなので Deserialize は持たない）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
#[allow(clippy::large_enum_variant)]
pub enum ServerMessage {
    ChatMessage(ChatMessage),
    /// ライブリアクションの更新（前回からの差分）
    Reaction {
        /// 多接続時にどの配信のリアクションかを区別する
        broadcaster_channel_id: Option<String>,
        #[serde(flatten)]
        update: ReactionUpdate,
    },
    Connected {
        client_id: ClientId,
    },
    ServerInfo {
        version: String,
        connected_clients: u32,
    },
    Error {
        message: String,
    },
}

/// Client to server message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Ping,
    GetInfo,
}

/// Server state
#[derive(Debug, Clone, PartialEq)]
pub enum ServerState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// WebSocket server
pub struct WebSocketServer {
    preferred_port: u16,
    actual_port: Arc<RwLock<Option<u16>>>,
    state: Arc<RwLock<ServerState>>,
    clients: Arc<RwLock<HashMap<ClientId, tokio::sync::mpsc::UnboundedSender<Message>>>>,
    message_tx: broadcast::Sender<ServerMessage>,
    client_event_tx: broadcast::Sender<ClientEvent>,
    next_client_id: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    /// 接続を受け付けるタスク。異常終了すると状態の旗は Running のまま残るので、終わっていないかも見る
    accept_task: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl WebSocketServer {
    pub fn new(port: u16) -> Self {
        let (message_tx, _) = broadcast::channel(1024);
        let (client_event_tx, _) = broadcast::channel(64);
        Self {
            preferred_port: port,
            actual_port: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(ServerState::Stopped)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            client_event_tx,
            next_client_id: Arc::new(AtomicU64::new(1)),
            shutdown: Arc::new(AtomicBool::new(false)),
            accept_task: std::sync::Mutex::new(None),
        }
    }

    /// Subscribe to client events (connect/disconnect)
    pub fn subscribe_events(&self) -> broadcast::Receiver<ClientEvent> {
        self.client_event_tx.subscribe()
    }

    pub async fn start(&self) -> anyhow::Result<u16> {
        {
            let mut state = self.state.write().await;
            if *state != ServerState::Stopped {
                return Err(anyhow::anyhow!("Server is already running"));
            }
            *state = ServerState::Starting;
        }

        self.shutdown.store(false, Ordering::SeqCst);

        // 開始ポートから 10 ポートを試す (spec: 8765-8774)。0 は OS に選ばせるので 1 回だけ
        let port_range_end = if self.preferred_port == 0 {
            0
        } else {
            self.preferred_port.saturating_add(9)
        };
        let (listener, bound_port) = self
            .try_bind_ports(self.preferred_port, port_range_end)
            .await?;

        {
            let mut actual = self.actual_port.write().await;
            *actual = Some(bound_port);
        }

        tracing::info!(
            "WebSocket server listening on ws://127.0.0.1:{}",
            bound_port
        );

        {
            let mut state = self.state.write().await;
            *state = ServerState::Running;
        }

        let clients = Arc::clone(&self.clients);
        let message_tx = self.message_tx.clone();
        let client_event_tx = self.client_event_tx.clone();
        let next_client_id = Arc::clone(&self.next_client_id);
        let shutdown = Arc::clone(&self.shutdown);
        let state = Arc::clone(&self.state);

        let accept_task = tokio::spawn(async move {
            while !shutdown.load(Ordering::SeqCst) {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                let client_id = next_client_id.fetch_add(1, Ordering::SeqCst);
                                let clients = Arc::clone(&clients);
                                let mut message_rx = message_tx.subscribe();
                                let event_tx = client_event_tx.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, addr, client_id, clients, &mut message_rx, event_tx).await {
                                        tracing::warn!("WebSocket error for client {}: {}", client_id, e);
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
                }
            }

            let mut state_guard = state.write().await;
            *state_guard = ServerState::Stopped;
        });
        *self
            .accept_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(accept_task);

        Ok(bound_port)
    }

    async fn try_bind_ports(
        &self,
        start_port: u16,
        end_port: u16,
    ) -> anyhow::Result<(TcpListener, u16)> {
        for port in start_port..=end_port {
            let addr = format!("127.0.0.1:{}", port);
            let Ok(listener) = TcpListener::bind(&addr).await else {
                continue;
            };
            // ポート 0 では OS が選んだ番号になるので、bind した結果から読む
            let bound_port = listener.local_addr()?.port();
            return Ok((listener, bound_port));
        }
        Err(anyhow::anyhow!(
            "No available ports in range {}-{}",
            start_port,
            end_port
        ))
    }

    pub async fn stop(&self) {
        {
            let mut state = self.state.write().await;
            *state = ServerState::Stopping;
        }

        self.shutdown.store(true, Ordering::SeqCst);

        {
            let mut actual = self.actual_port.write().await;
            *actual = None;
        }

        let mut clients = self.clients.write().await;
        clients.clear();
    }

    pub async fn broadcast_message(&self, message: &ChatMessage) {
        let server_msg = ServerMessage::ChatMessage(message.clone());
        let _ = self.message_tx.send(server_msg);
    }

    pub async fn broadcast_reaction(
        &self,
        broadcaster_channel_id: Option<&str>,
        update: &ReactionUpdate,
    ) {
        let _ = self.message_tx.send(ServerMessage::Reaction {
            broadcaster_channel_id: broadcaster_channel_id.map(str::to_string),
            update: update.clone(),
        });
    }

    pub async fn connected_clients(&self) -> u32 {
        self.clients.read().await.len() as u32
    }

    pub async fn get_state(&self) -> ServerState {
        self.state.read().await.clone()
    }

    /// 動いているか (03_websocket.md「サーバーライフサイクル」)。
    /// 状態の旗が Running でも、受け付けのタスクが終わっていたら (異常終了) 動いていない
    pub async fn is_running(&self) -> bool {
        let accepting = self
            .accept_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .is_some_and(|task| !task.is_finished());
        accepting && *self.state.read().await == ServerState::Running
    }

    /// テスト用: 受け付けのタスクを、状態の旗を戻さずに終わらせる (異常終了の代わり)
    #[cfg(test)]
    async fn abort_accept_task(&self) {
        let finished = || {
            let guard = self.accept_task.lock().unwrap();
            let task = guard.as_ref().expect("起動している");
            task.abort();
            task.is_finished()
        };
        // 取り消しはランタイムが進めるので、制御を返しながら待つ (ロックは await をまたがない)
        while !finished() {
            tokio::task::yield_now().await;
        }
    }

    pub async fn actual_port(&self) -> Option<u16> {
        *self.actual_port.read().await
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    client_id: ClientId,
    clients: Arc<RwLock<HashMap<ClientId, tokio::sync::mpsc::UnboundedSender<Message>>>>,
    message_rx: &mut broadcast::Receiver<ServerMessage>,
    event_tx: broadcast::Sender<ClientEvent>,
) -> anyhow::Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    {
        let mut clients_guard = clients.write().await;
        clients_guard.insert(client_id, tx);
    }

    // Emit client connected event
    let _ = event_tx.send(ClientEvent::Connected { client_id });

    let connected_msg = ServerMessage::Connected { client_id };
    let json = serde_json::to_string(&connected_msg)?;
    write.send(Message::Text(json)).await?;

    tracing::info!("Client {} connected from {}", client_id, addr);

    loop {
        tokio::select! {
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
                                        connected_clients: clients_guard.len() as u32,
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
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::warn!("WebSocket error for client {}: {}", client_id, e);
                        break;
                    }
                    _ => {}
                }
            }
            msg = message_rx.recv() => {
                if let Ok(server_msg) = msg {
                    let json = serde_json::to_string(&server_msg)?;
                    if write.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
            msg = rx.recv() => {
                if let Some(message) = msg {
                    if write.send(message).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    {
        let mut clients_guard = clients.write().await;
        clients_guard.remove(&client_id);
    }

    // Emit client disconnected event
    let _ = event_tx.send(ClientEvent::Disconnected { client_id });
    tracing::info!("Client {} disconnected", client_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 03_websocket.md「サーバーライフサイクル」: 起動した後に受け付けが止まったら、動いていないと返す
    #[tokio::test]
    async fn 受け付けのタスクが状態を戻さずに終わったら動いていないと返す() {
        let server = WebSocketServer::new(0);
        server.start().await.unwrap();
        assert!(server.is_running().await);

        // 異常終了 (panic) の代わりに、受け付けのタスクを外から止める。どちらも状態の旗は Running のまま残る
        server.abort_accept_task().await;

        assert!(!server.is_running().await);
    }

    #[tokio::test]
    async fn 止めたら動いていないと返す() {
        let server = WebSocketServer::new(0);
        server.start().await.unwrap();
        server.stop().await;
        assert!(!server.is_running().await);
    }
}
