//! Integration tests for WebSocket API (03_websocket.md)
//!
//! Tests verify the WebSocket server behavior:
//! - Connected message format
//! - Ping/Pong response
//! - GetInfo/ServerInfo response
//! - Multiple client connections
//! - ChatMessage broadcast

use app_lib::core::api::{ClientEvent, WebSocketServer};
use app_lib::core::models::ChatMessage;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helper to find an available port for testing
async fn get_test_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Helper to connect to WebSocket server
async fn connect_client(
    port: u16,
) -> (
    futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) {
    let url = format!("ws://127.0.0.1:{}", port);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    ws_stream.split()
}

/// Parse ServerMessage from WebSocket text message
fn parse_server_message(msg: &Message) -> Option<Value> {
    if let Message::Text(text) = msg {
        serde_json::from_str(text).ok()
    } else {
        None
    }
}

// ============================================================================
// Connected Message Tests
// ============================================================================

#[tokio::test]
async fn test_connected_message_sent_on_connection() {
    // Spec: 接続確立時に Connected メッセージを送信
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start server");

    let (mut _write, mut read) = connect_client(actual_port).await;

    // Should receive Connected message
    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for message")
        .expect("Stream ended")
        .expect("WebSocket error");

    let json = parse_server_message(&msg).expect("Failed to parse message");

    // Spec: { "type": "Connected", "data": { "client_id": <u64> } }
    assert_eq!(json["type"], "Connected");
    assert!(json["data"]["client_id"].is_u64());

    server.stop().await;
}

