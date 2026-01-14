//! WebSocket server for external app integration

use crate::core::models::ChatMessage;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;

type ClientId = u64;

/// Server to client message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    ChatMessage(ChatMessage),
    Connected { client_id: ClientId },
    ServerInfo { version: String, connected_clients: usize },
    Error { message: String },
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
    next_client_id: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
}

impl WebSocketServer {
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

    pub async fn start(&self) -> anyhow::Result<u16> {
        {
            let mut state = self.state.write().await;
            if *state != ServerState::Stopped {
                return Err(anyhow::anyhow!("Server is already running"));
            }
            *state = ServerState::Starting;
        }

        self.shutdown.store(false, Ordering::SeqCst);

        // Try to bind to ports
        let port_range_end = self.preferred_port.saturating_add(10);
        let (listener, bound_port) = self.try_bind_ports(self.preferred_port, port_range_end).await?;

        {
            let mut actual = self.actual_port.write().await;
            *actual = Some(bound_port);
        }

        tracing::info!("WebSocket server listening on ws://127.0.0.1:{}", bound_port);

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
                                let clients = Arc::clone(&clients);
                                let mut message_rx = message_tx.subscribe();

                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, addr, client_id, clients, &mut message_rx).await {
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

        Ok(bound_port)
    }

    async fn try_bind_ports(&self, start_port: u16, end_port: u16) -> anyhow::Result<(TcpListener, u16)> {
        for port in start_port..=end_port {
            let addr = format!("127.0.0.1:{}", port);
            match TcpListener::bind(&addr).await {
                Ok(listener) => return Ok((listener, port)),
                Err(_) => continue,
            }
        }
        Err(anyhow::anyhow!("No available ports in range {}-{}", start_port, end_port))
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

        let _ = self.message_tx.send(server_msg.clone());

        let json = match serde_json::to_string(&server_msg) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Failed to serialize message: {}", e);
                return;
            }
        };

        let clients = self.clients.read().await;
        for (_, sender) in clients.iter() {
            let _ = sender.send(Message::Text(json.clone()));
        }
    }

    pub async fn connected_clients(&self) -> usize {
        self.clients.read().await.len()
    }

    pub async fn get_state(&self) -> ServerState {
        self.state.read().await.clone()
    }

    pub async fn is_running(&self) -> bool {
        *self.state.read().await == ServerState::Running
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
) -> anyhow::Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    {
        let mut clients_guard = clients.write().await;
        clients_guard.insert(client_id, tx);
    }

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

    Ok(())
}
