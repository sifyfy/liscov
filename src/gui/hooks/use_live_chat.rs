//! LiveChatサービス用カスタムフック
//!
//! Phase 3実装: 既存LiveChatServiceとDioxusコンポーネントの統合

use dioxus::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};

use crate::gui::{
    models::{GuiChatMessage, MessageType},
    services::{LiveChatService, ServiceState},
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
    pub messages: Signal<Vec<GuiChatMessage>>,
    pub state: Signal<ServiceState>,
    pub is_connected: Signal<bool>,
    pub stats: Signal<ChatStats>,
    pub is_stopping: Signal<bool>,
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
                let mut guard = global_state.lock().unwrap();
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
                    let mut guard = global_state.lock().unwrap();
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
                    let mut guard = global_state.lock().unwrap();
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
            if let Some(global_state) = GLOBAL_LIVE_CHAT.get() {
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
            if let Some(global_state) = GLOBAL_LIVE_CHAT.get() {
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
        let test_message = GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            message_type,
            author: author.to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            metadata: None,
            is_member: false,
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
}

/// LiveChatサービス用カスタムフック
///
/// シンプルで確実な同期システム
pub fn use_live_chat() -> LiveChatHandle {
    // フックが呼び出されたことを軽量ログで記録
    tracing::debug!("🎯 use_live_chat hook called!");

    // StateManagerから初期値を取得（遅延初期化）
    let state_manager = get_state_manager();
    let initial_state = state_manager.get_state();

    // Signalを初期化（StateManagerの現在値で初期化）
    let messages = use_signal(move || {
        tracing::debug!(
            "📨 Initializing messages signal with {} messages",
            initial_state.messages.len()
        );
        initial_state.messages.clone()
    });
    let state = use_signal(move || {
        tracing::debug!(
            "🔄 Initializing state signal: {:?}",
            initial_state.service_state
        );
        initial_state.service_state.clone()
    });
    let is_connected = use_signal(move || {
        tracing::debug!(
            "🔗 Initializing connection signal: {}",
            initial_state.is_connected
        );
        initial_state.is_connected
    });
    let stats = use_signal(move || {
        tracing::debug!("📊 Initializing stats signal");
        initial_state.stats.clone()
    });
    let is_stopping = use_signal(move || {
        tracing::debug!(
            "🛑 Initializing stopping signal: {}",
            initial_state.is_stopping
        );
        initial_state.is_stopping
    });

    tracing::debug!("✅ All signals initialized (optimized)");

    // StateManagerからの変更を監視してUI同期（改良版）
    // リアルタイム性を重視し、応答性を向上させた同期処理
    use_effect(move || {
        let mut messages_clone = messages;
        let mut state_clone = state;
        let mut is_connected_clone = is_connected;
        let mut stats_clone = stats;
        let mut is_stopping_clone = is_stopping;

        spawn(async move {
            let mut last_sync_time = std::time::Instant::now();
            let mut last_message_count = 0;
            let mut last_state = ServiceState::Idle;
            let mut last_connected = false;
            let mut last_stopping = false;

            // 同期間隔を短縮（200ms間隔で応答性向上）
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(200));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            tracing::debug!("🔄 Starting responsive UI sync (200ms interval)");

            loop {
                interval.tick().await;

                let current_state = get_state_manager().get_state();

                // メッセージの更新チェック（重要度高）
                let current_message_count = current_state.messages.len();
                if current_message_count != last_message_count {
                    messages_clone.set(current_state.messages.clone());
                    tracing::debug!(
                        "📨 UI messages updated: {} → {}",
                        last_message_count,
                        current_message_count
                    );
                    last_message_count = current_message_count;
                }

                // サービス状態の更新チェック（停止ボタンなど）
                if current_state.service_state != last_state {
                    state_clone.set(current_state.service_state.clone());
                    tracing::debug!(
                        "🔄 UI service state updated: {:?}",
                        current_state.service_state
                    );
                    last_state = current_state.service_state.clone();
                }

                // 接続状態の更新チェック（接続インジケーター）
                if current_state.is_connected != last_connected {
                    is_connected_clone.set(current_state.is_connected);
                    tracing::debug!(
                        "🔗 UI connection state updated: {}",
                        current_state.is_connected
                    );
                    last_connected = current_state.is_connected;
                }

                // 停止処理状態の更新チェック（ボタン無効化など）
                if current_state.is_stopping != last_stopping {
                    is_stopping_clone.set(current_state.is_stopping);
                    tracing::debug!(
                        "🛑 UI stopping state updated: {}",
                        current_state.is_stopping
                    );
                    last_stopping = current_state.is_stopping;
                }

                // 統計情報の更新（頻度は低め）
                stats_clone.set(current_state.stats.clone());

                // 5秒ごとに生存確認ログ
                if last_sync_time.elapsed().as_secs() >= 5 {
                    tracing::debug!(
                        "🔄 UI sync alive: {} messages, state: {:?}, connected: {}",
                        current_message_count,
                        current_state.service_state,
                        current_state.is_connected
                    );
                    last_sync_time = std::time::Instant::now();
                }
            }
        });
    });

    tracing::debug!("🎯 use_live_chat hook completed, returning handle");

    LiveChatHandle {
        messages,
        state,
        is_connected,
        stats,
        is_stopping,
    }
}

/// 開発モードでのテストメッセージ生成（デバッグ用）
pub fn use_test_messages() -> Signal<Vec<GuiChatMessage>> {
    let messages = use_signal(Vec::<GuiChatMessage>::new);

    // パフォーマンス最適化のため、テストメッセージ生成を完全無効化
    // 起動時のCPU負荷問題を解決するため、自動テストメッセージ機能を無効化
    tracing::debug!("🧪 Test message generation disabled for performance optimization");

    messages
}
