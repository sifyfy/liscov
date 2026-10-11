//! チャット監視のオーケストレーション
//!
//! connect_to_stream コマンドから抽出された監視ロジック。
//! コマンド層は入出力の変換と MonitoringDeps / run_monitoring_loop への委譲のみを担う。

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, watch};
use tokio_util::sync::CancellationToken;

use crate::core::api::{InnerTubeClient, WebSocketServer};
use crate::core::models::{ChatMessage, ChatMode, MessageType, ReactionUpdate};
use crate::core::raw_response::{RawResponseSaver, SaveConfig};
use crate::database::{self, Database, ViewerAnnotation};
use crate::tts::{TtsManager, TtsPriority, TtsQueueItem};
use rusqlite::Connection;

/// 接続ごとに覚える受信済み message_id の件数（02_chat.md「設定値」）
const RECENT_MESSAGE_IDS_CAPACITY: usize = 10_000;

/// ポーリング間隔の下限（02_chat.md「設定値」）
const POLL_INTERVAL: Duration = Duration::from_millis(1500);
/// 空振りの 2 回目から 1 回ごとに延ばす幅（02_chat.md「設定値」）
const EMPTY_POLL_STEP: Duration = Duration::from_millis(1000);
/// 応答に timeoutMs が無いときの、成功している間の間隔の上限（02_chat.md「設定値」）
const DEFAULT_POLL_CEILING: Duration = Duration::from_millis(10_000);
/// この回数までの連続失敗は成功と同じ間隔で再試行する（02_chat.md「取得に失敗したとき」）
const FAILURES_BEFORE_BACKOFF: u32 = 10;
/// この回数続けて失敗したら切断する（02_chat.md「取得に失敗したとき」）
const FAILURES_BEFORE_DISCONNECT: u32 = 15;

/// 監視ループが終わった理由
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitoringEnd {
    /// 切断された（CancellationToken）
    Cancelled,
    /// 取得に続けて失敗した（02_chat.md「取得に失敗したとき」）
    FetchFailedRepeatedly,
}

impl MonitoringEnd {
    /// 切断されずに終わったときにユーザーへ出すエラー
    pub fn error_message(self) -> String {
        match self {
            Self::FetchFailedRepeatedly => format!(
                "チャットの取得に{}回続けて失敗したため切断しました",
                FAILURES_BEFORE_DISCONNECT
            ),
            Self::Cancelled => "監視タスクが予期せず終了しました".to_string(),
        }
    }
}

/// 監視タスクが必要とする共有依存をまとめた構造体
///
/// 複数接続間で共有されるリソース（DB、WebSocket、TTS）を保持する。
/// 接続固有の情報（session_id, broadcaster_id, client）は run_monitoring_loop の引数で渡す。
pub struct MonitoringDeps {
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
            database: Arc::clone(&state.database),
            websocket_server: Arc::clone(&state.websocket_server),
            tts_manager: Arc::clone(&state.tts_manager),
        }
    }
}

/// 1 つの接続の監視に要るもの（接続ごとに作る）
pub struct MonitoringTarget {
    /// この接続に割り当てられた接続 ID
    pub connection_id: u64,
    /// 監視対象の YouTube 動画 ID
    pub video_id: String,
    /// データベースセッション ID（DB を使えないときは None）
    pub session_id: Option<String>,
    /// 配信者チャンネル ID
    pub broadcaster_id: Option<String>,
    /// この接続のキャンセレーショントークン
    pub cancellation_token: CancellationToken,
    /// チャットモード変更要求を受信する watch チャネル
    pub chat_mode_rx: watch::Receiver<ChatMode>,
}

/// 監視ループの外とのやりとり（画面への通知と、その時点の保存設定）
///
/// core を Tauri に依存させないよう trait にする。本番の実装は commands::chat にある。
pub trait MonitoringOutput: Send + Sync + 'static {
    /// その時点の生レスポンス保存設定（接続中の設定変更を反映するため、ポーリングごとに呼ぶ）
    fn save_config(&self) -> SaveConfig;
    /// 新しいメッセージを画面に送る
    /// annotation は viewer_annotation で引いた、この接続の配信者のもとでの投稿者の読み仮名とメモ
    fn message(&self, message: &ChatMessage, annotation: &ViewerAnnotation);
    /// ライブリアクションの更新を画面に送る
    fn reaction(&self, update: &ReactionUpdate);
}

