//! チャット監視のオーケストレーション
//!
//! connect_to_stream コマンドから抽出された監視ロジック。
//! コマンド層は入出力の変換と MonitoringDeps / run_monitoring_loop への委譲のみを担う。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use tauri::AppHandle;

use crate::core::api::{InnerTubeClient, WebSocketServer};
use crate::core::models::ChatMessage;
use crate::core::raw_response::{RawResponseSaver, SaveConfig};
use crate::database::{self, Database};
use crate::state::MAX_MESSAGES;
use crate::tts::{TtsManager, TtsQueueItem, TtsPriority};

/// 監視タスクが必要とする依存をまとめた構造体
///
/// chat_monitoring_task の 13 引数をグループ化し、
/// connect_to_stream から MonitoringDeps::from_state で一括生成できるようにする。
pub struct MonitoringDeps {
    /// フロントエンドに送信するチャットメッセージバッファ
    pub messages: Arc<RwLock<VecDeque<ChatMessage>>>,
    /// 監視中フラグ（false にすることでループを停止）
    pub is_monitoring: Arc<RwLock<bool>>,
    /// 接続 ID（新規接続が始まると変化し、古いタスクを終了させる）
    pub connection_id: Arc<AtomicU64>,
    /// データベース接続
    pub database: Arc<RwLock<Option<Database>>>,
    /// WebSocket サーバー（外部アプリへのブロードキャスト）
    pub websocket_server: Arc<RwLock<Option<WebSocketServer>>>,
    /// TTS マネージャー
    pub tts_manager: Arc<TtsManager>,
    /// 現在のセッション ID
    pub current_session_id: Arc<RwLock<Option<String>>>,
    /// 現在の配信者チャンネル ID
    pub current_broadcaster_id: Arc<RwLock<Option<String>>>,
}

impl MonitoringDeps {
    /// AppState の各フィールドから Arc::clone して MonitoringDeps を構築する
    pub fn from_state(state: &crate::AppState) -> Self {
        Self {
            messages: Arc::clone(&state.messages),
            is_monitoring: Arc::clone(&state.is_monitoring),
            connection_id: Arc::clone(&state.connection_id),
            database: Arc::clone(&state.database),
            websocket_server: Arc::clone(&state.websocket_server),
            tts_manager: Arc::clone(&state.tts_manager),
            current_session_id: Arc::clone(&state.current_session_id),
            current_broadcaster_id: Arc::clone(&state.current_broadcaster_id),
        }
    }
}

