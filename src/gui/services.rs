// ライブチャットサービス層
// Phase 2で実装予定

use std::sync::{Arc, OnceLock};
use tokio::sync::{mpsc, Mutex as TokioMutex};

use super::models::GuiChatMessage;
use crate::api::innertube::{
    fetch_live_chat_messages, fetch_live_chat_page, get_next_continuation, InnerTube,
};
use crate::api::youtube::Continuation;
use crate::get_live_chat::Action;
use crate::io::{RawResponseSaver, SaveConfig};
use tracing;

/// 一時的にグローバル状態機能を無効化
// use crate::gui::hooks::{ChatStats, GlobalLiveChatState, GLOBAL_LIVE_CHAT};

/// ライブチャットサービス状態
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceState {
    Idle,
    Connecting,
    Connected,
    Paused,
    Error(String),
}

/// ライブチャットサービス
#[derive(Debug)]
pub struct LiveChatService {
    inner_tube: Arc<TokioMutex<Option<InnerTube>>>,
    state: Arc<TokioMutex<ServiceState>>,
    shutdown_sender: Option<mpsc::UnboundedSender<()>>,
    output_file: Arc<TokioMutex<Option<String>>>,
    response_saver: Arc<TokioMutex<RawResponseSaver>>,
    last_url: Option<String>,
}

impl LiveChatService {
    pub fn new() -> Self {
        Self {
            inner_tube: Arc::new(TokioMutex::new(None)),
            state: Arc::new(TokioMutex::new(ServiceState::Idle)),
            shutdown_sender: None,
            output_file: Arc::new(TokioMutex::new(None)),
            response_saver: Arc::new(TokioMutex::new(
                RawResponseSaver::new(SaveConfig::default()),
            )),
            last_url: None,
        }
    }