/// チャット監視のポーリングループ全体を実行する
///
/// この関数は tokio::spawn で別タスクとして起動される。
/// ループ終了後にセッションの終了処理（end_session / update_session_stats）を行う。
///
/// クライアントはこのループだけが持つ（ポーリングのたびに取り出して戻す必要はない）。
pub async fn run_monitoring_loop(
    deps: MonitoringDeps,
    mut client: InnerTubeClient,
    target: MonitoringTarget,
    output: impl MonitoringOutput,
) -> MonitoringEnd {
    let MonitoringTarget {
        connection_id,
        video_id,
        session_id,
        broadcaster_id,
        cancellation_token,
        mut chat_mode_rx,
    } = target;
    tracing::info!("チャット監視タスク開始 connection_id: {}", connection_id);
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

    // この接続で受け取った message_id（YouTube の再送を捨てるため）
    let mut recent_message_ids = RecentMessageIds::new(RECENT_MESSAGE_IDS_CAPACITY);

    let mut schedule = PollSchedule::default();

    let end = loop {
        // CancellationToken でループ停止を確認
        if cancellation_token.is_cancelled() {
            tracing::info!(
                "CancellationToken によりループ停止 connection_id: {} polls: {}",
                connection_id,
                poll_count
            );
            break MonitoringEnd::Cancelled;
        }

        poll_count += 1;

        // メッセージをフェッチする。応答を待つ間も切断を受け付ける
        let fetched = tokio::select! {
            _ = cancellation_token.cancelled() => {
                tracing::info!(
                    "フェッチ中にキャンセル検出（応答を待たずに終了） connection_id: {}",
                    connection_id
                );
                break MonitoringEnd::Cancelled;
            }
            fetched = client.fetch_chat() => fetched,
        };
        // fetched_timeout は成功したときだけ Some（中身は応答の timeoutMs）。
        // 成功は再送を除いた件数で数えるので、記録は再送を捨てた後に行う
        let (new_messages, new_reactions, raw_response, fetched_timeout) = match fetched {
            Ok(fetch) => {
                if !fetch.messages.is_empty() {
                    tracing::debug!("ポーリング {}: {} 件取得", poll_count, fetch.messages.len());
                }
                (
                    fetch.messages,
                    fetch.reactions,
                    Some(fetch.raw_json),
                    Some(fetch.timeout),
                )
            }
            Err(e) => {
                schedule = schedule.after(PollOutcome::Failed);
                tracing::warn!(
                    "ポーリング {}: メッセージ取得失敗（連続 {} 回）: {:#}",
                    poll_count,
                    schedule.consecutive_failures,
                    e
                );
                (vec![], vec![], None, None)
            }
        };

        // 応答を受けた直後に切断されていたら、保存も通知もせずに終わる
        if cancellation_token.is_cancelled() {
            tracing::info!(
                "フェッチ後にキャンセル検出 connection_id: {}",
                connection_id
            );
            break MonitoringEnd::Cancelled;
        }

        // 選ばれたモードと token のモードが違えば適用する
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

        // 生レスポンスを保存（設定が有効な場合）
        if let Some(raw_json) = raw_response {
            save_raw_response(&output.save_config(), &raw_json).await;
        }

        // 再送は保存・集計・GUI・WebSocket・TTS のどれにも流さない
        let mut fresh: Vec<ChatMessage> = new_messages
            .into_iter()
            .filter(|msg| {
                let is_new = recent_message_ids.insert(&msg.id);
                if !is_new {
                    tracing::debug!(
                        "再送されたメッセージを捨てる connection_id: {} message_id: {}",
                        connection_id,
                        msg.id
                    );
                }
                is_new
            })
            .collect();
        if let Some(timeout) = fetched_timeout {
            schedule = schedule.after(PollOutcome::Fetched {
                fresh: fresh.len(),
                timeout,
            });
        }

        // DB への保存と判定（DB のロックは 1 回、1 トランザクション。08_database.md「書き込み」）。
        // 読み上げと画面に使う読み仮名・メモも、同じロックのうちにこの接続の配信者のもとで引く
        let annotations: Vec<ViewerAnnotation> = if fresh.is_empty() {
            Vec::new()
        } else {
            let target = MessageTarget {
                video_id: &video_id,
                session_id: session_id.as_deref(),
                broadcaster_id: broadcaster_id.as_deref(),
            };
            let db_guard = deps.database.read().await;
            let conn = match db_guard.as_ref() {
                Some(db) => Some(db.connection().await),
                None => None,
            };
            process_messages(
                &mut fresh,
                conn.as_deref(),
                &target,
                &mut in_stream_counts,
                &mut known_handles,
            );
            let db_scope = conn.as_deref().zip(broadcaster_id.as_deref());
            fresh
                .iter()
                .map(|msg| viewer_annotation(msg, db_scope))
                .collect()
        };

        for (msg, annotation) in fresh.iter().zip(annotations) {
            // 画面に送る (読み仮名は読み上げと同じ値を名前の横に、メモは本文の下に出す)
            output.message(msg, &annotation);

            // WebSocket クライアントへブロードキャスト
            {
                let ws = deps.websocket_server.read().await;
                if let Some(server) = ws.as_ref() {
                    server.broadcast_message(msg).await;
                }
            }

            // TTS キューに追加
            enqueue_tts(&deps.tts_manager, msg, annotation.reading).await;
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

            output.reaction(&update);

            let ws = deps.websocket_server.read().await;
            if let Some(server) = ws.as_ref() {
                server
                    .broadcast_reaction(broadcaster_id.as_deref(), &update)
                    .await;
            }
        }

        let Some(wait) = schedule.next_wait() else {
            tracing::warn!(
                "取得に {} 回続けて失敗したため監視を停止 connection_id: {}",
                schedule.consecutive_failures,
                connection_id
            );
            break MonitoringEnd::FetchFailedRepeatedly;
        };
        tracing::debug!(
            "次の取得まで {:?} connection_id: {} 空振り: {} 回",
            wait,
            connection_id,
            schedule.consecutive_empty
        );

        // スリープ中もキャンセルを検知できるように select! を使用
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                tracing::info!("sleep中にCancellationTokenキャンセル connection_id: {}", connection_id);
                break MonitoringEnd::Cancelled;
            }
            _ = tokio::time::sleep(wait) => {}
        }
    };

    // セッション終了処理
    finish_session(&deps, connection_id, &session_id).await;

    tracing::info!(
        "チャット監視タスク停止 connection_id: {} polls: {} reason: {:?}",
        connection_id,
        poll_count,
        end
    );
    end
}