#[tokio::test]
async fn test_connected_message_has_unique_client_id() {
    // Spec: 各クライアントに一意のIDを割り当て
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start server");

    // Connect first client
    let (mut _write1, mut read1) = connect_client(actual_port).await;
    let msg1 = timeout(Duration::from_secs(5), read1.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let json1 = parse_server_message(&msg1).unwrap();
    let client_id_1 = json1["data"]["client_id"].as_u64().unwrap();

    // Connect second client
    let (mut _write2, mut read2) = connect_client(actual_port).await;
    let msg2 = timeout(Duration::from_secs(5), read2.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let json2 = parse_server_message(&msg2).unwrap();
    let client_id_2 = json2["data"]["client_id"].as_u64().unwrap();

    // IDs should be different
    assert_ne!(client_id_1, client_id_2);

    server.stop().await;
}

// ============================================================================
// Ping/Pong Tests
// ============================================================================

#[tokio::test]
async fn test_ping_json_receives_pong_frame() {
    // Spec: ClientMessage::Ping → サーバーは Pong フレームで応答
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start server");

    let (mut write, mut read) = connect_client(actual_port).await;

    // Consume Connected message
    let _ = read.next().await;

    // Send Ping JSON message
    write
        .send(Message::Text(r#"{"type":"Ping"}"#.to_string()))
        .await
        .expect("Failed to send");

    // Should receive Pong frame
    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout")
        .expect("Stream ended")
        .expect("WebSocket error");

    assert!(matches!(msg, Message::Pong(_)));

    server.stop().await;
}

#[tokio::test]
async fn test_websocket_ping_frame_receives_pong() {
    // WebSocket protocol: Ping frame → Pong frame
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start server");

    let (mut write, mut read) = connect_client(actual_port).await;

    // Consume Connected message
    let _ = read.next().await;

    // Send WebSocket Ping frame
    write
        .send(Message::Ping(vec![1, 2, 3]))
        .await
        .expect("Failed to send");

    // Should receive Pong frame with same data
    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Pong(data) = msg {
        assert_eq!(data, vec![1, 2, 3]);
    } else {
        panic!("Expected Pong frame, got {:?}", msg);
    }

    server.stop().await;
}

// ============================================================================
// GetInfo/ServerInfo Tests
// ============================================================================

#[tokio::test]
async fn test_getinfo_returns_server_info() {
    // Spec: GetInfo → ServerInfo { version, connected_clients }
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start server");

    let (mut write, mut read) = connect_client(actual_port).await;

    // Consume Connected message
    let _ = read.next().await;

    // Send GetInfo request
    write
        .send(Message::Text(r#"{"type":"GetInfo"}"#.to_string()))
        .await
        .expect("Failed to send");

    // Should receive ServerInfo response
    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout")
        .expect("Stream ended")
        .expect("WebSocket error");

    let json = parse_server_message(&msg).expect("Failed to parse");

    // Spec: { "type": "ServerInfo", "data": { "version": "...", "connected_clients": <u32> } }
    assert_eq!(json["type"], "ServerInfo");
    assert!(json["data"]["version"].is_string());
    assert!(
        json["data"]["connected_clients"].is_u64(),
        "connected_clients should be a number"
    );

    server.stop().await;
}

#[tokio::test]
async fn test_getinfo_returns_correct_client_count() {
    // Spec: connected_clients は現在接続中のクライアント数
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start server");

    // Connect 3 clients
    let (mut write1, mut read1) = connect_client(actual_port).await;
    let _ = read1.next().await; // Consume Connected

    let (_write2, mut read2) = connect_client(actual_port).await;
    let _ = read2.next().await;

    let (_write3, mut read3) = connect_client(actual_port).await;
    let _ = read3.next().await;

    // Small delay to ensure all connections are registered
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Request info from first client
    write1
        .send(Message::Text(r#"{"type":"GetInfo"}"#.to_string()))
        .await
        .unwrap();

    let msg = timeout(Duration::from_secs(5), read1.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let json = parse_server_message(&msg).unwrap();
    let count = json["data"]["connected_clients"].as_u64().unwrap();

    assert_eq!(count, 3, "Should have 3 connected clients");

    server.stop().await;
}

// ============================================================================
// Multiple Clients Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_clients_can_connect() {
    // Spec: 複数クライアントの同時接続をサポート
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start server");

    // Connect 5 clients
    let mut clients = Vec::new();
    for _ in 0..5 {
        let (write, mut read) = connect_client(actual_port).await;
        // Each should receive Connected message
        let msg = timeout(Duration::from_secs(5), read.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let json = parse_server_message(&msg).unwrap();
        assert_eq!(json["type"], "Connected");
        clients.push((write, read));
    }

    assert_eq!(server.connected_clients().await, 5);

    server.stop().await;
}

// ============================================================================
// Client Event Tests
// ============================================================================

#[tokio::test]
async fn test_client_connected_event_emitted() {
    // Spec: ClientEvent::Connected が発火される
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);

    // Subscribe to events before starting
    let mut event_rx = server.subscribe_events();

    let actual_port = server.start().await.expect("Failed to start server");

    // Connect a client
    let (_write, mut read) = connect_client(actual_port).await;
    let _ = read.next().await; // Consume Connected message

    // Should receive Connected event
    let event = timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel closed");

    match event {
        ClientEvent::Connected { client_id } => {
            assert!(client_id > 0);
        }
        _ => panic!("Expected Connected event"),
    }

    server.stop().await;
}

#[tokio::test]
async fn test_client_disconnected_event_emitted() {
    // Spec: ClientEvent::Disconnected が発火される
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);

    let mut event_rx = server.subscribe_events();

    let actual_port = server.start().await.expect("Failed to start server");

    // Connect a client
    let (mut write, mut read) = connect_client(actual_port).await;
    let _ = read.next().await;

    // Consume Connected event
    let _ = event_rx.recv().await;

    // Close the connection
    write.close().await.expect("Failed to close");

    // Should receive Disconnected event
    let event = timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel closed");

    match event {
        ClientEvent::Disconnected { client_id } => {
            assert!(client_id > 0);
        }
        _ => panic!("Expected Disconnected event"),
    }

    server.stop().await;
}

// ============================================================================
// Port Range Tests
// ============================================================================

#[tokio::test]
async fn test_port_fallback_when_occupied() {
    // Spec: 指定ポートが使用中の場合、自動的に次のポートを試行
    let base_port = get_test_port().await;

    // Occupy the base port
    let blocker = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", base_port))
        .await
        .unwrap();

    let server = WebSocketServer::new(base_port);
    let actual_port = server.start().await.expect("Failed to start server");

    // Should have fallen back to next port
    assert_ne!(actual_port, base_port);
    assert!(actual_port > base_port);
    assert!(actual_port <= base_port + 9); // Within 10-port range

    server.stop().await;
    drop(blocker);
}

#[tokio::test]
async fn test_server_reports_actual_port() {
    // Spec: actual_port を正しく報告
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);

    assert!(server.actual_port().await.is_none()); // Not started yet

    let actual_port = server.start().await.expect("Failed to start server");

    assert_eq!(server.actual_port().await, Some(actual_port));

    server.stop().await;

    // After stop, actual_port should be None
    assert!(server.actual_port().await.is_none());
}

// ============================================================================
// Server State Tests
// ============================================================================

#[tokio::test]
async fn test_server_state_transitions() {
    // Spec: ServerState: Stopped → Starting → Running → Stopping → Stopped
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);

    assert!(!server.is_running().await);

    server.start().await.expect("Failed to start");

    assert!(server.is_running().await);

    server.stop().await;

    // Give time for state transition
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(!server.is_running().await);
}

#[tokio::test]
async fn test_connected_clients_count() {
    // Spec: connected_clients メソッドが正しいカウントを返す
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    assert_eq!(server.connected_clients().await, 0);

    // Connect first client
    let (_w1, mut r1) = connect_client(actual_port).await;
    let _ = r1.next().await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(server.connected_clients().await, 1);

    // Connect second client
    let (_w2, mut r2) = connect_client(actual_port).await;
    let _ = r2.next().await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(server.connected_clients().await, 2);

    server.stop().await;
}

// ============================================================================
// Invalid Message Handling Tests
// ============================================================================

#[tokio::test]
async fn test_invalid_json_does_not_crash_server() {
    // Spec: 不正なメッセージ受信 → 警告ログ、接続は維持
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    let (mut write, mut read) = connect_client(actual_port).await;
    let _ = read.next().await; // Consume Connected

    // Send invalid JSON
    write
        .send(Message::Text("not valid json".to_string()))
        .await
        .expect("Failed to send");

    // Send valid GetInfo to verify connection still works
    write
        .send(Message::Text(r#"{"type":"GetInfo"}"#.to_string()))
        .await
        .expect("Failed to send");

    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout - connection may have been dropped")
        .expect("Stream ended")
        .expect("WebSocket error");

    let json = parse_server_message(&msg).unwrap();
    assert_eq!(json["type"], "ServerInfo");

    server.stop().await;
}

#[tokio::test]
async fn test_unknown_message_type_does_not_crash() {
    // Unknown message types should be ignored
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    let (mut write, mut read) = connect_client(actual_port).await;
    let _ = read.next().await;

    // Send unknown message type
    write
        .send(Message::Text(r#"{"type":"UnknownType"}"#.to_string()))
        .await
        .expect("Failed to send");

    // Verify connection still works
    write
        .send(Message::Text(r#"{"type":"GetInfo"}"#.to_string()))
        .await
        .expect("Failed to send");

    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout")
        .expect("Stream ended")
        .expect("WebSocket error");

    let json = parse_server_message(&msg).unwrap();
    assert_eq!(json["type"], "ServerInfo");

    server.stop().await;
}

// ============================================================================
// Message Format Tests (03_websocket.md compliance)
// ============================================================================

#[tokio::test]
async fn test_chatmessage_json_format_matches_spec() {
    // Spec (03_websocket.md):
    // {
    //   "type": "ChatMessage",
    //   "data": {
    //     "id": "...",
    //     "timestamp": "12:34:56",
    //     "timestamp_usec": "1234567890000000",
    //     "message_type": "Text",
    //     "author": "視聴者名",
    //     "author_icon_url": "https://...",
    //     "channel_id": "UC...",
    //     "content": "こんにちは！",
    //     "runs": [{ "type": "text", "text": "こんにちは！" }],
    //     "metadata": { "amount": null, "badges": [...], "is_moderator": false, "is_verified": false },
    //     "is_member": true,
    //     "is_first_time_viewer": false,
    //     "in_stream_comment_count": 5
    //   }
    // }

    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    let (_write, mut read) = connect_client(actual_port).await;
    let _ = read.next().await; // Consume Connected

    // Create a message with runs to test the format
    let test_msg = ChatMessage {
        id: "test-id-001".to_string(),
        timestamp: "12:34:56".to_string(),
        timestamp_usec: "1234567890000000".to_string(),
        message_type: app_lib::core::models::MessageType::Text,
        author: "TestAuthor".to_string(),
        author_icon_url: Some("https://example.com/icon.png".to_string()),
        channel_id: "UCtest123".to_string(),
        content: "Hello World".to_string(),
        runs: vec![
            app_lib::core::models::MessageRun::Text {
                content: "Hello World".to_string(),
            },
        ],
        metadata: Some(app_lib::core::models::MessageMetadata {
            amount: None,
            badges: vec!["Member".to_string()],
            badge_info: vec![],
            color: None,
            is_moderator: false,
            is_verified: true,
            superchat_colors: None,
        }),
        is_member: true,
        is_first_time_viewer: false,
        in_stream_comment_count: Some(5),
    };

    server.broadcast_message(&test_msg).await;

    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let json = parse_server_message(&msg).expect("Failed to parse");

    // Print actual JSON for debugging
    println!("Actual JSON: {}", serde_json::to_string_pretty(&json).unwrap());

    // Verify top-level structure
    assert_eq!(json["type"], "ChatMessage", "Top-level type should be 'ChatMessage'");
    assert!(json["data"].is_object(), "Should have 'data' field");

    let data = &json["data"];

    // Verify required fields exist with correct types
    assert_eq!(data["id"], "test-id-001");
    assert_eq!(data["timestamp"], "12:34:56");
    assert_eq!(data["timestamp_usec"], "1234567890000000");
    assert_eq!(data["author"], "TestAuthor");
    assert_eq!(data["channel_id"], "UCtest123");
    assert_eq!(data["content"], "Hello World");
    assert_eq!(data["is_member"], true);
    assert_eq!(data["is_first_time_viewer"], false);
    assert_eq!(data["in_stream_comment_count"], 5);

    // Verify message_type format: unit variant serializes to string "Text"
    assert_eq!(
        data["message_type"], "Text",
        "message_type should be a string 'Text' for unit variant"
    );

    // Verify runs format: [{"Text": {"content": "..."}}] (externally tagged enum)
    assert!(data["runs"].is_array(), "runs should be an array");
    let runs = data["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert!(
        runs[0]["Text"].is_object(),
        "runs[0] should have 'Text' key (externally tagged)"
    );
    assert_eq!(
        runs[0]["Text"]["content"], "Hello World",
        "Text variant should have 'content' field"
    );

    // Verify metadata format
    assert!(data["metadata"].is_object(), "metadata should be an object");
    let metadata = &data["metadata"];
    assert!(metadata["badges"].is_array());
    assert!(metadata["is_moderator"].is_boolean());
    assert!(metadata["is_verified"].is_boolean());

    server.stop().await;
}

#[tokio::test]
async fn test_connected_message_json_format_matches_spec() {
    // Spec: { "type": "Connected", "data": { "client_id": 1 } }
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    let (_write, mut read) = connect_client(actual_port).await;

    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let json = parse_server_message(&msg).expect("Failed to parse");

    println!("Connected message: {}", serde_json::to_string_pretty(&json).unwrap());

    // Verify exact format matches spec
    assert_eq!(json["type"], "Connected");
    assert!(json["data"].is_object());
    assert!(json["data"]["client_id"].is_u64());

    server.stop().await;
}

#[tokio::test]
async fn test_serverinfo_message_json_format_matches_spec() {
    // Spec: { "type": "ServerInfo", "data": { "version": "0.1.0", "connected_clients": 3 } }
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    let (mut write, mut read) = connect_client(actual_port).await;
    let _ = read.next().await; // Consume Connected

    write
        .send(Message::Text(r#"{"type":"GetInfo"}"#.to_string()))
        .await
        .unwrap();

    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let json = parse_server_message(&msg).expect("Failed to parse");

    println!("ServerInfo message: {}", serde_json::to_string_pretty(&json).unwrap());

    // Verify exact format matches spec
    assert_eq!(json["type"], "ServerInfo");
    assert!(json["data"].is_object());
    assert!(json["data"]["version"].is_string());
    assert!(json["data"]["connected_clients"].is_u64());

    server.stop().await;
}

// ============================================================================
// ChatMessage Broadcast Tests
// ============================================================================

/// Create a test ChatMessage
fn create_test_message(id: &str, author: &str, content: &str) -> ChatMessage {
    ChatMessage {
        id: id.to_string(),
        timestamp: "12:34:56".to_string(),
        timestamp_usec: "1234567890000000".to_string(),
        message_type: app_lib::core::models::MessageType::Text,
        author: author.to_string(),
        author_icon_url: Some("https://example.com/icon.jpg".to_string()),
        channel_id: "UCtest123".to_string(),
        content: content.to_string(),
        runs: vec![],
        metadata: None,
        is_member: false,
        is_first_time_viewer: false,
        in_stream_comment_count: Some(1),
    }
}

#[tokio::test]
async fn test_broadcast_message_to_single_client() {
    // Spec: チャットメッセージ受信時、接続中のすべてのクライアントに配信
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    let (_write, mut read) = connect_client(actual_port).await;
    let _ = read.next().await; // Consume Connected

    // Broadcast a chat message
    let test_msg = create_test_message("msg-001", "TestUser", "Hello World!");
    server.broadcast_message(&test_msg).await;

    // Client should receive the message
    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout")
        .expect("Stream ended")
        .expect("WebSocket error");

    let json = parse_server_message(&msg).expect("Failed to parse");

    // Spec: { "type": "ChatMessage", "data": { ... } }
    assert_eq!(json["type"], "ChatMessage");
    assert_eq!(json["data"]["id"], "msg-001");
    assert_eq!(json["data"]["author"], "TestUser");
    assert_eq!(json["data"]["content"], "Hello World!");

    server.stop().await;
}

#[tokio::test]
async fn test_broadcast_message_to_multiple_clients() {
    // Spec: すべてのクライアントにメッセージをブロードキャスト
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    // Connect 3 clients
    let (_w1, mut r1) = connect_client(actual_port).await;
    let _ = r1.next().await;
    let (_w2, mut r2) = connect_client(actual_port).await;
    let _ = r2.next().await;
    let (_w3, mut r3) = connect_client(actual_port).await;
    let _ = r3.next().await;

    // Broadcast a message
    let test_msg = create_test_message("msg-002", "Broadcaster", "Welcome!");
    server.broadcast_message(&test_msg).await;

    // All clients should receive the message
    for (i, read) in [&mut r1, &mut r2, &mut r3].iter_mut().enumerate() {
        let msg = timeout(Duration::from_secs(5), read.next())
            .await
            .expect(&format!("Timeout for client {}", i + 1))
            .expect("Stream ended")
            .expect("WebSocket error");

        let json = parse_server_message(&msg).expect("Failed to parse");
        assert_eq!(json["type"], "ChatMessage");
        assert_eq!(json["data"]["id"], "msg-002");
    }

    server.stop().await;
}

#[tokio::test]
async fn test_broadcast_preserves_message_content() {
    // Verify all message fields are preserved in broadcast
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    let (_write, mut read) = connect_client(actual_port).await;
    let _ = read.next().await;

    let test_msg = ChatMessage {
        id: "msg-preserve".to_string(),
        timestamp: "23:59:59".to_string(),
        timestamp_usec: "9999999999999999".to_string(),
        message_type: app_lib::core::models::MessageType::Text,
        author: "SpecialUser".to_string(),
        author_icon_url: Some("https://example.com/special.png".to_string()),
        channel_id: "UCspecial".to_string(),
        content: "特殊なメッセージ 🎉".to_string(),
        runs: vec![],
        metadata: None,
        is_member: true,
        is_first_time_viewer: false,
        in_stream_comment_count: Some(42),
    };
    server.broadcast_message(&test_msg).await;

    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let json = parse_server_message(&msg).unwrap();

    assert_eq!(json["data"]["id"], "msg-preserve");
    assert_eq!(json["data"]["timestamp"], "23:59:59");
    assert_eq!(json["data"]["timestamp_usec"], "9999999999999999");
    assert_eq!(json["data"]["author"], "SpecialUser");
    assert_eq!(json["data"]["channel_id"], "UCspecial");
    assert_eq!(json["data"]["content"], "特殊なメッセージ 🎉");
    assert_eq!(json["data"]["is_member"], true);
    assert_eq!(json["data"]["is_first_time_viewer"], false);
    assert_eq!(json["data"]["in_stream_comment_count"], 42);

    server.stop().await;
}

#[tokio::test]
async fn test_new_client_does_not_receive_past_messages() {
    // Spec: 過去メッセージは送信しない、接続後に受信したメッセージのみ配信
    let port = get_test_port().await;
    let server = WebSocketServer::new(port);
    let actual_port = server.start().await.expect("Failed to start");

    // First client connects
    let (_w1, mut r1) = connect_client(actual_port).await;
    let _ = r1.next().await;

    // Broadcast a message while only first client is connected
    let msg1 = create_test_message("old-msg", "OldUser", "Old message");
    server.broadcast_message(&msg1).await;

    // First client receives it
    let _ = r1.next().await;

    // Second client connects after the message
    let (_w2, mut r2) = connect_client(actual_port).await;

    // Second client should only get Connected, not the old message
    let msg = timeout(Duration::from_secs(2), r2.next())
        .await
        .expect("Timeout")
        .expect("Stream ended")
        .expect("WebSocket error");

    let json = parse_server_message(&msg).unwrap();
    assert_eq!(json["type"], "Connected", "New client should only receive Connected, not past messages");

    // Now broadcast a new message
    let msg2 = create_test_message("new-msg", "NewUser", "New message");
    server.broadcast_message(&msg2).await;

    // Second client should receive the new message
    let msg = timeout(Duration::from_secs(2), r2.next())
        .await
        .expect("Timeout")
        .expect("Stream ended")
        .expect("WebSocket error");

    let json = parse_server_message(&msg).unwrap();
    assert_eq!(json["type"], "ChatMessage");
    assert_eq!(json["data"]["id"], "new-msg");

    server.stop().await;
}
