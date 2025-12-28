//! LiveChatサービス用カスタムフック
//!
//! Phase 3実装: 既存LiveChatServiceとDioxusコンポーネントの統合

use dioxus::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};

use crate::api::youtube::ChatMode;
use crate::gui::{
    models::{GuiChatMessage, MessageType},
    services::{LiveChatService, ServiceState},
    signal_manager::use_optimized_signals,
    state_management::{get_state_manager, AppEvent, ChatStats},
};

/// グローバルライブチャット状態
pub struct GlobalLiveChatState {
    pub service: Arc<Mutex<LiveChatService>>,
    pub stopping: bool,
}

impl GlobalLiveChatState {
    fn new() -> Self {
        Self {
            service: Arc::new(Mutex::new(LiveChatService::new())),
            stopping: false,
        }
    }
}

/// グローバルライブチャット状態のシングルトン（サービスのみ管理）
pub static GLOBAL_LIVE_CHAT: OnceLock<Arc<Mutex<GlobalLiveChatState>>> = OnceLock::new();

/// ライブチャットハンドル
#[derive(Clone)]
pub struct LiveChatHandle {
    pub messages: Signal<Vec<GuiChatMessage>>, // 後方互換性のため保持
    pub new_message: Signal<Option<GuiChatMessage>>, // 新着メッセージのみ
    pub message_added_event: Signal<u64>,      // メッセージ追加イベント (カウンター)
    pub state: Signal<ServiceState>,
    pub is_connected: Signal<bool>,
    pub stats: Signal<ChatStats>,
    pub is_stopping: Signal<bool>,
    /// 現在のチャットモード（トップチャット or すべてのチャット）
    pub chat_mode: Signal<ChatMode>,
}

impl PartialEq for LiveChatHandle {
    fn eq(&self, _other: &Self) -> bool {
        // Signalの比較は困難なので、常にfalseとして扱う
        // これによりpropsの変更が検出される
        false
    }
}