/// チャット監視のポーリングループ全体を実行する
///
/// この関数は tokio::spawn で別タスクとして起動される。
/// ループ終了後にセッションの終了処理（end_session / update_session_stats）を行う。
///
/// # 引数
/// - `deps` — 監視タスクが必要とする依存一式
/// - `innertube_client` — InnerTube クライアント（AppState から Arc::clone 済み）
/// - `app` — Tauri AppHandle（フロントエンドへの emit に使用）
/// - `video_id` — 監視対象の YouTube 動画 ID
/// - `my_connection_id` — このタスクが生成された時点での接続 ID
/// - `save_config` — レスポンス保存設定
/// - `emit_gui_message` — ChatMessage を GUI 用に変換して emit するコールバック
pub async fn run_monitoring_loop<F>(
    deps: MonitoringDeps,
    innertube_client: Arc<RwLock<Option<InnerTubeClient>>>,
    app: AppHandle,
    video_id: String,
    my_connection_id: u64,
    save_config: SaveConfig,
    emit_gui_message: F,
) where
    F: Fn(&AppHandle, &ChatMessage) + Send + Sync + 'static,
{
    tracing::info!("チャット監視タスク開始 connection_id: {}", my_connection_id);
    let poll_interval = std::time::Duration::from_millis(1500);
    let raw_response_saver = RawResponseSaver::new(save_config);
    let mut poll_count = 0u64;

    // セッション開始時点のコメント数をDBから復元してカウンターを初期化
    let mut in_stream_counts: std::collections::HashMap<String, u32> = {
        let db_guard = deps.database.read().await;
        if let Some(db) = db_guard.as_ref() {
            let conn = db.connection().await;
            database::get_in_stream_comment_counts(&conn, &video_id).unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        }
    };

    loop {
        // 監視停止フラグを確認
        {
            let monitoring = deps.is_monitoring.read().await;
            if !*monitoring {
                tracing::info!(
                    "監視フラグにより停止 polls: {} connection_id: {}",
                    poll_count,
                    my_connection_id
                );
                break;
            }
        }

        // 接続 ID が変わっていれば（新規接続が始まった）終了
        {
            let current_id = deps.connection_id.load(Ordering::SeqCst);
            if current_id != my_connection_id {
                tracing::info!(
                    "接続 ID 変化により停止 {} → {} polls: {}",
                    my_connection_id,
                    current_id,
                    poll_count
                );
                break;
            }
        }

        poll_count += 1;

        // ネットワーク呼び出し中にロックを手放すため、クライアントを一時的に取り出す
        let client_opt = {
            let mut client_guard = innertube_client.write().await;
            client_guard.take()
        };

        let Some(mut client) = client_opt else {
            tracing::warn!("InnerTube クライアントが存在しないため監視を停止");
            break;
        };

        // フェッチ前にも接続 ID を確認（disconnect が呼ばれた場合への対応）
        {
            let current_id = deps.connection_id.load(Ordering::SeqCst);
            if current_id != my_connection_id {
                tracing::info!(
                    "フェッチ前に接続 ID 変化 {} → {}",
                    my_connection_id,
                    current_id
                );
                break;
            }
        }

        // メッセージをフェッチ（ロックを保持しない）
        let (new_messages, raw_response) = match client.fetch_messages_with_raw().await {
            Ok((msgs, raw)) => {
                if !msgs.is_empty() {
                    tracing::debug!("ポーリング {}: {} 件取得", poll_count, msgs.len());
                }
                (msgs, Some(raw))
            }
            Err(e) => {
                tracing::warn!("ポーリング {}: メッセージ取得失敗: {}", poll_count, e);
                (vec![], None)
            }
        };

        // クライアントを戻す（接続 ID が変わっていなければ）
        {
            let current_id = deps.connection_id.load(Ordering::SeqCst);
            if current_id == my_connection_id {
                let mut client_guard = innertube_client.write().await;
                *client_guard = Some(client);
            } else {
                tracing::info!(
                    "フェッチ中に接続 ID 変化 {} → {}（クライアントを戻さず終了）",
                    my_connection_id,
                    current_id
                );
                break;
            }
        }

        // 生レスポンスを保存（設定が有効な場合）
        if let Some(raw_json) = raw_response {
            if let Err(e) = raw_response_saver.save_response(&raw_json).await {
                tracing::warn!("生レスポンス保存失敗: {}", e);
            }
        }

        // セッション ID と配信者 ID を取得
        let (session_id, broadcaster_id) = {
            let session = deps.current_session_id.read().await;
            let broadcaster = deps.current_broadcaster_id.read().await;
            (session.clone(), broadcaster.clone())
        };

        // 各メッセージを処理
        for mut msg in new_messages {
            process_message(
                &mut msg,
                &video_id,
                &session_id,
                &broadcaster_id,
                &mut in_stream_counts,
                &deps,
            )
            .await;

            // メッセージバッファに追加
            {
                let mut msgs = deps.messages.write().await;
                if msgs.len() >= MAX_MESSAGES {
                    msgs.pop_front();
                }
                msgs.push_back(msg.clone());
            }

            // GUI メッセージをフロントエンドに emit（コールバック経由）
            emit_gui_message(&app, &msg);

            // WebSocket クライアントへブロードキャスト
            {
                let ws = deps.websocket_server.read().await;
                if let Some(server) = ws.as_ref() {
                    server.broadcast_message(&msg).await;
                }
            }

            // TTS キューに追加
            enqueue_tts(&deps.tts_manager, &msg).await;
        }

        tokio::time::sleep(poll_interval).await;
    }

    // セッション終了処理
    finish_session(&deps, my_connection_id).await;

    tracing::info!(
        "チャット監視タスク停止 connection_id: {} polls: {}",
        my_connection_id,
        poll_count
    );
}