    /// ライブチャット監視開始
    pub async fn start_monitoring(
        &mut self,
        url: &str,
        output_file: Option<String>,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<GuiChatMessage>> {
        // URLを保存
        self.last_url = Some(url.to_string());

        // StateManagerにURLを通知
        use crate::gui::state_management::{get_state_manager, AppEvent};
        let _ = get_state_manager().send_event(AppEvent::CurrentUrlUpdated(Some(url.to_string())));
        // 状態をConnectingに変更
        {
            let mut state = self.state.lock().await;
            *state = ServiceState::Connecting;
        }

        // 出力ファイルパスを保存
        {
            let mut file_path = self.output_file.lock().await;
            *file_path = output_file;
        }

        // InnerTubeクライアントを初期化
        match fetch_live_chat_page(url).await {
            Ok(inner_tube) => {
                let mut inner_tube_guard = self.inner_tube.lock().await;
                *inner_tube_guard = Some(inner_tube);
                drop(inner_tube_guard);

                // ダミーレシーバー（互換性のため）
                let (_dummy_tx, message_rx) = mpsc::unbounded_channel();
                let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();

                self.shutdown_sender = Some(shutdown_tx);

                // 状態をConnectedに変更
                {
                    let mut state = self.state.lock().await;
                    *state = ServiceState::Connected;
                }

                // バックグラウンドでメッセージ受信タスクを開始
                self.spawn_global_message_receiver_task(shutdown_rx).await;

                Ok(message_rx)
            }
            Err(e) => {
                let error_msg = format!("Failed to initialize live chat: {}", e);
                let mut state = self.state.lock().await;
                *state = ServiceState::Error(error_msg.clone());
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    /// ライブチャット監視停止（完全停止）
    pub async fn stop_monitoring(&mut self) -> anyhow::Result<()> {
        // シャットダウンシグナルを送信
        if let Some(shutdown_sender) = &self.shutdown_sender {
            let _ = shutdown_sender.send(());
        }

        // InnerTubeクライアントをクリア
        {
            let mut inner_tube = self.inner_tube.lock().await;
            *inner_tube = None;
        }

        // 状態をIdleに変更
        {
            let mut state = self.state.lock().await;
            *state = ServiceState::Idle;
        }

        // チャネルをクリア
        self.shutdown_sender = None;

        // URLも破棄（完全停止）
        self.last_url = None;

        println!("Live chat monitoring stopped");
        Ok(())
    }

    /// ライブチャット監視の一時停止（継続トークンを保持）
    pub async fn pause_monitoring(&mut self) -> anyhow::Result<()> {
        // シャットダウンシグナルを送信
        if let Some(shutdown_sender) = &self.shutdown_sender {
            let _ = shutdown_sender.send(());
        }

        // 状態をPausedに変更（InnerTubeクライアントは保持）
        {
            let mut state = self.state.lock().await;
            *state = ServiceState::Paused;
        }

        // チャネルをクリア
        self.shutdown_sender = None;

        // 継続トークンを保存
        if let Some(inner_tube) = self.inner_tube.lock().await.as_ref() {
            use crate::gui::state_management::{get_state_manager, AppEvent};
            let continuation = inner_tube.continuation.0.clone();
            let _ = get_state_manager()
                .send_event(AppEvent::ContinuationTokenUpdated(Some(continuation)));
        }

        println!("Live chat monitoring paused");
        Ok(())
    }

    /// ライブチャット監視の再開（保存された継続トークンから）
    pub async fn resume_monitoring(
        &mut self,
        output_file: Option<String>,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<GuiChatMessage>> {
        use crate::gui::state_management::get_state_manager;
        let state_manager = get_state_manager();
        let current_state = state_manager.get_state();

        // 保存されたURLと継続トークンを取得
        let url = match (&self.last_url, &current_state.current_url) {
            (Some(last), _) => last.clone(),
            (None, Some(current)) => current.clone(),
            _ => return Err(anyhow::anyhow!("No URL available for resuming")),
        };

        let continuation_token = current_state.continuation_token.clone();

        // InnerTubeクライアントの準備
        let mut inner_tube = self.inner_tube.lock().await;
        if inner_tube.is_none() {
            // 新しいクライアントを作成
            use crate::api::innertube::fetch_live_chat_page;
            let client = fetch_live_chat_page(&url).await?;
            *inner_tube = Some(client);
        }

        // 継続トークンを復元
        if let (Some(client), Some(token)) = (inner_tube.as_mut(), continuation_token) {
            client.continuation = Continuation(token);
            tracing::info!("🔄 Resuming with saved continuation token");
        } else {
            tracing::warn!("⚠️ No continuation token available, starting fresh");
        }

        drop(inner_tube);

        // 状態をConnectingに変更
        {
            let mut state = self.state.lock().await;
            *state = ServiceState::Connecting;
        }

        // 出力ファイルを設定
        {
            let mut output = self.output_file.lock().await;
            *output = output_file;
        }

        // URLを更新
        self.last_url = Some(url);

        // チャネルを作成してメッセージ受信タスクを開始
        let (_message_sender, message_receiver) = mpsc::unbounded_channel();
        let (shutdown_sender, shutdown_receiver) = mpsc::unbounded_channel();

        self.shutdown_sender = Some(shutdown_sender);

        // バックグラウンドタスクを開始
        self.spawn_global_message_receiver_task(shutdown_receiver)
            .await;

        // 状態をConnectedに変更
        {
            let mut state = self.state.lock().await;
            *state = ServiceState::Connected;
        }

        println!("Live chat monitoring resumed");
        Ok(message_receiver)
    }

    /// 現在の状態を取得
    pub async fn get_state(&self) -> ServiceState {
        let state = self.state.lock().await;
        state.clone()
    }

    /// レスポンス保存設定を更新
    pub async fn update_save_config(&self, config: SaveConfig) {
        tracing::info!(
            "🔧 Updating save config: enabled={}, file_path={}, max_size_mb={}",
            config.enabled,
            config.file_path,
            config.max_file_size_mb
        );

        let mut saver = self.response_saver.lock().await;
        let old_config = saver.get_config().clone();
        saver.update_config(config.clone());

        tracing::info!(
            "✅ Raw response save config updated: {} -> {}",
            if old_config.enabled {
                "enabled"
            } else {
                "disabled"
            },
            if config.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    /// 現在の保存設定を取得
    pub async fn get_save_config(&self) -> SaveConfig {
        let saver = self.response_saver.lock().await;
        saver.get_config().clone()
    }

    /// 保存されたレスポンス数を取得
    pub async fn get_saved_response_count(&self) -> anyhow::Result<usize> {
        let saver = self.response_saver.lock().await;
        saver.get_saved_response_count().await
    }

    /// グローバル状態に直接メッセージを送信するバックグラウンドタスク
    async fn spawn_global_message_receiver_task(
        &self,
        mut shutdown_receiver: mpsc::UnboundedReceiver<()>,
    ) {
        let inner_tube = Arc::clone(&self.inner_tube);
        let state = Arc::clone(&self.state);
        let output_file = Arc::clone(&self.output_file);
        let response_saver = Arc::clone(&self.response_saver);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
            let mut request_count = 0;
            let start_time = std::time::Instant::now();

            tracing::info!("🚀 Message receiver task started");

            loop {
                tokio::select! {
                    _ = shutdown_receiver.recv() => {
                        tracing::info!("🛑 Shutdown signal received, stopping message receiver");
                        break;
                    }
                    _ = interval.tick() => {
                        request_count += 1;

                        // APIリクエストは1分に1回のみログ出力（デバッグ時は除く）
                        let should_log_request = (cfg!(debug_assertions) && tracing::level_enabled!(tracing::Level::DEBUG)) || request_count % 30 == 1; // 30回に1回 = 1分に1回

                        if should_log_request {
                            tracing::debug!("📡 Request #{} - Attempting to fetch live chat messages", request_count);
                        }

                        // InnerTubeクライアントを取得
                        let mut inner_tube_guard = inner_tube.lock().await;
                        if let Some(ref mut inner_tube_client) = inner_tube_guard.as_mut() {
                            if should_log_request {
                                tracing::debug!("🔧 InnerTube client available, making API request");
                            }

                            match fetch_live_chat_messages(inner_tube_client).await {
                                Ok(response) => {
                                    // レスポンス受信の詳細ログは控えめに
                                    if should_log_request {
                                        tracing::debug!("✅ Received response from API, processing actions");
                                    }

                                    // アクション数をログ
                                    let action_count = response.continuation_contents.live_chat_continuation.actions.len();
                                    if action_count > 0 {
                                        // 新しいメッセージがある場合は必ずログ出力
                                        tracing::info!("📬 Received {} actions from API", action_count);
                                    } else if should_log_request {
                                        // アクションなしの場合はデバッグ時のみ
                                        tracing::debug!("📪 No actions in response");
                                    }

                                    // 継続トークンを更新
                                    if let Some(next_continuation) = get_next_continuation(&response) {
                                        if should_log_request {
                                            tracing::debug!("🔄 Updating continuation token");
                                        }
                                        inner_tube_client.continuation = Continuation(next_continuation.clone());

                                        // StateManagerにも継続トークンを保存
                                        use crate::gui::state_management::{get_state_manager, AppEvent};
                                        let _ = get_state_manager().send_event(AppEvent::ContinuationTokenUpdated(Some(next_continuation)));
                                    } else {
                                        tracing::warn!("⚠️ No next continuation token found");
                                    }

                                    // アクションを処理
                                    for (index, action) in response.continuation_contents.live_chat_continuation.actions.iter().enumerate() {
                                        if let Action::AddChatItem(add_item_wrapper) = action {
                                            let chat_item = add_item_wrapper.action.get_item();
                                            if should_log_request {
                                                tracing::debug!("💬 Processing chat item #{}", index + 1);
                                            }

                                            // ChatItemをGuiChatMessageに変換
                                            let gui_message: GuiChatMessage = chat_item.clone().into();

                                            // 新しいメッセージのログをdebugレベルに変更
                                            tracing::debug!("📝 New message: {} - {}", gui_message.author, gui_message.content);

                                            // グローバル状態に直接メッセージを追加
                                            Self::add_message_to_global_state(gui_message.clone(), &start_time);

                                            // イベント駆動状態管理にもメッセージを送信
                                            use crate::gui::state_management::{get_state_manager, AppEvent};
                                            let _ = get_state_manager().send_event(AppEvent::MessageAdded(gui_message.clone()));

                                            // ファイルに保存（オプション・自動保存設定に基づく）
                                            let file_path = output_file.lock().await;
                                            if let Some(ref path) = *file_path {
                                                                                                // 設定管理から自動保存設定を確認
                                                use crate::gui::config_manager::get_current_config;
                                                let should_auto_save = if let Some(config) = get_current_config() {
                                                    config.auto_save_enabled
                                                } else {
                                                    // 設定が取得できない場合は、出力ファイルが指定されていれば保存
                                                    true
                                                };

                                                if should_auto_save {
                                                    if let Err(e) = Self::save_message_to_file(path, &gui_message).await {
                                                        tracing::error!("❌ Failed to save message to file: {}", e);
                                                    } else {
                                                        tracing::debug!("💾 Message auto-saved to: {}", path);
                                                    }
                                                } else {
                                                    tracing::debug!("⏭️ Auto save disabled, skipping file save");
                                                }
                                            }
                                        } else if should_log_request {
                                            tracing::debug!("🔄 Non-message action received: {:?}", std::mem::discriminant(action));
                                        }
                                    }

                                                                        // 生レスポンスの保存
                                    let saver = response_saver.lock().await;
                                    let is_enabled = saver.is_enabled();
                                    let config = saver.get_config();

                                    // 保存処理のログは常に出力（デバッグ用）
                                    tracing::info!("💾 Raw response save attempt: enabled={}, file_path={}", is_enabled, config.file_path);

                                    if let Err(e) = saver.save_response(&response).await {
                                        tracing::warn!("❌ Failed to save raw response: {}", e);
                                    } else if is_enabled {
                                        tracing::info!("💾 Raw response saved successfully to: {}", config.file_path);
                                    } else {
                                        tracing::debug!("💾 Raw response save skipped (disabled)");
                                    }
                                }
                                Err(e) => {
                                    // エラーは必ずログ出力
                                    tracing::error!("❌ Error fetching live chat messages: {}", e);
                                    if cfg!(debug_assertions) {
                                        tracing::error!("🔍 Error details: {:?}", e);
                                    }

                                    let mut state_guard = state.lock().await;
                                    *state_guard = ServiceState::Error(format!("Fetch error: {}", e));

                                    // エラー時もタスクを継続（一時的なネットワークエラーの可能性）
                                    tracing::warn!("⚠️ Continuing despite error - this might be temporary");
                                }
                            }
                        } else {
                            tracing::error!("❌ InnerTube client is not available");
                            break;
                        }
                    }
                }
            }

            tracing::info!(
                "🏁 Message receiver task completed. Total requests: {}",
                request_count
            );
        });
    }

    /// グローバル状態にメッセージを追加（一時的に無効化）
    fn add_message_to_global_state(_message: GuiChatMessage, _start_time: &std::time::Instant) {
        // 一時的にグローバル状態機能を無効化
        // TODO: 新しい状態管理システムに統合
        tracing::debug!("Global state functionality temporarily disabled");
    }

    /// メッセージをファイルに保存
    async fn save_message_to_file(file_path: &str, message: &GuiChatMessage) -> anyhow::Result<()> {
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;

        let json_line = serde_json::to_string(message)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .await?;

        file.write_all(format!("{}\n", json_line).as_bytes())
            .await?;
        file.flush().await?;

        Ok(())
    }
}

impl Default for LiveChatService {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルライブチャットサービスインスタンス
pub static GLOBAL_SERVICE: OnceLock<Arc<TokioMutex<LiveChatService>>> = OnceLock::new();

/// グローバルサービスを取得（遅延初期化）
pub fn get_global_service() -> &'static Arc<TokioMutex<LiveChatService>> {
    GLOBAL_SERVICE.get_or_init(|| {
        tracing::debug!("🏗️ Creating global live chat service");
        Arc::new(TokioMutex::new(LiveChatService::new()))
    })
}