/// メッセージの保存先（どの配信・セッションのメッセージか）
struct MessageTarget<'a> {
    video_id: &'a str,
    session_id: Option<&'a str>,
    broadcaster_id: Option<&'a str>,
}

/// 1 回のポーリングで届いた新しいメッセージを、1 つのトランザクションで保存・判定する（08_database.md「書き込み」）
///
/// 1 件の保存に失敗しても警告ログを出して次へ進む。トランザクションを始められなければ 1 件ずつ保存する。
fn process_messages(
    msgs: &mut [ChatMessage],
    conn: Option<&Connection>,
    target: &MessageTarget,
    in_stream_counts: &mut HashMap<String, u32>,
    known_handles: &mut HashMap<String, String>,
) {
    let tx = conn.and_then(|conn| {
        conn.unchecked_transaction()
            .inspect_err(|e| {
                tracing::warn!("トランザクションを始められない（1 件ずつ保存する）: {}", e)
            })
            .ok()
    });
    let db = tx.as_deref().or(conn);
    for msg in msgs.iter_mut() {
        process_message(msg, db, target, in_stream_counts, known_handles);
    }
    if let Some(tx) = tx {
        if let Err(e) = tx.commit() {
            tracing::warn!("メッセージ保存のコミットに失敗: {}", e);
        }
    }
}

