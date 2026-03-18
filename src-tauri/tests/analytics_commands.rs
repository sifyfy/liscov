//! analytics コマンドの統合テスト (07_revenue.md)
//!
//! AppState を直接構築して Tauri IPC コマンドレイヤーをテストする。
//! ファイル書き込みテストは tempdir を使用し、本番データと分離する。

mod common;

use app_lib::commands::analytics::RevenueAnalytics;
use app_lib::core::{ChatMessage, MessageType};
use app_lib::state::AppState;
use common::{invoke_no_args, invoke_with_args};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets};
use tokio::sync::RwLock;

// ============================================================================
// テストヘルパー
// ============================================================================

/// テスト用 ChatMessage を生成するヘルパー
fn make_chat_message(id: &str, author: &str, channel_id: &str, message_type: MessageType) -> ChatMessage {
    ChatMessage {
        id: id.to_string(),
        author: author.to_string(),
        channel_id: channel_id.to_string(),
        message_type,
        ..Default::default()
    }
}

/// 指定メッセージを持つ AppState を直接構築する
fn build_app_state(messages: Vec<ChatMessage>) -> AppState {
    AppState {
        websocket_server: Arc::new(RwLock::new(None)),
        messages: Arc::new(RwLock::new(VecDeque::from(messages))),
        database: Arc::new(RwLock::new(None)),
        tts_manager: Arc::new(app_lib::tts::TtsManager::default()),
        tts_process_manager: Arc::new(app_lib::tts::TtsProcessManager::new()),
        next_connection_id: Arc::new(AtomicU64::new(0)),
        connections: Arc::new(RwLock::new(HashMap::new())),
    }
}

/// Tauri テストアプリを構築するヘルパー
fn build_test_app(app_state: AppState) -> tauri::App<tauri::test::MockRuntime> {
    mock_builder()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::analytics::get_revenue_analytics,
            app_lib::commands::analytics::export_current_messages,
        ])
        .build(mock_context(noop_assets()))
        .expect("テスト用アプリのビルドに失敗")
}

// ============================================================================
// get_revenue_analytics テスト
// ============================================================================

#[tokio::test]
async fn get_revenue_analytics_empty_messages_returns_default() {
    // 仕様: メッセージなし → 全カウント 0 の RevenueAnalytics を返す
    let app = build_test_app(build_app_state(vec![]));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(&webview, invoke_no_args("get_revenue_analytics"));

    assert!(response.is_ok(), "get_revenue_analytics は成功するべき: {:?}", response.err());

    let analytics: RevenueAnalytics = response.unwrap().deserialize().expect("RevenueAnalytics のデシリアライズに失敗");

    assert_eq!(analytics.super_chat_count, 0);
    assert_eq!(analytics.super_sticker_count, 0);
    assert_eq!(analytics.membership_gains, 0);
    assert_eq!(analytics.super_chat_by_tier.tier_red, 0);
    assert_eq!(analytics.super_chat_by_tier.tier_blue, 0);
    assert!(analytics.top_contributors.is_empty());
    assert!(analytics.hourly_stats.is_empty());
}

#[tokio::test]
async fn get_revenue_analytics_with_superchat_messages() {
    // 仕様: SuperChat 2件（異なる送信者）→ super_chat_count=2, top_contributors の件数も正しい
    let messages = vec![
        make_chat_message(
            "sc1", "UserA", "UC_a",
            MessageType::SuperChat { amount: "$10.00".to_string() },
        ),
        make_chat_message(
            "sc2", "UserB", "UC_b",
            MessageType::SuperChat { amount: "$200.00".to_string() },
        ),
    ];

    let app = build_test_app(build_app_state(messages));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(&webview, invoke_no_args("get_revenue_analytics"));

    assert!(response.is_ok(), "get_revenue_analytics は成功するべき: {:?}", response.err());

    let analytics: RevenueAnalytics = response.unwrap().deserialize().expect("デシリアライズ失敗");

    // 仕様: SuperChat 2件が集計される
    assert_eq!(analytics.super_chat_count, 2);
    assert_eq!(analytics.super_sticker_count, 0);
    assert_eq!(analytics.membership_gains, 0);

    // 仕様: 2人の送信者が top_contributors に含まれる
    assert_eq!(analytics.top_contributors.len(), 2);
}

#[tokio::test]
async fn get_revenue_analytics_mixed_message_types() {
    // 仕様: SuperChat×1 + SuperSticker×1 + Membership×1 + Text×1 が正しく集計される
    let messages = vec![
        make_chat_message("sc1", "UserA", "UC_a", MessageType::SuperChat { amount: "$10.00".to_string() }),
        make_chat_message("ss1", "UserB", "UC_b", MessageType::SuperSticker { amount: "$5.00".to_string() }),
        make_chat_message("m1", "UserC", "UC_c", MessageType::Membership { milestone_months: Some(3) }),
        make_chat_message("t1", "UserD", "UC_d", MessageType::Text),
    ];

    let app = build_test_app(build_app_state(messages));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let response = get_ipc_response(&webview, invoke_no_args("get_revenue_analytics"));
    assert!(response.is_ok(), "get_revenue_analytics は成功するべき: {:?}", response.err());

    let analytics: RevenueAnalytics = response.unwrap().deserialize().expect("デシリアライズ失敗");

    assert_eq!(analytics.super_chat_count, 1);
    assert_eq!(analytics.super_sticker_count, 1);
    assert_eq!(analytics.membership_gains, 1);
}

// ============================================================================
// export_current_messages テスト
// ============================================================================