impl LiveChatHandle {
    /// ライブチャット監視を開始
    pub fn start_monitoring(&self, url: String, output_file: Option<String>) {
        let mut state = self.state;
        let mut is_connected = self.is_connected;
        let mut is_stopping = self.is_stopping;

        // 開始時に停止フラグをリセット
        is_stopping.set(false);

        spawn(async move {
            // グローバルサービスを使用（設定が共有される）
            let service_arc = crate::gui::services::get_global_service().clone();

            // グローバル状態を初期化（停止フラグ管理用）
            let global_state =
                GLOBAL_LIVE_CHAT.get_or_init(|| Arc::new(Mutex::new(GlobalLiveChatState::new())));
            {
                let mut guard = match global_state.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => {
                        tracing::error!("⚠️ Global live chat state mutex poisoned, recovering");
                        poisoned.into_inner()
                    }
                };
                guard.stopping = false; // 停止フラグをリセット
            }

            // StateManagerに状態変更を通知
            let state_manager = get_state_manager();

            state.set(ServiceState::Connecting);
            is_connected.set(false);
            let _ =
                state_manager.send_event(AppEvent::ServiceStateChanged(ServiceState::Connecting));
            let _ = state_manager.send_event(AppEvent::ConnectionChanged {
                is_connected: false,
            });

            let result = {
                let mut service = service_arc.lock().await;
                service.start_monitoring(&url, output_file).await
            };

            match result {
                Ok(_) => {
                    tracing::info!("✅ Live chat monitoring started");
                    state.set(ServiceState::Connected);
                    is_connected.set(true);

                    // StateManagerに成功状態を通知
                    let _ = state_manager
                        .send_event(AppEvent::ServiceStateChanged(ServiceState::Connected));
                    let _ = state_manager
                        .send_event(AppEvent::ConnectionChanged { is_connected: true });
                }
                Err(e) => {
                    let error_message = e.to_string();
                    tracing::error!("❌ Failed to start monitoring: {}", error_message);

                    // エラーメッセージに基づいて適切なアドバイスを提供
                    let user_message = if error_message.contains("continuation not found") {
                        "❌ YouTubeライブ配信が見つかりません。\n\n考えられる原因:\n• 配信が終了している\n• URLが間違っている\n• 配信がプライベートまたは制限されている\n• チャットが無効になっている\n\n✅ 解決方法:\n• 現在進行中のライブ配信URLを使用してください\n• URLが正確であることを確認してください".to_string()
                    } else if error_message.contains("network") || error_message.contains("timeout")
                    {
                        "❌ ネットワーク接続エラー\n\n• インターネット接続を確認してください\n• ファイアウォールがブロックしていないか確認してください".to_string()
                    } else if error_message.contains("rate limit") {
                        "❌ API制限に達しました\n\n• しばらく待ってから再試行してください\n• 短時間での連続アクセスを避けてください".to_string()
                    } else {
                        format!("❌ 監視開始エラー: {}", error_message)
                    };

                    let error_state = ServiceState::Error(user_message.clone());
                    state.set(error_state.clone());
                    is_connected.set(false);

                    // StateManagerにエラー状態を通知
                    let _ = state_manager.send_event(AppEvent::ServiceStateChanged(error_state));
                    let _ = state_manager.send_event(AppEvent::ConnectionChanged {
                        is_connected: false,
                    });
                }
            }
        });
    }

    /// ライブチャット監視を停止
    pub fn stop_monitoring(&self) {
        let mut state = self.state;
        let mut is_connected = self.is_connected;
        let mut is_stopping = self.is_stopping;

        // 即座に停止処理中フラグを設定（UIに瞬時に反映）
        is_stopping.set(true);

        spawn(async move {
            // グローバル状態をチェックして、既に停止処理中なら何もしない
            if let Some(global_state) = GLOBAL_LIVE_CHAT.get() {
                {
                    let mut guard = match global_state.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => {
                            tracing::error!(
                                "⚠️ Global live chat state mutex poisoned during stop, recovering"
                            );
                            poisoned.into_inner()
                        }
                    };
                    if guard.stopping {
                        tracing::debug!("Stop already in progress, skipping");
                        return;
                    }
                    guard.stopping = true; // 停止処理中フラグを設定
                }

                tracing::info!("⏹️ Stopping live chat monitoring");

                let service_arc = crate::gui::services::get_global_service().clone();

                // StateManagerに停止状態を通知
                let state_manager = get_state_manager();
                let _ =
                    state_manager.send_event(AppEvent::StoppingStateChanged { is_stopping: true });

                // サービスを停止
                let result = {
                    let mut service = service_arc.lock().await;
                    service.stop_monitoring().await
                };

                if let Err(e) = result {
                    tracing::error!("Error stopping monitoring: {}", e);
                }

                // グローバル状態を更新
                {
                    let mut guard = match global_state.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => {
                            tracing::error!("⚠️ Global live chat state mutex poisoned during cleanup, recovering");
                            poisoned.into_inner()
                        }
                    };
                    guard.stopping = false; // 停止処理完了
                }

                state.set(ServiceState::Idle);
                is_connected.set(false);
                is_stopping.set(false);

                // StateManagerに完了状態を通知
                let _ = state_manager.send_event(AppEvent::ServiceStateChanged(ServiceState::Idle));
                let _ = state_manager.send_event(AppEvent::ConnectionChanged {
                    is_connected: false,
                });
                let _ =
                    state_manager.send_event(AppEvent::StoppingStateChanged { is_stopping: false });
            }
        });
    }

    /// ライブチャット監視の一時停止（継続トークンを保持）
    pub fn pause_monitoring(&self) {
        let mut state = self.state;
        let mut is_connected = self.is_connected;

        spawn(async move {
            if let Some(_global_state) = GLOBAL_LIVE_CHAT.get() {
                let service_arc = crate::gui::services::get_global_service().clone();

                tracing::info!("⏸️ Pausing live chat monitoring");

                let result = {
                    let mut service = service_arc.lock().await;
                    service.pause_monitoring().await
                };

                match result {
                    Ok(()) => {
                        state.set(ServiceState::Paused);
                        is_connected.set(false);

                        // StateManagerに一時停止状態を通知
                        let state_manager = get_state_manager();
                        let _ = state_manager
                            .send_event(AppEvent::ServiceStateChanged(ServiceState::Paused));
                        let _ = state_manager.send_event(AppEvent::ConnectionChanged {
                            is_connected: false,
                        });

                        tracing::info!("✅ Live chat monitoring paused");
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to pause monitoring: {}", e);
                    }
                }
            }
        });
    }

    /// ライブチャット監視の再開（保存された継続トークンから）
    pub fn resume_monitoring(&self, output_file: Option<String>) {
        let mut state = self.state;
        let mut is_connected = self.is_connected;

        spawn(async move {
            if let Some(_global_state) = GLOBAL_LIVE_CHAT.get() {
                let service_arc = crate::gui::services::get_global_service().clone();

                tracing::info!("▶️ Resuming live chat monitoring");
                state.set(ServiceState::Connecting);

                let state_manager = get_state_manager();
                let _ = state_manager
                    .send_event(AppEvent::ServiceStateChanged(ServiceState::Connecting));

                let result = {
                    let mut service = service_arc.lock().await;
                    service.resume_monitoring(output_file).await
                };

                match result {
                    Ok(_) => {
                        state.set(ServiceState::Connected);
                        is_connected.set(true);

                        // StateManagerに再開成功状態を通知
                        let _ = state_manager
                            .send_event(AppEvent::ServiceStateChanged(ServiceState::Connected));
                        let _ = state_manager
                            .send_event(AppEvent::ConnectionChanged { is_connected: true });

                        tracing::info!("✅ Live chat monitoring resumed");
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to resume monitoring: {}", e);

                        // 再開失敗時は再開ボタンのままにする（ユーザー要件）
                        state.set(ServiceState::Paused);

                        let _ = state_manager
                            .send_event(AppEvent::ServiceStateChanged(ServiceState::Paused));

                        // 継続トークンが無効な場合の特別処理
                        if e.to_string().contains("continuation")
                            || e.to_string().contains("token")
                            || e.to_string().contains("invalid")
                        {
                            tracing::warn!(
                                "⚠️ Continuation token may be invalid. User should choose action."
                            );
                            // TODO: ユーザー通知とアクション選択UIを実装
                        }
                    }
                }
            }
        });
    }

    /// メッセージをクリア
    pub fn clear_messages(&self) {
        let mut messages = self.messages;
        let mut stats = self.stats;

        // ローカル状態をクリア
        messages.set(Vec::new());
        stats.set(ChatStats::default());

        // StateManagerに通知
        let state_manager = get_state_manager();
        let _ = state_manager.send_event(AppEvent::MessagesCleared);

        tracing::info!("🗑️ Messages cleared via LiveChatHandle");
    }

    /// テストメッセージを追加
    pub fn add_test_message(&self, author: &str, content: &str, message_type: MessageType) {
        let now = chrono::Utc::now();
        let timestamp_usec = now.timestamp_micros().to_string();
        let display_timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

        let test_message = GuiChatMessage {
            id: format!("test_{}", timestamp_usec),
            timestamp: display_timestamp,
            timestamp_usec,
            message_type,
            author: author.to_string(),
            author_icon_url: None, // テストメッセージにはアイコンなし
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            runs: Vec::new(), // テストメッセージは通常テキストのみ
            metadata: None,
            is_member: false,
            comment_count: None, // テストメッセージには回数なし
        };

        tracing::info!(
            "🧪 Adding test message: {} - {}",
            test_message.author,
            test_message.content
        );

        // StateManagerに追加
        let state_manager = get_state_manager();
        let _ = state_manager.send_event(AppEvent::MessageAdded(test_message.clone()));

        // ローカル状態の更新はStateManagerから自動的に同期される
        // 直接的なローカル状態更新は不要
    }

    /// チャットモードを設定
    ///
    /// トップチャット (TopChat): フィルタリングされた重要なメッセージのみ
    /// すべてのチャット (AllChat): すべてのメッセージを表示
    pub fn set_chat_mode(&self, mode: ChatMode) {
        let mut chat_mode = self.chat_mode;

        tracing::info!("🔄 Setting chat mode to: {}", mode);

        spawn(async move {
            let service_arc = crate::gui::services::get_global_service().clone();
            let mut service = service_arc.lock().await;

            match service.change_chat_mode(mode).await {
                Ok(true) => {
                    chat_mode.set(mode);
                    tracing::info!("✅ Chat mode changed successfully to: {}", mode);
                }
                Ok(false) => {
                    tracing::warn!("⚠️ Chat mode {} not available", mode);
                }
                Err(e) => {
                    tracing::error!("❌ Failed to change chat mode: {}", e);
                }
            }
        });
    }

    /// 現在のチャットモードを取得
    pub fn get_chat_mode(&self) -> ChatMode {
        *self.chat_mode.read()
    }
}