/// 1 件のメッセージに対して、DB 保存・初回視聴者判定・in-stream カウント更新を行う
fn process_message(
    msg: &mut ChatMessage,
    db: Option<&Connection>,
    target: &MessageTarget,
    in_stream_counts: &mut HashMap<String, u32>,
    known_handles: &mut HashMap<String, String>,
) {
    let is_system = matches!(msg.message_type, MessageType::System);

    // ギフトには channel_id が届かないので handle から特定する（特定できなければ空のまま）
    if matches!(msg.message_type, MessageType::Gift(_)) && msg.channel_id.is_empty() {
        let db_scope = db.zip(target.broadcaster_id);
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

    let Some(db) = db else {
        return;
    };

    // DB に保存（viewer_profile + viewer_stream を生成・更新）
    if let Some(sid) = target.session_id {
        if let Err(e) =
            database::save_message(db, sid, target.broadcaster_id, msg, Some(target.video_id))
        {
            tracing::warn!("メッセージ保存失敗: {}", e);
        }
    }

    // DB 保存後に初回視聴者かどうかを判定（viewer_streams が更新済みのため）
    if !is_system && has_viewer {
        if let Some(bid) = target.broadcaster_id {
            msg.is_first_time_viewer =
                database::is_first_time_viewer(db, bid, &msg.channel_id, target.video_id)
                    .unwrap_or(false);
        }
    }
}

/// 1 回の取得の結果のうち、次の取得までの待ちに関わるもの
#[derive(Debug, Clone, Copy)]
enum PollOutcome {
    /// 取得に成功した。fresh は再送を除いた新しいメッセージの件数、timeout は応答の timeoutMs
    Fetched {
        fresh: usize,
        timeout: Option<Duration>,
    },
    Failed,
}

/// 次の取得までの待ちを決める状態
/// （02_chat.md「取得の間隔」「取得に失敗したとき」）
#[derive(Debug, Clone, Copy)]
struct PollSchedule {
    consecutive_failures: u32,
    consecutive_empty: u32,
    /// 直近の成功した応答の上限
    ceiling: Duration,
}

impl Default for PollSchedule {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            consecutive_empty: 0,
            ceiling: DEFAULT_POLL_CEILING,
        }
    }
}

impl PollSchedule {
    /// 1 回の取得の結果を数えた後の状態。失敗は空振りの回数を変えない
    #[must_use]
    fn after(self, outcome: PollOutcome) -> Self {
        match outcome {
            PollOutcome::Fetched { fresh, timeout } => Self {
                consecutive_failures: 0,
                consecutive_empty: if fresh == 0 {
                    self.consecutive_empty.saturating_add(1)
                } else {
                    0
                },
                ceiling: timeout.unwrap_or(DEFAULT_POLL_CEILING),
            },
            PollOutcome::Failed => Self {
                consecutive_failures: self.consecutive_failures + 1,
                ..self
            },
        }
    }

    /// 次の取得までの待ち。`None` なら切断する
    ///
    /// 失敗の 11 回目からは失敗のたびに倍にする（3・6・12・24 秒）。
    /// 成功している間は空振りの 2 回目から延ばし、上限（下限より短ければ下限）で止める。
    fn next_wait(&self) -> Option<Duration> {
        match self.consecutive_failures {
            n if n >= FAILURES_BEFORE_DISCONNECT => None,
            n if n > FAILURES_BEFORE_BACKOFF => {
                Some(POLL_INTERVAL * 2u32.pow(n - FAILURES_BEFORE_BACKOFF))
            }
            0 => {
                let grown =
                    POLL_INTERVAL + EMPTY_POLL_STEP * self.consecutive_empty.saturating_sub(1);
                Some(grown.min(self.ceiling).max(POLL_INTERVAL))
            }
            _ => Some(POLL_INTERVAL),
        }
    }
}

/// この接続で受け取った message_id の記憶（02_chat.md「受信済み message_id の記憶」）
///
/// 直近 `capacity` 件だけを覚え、超えたら古いものから忘れる。
/// id は `Arc<str>` で順序と集合に共有し、1 件 1 回の確保で済ませる。
struct RecentMessageIds {
    capacity: usize,
    order: VecDeque<Arc<str>>,
    ids: HashSet<Arc<str>>,
}

impl RecentMessageIds {
    fn new(capacity: usize) -> Self {
        debug_assert!(capacity > 0, "記憶件数は1以上");
        Self {
            capacity,
            order: VecDeque::with_capacity(capacity),
            ids: HashSet::with_capacity(capacity),
        }
    }