#[tokio::test]
async fn export_current_messages_json_format() {
    // 仕様: メッセージあり + JSON形式 → ファイルに出力され、有効な JSON を含む
    let messages = vec![
        make_chat_message("msg1", "UserA", "UC_a", MessageType::Text),
        make_chat_message("sc1", "UserB", "UC_b", MessageType::SuperChat { amount: "$10.00".to_string() }),
    ];

    let app = build_test_app(build_app_state(messages));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // tempdir を使用して本番データを保護
    let tmp_dir = tempfile::tempdir().expect("tempdir の作成に失敗");
    let file_path = tmp_dir.path().join("export.json");
    let file_path_str = file_path.to_str().expect("パスの変換に失敗").to_string();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("export_current_messages", serde_json::json!({
            "filePath": file_path_str,
            "config": {
                "format": "json",
                "include_metadata": true,
                "include_system_messages": false,
                "max_records": null,
                "sort_order": null
            }
        })),
    );

    assert!(response.is_ok(), "export_current_messages (JSON) は成功するべき: {:?}", response.err());

    // ファイルが作成されたことを確認
    assert!(file_path.exists(), "エクスポートファイルが作成されていない");

    // ファイルの内容が有効な JSON であることを確認
    let content = std::fs::read_to_string(&file_path).expect("ファイル読み込みに失敗");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("JSON パースに失敗");

    // include_metadata=true なので metadata/messages/statistics を含む
    assert!(parsed.get("metadata").is_some(), "metadata フィールドが存在しない");
    assert!(parsed.get("messages").is_some(), "messages フィールドが存在しない");

    let messages_arr = parsed["messages"].as_array().expect("messages は配列であるべき");
    assert_eq!(messages_arr.len(), 2);
}

#[tokio::test]
async fn export_current_messages_csv_format() {
    // 仕様: メッセージあり + CSV形式 → ファイルに出力され、CSVヘッダーとデータ行を含む
    let messages = vec![
        make_chat_message("msg1", "UserA", "UC_a", MessageType::Text),
        make_chat_message("sc1", "UserB", "UC_b", MessageType::SuperChat { amount: "$10.00".to_string() }),
    ];

    let app = build_test_app(build_app_state(messages));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let tmp_dir = tempfile::tempdir().expect("tempdir の作成に失敗");
    let file_path = tmp_dir.path().join("export.csv");
    let file_path_str = file_path.to_str().expect("パスの変換に失敗").to_string();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("export_current_messages", serde_json::json!({
            "filePath": file_path_str,
            "config": {
                "format": "csv",
                "include_metadata": false,
                "include_system_messages": false,
                "max_records": null,
                "sort_order": null
            }
        })),
    );

    assert!(response.is_ok(), "export_current_messages (CSV) は成功するべき: {:?}", response.err());

    assert!(file_path.exists(), "エクスポートファイルが作成されていない");

    let content = std::fs::read_to_string(&file_path).expect("ファイル読み込みに失敗");

    // 仕様: CSVヘッダーが正しい形式
    assert!(
        content.starts_with("id,timestamp,author,author_id,content,message_type,"),
        "CSV ヘッダーが正しくない: {}",
        content.lines().next().unwrap_or("")
    );

    // データ行が含まれる
    assert!(content.contains("\"msg1\""), "msg1 がエクスポートされていない");
    assert!(content.contains("\"sc1\""), "sc1 がエクスポートされていない");
}

#[tokio::test]
async fn export_current_messages_empty_connections_uses_default_session_id() {
    // 仕様: 接続が空の場合、session_id はデフォルト値 "current" になる
    // （export_current_messages 内のフォールバックロジックの確認）
    let messages = vec![
        make_chat_message("msg1", "UserA", "UC_a", MessageType::Text),
    ];

    let app = build_test_app(build_app_state(messages));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let tmp_dir = tempfile::tempdir().expect("tempdir の作成に失敗");
    let file_path = tmp_dir.path().join("export_default_session.json");
    let file_path_str = file_path.to_str().expect("パスの変換に失敗").to_string();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("export_current_messages", serde_json::json!({
            "filePath": file_path_str,
            "config": {
                "format": "json",
                "include_metadata": true,
                "include_system_messages": false,
                "max_records": null,
                "sort_order": null
            }
        })),
    );

    assert!(response.is_ok(), "export_current_messages は成功するべき: {:?}", response.err());

    let content = std::fs::read_to_string(&file_path).expect("ファイル読み込みに失敗");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("JSON パースに失敗");

    // 接続なし → session_id は "current"
    assert_eq!(parsed["metadata"]["session_id"], "current", "接続なしの session_id は 'current' であるべき");

    // 接続なし → broadcaster_channel_id は null（空文字列はフィルタされる）
    assert!(
        parsed["metadata"]["broadcaster_channel_id"].is_null(),
        "接続なしの broadcaster_channel_id は null であるべき"
    );
}

#[tokio::test]
async fn export_current_messages_unsupported_format_returns_error() {
    // 仕様: サポートされていないフォーマット → エラーを返す
    let app = build_test_app(build_app_state(vec![]));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let tmp_dir = tempfile::tempdir().expect("tempdir の作成に失敗");
    let file_path = tmp_dir.path().join("export.xml");
    let file_path_str = file_path.to_str().expect("パスの変換に失敗").to_string();

    let response = get_ipc_response(
        &webview,
        invoke_with_args("export_current_messages", serde_json::json!({
            "filePath": file_path_str,
            "config": {
                "format": "xml",
                "include_metadata": false,
                "include_system_messages": false,
                "max_records": null,
                "sort_order": null
            }
        })),
    );

    assert!(response.is_err(), "未対応フォーマットはエラーを返すべき");
}