/// LiveChatサービス用カスタムフック
///
/// シンプルで確実な同期システム
pub fn use_live_chat() -> LiveChatHandle {
    tracing::debug!("use_live_chat hook called");

    // StateManagerから初期値を取得（遅延初期化）
    let state_manager = get_state_manager();
    let initial_state = state_manager.get_state_unchecked();

    // 初期値を事前にクローン（移動問題を回避）
    let initial_messages = initial_state.messages();
    let initial_service_state = initial_state.service_state.clone();
    let initial_is_connected = initial_state.is_connected;
    let initial_stats = initial_state.stats.clone();
    let initial_is_stopping = initial_state.is_stopping;

    // Signalを初期化（StateManagerの現在値で初期化）
    let messages = use_signal(move || {
        tracing::debug!(
            "📨 Initializing messages signal with {} messages",
            initial_messages.len()
        );
        initial_messages.clone()
    });
    let state = use_signal(move || {
        tracing::debug!("🔄 Initializing state signal: {:?}", initial_service_state);
        initial_service_state.clone()
    });
    let is_connected = use_signal(move || {
        tracing::debug!(
            "🔗 Initializing connection signal: {}",
            initial_is_connected
        );
        initial_is_connected
    });
    let stats = use_signal(move || {
        tracing::debug!("📊 Initializing stats signal");
        initial_stats.clone()
    });

    // 差分更新システム用のSignal初期化
    let new_message = use_signal(|| None::<GuiChatMessage>);
    let message_added_event = use_signal(|| 0u64);
    let is_stopping = use_signal(move || {
        tracing::debug!("🛑 Initializing stopping signal: {}", initial_is_stopping);
        initial_is_stopping
    });

    // チャットモードのSignal初期化
    let chat_mode = use_signal(|| {
        tracing::debug!("🎯 Initializing chat mode signal with default: {:?}", ChatMode::default());
        ChatMode::default()
    });

    tracing::debug!("✅ All signals initialized (optimized)");

    // Phase 2.3: 最適化されたSignal管理システムを初期化
    let _optimized_signals = use_optimized_signals();

    // 🎯 Phase 2.3: イベント駆動型同期（ポーリング廃止）
    use_effect(move || {
        let mut messages_clone = messages;
        let mut new_message_clone = new_message;
        let mut message_added_event_clone = message_added_event;
        let mut state_clone = state;
        let mut is_connected_clone = is_connected;
        let mut stats_clone = stats;
        let mut is_stopping_clone = is_stopping;

        tracing::info!("🎯 [EVENT_SYNC] Event-driven sync initialized (no polling)");

        // イベント駆動型同期: StateManager → UI Signals
        spawn(async move {
            use crate::gui::state_broadcaster::StateChange;

            // StateManagerからブロードキャストサブスクリプションを取得
            let mut rx = get_state_manager().subscribe();
            let mut event_count = 0u64;

            tracing::info!("📡 [EVENT_SYNC] Subscribed to state broadcaster");

            loop {
                // イベントを非同期で待機（ブロッキングなし）
                match rx.recv().await {
                    Ok(change) => {
                        event_count += 1;

                        match change {
                            StateChange::MessageAdded { count, latest } => {
                                tracing::debug!(
                                    "📬 [EVENT_SYNC] MessageAdded event #{}: count={}",
                                    event_count,
                                    count
                                );

                                // 最新メッセージを更新
                                if let Some(msg) = latest {
                                    new_message_clone.set(Some(msg));
                                    let current_event_count = message_added_event_clone();
                                    message_added_event_clone.set(current_event_count + 1);
                                }

                                // メッセージリストを非同期で取得して更新
                                let current_messages =
                                    get_state_manager().get_state_async().await.messages();
                                messages_clone.set(current_messages);
                            }

                            StateChange::MessagesAdded { count, added_count } => {
                                tracing::debug!(
                                    "📬 [EVENT_SYNC] MessagesAdded event #{}: {} added, total={}",
                                    event_count,
                                    added_count,
                                    count
                                );

                                // メッセージリストを更新
                                let current_messages =
                                    get_state_manager().get_state_async().await.messages();
                                messages_clone.set(current_messages);
                            }

                            StateChange::MessagesCleared => {
                                tracing::info!("🗑️ [EVENT_SYNC] MessagesCleared event #{}", event_count);
                                messages_clone.set(Vec::new());
                                new_message_clone.set(None);
                            }

                            StateChange::ConnectionChanged { is_connected: connected } => {
                                tracing::info!(
                                    "🔗 [EVENT_SYNC] ConnectionChanged event #{}: {}",
                                    event_count,
                                    connected
                                );
                                is_connected_clone.set(connected);
                            }

                            StateChange::ServiceStateChanged(new_state) => {
                                tracing::info!(
                                    "🔄 [EVENT_SYNC] ServiceStateChanged event #{}: {:?}",
                                    event_count,
                                    new_state
                                );
                                state_clone.set(new_state);
                            }

                            StateChange::StoppingChanged(stopping) => {
                                tracing::info!(
                                    "🛑 [EVENT_SYNC] StoppingChanged event #{}: {}",
                                    event_count,
                                    stopping
                                );
                                is_stopping_clone.set(stopping);
                            }

                            StateChange::StatsUpdated(new_stats) => {
                                tracing::debug!(
                                    "📊 [EVENT_SYNC] StatsUpdated event #{}: {} msgs",
                                    event_count,
                                    new_stats.total_messages
                                );
                                stats_clone.set(new_stats);
                            }

                            StateChange::ContinuationTokenUpdated(_) |
                            StateChange::CurrentUrlUpdated(_) => {
                                // これらのイベントはUI表示に影響しないので無視
                                tracing::debug!(
                                    "🔧 [EVENT_SYNC] Internal event #{} (ignored)",
                                    event_count
                                );
                            }
                        }

                        // 100イベントごとのステータスログ
                        if event_count % 100 == 0 {
                            tracing::info!(
                                "💓 [EVENT_SYNC] Processed {} events",
                                event_count
                            );
                        }
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        // サブスクライバーが遅延してイベントがスキップされた場合
                        tracing::warn!(
                            "⚠️ [EVENT_SYNC] Lagged: skipped {} events, resyncing state",
                            skipped
                        );

                        // 完全な状態を再同期
                        let current_state = get_state_manager().get_state_async().await;
                        messages_clone.set(current_state.messages());
                        state_clone.set(current_state.service_state);
                        is_connected_clone.set(current_state.is_connected);
                        stats_clone.set(current_state.stats);
                        is_stopping_clone.set(current_state.is_stopping);
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // ブロードキャスターがクローズされた場合（通常は発生しない）
                        tracing::error!("❌ [EVENT_SYNC] Broadcaster closed, sync loop ended");
                        break;
                    }
                }
            }
        });
    });

    tracing::debug!("🎯 use_live_chat hook completed, returning handle");

    LiveChatHandle {
        messages,
        new_message,
        message_added_event,
        state,
        is_connected,
        stats,
        is_stopping,
        chat_mode,
    }
}

/// 開発モードでのテストメッセージ生成（デバッグ用）
pub fn use_test_messages() -> Signal<Vec<GuiChatMessage>> {
    use_signal(Vec::<GuiChatMessage>::new)
}
