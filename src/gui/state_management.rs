use crate::gui::memory_optimized::{ComprehensiveStats, OptimizedMessageManager};
use crate::gui::models::GuiChatMessage;
use crate::gui::services::ServiceState;
use crate::gui::state_broadcaster::{get_broadcaster, StateChange};
use crate::io::SaveConfig;
use crate::LiscovResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::{broadcast, mpsc, RwLock};

/// アプリケーション全体の状態イベント
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// 新しいメッセージが追加された
    MessageAdded(GuiChatMessage),
    /// 複数のメッセージが追加された
    MessagesAdded(Vec<GuiChatMessage>),
    /// 接続状態が変更された
    ConnectionChanged { is_connected: bool },
    /// サービス状態が変更された
    ServiceStateChanged(ServiceState),
    /// 停止処理状態が変更された
    StoppingStateChanged { is_stopping: bool },
    /// 統計情報が更新された
    StatsUpdated(ChatStats),
    /// メッセージがクリアされた
    MessagesCleared,
    /// 継続トークンが更新された
    ContinuationTokenUpdated(Option<String>),
    /// 現在のURLが更新された
    CurrentUrlUpdated(Option<String>),
    /// 保存設定が更新された
    UpdateSaveConfig(SaveConfig),
}

/// チャット統計情報
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChatStats {
    pub total_messages: usize,
    pub messages_per_minute: f64,
    pub uptime_seconds: u64,
    pub last_message_time: Option<chrono::DateTime<chrono::Utc>>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// アプリケーションの状態（メモリ最適化版）
#[derive(Debug)]
pub struct AppState {
    /// メモリ最適化されたメッセージマネージャー
    pub message_manager: OptimizedMessageManager,
    pub service_state: ServiceState,
    pub is_connected: bool,
    pub is_stopping: bool,
    pub stats: ChatStats,
    pub continuation_token: Option<String>,
    pub current_url: Option<String>,
    /// 投稿者ごとのコメント回数（この配信で何回目かをカウント）
    pub author_comment_counts: std::collections::HashMap<String, u32>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        // メッセージマネージャーの内容を手動でクローン
        let mut new_message_manager = OptimizedMessageManager::with_defaults();

        // 既存のメッセージをバッチで新しいマネージャーに追加
        let existing_messages = self.message_manager.messages();
        if !existing_messages.is_empty() {
            new_message_manager.add_messages_batch(existing_messages);
        }

        Self {
            message_manager: new_message_manager,
            service_state: self.service_state.clone(),
            is_connected: self.is_connected,
            is_stopping: self.is_stopping,
            stats: self.stats.clone(),
            continuation_token: self.continuation_token.clone(),
            current_url: self.current_url.clone(),
            author_comment_counts: self.author_comment_counts.clone(),
        }
    }
}

impl AppState {
    /// メッセージ一覧を取得（互換性のため）
    pub fn messages(&self) -> Vec<GuiChatMessage> {
        self.message_manager.messages()
    }

    /// 最新のN件のメッセージを取得
    pub fn recent_messages(&self, n: usize) -> Vec<GuiChatMessage> {
        self.message_manager.recent_messages(n)
    }

    /// メッセージ数を取得
    pub fn message_count(&self) -> usize {
        let stats = self.message_manager.comprehensive_stats();
        stats.message_count
    }

    /// 総処理メッセージ数を取得
    pub fn total_processed_messages(&self) -> usize {
        let stats = self.message_manager.comprehensive_stats();
        stats.total_processed
    }

    /// メモリ統計を取得
    pub fn memory_stats(&self) -> ComprehensiveStats {
        self.message_manager.comprehensive_stats()
    }

    /// メモリ最適化を実行
    pub fn optimize_memory(&mut self) {
        self.message_manager.optimize_memory();
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            message_manager: OptimizedMessageManager::with_defaults(),
            service_state: ServiceState::Idle,
            is_connected: false,
            is_stopping: false,
            stats: ChatStats::default(),
            continuation_token: None,
            current_url: None,
            author_comment_counts: std::collections::HashMap::new(),
        }
    }
}

/// イベント駆動状態マネージャー
///
/// 非ブロッキング設計:
/// - RwLockにより読み取りは並行、書き込みは排他的
/// - AtomicBoolでシンプルなフラグ管理
/// - StateBroadcasterで状態変更をプッシュ通知
pub struct StateManager {
    /// アプリケーション状態（RwLockで非ブロッキング読み取り）
    state: Arc<RwLock<AppState>>,
    /// イベント送信チャネル
    event_sender: mpsc::UnboundedSender<AppEvent>,
    /// 開始フラグ（Atomicで非ブロッキング）
    is_started: Arc<AtomicBool>,
}

