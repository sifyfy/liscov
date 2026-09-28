//! チャット監視のオーケストレーション
//!
//! connect_to_stream コマンドから抽出された監視ロジック。
//! コマンド層は入出力の変換と MonitoringDeps / run_monitoring_loop への委譲のみを担う。

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tokio_util::sync::CancellationToken;

use tauri::AppHandle;

use crate::core::api::{InnerTubeClient, WebSocketServer};
use crate::core::models::{ChatMessage, ChatMode, MessageType, ReactionUpdate};
use crate::core::raw_response::{RawResponseSaver, SaveConfig};
use crate::database::{self, Database};
use crate::state::MAX_MESSAGES;
use crate::tts::{TtsManager, TtsPriority, TtsQueueItem};

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
/// - `current_save_config` — その時点のレスポンス保存設定を返す（接続中の設定変更を反映するため毎回呼ぶ）
/// - `chat_mode_rx` — チャットモード変更要求を受信する watch チャネル
/// - `emit_gui_message` — ChatMessage を GUI 用に変換して emit するコールバック
/// - `emit_gui_reaction` — ReactionUpdate を GUI 用に変換して emit するコールバック
#[allow(clippy::too_many_arguments)]
pub async fn run_monitoring_loop<F, G, H>(
    deps: MonitoringDeps,
    innertube_client: Arc<RwLock<Option<InnerTubeClient>>>,
    app: AppHandle,
    video_id: String,
    connection_id: u64,
    session_id: Option<String>,
    broadcaster_id: Option<String>,
    cancellation_token: CancellationToken,
    current_save_config: G,
    mut chat_mode_rx: watch::Receiver<ChatMode>,
    emit_gui_message: F,
    emit_gui_reaction: H,
) where
    F: Fn(&AppHandle, &ChatMessage) + Send + Sync + 'static,
    G: Fn() -> SaveConfig + Send + Sync + 'static,
    H: Fn(&AppHandle, &ReactionUpdate) + Send + Sync + 'static,
{
    tracing::info!("チャット監視タスク開始 connection_id: {}", connection_id);
    let poll_interval = std::time::Duration::from_millis(1500);
    let mut poll_count = 0u64;

    // セッション開始時点のコメント数をDBから復元してカウンターを初期化
    // 復元失敗時に silent に空マップへフォールバックすると既存コメント者も
    // 「初回扱い」となり first_comment_only / プレフィックス機能の挙動が崩れるため、
    // 失敗時は warn ログで副作用を明示する (provenance: branch-owned)
    let mut in_stream_counts: std::collections::HashMap<String, u32> = {
        let db_guard = deps.database.read().await;
        match db_guard.as_ref() {
            Some(db) => {
                let conn = db.connection().await;
                match database::get_in_stream_comment_counts(&conn, &video_id) {
                    Ok(counts) => counts,
                    Err(e) => {
                        tracing::warn!(
                            "in_stream_comment_count の DB 復元失敗 video_id={}: {}。\
                             空状態で続行するため、既存コメント者も「初回扱い」となり \
                             first_comment_only / プレフィックス機能に影響する可能性あり",
                            video_id,
                            e
                        );
                        std::collections::HashMap::new()
                    }
                }
            }
            None => std::collections::HashMap::new(),
        }
    };

    // この接続で見た handle → channel_id（ギフトの視聴者特定用）
    let mut known_handles: HashMap<String, String> = HashMap::new();

    // この接続で最後に受けたリアクション更新の時刻（再送を捨てるため）
    let mut last_reaction_time: Option<i64> = None;

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
        let (new_messages, new_reactions, raw_response) = match client.fetch_chat().await {
            Ok(fetch) => {
                if !fetch.messages.is_empty() {
                    tracing::debug!("ポーリング {}: {} 件取得", poll_count, fetch.messages.len());
                }
                (fetch.messages, fetch.reactions, Some(fetch.raw_json))
            }
            Err(e) => {
                tracing::warn!("ポーリング {}: メッセージ取得失敗: {}", poll_count, e);
                (vec![], vec![], None)
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

        // 選ばれたモードと token のモードが違えば適用する（クライアントを戻す前に処理）
        // 書き換えに失敗しても、次のポーリングで新しい token に対してやり直す（02_chat.md）
        let desired_mode = *chat_mode_rx.borrow_and_update();
        if client.get_chat_mode() != desired_mode {
            if client.set_chat_mode(desired_mode) {
                tracing::info!(
                    "チャットモード変更適用: connection_id={}, mode={:?}",
                    connection_id,
                    desired_mode
                );
            } else {
                tracing::warn!(
                    "チャットモード変更失敗（次のポーリングで再試行）: connection_id={}, mode={:?}",
                    connection_id,
                    desired_mode
                );
            }
        }

        {
            let mut client_guard = innertube_client.write().await;
            *client_guard = Some(client);
        }

        // 生レスポンスを保存（設定が有効な場合）
        if let Some(raw_json) = raw_response {
            save_raw_response(&current_save_config(), &raw_json).await;
        }

        // 各メッセージを処理
        for mut msg in new_messages {
            process_message(
                &mut msg,
                &video_id,
                &session_id,
                &broadcaster_id,
                &mut in_stream_counts,
                &mut known_handles,
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

        // ライブリアクション: 保存・GUI・WebSocket（読み上げはしない）
        for update in new_reactions {
            if !is_new_reaction(last_reaction_time, &update) {
                continue;
            }
            last_reaction_time = Some(update.update_time_usec());

            if let Some(sid) = &session_id {
                let db_guard = deps.database.read().await;
                if let Some(db) = db_guard.as_ref() {
                    let conn = db.connection().await;
                    if let Err(e) = database::save_reaction_update(&conn, sid, &update) {
                        tracing::warn!("リアクション保存失敗: {}", e);
                    }
                }
            }

            emit_gui_reaction(&app, &update);

            let ws = deps.websocket_server.read().await;
            if let Some(server) = ws.as_ref() {
                server
                    .broadcast_reaction(broadcaster_id.as_deref(), &update)
                    .await;
            }
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
    in_stream_counts: &mut HashMap<String, u32>,
    known_handles: &mut HashMap<String, String>,
    deps: &MonitoringDeps,
) {
    let is_system = matches!(msg.message_type, MessageType::System);

    // ギフトには channel_id が届かないので handle から特定する（特定できなければ空のまま）
    if matches!(msg.message_type, MessageType::Gift(_)) && msg.channel_id.is_empty() {
        let db_guard = deps.database.read().await;
        let conn = match db_guard.as_ref() {
            Some(db) => Some(db.connection().await),
            None => None,
        };
        let db_scope = conn.as_deref().zip(broadcaster_id.as_deref());
        msg.channel_id =
            resolve_gift_channel_id(known_handles, &msg.author, db_scope).unwrap_or_default();
    }
    if !msg.channel_id.is_empty() {
        known_handles.insert(msg.author.clone(), msg.channel_id.clone());
    }
    // 視聴者を特定できないメッセージは視聴者単位の集計・判定の対象外
    let has_viewer = !msg.channel_id.is_empty();

    // システムメッセージ以外は in-stream コメントカウンターをインクリメント
    if !is_system && has_viewer {
        let count = in_stream_counts.entry(msg.channel_id.clone()).or_insert(0);
        *count += 1;
        msg.in_stream_comment_count = Some(*count);
    }

    // DB に保存（viewer_profile + viewer_stream を生成・更新）
    if let Some(sid) = session_id {
        let db_guard = deps.database.read().await;
        if let Some(db) = db_guard.as_ref() {
            let conn = db.connection().await;
            if let Err(e) =
                database::save_message(&conn, sid, broadcaster_id.as_deref(), msg, Some(video_id))
            {
                tracing::warn!("メッセージ保存失敗: {}", e);
            }
        }
    }

    // DB 保存後に初回視聴者かどうかを判定（viewer_streams が更新済みのため）
    if !is_system && has_viewer {
        if let Some(bid) = broadcaster_id {
            let db_guard = deps.database.read().await;
            if let Some(db) = db_guard.as_ref() {
                let conn = db.connection().await;
                msg.is_first_time_viewer =
                    database::is_first_time_viewer(&conn, bid, &msg.channel_id, video_id)
                        .unwrap_or(false);
            }
        }
    }
}

/// ギフトの handle から channel_id を特定する（02_chat.md「視聴者の特定」）
///
/// 1. この接続で見た handle → channel_id
/// 2. DB: その配信者の viewer_profiles で display_name が一致するもの（候補が 1 件のときだけ）
///
/// `db_scope` は (DB 接続, 配信者 channel_id)。配信者が分からない接続では None で 1 だけを使う。
fn resolve_gift_channel_id(
    known_handles: &HashMap<String, String>,
    handle: &str,
    db_scope: Option<(&rusqlite::Connection, &str)>,
) -> Option<String> {
    if let Some(channel_id) = known_handles.get(handle) {
        return Some(channel_id.clone());
    }
    let (conn, broadcaster_id) = db_scope?;
    database::find_channel_id_by_handle(conn, broadcaster_id, handle)
        .inspect_err(|e| tracing::warn!("ギフトの視聴者検索に失敗: {}", e))
        .ok()
        .flatten()
}

/// 同じ接続で前回受けた更新より新しいか（02_chat.md「ライブリアクション」の再送判定）
fn is_new_reaction(last_update_time_usec: Option<i64>, update: &ReactionUpdate) -> bool {
    last_update_time_usec.is_none_or(|last| update.update_time_usec() > last)
}

/// その時点の保存設定で生レスポンスを保存する（05_raw_response.md 書き込み処理）
async fn save_raw_response(config: &SaveConfig, raw_json: &str) {
    if !config.enabled {
        return;
    }
    let resolved = crate::paths::data_dir().and_then(|dir| config.resolved_for_write(&dir));
    match resolved {
        Ok(config) => {
            if let Err(e) = RawResponseSaver::new(config).save_response(raw_json).await {
                tracing::warn!("生レスポンス保存失敗: {}", e);
            }
        }
        Err(e) => tracing::warn!("生レスポンス保存をスキップ（保存先を解決できない）: {}", e),
    }
}

/// メッセージを TTS キューに追加する
async fn enqueue_tts(tts_manager: &TtsManager, msg: &ChatMessage) {
    let priority = match &msg.message_type {
        MessageType::SuperChat { .. } | MessageType::SuperSticker { .. } | MessageType::Gift(_) => {
            TtsPriority::SuperChat
        }
        MessageType::Membership { .. } | MessageType::MembershipGift { .. } => {
            TtsPriority::Membership
        }
        _ => TtsPriority::Normal,
    };

    let amount = match &msg.message_type {
        MessageType::SuperChat { amount } | MessageType::SuperSticker { amount } => {
            Some(amount.clone())
        }
        _ => None,
    };

    // ギフトは本文の代わりに「{n}ジュエルの{名前}のギフト」を読む
    let text = match &msg.message_type {
        MessageType::Gift(gift) => {
            let read_jewels = tts_manager.get_config().await.read_superchat_amount;
            crate::tts::gift_message(gift, read_jewels)
        }
        _ => msg.content.clone(),
    };

    let item = TtsQueueItem {
        text,
        priority,
        author_name: Some(msg.author.clone()),
        amount,
        in_stream_comment_count: msg.in_stream_comment_count,
        message_id: Some(msg.id.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn reaction_at(time: i64) -> ReactionUpdate {
        let counts = std::collections::BTreeMap::from([("❤".to_string(), 1)]);
        ReactionUpdate::new(time, 1, counts).unwrap()
    }

    // 02_chat.md: 同じ接続で update_time_usec が前回受けた値以下なら捨てる（同じ更新の再送）
    #[test]
    fn reaction_newer_than_last_is_new() {
        assert!(is_new_reaction(None, &reaction_at(100)));
        assert!(is_new_reaction(Some(100), &reaction_at(101)));
    }

    #[test]
    fn reaction_not_newer_than_last_is_resend() {
        assert!(!is_new_reaction(Some(100), &reaction_at(100)));
        assert!(!is_new_reaction(Some(100), &reaction_at(99)));
    }

    // 02_chat.md「視聴者の特定」の例（配信者 UCown）
    async fn resolve_with_db(
        profiles: &[(&str, &str)],
        known: &HashMap<String, String>,
        handle: &str,
    ) -> Option<String> {
        let db = Database::new_in_memory().expect("in-memory DB");
        let conn = db.connection().await;
        for (channel_id, display_name) in profiles {
            database::upsert_viewer_profile(&conn, "UCown", channel_id, display_name, None)
                .unwrap();
        }
        resolve_gift_channel_id(known, handle, Some((&conn, "UCown")))
    }

    #[tokio::test]
    async fn gift_resolved_from_this_connection_first() {
        let known = HashMap::from([("@viewer-a1b".to_string(), "UCa1b".to_string())]);
        let resolved = resolve_with_db(&[("UCfromdb", "@viewer-a1b")], &known, "@viewer-a1b").await;
        assert_eq!(resolved.as_deref(), Some("UCa1b"));
    }

    #[tokio::test]
    async fn gift_resolved_from_db_when_not_seen_in_connection() {
        let resolved =
            resolve_with_db(&[("UCa1b", "@viewer-a1b")], &HashMap::new(), "@viewer-a1b").await;
        assert_eq!(resolved.as_deref(), Some("UCa1b"));
    }

    #[tokio::test]
    async fn gift_unresolved_when_db_has_two_candidates() {
        let profiles = [("UCa1b", "@viewer-a1b"), ("UCzzz", "@viewer-a1b")];
        assert_eq!(
            resolve_with_db(&profiles, &HashMap::new(), "@viewer-a1b").await,
            None
        );
    }

    #[tokio::test]
    async fn gift_unresolved_when_unknown() {
        assert_eq!(resolve_with_db(&[], &HashMap::new(), "@new").await, None);
    }

    #[test]
    fn gift_without_broadcaster_uses_connection_only() {
        let known = HashMap::from([("@a".to_string(), "UCa".to_string())]);
        assert_eq!(
            resolve_gift_channel_id(&known, "@a", None).as_deref(),
            Some("UCa")
        );
        assert_eq!(resolve_gift_channel_id(&known, "@b", None), None);
    }
}
