//! Dioxus 0.6.3準拠の統一アプリケーションコンテキスト
//!
//! Phase 2.1実装: GLOBAL_LIVE_CHATとSTATE_MANAGERの統合
//! Dioxus推奨の単一コンテキストパターンを採用

use dioxus::prelude::*;
use tokio::sync::mpsc;

use crate::gui::{
    memory_optimized::{ComprehensiveStats, OptimizedMessageManager},
    models::GuiChatMessage,
    services::ServiceState,
    state_management::{AppEvent, ChatStats},
};

/// Dioxus推奨の統一アプリケーションコンテキスト
#[derive(Clone, Copy)]
pub struct AppContext {
    /// ライブチャット状態（旧GLOBAL_LIVE_CHAT）
    pub live_chat: Signal<LiveChatState>,
    /// メッセージストリーム状態（旧STATE_MANAGER）  
    pub message_stream: Signal<MessageStreamState>,
    /// UI状態
    pub ui_state: Signal<UiState>,
    /// 統計情報
    pub stats: Signal<ChatStats>,
}

/// ライブチャット状態（旧GlobalLiveChatState統合）
#[derive(Debug, Clone, PartialEq)]
pub struct LiveChatState {
    pub service_state: ServiceState,
    pub is_connected: bool,
    pub is_stopping: bool,
    pub current_url: Option<String>,
    pub continuation_token: Option<String>,
}

impl Default for LiveChatState {
    fn default() -> Self {
        Self {
            service_state: ServiceState::Idle,
            is_connected: false,
            is_stopping: false,
            current_url: None,
            continuation_token: None,
        }
    }
}

/// メッセージストリーム状態（旧AppState統合）
#[derive(Debug)]
pub struct MessageStreamState {
    /// メモリ最適化されたメッセージマネージャー
    pub message_manager: OptimizedMessageManager,
    /// 投稿者ごとのコメント回数
    pub author_comment_counts: std::collections::HashMap<String, u32>,
    /// 🚀 差分更新システム: 新着メッセージ
    pub new_message: Option<GuiChatMessage>,
    /// 🚀 差分更新システム: メッセージ追加イベントカウンター
    pub message_added_event: u64,
}

impl Default for MessageStreamState {
    fn default() -> Self {
        Self {
            message_manager: OptimizedMessageManager::with_defaults(),
            author_comment_counts: std::collections::HashMap::new(),
            new_message: None,
            message_added_event: 0,
        }
    }
}

impl Clone for MessageStreamState {
    fn clone(&self) -> Self {
        // OptimizedMessageManagerは手動でクローン
        let mut new_message_manager = OptimizedMessageManager::with_defaults();
        let existing_messages = self.message_manager.messages();
        if !existing_messages.is_empty() {
            new_message_manager.add_messages_batch(existing_messages);
        }
        
        Self {
            message_manager: new_message_manager,
            author_comment_counts: self.author_comment_counts.clone(),
            new_message: self.new_message.clone(),
            message_added_event: self.message_added_event,
        }
    }
}

impl MessageStreamState {
    /// メッセージ一覧を取得
    pub fn messages(&self) -> Vec<GuiChatMessage> {
        self.message_manager.messages()
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
}

/// UI状態
#[derive(Debug, Clone, PartialEq)]
pub struct UiState {
    pub show_filter_panel: bool,
    pub auto_scroll_enabled: bool,
    pub show_timestamps: bool,
    pub highlight_enabled: bool,
    pub message_font_size: f32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_filter_panel: false,
            auto_scroll_enabled: true,
            show_timestamps: true,
            highlight_enabled: true,
            message_font_size: 14.0,
        }
    }
}

/// 統一されたアプリケーションコンテキストプロバイダー
#[component]
pub fn AppContextProvider(children: Element) -> Element {
    // 統一状態の初期化
    let live_chat = use_signal(LiveChatState::default);
    let message_stream = use_signal(MessageStreamState::default);
    let ui_state = use_signal(UiState::default);
    let stats = use_signal(ChatStats::default);

    let app_context = AppContext {
        live_chat,
        message_stream,
        ui_state,
        stats,
    };

    // イベント処理システムの初期化
    use_effect(move || {
        let live_chat_clone = live_chat;
        let message_stream_clone = message_stream;
        let stats_clone = stats;

        spawn(async move {
            let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<AppEvent>();
            
            // グローバルイベントハンドラーを設定
            GLOBAL_EVENT_SENDER.set(event_sender).ok();

            tracing::info!("🚀 [APP_CONTEXT] Unified event processing system started");

            // イベント処理ループ
            while let Some(event) = event_receiver.recv().await {
                handle_unified_event(
                    event,
                    live_chat_clone,
                    message_stream_clone,
                    stats_clone,
                ).await;
            }
        });
    });

    // コンテキストを提供
    use_context_provider(|| app_context);
    
    children
}

/// グローバルイベント送信者（旧STATE_MANAGER代替）
static GLOBAL_EVENT_SENDER: std::sync::OnceLock<mpsc::UnboundedSender<AppEvent>> = std::sync::OnceLock::new();