impl StateManager {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let state = Arc::new(RwLock::new(AppState::default()));
        let is_started = Arc::new(AtomicBool::new(false));

        // イベント処理ループをすぐに開始
        let state_clone = Arc::clone(&state);
        let is_started_clone = Arc::clone(&is_started);

        tokio::spawn(async move {
            // AtomicBoolでアトミックにフラグをチェック・設定
            if is_started_clone.swap(true, Ordering::SeqCst) {
                tracing::error!("🚨 [STATE_MGR] Event loop already started, returning");
                return; // 既に開始されている
            }

            tracing::info!("StateManager event loop starting (non-blocking version)");
            Self::run_event_loop(state_clone, event_receiver).await;
            tracing::info!("StateManager event loop ended");
        });

        Self {
            state,
            event_sender,
            is_started,
        }
    }

    /// イベント処理ループを実行
    async fn run_event_loop(
        state: Arc<RwLock<AppState>>,
        mut event_receiver: mpsc::UnboundedReceiver<AppEvent>,
    ) {
        tracing::debug!("StateManager event loop ready (async RwLock)");
        let mut event_count = 0;

        while let Some(event) = event_receiver.recv().await {
            event_count += 1;
            tracing::debug!(
                "Processing event #{}: {:?}",
                event_count,
                std::mem::discriminant(&event)
            );
            Self::handle_event_async(&state, event).await;
        }
        tracing::debug!("Event loop stopped after {} events", event_count);
    }

    /// 現在の状態を取得（非同期）
    pub async fn get_state_async(&self) -> AppState {
        self.state.read().await.clone()
    }

    /// 現在の状態を取得（ブロッキング - レガシー互換性のため）
    /// 新しいコードでは get_state_async() を使用してください
    pub fn get_state(&self) -> LiscovResult<AppState> {
        // try_read()でブロッキングなしにロック取得を試みる
        match self.state.try_read() {
            Ok(guard) => Ok(guard.clone()),
            Err(_) => {
                // ロックが取得できない場合はブロッキングで待機
                // 注意: これは非同期コンテキストでは使用しないでください
                tracing::warn!("⚠️ [STATE_MGR] get_state() called with lock contention, consider using get_state_async()");
                Ok(tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.state.read().await.clone()
                    })
                }))
            }
        }
    }

    /// 現在の状態を取得（非安全版・レガシー互換性のため）
    /// 新しいコードでは get_state_async() を使用してください
    pub fn get_state_unchecked(&self) -> AppState {
        match self.get_state() {
            Ok(state) => state,
            Err(e) => {
                tracing::error!("⚠️ State lock error, returning default state: {}", e);
                AppState::default()
            }
        }
    }

    /// ブロードキャスターのサブスクリプションを取得
    ///
    /// 状態変更をリアルタイムで受信するためのReceiverを返す。
    /// ポーリングの代わりにこれを使用することで、UIのブロッキングを回避できる。
    pub fn subscribe(&self) -> broadcast::Receiver<StateChange> {
        get_broadcaster().subscribe()
    }

    #[cfg(test)]
    pub async fn reset_state_for_tests(&self) {
        let mut state = self.state.write().await;
        *state = AppState::default();
    }

    /// イベントを送信
    pub fn send_event(&self, event: AppEvent) -> Result<(), mpsc::error::SendError<AppEvent>> {
        // メッセージ追加イベントのログを削減
        match &event {
            AppEvent::MessageAdded(msg) => {
                // デバッグ用にメッセージ追加ログを一時的に有効化
                tracing::info!(
                    "📤 [STATE_MGR] Receiving MessageAdded event: {} - {}",
                    msg.author,
                    msg.content.chars().take(30).collect::<String>()
                );
            }
            AppEvent::MessagesAdded(messages) => {
                tracing::debug!(
                    "📤 Sending MessagesAdded event: {} messages",
                    messages.len()
                );
            }
            _ => {
                tracing::debug!("📤 Sending event: {:?}", std::mem::discriminant(&event));
            }
        }
        self.event_sender.send(event)
    }

    /// イベントを処理して状態を更新（非同期メソッド）
    async fn handle_event_async(state: &Arc<RwLock<AppState>>, event: AppEvent) {
        // 簡素化ログ：イベント処理開始
        tracing::debug!(
            "StateManager handling event: {:?}",
            std::mem::discriminant(&event)
        );

        let broadcaster = get_broadcaster();
        let mut state_guard = state.write().await;

        match event {
            AppEvent::MessageAdded(mut message) => {
                // メッセージ追加処理の詳細ログ（デバッグ強化版）
                let before_count = state_guard.message_manager.len();
                let before_total = state_guard
                    .message_manager
                    .comprehensive_stats()
                    .total_processed;

                // 投稿者のコメント回数を更新
                let comment_count = {
                    let count = state_guard
                        .author_comment_counts
                        .entry(message.author.clone())
                        .or_insert(0);
                    *count += 1;
                    *count
                };

                // メッセージにコメント回数を設定
                message.comment_count = Some(comment_count);

                tracing::info!(
                    "📝 [STATE_MGR] Received new message: {} - '{}' (#{}, Before: {} in buffer, {} total)",
                    message.author,
                    message.content.chars().take(50).collect::<String>(),
                    comment_count,
                    before_count,
                    before_total
                );

                // メッセージをバッファに追加
                let add_start = std::time::Instant::now();
                state_guard.message_manager.add_message(message.clone());
                let add_duration = add_start.elapsed();

                // 追加後の状態を確認
                let after_count = state_guard.message_manager.len();
                let after_total = state_guard
                    .message_manager
                    .comprehensive_stats()
                    .total_processed;
                let stats = state_guard.message_manager.comprehensive_stats();

                tracing::info!(
                    "📝 [STATE_MGR] Message added in {:?}: Buffer {} → {} (total {} → {}), dropped: {}, memory: {} bytes",
                    add_duration,
                    before_count,
                    after_count,
                    before_total,
                    after_total,
                    stats.dropped_count,
                    stats.memory_stats.used_memory
                );

                // メッセージバッファが期待通りに増加していない場合の警告
                if after_count != before_count + 1 && after_count != before_count {
                    tracing::warn!(
                        "⚠️ [STATE_MGR] Unexpected buffer size change: {} → {} (expected {} or {})",
                        before_count,
                        after_count,
                        before_count + 1,
                        before_count // 循環バッファによる削除の可能性
                    );
                }

                // 統計情報を更新
                let stats_start = std::time::Instant::now();
                Self::update_stats_static(&mut state_guard);
                let stats_duration = stats_start.elapsed();

                tracing::debug!(
                    "📊 [STATE_MGR] Stats updated in {:?}: {} total messages, uptime: {}s",
                    stats_duration,
                    state_guard.stats.total_messages,
                    state_guard.stats.uptime_seconds
                );

                // ブロードキャスト: 新着メッセージを通知
                broadcaster.broadcast(StateChange::MessageAdded {
                    count: after_count,
                    latest: Some(message),
                });
            }

            AppEvent::MessagesAdded(messages) => {
                let added_count = messages.len();
                tracing::debug!("📬 Added {} messages", added_count);
                state_guard.message_manager.add_messages_batch(messages);
                Self::update_stats_static(&mut state_guard);

                // ブロードキャスト: 複数メッセージ追加を通知
                broadcaster.broadcast(StateChange::MessagesAdded {
                    count: state_guard.message_manager.len(),
                    added_count,
                });
            }

            AppEvent::ConnectionChanged { is_connected } => {
                tracing::info!("🔗 Connection changed: {}", is_connected);
                state_guard.is_connected = is_connected;

                // 接続開始時に統計をリセット
                if is_connected && state_guard.stats.start_time.is_none() {
                    state_guard.stats.start_time = Some(chrono::Utc::now());
                    tracing::debug!("⏰ Stats timer started");
                }

                // 接続状態に応じてサービス状態も更新
                if is_connected {
                    state_guard.service_state = ServiceState::Connected;
                } else if matches!(state_guard.service_state, ServiceState::Connected) {
                    state_guard.service_state = ServiceState::Idle;
                }

                // ブロードキャスト: 接続状態変更を通知
                broadcaster.broadcast(StateChange::ConnectionChanged { is_connected });
            }

            AppEvent::ServiceStateChanged(new_state) => {
                tracing::info!("🔄 Service state changed: {:?}", new_state);
                state_guard.service_state = new_state.clone();

                // ブロードキャスト: サービス状態変更を通知
                broadcaster.broadcast(StateChange::ServiceStateChanged(new_state));
            }

            AppEvent::StoppingStateChanged { is_stopping } => {
                tracing::info!("🛑 Stopping state changed: {}", is_stopping);
                state_guard.is_stopping = is_stopping;

                // ブロードキャスト: 停止状態変更を通知
                broadcaster.broadcast(StateChange::StoppingChanged(is_stopping));
            }

            AppEvent::StatsUpdated(new_stats) => {
                tracing::debug!("📊 Stats updated");
                state_guard.stats = new_stats.clone();

                // ブロードキャスト: 統計情報更新を通知
                broadcaster.broadcast(StateChange::StatsUpdated(new_stats));
            }

            AppEvent::MessagesCleared => {
                tracing::info!("🗑️ Messages cleared");
                state_guard.message_manager.clear_all();
                state_guard.stats = ChatStats::default();
                // コメント回数もリセット
                state_guard.author_comment_counts.clear();
                tracing::debug!("🔄 Author comment counts reset");

                // ブロードキャスト: メッセージクリアを通知
                broadcaster.broadcast(StateChange::MessagesCleared);
            }

            AppEvent::ContinuationTokenUpdated(token) => {
                tracing::debug!("🔄 Continuation token updated");
                state_guard.continuation_token = token.clone();

                // ブロードキャスト: 継続トークン更新を通知
                broadcaster.broadcast(StateChange::ContinuationTokenUpdated(token));
            }

            AppEvent::CurrentUrlUpdated(url) => {
                tracing::debug!("🔗 Current URL updated: {:?}", url);
                state_guard.current_url = url.clone();
                // URL変更時は継続トークンをクリア（新しい配信のため）
                if state_guard.current_url.is_some() {
                    state_guard.continuation_token = None;
                    // 新しい配信なのでコメント回数もリセット
                    state_guard.author_comment_counts.clear();
                    tracing::debug!("🔄 Author comment counts reset for new stream");
                }

                // ブロードキャスト: URL更新を通知
                broadcaster.broadcast(StateChange::CurrentUrlUpdated(url));
            }

            AppEvent::UpdateSaveConfig(config) => {
                tracing::info!(
                    "⚙️ Save config update requested: enabled={}, file={}",
                    config.enabled,
                    config.file_path
                );

                // サービスに設定を送信
                let service = crate::gui::services::get_global_service();
                let service_clone = service.clone();
                tokio::spawn(async move {
                    service_clone.lock().await.update_save_config(config).await;
                });
                // 注意: SaveConfig変更はブロードキャストしない（サービス内部の処理のため）
            }
        }
    }

    /// 統計情報を更新（静的メソッド）- メモリ最適化版
    fn update_stats_static(state: &mut AppState) {
        let comprehensive_stats = state.message_manager.comprehensive_stats();

        state.stats.total_messages = comprehensive_stats.total_processed;
        state.stats.last_message_time = Some(chrono::Utc::now());

        // 稼働時間の計算
        if let Some(start_time) = state.stats.start_time {
            let duration = chrono::Utc::now().signed_duration_since(start_time);
            state.stats.uptime_seconds = duration.num_seconds().max(0) as u64;
        }

        // メッセージレートの計算
        if state.stats.uptime_seconds > 0 {
            state.stats.messages_per_minute =
                (state.stats.total_messages as f64) / (state.stats.uptime_seconds as f64 / 60.0);
        }

        // メモリ最適化によるメッセージ制限は自動的に処理される
        if comprehensive_stats.dropped_count > 0 {
            tracing::debug!(
                "🧹 Memory manager: {} messages in buffer, {} total processed, {} dropped",
                comprehensive_stats.message_count,
                comprehensive_stats.total_processed,
                comprehensive_stats.dropped_count
            );
        }
    }

    /// 状態マネージャーが開始されているかチェック（非ブロッキング）
    pub fn is_started(&self) -> bool {
        self.is_started.load(Ordering::SeqCst)
    }
}

/// グローバル状態マネージャーのインスタンス
static STATE_MANAGER: OnceLock<StateManager> = OnceLock::new();

/// グローバル状態マネージャーを取得（遅延初期化）
pub fn get_state_manager() -> &'static StateManager {
    STATE_MANAGER.get_or_init(|| {
        tracing::info!("Creating global StateManager");
        let manager = StateManager::new();
        tracing::info!("Global StateManager ready");
        manager
    })
}

/// 状態マネージャーを初期化（互換性のため残すが不要）
pub async fn initialize_state_manager() {
    let manager = get_state_manager();
    tracing::info!(
        "✅ StateManager is ready (started: {})",
        manager.is_started()
    );
}