    /// 初めて見る id なら覚えて true、受け取り済みなら false を返す。空の id は常に true
    fn insert(&mut self, id: &str) -> bool {
        if id.is_empty() {
            return true;
        }
        if self.ids.contains(id) {
            return false;
        }
        if self.order.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.ids.remove(&oldest);
            }
        }
        let id: Arc<str> = Arc::from(id);
        self.order.push_back(Arc::clone(&id));
        self.ids.insert(id);
        true
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

/// メッセージの視聴者に、この接続の配信者のもとで登録した読み仮名とメモ
/// （04_tts.md「どの読み仮名を使うか」、02_chat.md「1 行の形」）
///
/// 視聴者を特定できない・配信者が分からない・DB を使えないときは、どちらも None。
/// 検索に失敗したら警告ログを出して、どちらも None（投稿者名で読み上げる）。
fn viewer_annotation(
    msg: &ChatMessage,
    db_scope: Option<(&Connection, &str)>,
) -> ViewerAnnotation {
    let Some((conn, broadcaster_id)) = db_scope.filter(|_| !msg.channel_id.is_empty()) else {
        return ViewerAnnotation::default();
    };
    database::get_viewer_annotation(conn, broadcaster_id, &msg.channel_id)
        .inspect_err(|e| {
            tracing::warn!(
                "読み仮名・メモを引けない（投稿者名で読み上げる） message_id: {}: {}",
                msg.id,
                e
            )
        })
        .unwrap_or_default()
}