/// イベントを送信（旧StateManager::send_eventの代替）
pub fn send_app_event(event: AppEvent) -> Result<(), String> {
    if let Some(sender) = GLOBAL_EVENT_SENDER.get() {
        sender.send(event).map_err(|e| e.to_string())
    } else {
        Err("Event system not initialized".to_string())
    }
}

/// 統一イベント処理（旧StateManager::handle_event_staticの代替）
async fn handle_unified_event(
    event: AppEvent,
    mut live_chat: Signal<LiveChatState>,
    mut message_stream: Signal<MessageStreamState>,
    mut stats: Signal<ChatStats>,
) {
    tracing::debug!(
        "🚀 [APP_CONTEXT] Processing unified event: {:?}",
        std::mem::discriminant(&event)
    );

    match event {
        AppEvent::MessageAdded(mut message) => {
            // メッセージ追加処理（差分更新システム統合）
            message_stream.with_mut(|stream_state| {
                let before_count = stream_state.message_manager.len();
                let before_total = stream_state.message_manager.comprehensive_stats().total_processed;

                // 投稿者のコメント回数を更新
                let comment_count = {
                    let count = stream_state
                        .author_comment_counts
                        .entry(message.author.clone())
                        .or_insert(0);
                    *count += 1;
                    *count
                };

                message.comment_count = Some(comment_count);

                tracing::info!(
                    "📝 [APP_CONTEXT] New message: {} - '{}' (#{}, Before: {} in buffer, {} total)",
                    message.author,
                    message.content.chars().take(50).collect::<String>(),
                    comment_count,
                    before_count,
                    before_total
                );

                // メッセージをバッファに追加
                stream_state.message_manager.add_message(message.clone());

                // 🚀 差分更新システム: 新着メッセージ設定
                stream_state.new_message = Some(message.clone());
                stream_state.message_added_event += 1;

                let after_count = stream_state.message_manager.len();
                let after_total = stream_state.message_manager.comprehensive_stats().total_processed;
                let memory_stats = stream_state.message_manager.comprehensive_stats();

                tracing::info!(
                    "📝 [APP_CONTEXT] Message added: Buffer {} → {} (total {} → {}), memory: {} bytes",
                    before_count,
                    after_count,
                    before_total,
                    after_total,
                    memory_stats.memory_stats.used_memory
                );
            });

            // 統計情報更新
            update_stats(stats);
        }

        AppEvent::MessagesAdded(messages) => {
            message_stream.with_mut(|stream_state| {
                stream_state.message_manager.add_messages_batch(messages);
            });
            update_stats(stats);
        }

        AppEvent::ConnectionChanged { is_connected } => {
            live_chat.with_mut(|chat_state| {
                chat_state.is_connected = is_connected;
                
                // 接続開始時に統計をリセット
                if is_connected && stats.read().start_time.is_none() {
                    stats.with_mut(|stats_state| {
                        stats_state.start_time = Some(chrono::Utc::now());
                    });
                }

                // 接続状態に応じてサービス状態も更新
                if is_connected {
                    chat_state.service_state = ServiceState::Connected;
                } else if matches!(chat_state.service_state, ServiceState::Connected) {
                    chat_state.service_state = ServiceState::Idle;
                }
            });

            tracing::info!("🔗 [APP_CONTEXT] Connection changed: {}", is_connected);
        }

        AppEvent::ServiceStateChanged(new_state) => {
            live_chat.with_mut(|chat_state| {
                chat_state.service_state = new_state.clone();
            });
            tracing::info!("🔄 [APP_CONTEXT] Service state changed: {:?}", new_state);
        }

        AppEvent::StoppingStateChanged { is_stopping } => {
            live_chat.with_mut(|chat_state| {
                chat_state.is_stopping = is_stopping;
            });
            tracing::info!("🛑 [APP_CONTEXT] Stopping state changed: {}", is_stopping);
        }

        AppEvent::StatsUpdated(new_stats) => {
            stats.set(new_stats);
        }

        AppEvent::MessagesCleared => {
            message_stream.with_mut(|stream_state| {
                stream_state.message_manager.clear_all();
                stream_state.author_comment_counts.clear();
                stream_state.new_message = None;
                stream_state.message_added_event = 0;
            });
            stats.set(ChatStats::default());
            tracing::info!("🗑️ [APP_CONTEXT] Messages cleared");
        }

        AppEvent::ContinuationTokenUpdated(token) => {
            live_chat.with_mut(|chat_state| {
                chat_state.continuation_token = token;
            });
        }

        AppEvent::CurrentUrlUpdated(url) => {
            live_chat.with_mut(|chat_state| {
                chat_state.current_url = url.clone();
                // URL変更時は継続トークンをクリア
                if url.is_some() {
                    chat_state.continuation_token = None;
                }
            });
            // 新しい配信なのでコメント回数もリセット
            message_stream.with_mut(|stream_state| {
                stream_state.author_comment_counts.clear();
            });
            tracing::info!("🔗 [APP_CONTEXT] Current URL updated: {:?}", url);
        }

        AppEvent::UpdateSaveConfig(config) => {
            tracing::info!(
                "⚙️ [APP_CONTEXT] Save config update: enabled={}, file={}",
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

/// 統計情報更新
fn update_stats(mut stats: Signal<ChatStats>) {
    stats.with_mut(|stats_state| {
        // 実装は旧StateManager::update_stats_staticを参考
        stats_state.last_message_time = Some(chrono::Utc::now());
        
        // 稼働時間の計算
        if let Some(start_time) = stats_state.start_time {
            let duration = chrono::Utc::now().signed_duration_since(start_time);
            stats_state.uptime_seconds = duration.num_seconds().max(0) as u64;
        }

        // メッセージレート計算は、メッセージ数が確定後に実装
    });
}

/// Dioxus推奨のコンテキスト使用フック
pub fn use_app_context() -> AppContext {
    use_context::<AppContext>()
}

/// 旧use_live_chatの代替（後方互換性）
pub fn use_unified_live_chat() -> LiveChatHandle {
    let app_context = use_app_context();
    
    // 後方互換性のためのハンドル作成
    LiveChatHandle {
        live_chat_state: app_context.live_chat,
        message_stream_state: app_context.message_stream,
        stats: app_context.stats,
    }
}

/// 統一されたライブチャットハンドル（旧LiveChatHandle代替）
#[derive(Clone, Copy)]
pub struct LiveChatHandle {
    pub live_chat_state: Signal<LiveChatState>,
    pub message_stream_state: Signal<MessageStreamState>,
    pub stats: Signal<ChatStats>,
}

impl LiveChatHandle {
    /// ライブチャット監視を開始
    pub fn start_monitoring(&self, url: String, output_file: Option<String>) {
        let mut live_chat_signal = self.live_chat_state;
        
        spawn(async move {
            // 開始時に停止フラグをリセット
            live_chat_signal.with_mut(|state| {
                state.is_stopping = false;
            });

            let service_arc = crate::gui::services::get_global_service().clone();

            // 接続中状態に更新
            live_chat_signal.with_mut(|state| {
                state.service_state = ServiceState::Connecting;
                state.is_connected = false;
            });

            // イベント送信
            let _ = send_app_event(AppEvent::ServiceStateChanged(ServiceState::Connecting));
            let _ = send_app_event(AppEvent::ConnectionChanged { is_connected: false });

            let result = {
                let mut service = service_arc.lock().await;
                service.start_monitoring(&url, output_file).await
            };

            match result {
                Ok(_) => {
                    live_chat_signal.with_mut(|state| {
                        state.service_state = ServiceState::Connected;
                        state.is_connected = true;
                        state.current_url = Some(url);
                    });

                    let _ = send_app_event(AppEvent::ServiceStateChanged(ServiceState::Connected));
                    let _ = send_app_event(AppEvent::ConnectionChanged { is_connected: true });
                    
                    tracing::info!("✅ [APP_CONTEXT] Live chat monitoring started");
                }
                Err(e) => {
                    let error_message = format!("❌ 監視開始エラー: {}", e);
                    let error_state = ServiceState::Error(error_message);
                    
                    live_chat_signal.with_mut(|state| {
                        state.service_state = error_state.clone();
                        state.is_connected = false;
                    });

                    let _ = send_app_event(AppEvent::ServiceStateChanged(error_state));
                    let _ = send_app_event(AppEvent::ConnectionChanged { is_connected: false });
                    
                    tracing::error!("❌ [APP_CONTEXT] Failed to start monitoring: {}", e);
                }
            }
        });
    }

    /// ライブチャット監視を停止
    pub fn stop_monitoring(&self) {
        let mut live_chat_signal = self.live_chat_state;
        
        spawn(async move {
            // 即座に停止処理中フラグを設定
            live_chat_signal.with_mut(|state| {
                state.is_stopping = true;
            });

            let _ = send_app_event(AppEvent::StoppingStateChanged { is_stopping: true });

            let service_arc = crate::gui::services::get_global_service().clone();
            
            let result = {
                let mut service = service_arc.lock().await;
                service.stop_monitoring().await
            };

            if let Err(e) = result {
                tracing::error!("Error stopping monitoring: {}", e);
            }

            live_chat_signal.with_mut(|state| {
                state.service_state = ServiceState::Idle;
                state.is_connected = false;
                state.is_stopping = false;
            });

            let _ = send_app_event(AppEvent::ServiceStateChanged(ServiceState::Idle));
            let _ = send_app_event(AppEvent::ConnectionChanged { is_connected: false });
            let _ = send_app_event(AppEvent::StoppingStateChanged { is_stopping: false });
            
            tracing::info!("⏹️ [APP_CONTEXT] Live chat monitoring stopped");
        });
    }

    /// メッセージをクリア
    pub fn clear_messages(&self) {
        let _ = send_app_event(AppEvent::MessagesCleared);
        tracing::info!("🗑️ [APP_CONTEXT] Messages cleared via handle");
    }
}