//! チャット監視のオーケストレーション
//!
//! connect_to_stream コマンドから抽出された監視ロジック。
//! コマンド層は入出力の変換と MonitoringDeps / run_monitoring_loop への委譲のみを担う。

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tokio_util::sync::CancellationToken;

use tauri::AppHandle;

use crate::core::api::{InnerTubeClient, WebSocketServer};
use crate::core::models::{ChatMessage, ChatMode};
use crate::core::raw_response::{RawResponseSaver, SaveConfig};
use crate::database::{self, Database};
use crate::state::MAX_MESSAGES;
use crate::tts::{TtsManager, TtsQueueItem, TtsPriority};

/// 監視タスクが必要とする共有依存をまとめた構造体
///
/// 複数接続間で共有されるリソース（メッセージバッファ、DB、WebSocket、TTS）を保持する。
/// 接続固有の情報（session_id, broadcaster_id, client）は run_monitoring_loop の引数で渡す。
pub struct MonitoringDeps {
    /// 全接続のメッセージを統合するグローバルバッファ
    pub messages: Arc<RwLock<VecDeque<ChatMessage>>>,
    /// データベース接続
    pub database: Arc<RwLock<Option<Database>>>,
    /// WebSocket サーバー（外部アプリへのブロードキャスト）
    pub websocket_server: Arc<RwLock<Option<WebSocketServer>>>,
    /// TTS マネージャー
    pub tts_manager: Arc<TtsManager>,
}

impl MonitoringDeps {
    /// AppState の各フィールドから Arc::clone して MonitoringDeps を構築する
    pub fn from_state(state: &crate::AppState) -> Self {
        Self {
            messages: Arc::clone(&state.messages),
            database: Arc::clone(&state.database),
            websocket_server: Arc::clone(&state.websocket_server),
            tts_manager: Arc::clone(&state.tts_manager),
        }
    }
}

/// チャット監視のポーリングループ全体を実行する
///
/// この関数は tokio::spawn で別タスクとして起動される。
/// ループ終了後にセッションの終了処理（end_session / update_session_stats）を行う。
///
/// # 引数
/// - `deps` — 監視タスクが必要とする共有依存一式
/// - `innertube_client` — InnerTube クライアント（Arc<RwLock> でラップ済み）
/// - `app` — Tauri AppHandle（フロントエンドへの emit に使用）
/// - `video_id` — 監視対象の YouTube 動画 ID
/// - `connection_id` — この接続に割り当てられた接続 ID
/// - `session_id` — データベースセッション ID
/// - `broadcaster_id` — 配信者チャンネル ID
/// - `cancellation_token` — この接続のキャンセレーショントークン
/// - `save_config` — レスポンス保存設定
/// - `chat_mode_rx` — チャットモード変更要求を受信する watch チャネル
/// - `emit_gui_message` — ChatMessage を GUI 用に変換して emit するコールバック
#[allow(clippy::too_many_arguments)]
pub async fn run_monitoring_loop<F>(
    deps: MonitoringDeps,
    innertube_client: Arc<RwLock<Option<InnerTubeClient>>>,
    app: AppHandle,
    video_id: String,
    connection_id: u64,
    session_id: Option<String>,
    broadcaster_id: Option<String>,
    cancellation_token: CancellationToken,
    save_config: SaveConfig,
    mut chat_mode_rx: watch::Receiver<ChatMode>,
    emit_gui_message: F,
) where
    F: Fn(&AppHandle, &ChatMessage) + Send + Sync + 'static,
{
    tracing::info!("チャット監視タスク開始 connection_id: {}", connection_id);
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
        // CancellationToken でループ停止を確認
        if cancellation_token.is_cancelled() {
            tracing::info!(
                "CancellationToken によりループ停止 connection_id: {} polls: {}",
                connection_id,
                poll_count
            );
            break;
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

        // フェッチ前にもキャンセルを確認
        if cancellation_token.is_cancelled() {
            tracing::info!(
                "フェッチ前にキャンセル検出 connection_id: {}",
                connection_id
            );
            break;
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

        // キャンセルされていなければクライアントを戻す
        if cancellation_token.is_cancelled() {
            tracing::info!(
                "フェッチ中にキャンセル検出（クライアントを戻さず終了） connection_id: {}",
                connection_id
            );
            break;
        }

        // チャットモード変更要求があれば適用（クライアントを戻す前に処理）
        if chat_mode_rx.has_changed().unwrap_or(false) {
            let new_mode = *chat_mode_rx.borrow_and_update();
            if client.set_chat_mode(new_mode) {
                tracing::info!(
                    "チャットモード変更適用: connection_id={}, mode={:?}",
                    connection_id,
                    new_mode
                );
            } else {
                tracing::warn!(
                    "チャットモード変更失敗: connection_id={}, mode={:?}",
                    connection_id,
                    new_mode
                );
            }
        }

        {
            let mut client_guard = innertube_client.write().await;
            *client_guard = Some(client);
        }

        // 生レスポンスを保存（設定が有効な場合）
        if let Some(raw_json) = raw_response {
            if let Err(e) = raw_response_saver.save_response(&raw_json).await {
                tracing::warn!("生レスポンス保存失敗: {}", e);
            }
        }

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

        // スリープ中もキャンセルを検知できるように select! を使用
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                tracing::info!("sleep中にCancellationTokenキャンセル connection_id: {}", connection_id);
                break;
            }
            _ = tokio::time::sleep(poll_interval) => {}
        }
    }

    // セッション終了処理
    finish_session(&deps, connection_id, &session_id).await;

    tracing::info!(
        "チャット監視タスク停止 connection_id: {} polls: {}",
        connection_id,
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
async fn finish_session(deps: &MonitoringDeps, connection_id: u64, session_id: &Option<String>) {
    tracing::debug!(
        "監視タスク終了処理: セッション確認 connection_id: {}",
        connection_id
    );
    if let Some(sid) = session_id.as_ref() {
        tracing::debug!(
            "監視タスク終了処理: セッション {} を終了 connection_id: {}",
            sid,
            connection_id
        );
        let db_guard = deps.database.read().await;
        if let Some(db) = db_guard.as_ref() {
            let conn = db.connection().await;
            if let Err(e) = database::end_session(&conn, sid) {
                tracing::warn!("セッション終了失敗: {}", e);
            }
            if let Err(e) = database::update_session_stats(&conn, sid) {
                tracing::warn!("セッション統計更新失敗: {}", e);
            }
            tracing::debug!(
                "監視タスク終了処理: セッション終了完了 connection_id: {}",
                connection_id
            );
        }
    } else {
        tracing::debug!(
            "監視タスク終了処理: 終了すべきセッションなし connection_id: {}",
            connection_id
        );
    }
}
