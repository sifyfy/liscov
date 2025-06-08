use crate::gui::models::GuiChatMessage;
use crate::gui::services::ServiceState;
use crate::io::SaveConfig;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::mpsc;

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

/// アプリケーションの状態
#[derive(Debug, Clone)]
pub struct AppState {
    pub messages: Vec<GuiChatMessage>,
    pub service_state: ServiceState,
    pub is_connected: bool,
    pub is_stopping: bool,
    pub stats: ChatStats,
    pub continuation_token: Option<String>,
    pub current_url: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            service_state: ServiceState::Idle,
            is_connected: false,
            is_stopping: false,
            stats: ChatStats::default(),
            continuation_token: None,
            current_url: None,
        }
    }
}

/// イベント駆動状態マネージャー
pub struct StateManager {
    state: Arc<Mutex<AppState>>,
    event_sender: mpsc::UnboundedSender<AppEvent>,
    is_started: Arc<Mutex<bool>>,
}

impl StateManager {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let state = Arc::new(Mutex::new(AppState::default()));
        let is_started = Arc::new(Mutex::new(false));

        // イベント処理ループをすぐに開始
        let state_clone = Arc::clone(&state);
        let is_started_clone = Arc::clone(&is_started);

        tokio::spawn(async move {
            {
                let mut started = is_started_clone.lock().unwrap();
                if *started {
                    return; // 既に開始されている
                }
                *started = true;
            }

            tracing::debug!("🚀 StateManager event loop started (optimized)");
            Self::run_event_loop(state_clone, event_receiver).await;
        });

        Self {
            state,
            event_sender,
            is_started,
        }
    }

    /// イベント処理ループを実行
    async fn run_event_loop(
        state: Arc<Mutex<AppState>>,
        mut event_receiver: mpsc::UnboundedReceiver<AppEvent>,
    ) {
        while let Some(event) = event_receiver.recv().await {
            Self::handle_event_static(&state, event);
        }
        tracing::info!("🏁 StateManager event loop stopped");
    }

    /// 現在の状態を取得
    pub fn get_state(&self) -> AppState {
        self.state.lock().unwrap().clone()
    }

    /// イベントを送信
    pub fn send_event(&self, event: AppEvent) -> Result<(), mpsc::error::SendError<AppEvent>> {
        // メッセージ追加イベントのログを削減
        match &event {
            AppEvent::MessageAdded(_) => {
                // メッセージ追加は頻繁なため、ログ出力を完全削除
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

    /// イベントを処理して状態を更新（静的メソッド）
    fn handle_event_static(state: &Arc<Mutex<AppState>>, event: AppEvent) {
        let mut state_guard = state.lock().unwrap();

        match event {
            AppEvent::MessageAdded(message) => {
                // メッセージ追加ログを軽量化（デバッグレベルかつ簡潔に）
                tracing::debug!("📝 New message: {}", message.author);
                state_guard.messages.push(message);
                Self::update_stats_static(&mut state_guard);
            }

            AppEvent::MessagesAdded(messages) => {
                tracing::debug!("📬 Added {} messages", messages.len());
                state_guard.messages.extend(messages);
                Self::update_stats_static(&mut state_guard);
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
            }

            AppEvent::ServiceStateChanged(new_state) => {
                tracing::info!("🔄 Service state changed: {:?}", new_state);
                state_guard.service_state = new_state;
            }

            AppEvent::StoppingStateChanged { is_stopping } => {
                tracing::info!("🛑 Stopping state changed: {}", is_stopping);
                state_guard.is_stopping = is_stopping;
            }

            AppEvent::StatsUpdated(new_stats) => {
                tracing::debug!("📊 Stats updated");
                state_guard.stats = new_stats;
            }

            AppEvent::MessagesCleared => {
                tracing::info!("🗑️ Messages cleared");
                state_guard.messages.clear();
                state_guard.stats = ChatStats::default();
            }

            AppEvent::ContinuationTokenUpdated(token) => {
                tracing::debug!("🔄 Continuation token updated");
                state_guard.continuation_token = token;
            }

            AppEvent::CurrentUrlUpdated(url) => {
                tracing::debug!("🔗 Current URL updated: {:?}", url);
                state_guard.current_url = url;
                // URL変更時は継続トークンをクリア（新しい配信のため）
                if state_guard.current_url.is_some() {
                    state_guard.continuation_token = None;
                }
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
            }
        }
    }

    /// 統計情報を更新（静的メソッド）
    fn update_stats_static(state: &mut AppState) {
        state.stats.total_messages = state.messages.len();
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

        // メッセージ数の制限
        if state.messages.len() > 1000 {
            let drain_count = state.messages.len() - 1000;
            state.messages.drain(..drain_count);
            tracing::debug!("🧹 Trimmed {} old messages", drain_count);
        }
    }

    /// 状態マネージャーが開始されているかチェック
    pub fn is_started(&self) -> bool {
        *self.is_started.lock().unwrap()
    }
}

/// グローバル状態マネージャーのインスタンス
static STATE_MANAGER: OnceLock<StateManager> = OnceLock::new();

/// グローバル状態マネージャーを取得（遅延初期化）
pub fn get_state_manager() -> &'static StateManager {
    STATE_MANAGER.get_or_init(|| {
        tracing::debug!("🏗️ Creating global state manager (lazy init)");
        StateManager::new()
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