/// 1 件のメッセージに対して、DB 保存・初回視聴者判定・in-stream カウント更新を行う
async fn process_message(
    msg: &mut ChatMessage,
    video_id: &str,
    session_id: &Option<String>,
    broadcaster_id: &Option<String>,
    in_stream_counts: &mut std::collections::HashMap<String, u32>,
    deps: &MonitoringDeps,
) {
    let is_system = matches!(msg.message_type, crate::core::models::MessageType::System);

    // システムメッセージ以外は in-stream コメントカウンターをインクリメント
    if !is_system {
        let count = in_stream_counts.entry(msg.channel_id.clone()).or_insert(0);
        *count += 1;
        msg.in_stream_comment_count = Some(*count);
    }

    // DB に保存（viewer_profile + viewer_stream を生成・更新）
    if let Some(sid) = session_id {
        let db_guard = deps.database.read().await;
        if let Some(db) = db_guard.as_ref() {
            let conn = db.connection().await;
            if let Err(e) = database::save_message(
                &conn,
                sid,
                broadcaster_id.as_deref(),
                msg,
                Some(video_id),
            ) {
                tracing::warn!("メッセージ保存失敗: {}", e);
            }
        }
    }

    // DB 保存後に初回視聴者かどうかを判定（viewer_streams が更新済みのため）
    if !is_system {
        if let Some(bid) = broadcaster_id {
            let db_guard = deps.database.read().await;
            if let Some(db) = db_guard.as_ref() {
                let conn = db.connection().await;
                msg.is_first_time_viewer = database::is_first_time_viewer(
                    &conn,
                    bid,
                    &msg.channel_id,
                    video_id,
                )
                .unwrap_or(false);
            }
        }
    }
}

/// メッセージを TTS キューに追加する
async fn enqueue_tts(tts_manager: &TtsManager, msg: &ChatMessage) {
    let priority = match &msg.message_type {
        crate::core::models::MessageType::SuperChat { .. }
        | crate::core::models::MessageType::SuperSticker { .. } => TtsPriority::SuperChat,
        crate::core::models::MessageType::Membership { .. }
        | crate::core::models::MessageType::MembershipGift { .. } => TtsPriority::Membership,
        _ => TtsPriority::Normal,
    };

    let amount = match &msg.message_type {
        crate::core::models::MessageType::SuperChat { amount }
        | crate::core::models::MessageType::SuperSticker { amount } => Some(amount.clone()),
        _ => None,
    };

    let item = TtsQueueItem {
        text: msg.content.clone(),
        priority,
        author_name: Some(msg.author.clone()),
        amount,
    };
    tts_manager.enqueue(item).await;
}

/// ループ終了後のセッション終了処理
async fn finish_session(deps: &MonitoringDeps, my_connection_id: u64) {
    tracing::debug!(
        "監視タスク終了処理: セッション確認 connection_id: {}",
        my_connection_id
    );
    if let Some(session_id) = deps.current_session_id.read().await.as_ref() {
        tracing::debug!(
            "監視タスク終了処理: セッション {} を終了 connection_id: {}",
            session_id,
            my_connection_id
        );
        let db_guard = deps.database.read().await;
        if let Some(db) = db_guard.as_ref() {
            let conn = db.connection().await;
            if let Err(e) = database::end_session(&conn, session_id) {
                tracing::warn!("セッション終了失敗: {}", e);
            }
            if let Err(e) = database::update_session_stats(&conn, session_id) {
                tracing::warn!("セッション統計更新失敗: {}", e);
            }
            tracing::debug!(
                "監視タスク終了処理: セッション終了完了 connection_id: {}",
                my_connection_id
            );
        }
    } else {
        tracing::debug!(
            "監視タスク終了処理: 終了すべきセッションなし connection_id: {}",
            my_connection_id
        );
    }
}