/// メッセージを TTS キューに追加する（author_reading は viewer_annotation で引いたもの）
async fn enqueue_tts(tts_manager: &TtsManager, msg: &ChatMessage, author_reading: Option<String>) {
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
        author_reading,
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

    const FRESH: PollOutcome = PollOutcome::Fetched {
        fresh: 1,
        timeout: Some(Duration::from_millis(10_000)),
    };
    const FAILED: PollOutcome = PollOutcome::Failed;

    /// 新しいメッセージが 0 件の成功（timeoutMs はミリ秒、None は応答に無い）
    fn empty(timeout_ms: Option<u64>) -> PollOutcome {
        PollOutcome::Fetched {
            fresh: 0,
            timeout: timeout_ms.map(Duration::from_millis),
        }
    }

    fn schedule_after(outcomes: &[PollOutcome]) -> PollSchedule {
        outcomes
            .iter()
            .fold(PollSchedule::default(), |schedule, &outcome| {
                schedule.after(outcome)
            })
    }

    fn wait_ms(schedule: PollSchedule) -> Option<u128> {
        schedule.next_wait().map(|wait| wait.as_millis())
    }

    /// 空振りを続けたときの、1 回ごとの次の取得までの待ち（ミリ秒）
    fn waits_while_empty(timeout_ms: Option<u64>, count: usize) -> Vec<u128> {
        (1..=count)
            .map(|n| wait_ms(schedule_after(&vec![empty(timeout_ms); n])).unwrap())
            .collect()
    }

    // 02_chat.md「取得の間隔」: 新しいメッセージが届いたら 1.5秒
    #[test]
    fn wait_after_fresh_messages_is_minimum() {
        assert_eq!(wait_ms(schedule_after(&[FRESH])), Some(1500));
    }

    // 02_chat.md「取得の間隔」: timeoutMs 10000 の応答で空振りが続く（10回目から 10秒）
    #[test]
    fn wait_grows_while_empty_up_to_timeout() {
        assert_eq!(
            waits_while_empty(Some(10_000), 12),
            [
                1500, 2500, 3500, 4500, 5500, 6500, 7500, 8500, 9500, 10_000, 10_000, 10_000
            ]
        );
    }

    // 02_chat.md「取得の間隔」: 空振り6回（待ち 6.5秒）のあと新しいメッセージが届く
    #[test]
    fn fresh_messages_reset_the_wait() {
        let mut outcomes = vec![empty(Some(10_000)); 6];
        assert_eq!(wait_ms(schedule_after(&outcomes)), Some(6500));
        outcomes.push(FRESH);
        assert_eq!(wait_ms(schedule_after(&outcomes)), Some(1500));
    }

    // 02_chat.md「取得の間隔」: timeoutMs 3000 の応答で空振りが続く
    #[test]
    fn short_timeout_caps_the_wait() {
        assert_eq!(waits_while_empty(Some(3000), 4), [1500, 2500, 3000, 3000]);
    }

    // 02_chat.md「取得の間隔」: 上限が 1.5秒より短いときは下限を優先する
    #[test]
    fn timeout_below_minimum_keeps_minimum() {
        assert_eq!(waits_while_empty(Some(1000), 4), [1500; 4]);
    }

    // 02_chat.md「取得の間隔」: timeoutMs が無い応答で空振り12回 → 10秒
    #[test]
    fn missing_timeout_uses_default_cap() {
        assert_eq!(waits_while_empty(None, 12)[11], 10_000);
    }

    // 02_chat.md「取得の間隔」: 空振り5回 → 失敗 → 成功して 0 件
    #[test]
    fn failure_does_not_change_empty_count() {
        let mut outcomes = vec![empty(Some(10_000)); 5];
        outcomes.push(FAILED);
        assert_eq!(wait_ms(schedule_after(&outcomes)), Some(1500));
        outcomes.push(empty(Some(10_000)));
        assert_eq!(wait_ms(schedule_after(&outcomes)), Some(6500));
    }

    // 02_chat.md「取得に失敗したとき」の表: 連続失敗回数 → 次の取得までの待ち（None は切断）
    #[test]
    fn wait_after_consecutive_failures_follows_spec_table() {
        let cases = [
            (1, Some(1500)),
            (10, Some(1500)),
            (11, Some(3000)),
            (12, Some(6000)),
            (13, Some(12000)),
            (14, Some(24000)),
            (15, None),
        ];
        for (count, expected) in cases {
            assert_eq!(
                wait_ms(schedule_after(&vec![FAILED; count])),
                expected,
                "{count} 回連続で失敗"
            );
        }
    }

    // 02_chat.md: 失敗12回 → 成功 → 失敗 なら、次の待ちは 1.5秒
    #[test]
    fn success_resets_consecutive_failures() {
        let mut outcomes = vec![FAILED; 12];
        outcomes.extend([FRESH, FAILED]);
        assert_eq!(wait_ms(schedule_after(&outcomes)), Some(1500));
    }

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

    /// 届いた順に id を渡し、処理されたもの（初めて見たもの）だけを返す
    fn processed(capacity: usize, ids: &[&str]) -> Vec<String> {
        let mut seen = RecentMessageIds::new(capacity);
        ids.iter()
            .filter(|id| seen.insert(id))
            .map(|id| id.to_string())
            .collect()
    }

    // 02_chat.md「受信済み message_id の記憶」の例（上限3件）
    #[test]
    fn resent_message_is_dropped() {
        assert_eq!(processed(3, &["A", "B", "A"]), ["A", "B"]);
    }

    #[test]
    fn forgotten_message_is_processed_again() {
        assert_eq!(
            processed(3, &["A", "B", "C", "D", "A"]),
            ["A", "B", "C", "D", "A"]
        );
    }

    #[test]
    fn empty_message_id_is_always_processed() {
        assert_eq!(processed(3, &["", "A", ""]), ["", "A", ""]);
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

    fn text_message(id: &str, channel_id: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            timestamp: "12:00:00".to_string(),
            timestamp_usec: "1000000".to_string(),
            message_type: MessageType::Text,
            author: format!("@{channel_id}"),
            author_icon_url: None,
            channel_id: channel_id.to_string(),
            content: "こんにちは".to_string(),
            runs: vec![],
            metadata: None,
            is_member: false,
            is_first_time_viewer: false,
            in_stream_comment_count: None,
        }
    }

    // 04_tts.md「どの読み仮名を使うか」: 視聴者X の読み仮名は A のもとで「たなか」、B のもとで「タナカ」
    async fn db_with_readings() -> Database {
        let db = Database::new_in_memory().expect("in-memory DB");
        {
            let conn = db.connection().await;
            for (broadcaster, reading) in [("UCaaa", "たなか"), ("UCbbb", "タナカ")] {
                let id =
                    database::upsert_viewer_profile(&conn, broadcaster, "UCxxx", "@田中-abc", None)
                        .unwrap();
                let info = database::ViewerCustomInfo::new(id).with_reading(reading);
                database::upsert_viewer_custom_info(&conn, &info).unwrap();
            }
        }
        db
    }

    #[tokio::test]
    async fn reading_comes_from_broadcaster_of_the_connection() {
        // spec: 配信者A・B を同時に接続。B の接続に X のコメント → タナカ（A の接続なら たなか）
        let db = db_with_readings().await;
        let conn = db.connection().await;
        let msg = text_message("m1", "UCxxx");
        assert_eq!(
            viewer_annotation(&msg, Some((&conn, "UCbbb")))
                .reading
                .as_deref(),
            Some("タナカ")
        );
        assert_eq!(
            viewer_annotation(&msg, Some((&conn, "UCaaa")))
                .reading
                .as_deref(),
            Some("たなか")
        );
    }

    // 02_chat.md「1 行の形」: メモも届いた接続の配信者のもとのもの (B のもとにだけメモがある)
    #[tokio::test]
    async fn notes_come_from_broadcaster_of_the_connection() {
        let db = db_with_readings().await;
        {
            let conn = db.connection().await;
            let id = database::upsert_viewer_profile(&conn, "UCbbb", "UCxxx", "@田中-abc", None)
                .unwrap();
            let info = database::ViewerCustomInfo::new(id)
                .with_reading("タナカ")
                .with_notes("赤スパの人");
            database::upsert_viewer_custom_info(&conn, &info).unwrap();
        }
        let conn = db.connection().await;
        let msg = text_message("m1", "UCxxx");
        assert_eq!(
            viewer_annotation(&msg, Some((&conn, "UCbbb")))
                .notes
                .as_deref(),
            Some("赤スパの人")
        );
        assert_eq!(viewer_annotation(&msg, Some((&conn, "UCaaa"))).notes, None);
    }

    #[tokio::test]
    async fn unidentified_viewer_has_no_reading() {
        // spec: 視聴者を特定できないギフト（channel_id が無い）→ 投稿者名で読む
        let db = db_with_readings().await;
        let conn = db.connection().await;
        let msg = text_message("m1", "");
        assert_eq!(
            viewer_annotation(&msg, Some((&conn, "UCaaa"))),
            ViewerAnnotation::default()
        );
    }

    #[test]
    fn no_reading_without_db_or_broadcaster() {
        assert_eq!(
            viewer_annotation(&text_message("m1", "UCxxx"), None),
            ViewerAnnotation::default()
        );
    }

    // 08_database.md「書き込み」: 1 回のポーリング分をまとめて保存し、1 件の失敗はほかを止めない。
    // 初見判定は保存したあとに同じトランザクションの中で行う
    #[tokio::test]
    async fn batch_saves_messages_and_skips_only_the_failed_one() {
        let db = Database::new_in_memory().expect("in-memory DB");
        let conn = db.connection().await;
        let session_id = database::create_session(
            &conn,
            Some("https://www.youtube.com/watch?v=abc"),
            None,
            Some("UCown"),
            None,
        )
        .unwrap();
        conn.execute_batch(
            "CREATE TRIGGER fail_bad BEFORE INSERT ON messages WHEN NEW.message_id = 'bad'
             BEGIN SELECT RAISE(ABORT, 'test'); END;",
        )
        .unwrap();

        let mut msgs = vec![
            text_message("A", "UCa"),
            text_message("bad", "UCx"),
            text_message("B", "UCb"),
            text_message("A2", "UCa"),
        ];
        let target = MessageTarget {
            video_id: "abc",
            session_id: Some(&session_id),
            broadcaster_id: Some("UCown"),
        };
        process_messages(
            &mut msgs,
            Some(&conn),
            &target,
            &mut HashMap::new(),
            &mut HashMap::new(),
        );

        let saved: Vec<String> = conn
            .prepare("SELECT message_id FROM messages ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(saved, ["A", "B", "A2"]);
        assert!(msgs[0].is_first_time_viewer);
        assert!(msgs[2].is_first_time_viewer);
        assert_eq!(msgs[3].in_stream_comment_count, Some(2));
    }
}
