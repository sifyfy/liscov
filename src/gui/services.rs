// ライブチャットサービス層
// Phase 2で実装予定

use async_trait::async_trait;
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

        tracing::info!("Live chat monitoring stopped");
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

        tracing::info!("Live chat monitoring paused");
        Ok(())
    }

    /// ライブチャット監視の再開（保存された継続トークンから）
    pub async fn resume_monitoring(
        &mut self,
        output_file: Option<String>,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<GuiChatMessage>> {
        use crate::gui::state_management::get_state_manager;
        let state_manager = get_state_manager();
        let current_state = match state_manager.get_state() {
            Ok(state) => state,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to get state for resume: {}", e));
            }
        };

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

        tracing::info!("Live chat monitoring resumed");
        Ok(message_receiver)
    }

    /// 現在の状態を取得
    pub async fn get_state(&self) -> ServiceState {
        let state = self.state.lock().await;
        state.clone()
    }

    /// レスポンス保存設定を更新
    pub async fn update_save_config(&self, config: SaveConfig) {
        let mut saver = self.response_saver.lock().await;
        let old_config = saver.get_config().clone();

        // 設定が実際に変わった場合のみログ出力
        if old_config.enabled != config.enabled || old_config.file_path != config.file_path {
            tracing::info!(
                "✅ Raw response save config updated: {} -> {} (file: {})",
                if old_config.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                if config.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                config.file_path
            );
        } else {
            tracing::debug!(
                "🔧 Save config unchanged: enabled={}, file_path={}",
                config.enabled,
                config.file_path
            );
        }

        saver.update_config(config.clone());
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
            let mut consecutive_errors = 0;
            let mut last_successful_request = std::time::Instant::now();
            let _start_time = std::time::Instant::now();
            const MAX_CONSECUTIVE_ERRORS: usize = 5;
            const HEALTH_CHECK_INTERVAL_SECS: u64 = 30;

            tracing::info!("🚀 Message receiver task started");

            loop {
                tokio::select! {
                    _ = shutdown_receiver.recv() => {
                        tracing::info!("🛑 Shutdown signal received, stopping message receiver");
                        break;
                    }
                    _ = interval.tick() => {
                        request_count += 1;
                        let request_start = std::time::Instant::now();

                        // ヘルスチェック: 長時間成功していない場合は警告
                        let time_since_success = last_successful_request.elapsed().as_secs();
                        if time_since_success > HEALTH_CHECK_INTERVAL_SECS {
                            tracing::warn!(
                                "⚠️ [HEALTH_CHECK] No successful API response for {} seconds (consecutive errors: {})",
                                time_since_success,
                                consecutive_errors
                            );
                        }

                        // デバッグ時は全てのAPIリクエストをログ出力（問題調査のため）
                        let should_log_request = true; // 一時的に全てのリクエストをログ出力

                        if should_log_request {
                            tracing::debug!("📡 Request #{} - Attempting to fetch live chat messages", request_count);
                        }

                        // InnerTubeクライアントを取得
                        let mut inner_tube_guard = inner_tube.lock().await;
                        if let Some(ref mut inner_tube_client) = inner_tube_guard.as_mut() {
                            if should_log_request {
                                tracing::debug!("🔧 InnerTube client available, making API request");
                            }

                            // タイムアウト付きでAPI呼び出しを実行
                            let api_result = tokio::time::timeout(
                                tokio::time::Duration::from_secs(15),
                                fetch_live_chat_messages(inner_tube_client)
                            ).await;

                            match api_result {
                                Ok(Ok(response)) => {
                                    // 成功: エラーカウンターをリセット
                                    consecutive_errors = 0;
                                    last_successful_request = std::time::Instant::now();
                                    let request_duration = request_start.elapsed();

                                    let _api_response_time = std::time::Instant::now();

                                    // アクション数をログ
                                    let action_count = response.continuation_contents.live_chat_continuation.actions.len();

                                    tracing::info!(
                                        "✅ [API_SERVICE] API Response #{}: {} actions received (took {:?})",
                                        request_count,
                                        action_count,
                                        request_duration
                                    );

                                    if action_count > 0 {
                                        // 新しいメッセージがある場合は必ずログ出力
                                        tracing::info!(
                                            "📬 [API_SERVICE] Processing {} actions from API (request #{})",
                                            action_count,
                                            request_count
                                        );
                                    } else {
                                        // アクションなしの場合もデバッグレベルで記録
                                        tracing::debug!(
                                            "📪 [API_SERVICE] No actions in response #{}",
                                            request_count
                                        );
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
                                        tracing::warn!("⚠️ No next continuation token found in response #{}", request_count);
                                        // 継続トークンがない場合は警告レベルで記録
                                    }

                                    // アクションを処理
                                    let mut processed_messages = 0;
                                    let mut state_manager_send_results = Vec::new();

                                    for (index, action) in response.continuation_contents.live_chat_continuation.actions.iter().enumerate() {
                                        if let Action::AddChatItem(add_item_wrapper) = action {
                                            let chat_item = add_item_wrapper.action.get_item();

                                            tracing::debug!(
                                                "💬 [API_SERVICE] Processing chat item #{}/{} in request #{}",
                                                index + 1,
                                                action_count,
                                                request_count
                                            );

                                            // ChatItemをGuiChatMessageに変換
                                            let conversion_start = std::time::Instant::now();
                                            let gui_message: GuiChatMessage = chat_item.clone().into();
                                            let conversion_duration = conversion_start.elapsed();

                                            tracing::info!(
                                                "📝 [API_SERVICE] New message converted in {:?}: {} - '{}'",
                                                conversion_duration,
                                                gui_message.author,
                                                gui_message.content.chars().take(50).collect::<String>()
                                            );

                                            // 新しい状態管理システム（StateManager）のみを使用

                                            // イベント駆動状態管理にメッセージを送信
                                            use crate::gui::state_management::{get_state_manager, AppEvent};
                                            let state_send_start = std::time::Instant::now();
                                            let send_result = get_state_manager().send_event(AppEvent::MessageAdded(gui_message.clone()));
                                            let state_send_duration = state_send_start.elapsed();

                                            match send_result {
                                                Ok(()) => {
                                                    tracing::info!(
                                                        "📤 [API_SERVICE] Message sent to StateManager in {:?}: {} - {}",
                                                        state_send_duration,
                                                        gui_message.author,
                                                        gui_message.content.chars().take(30).collect::<String>()
                                                    );
                                                    state_manager_send_results.push(true);
                                                    processed_messages += 1;
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "❌ [API_SERVICE] Failed to send message to StateManager: {:?}",
                                                        e
                                                    );
                                                    state_manager_send_results.push(false);
                                                }
                                            }

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
                                                        tracing::error!("❌ [API_SERVICE] Failed to save message to file: {}", e);
                                                    } else {
                                                        tracing::debug!("💾 [API_SERVICE] Message auto-saved to: {}", path);
                                                    }
                                                } else {
                                                    tracing::debug!("⏭️ [API_SERVICE] Auto save disabled, skipping file save");
                                                }
                                            }
                                        } else {
                                            tracing::debug!(
                                                "🔄 [API_SERVICE] Non-message action #{}/{}: {:?}",
                                                index + 1,
                                                action_count,
                                                std::mem::discriminant(action)
                                            );
                                        }
                                    }

                                    // 処理結果の集計ログ
                                    let successful_sends = state_manager_send_results.iter().filter(|&&success| success).count();
                                    let failed_sends = state_manager_send_results.len() - successful_sends;

                                    if processed_messages > 0 {
                                        tracing::info!(
                                            "📊 [API_SERVICE] Request #{} summary: {} messages processed, {} sent to StateManager successfully, {} failed",
                                            request_count,
                                            processed_messages,
                                            successful_sends,
                                            failed_sends
                                        );
                                    }

                                                                        // 生レスポンスの保存
                                    let saver = response_saver.lock().await;
                                    let is_enabled = saver.is_enabled();
                                    let config = saver.get_config();

                                    if let Err(e) = saver.save_response(&response).await {
                                        tracing::warn!("❌ Failed to save raw response: {}", e);
                                    } else if is_enabled {
                                        tracing::info!("💾 Raw response saved successfully to: {}", config.file_path);
                                    }
                                }
                                Ok(Err(e)) => {
                                    consecutive_errors += 1;
                                    let request_duration = request_start.elapsed();

                                    // エラーは必ずログ出力
                                    tracing::error!(
                                        "❌ [API_SERVICE] API Error (#{}, consecutive: {}, took {:?}): {}",
                                        request_count,
                                        consecutive_errors,
                                        request_duration,
                                        e
                                    );

                                    if cfg!(debug_assertions) {
                                        tracing::error!("🔍 Error details: {:?}", e);
                                    }

                                    // 連続エラーが多い場合の特別処理
                                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                                        tracing::error!(
                                            "🚨 [API_SERVICE] Too many consecutive errors ({}). This may indicate:",
                                            consecutive_errors
                                        );
                                        tracing::error!("   - Stream has ended");
                                        tracing::error!("   - Network connectivity issues");
                                        tracing::error!("   - YouTube API rate limits");
                                        tracing::error!("   - Invalid continuation token");

                                        // エラー情報をより詳細に記録
                                        let error_str = e.to_string();
                                        if error_str.contains("404") || error_str.contains("Not Found") {
                                            tracing::error!("💡 [DIAGNOSIS] Likely cause: Stream ended or chat disabled");
                                        } else if error_str.contains("403") || error_str.contains("Forbidden") {
                                            tracing::error!("💡 [DIAGNOSIS] Likely cause: API access denied or rate limited");
                                        } else if error_str.contains("timeout") || error_str.contains("Timeout") {
                                            tracing::error!("💡 [DIAGNOSIS] Likely cause: Network timeout or slow connection");
                                        } else if error_str.contains("connection") {
                                            tracing::error!("💡 [DIAGNOSIS] Likely cause: Network connectivity problem");
                                        }
                                    }

                                    let mut state_guard = state.lock().await;
                                    *state_guard = ServiceState::Error(format!("API Error ({}): {}", consecutive_errors, e));

                                    // 多連続エラー時は少し待機してから継続
                                    if consecutive_errors >= 3 {
                                        let wait_duration = std::cmp::min(consecutive_errors * 2, 30);
                                        tracing::warn!("⏳ [API_SERVICE] Waiting {} seconds before next attempt", wait_duration);
                                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_duration as u64)).await;
                                    }

                                    tracing::warn!("⚠️ [API_SERVICE] Continuing despite error - this might be temporary (attempt {}/{})", consecutive_errors, MAX_CONSECUTIVE_ERRORS);
                                }
                                Err(_timeout_error) => {
                                    consecutive_errors += 1;
                                    let request_duration = request_start.elapsed();

                                    tracing::error!(
                                        "⏰ [API_SERVICE] Request #{} timed out after {:?} (consecutive timeouts: {})",
                                        request_count,
                                        request_duration,
                                        consecutive_errors
                                    );

                                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                                        tracing::error!("🚨 [TIMEOUT] Multiple consecutive timeouts detected. This may indicate:");
                                        tracing::error!("   - Slow network connection");
                                        tracing::error!("   - YouTube API server issues");
                                        tracing::error!("   - Local firewall/proxy problems");
                                    }

                                    let mut state_guard = state.lock().await;
                                    *state_guard = ServiceState::Error(format!("Timeout ({})", consecutive_errors));

                                    // タイムアウト時も少し待機
                                    if consecutive_errors >= 3 {
                                        let wait_duration = std::cmp::min(consecutive_errors * 2, 30);
                                        tracing::warn!("⏳ [TIMEOUT] Waiting {} seconds before next attempt", wait_duration);
                                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_duration as u64)).await;
                                    }

                                    tracing::warn!("⚠️ [TIMEOUT] Continuing despite timeout - this might be temporary");
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
                                        "🏁 Message receiver task completed. Total requests: {}, consecutive errors at end: {}",
                                        request_count,
                                        consecutive_errors
                                    );
        });
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

/// ChatServiceトレイトの実装（Phase 2: トレイトベース設計への移行）
#[async_trait]
impl super::traits::ChatService for LiveChatService {
    async fn start_monitoring(
        &mut self,
        url: &str,
        output_file: Option<String>,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<GuiChatMessage>> {
        self.start_monitoring(url, output_file).await
    }

    async fn stop_monitoring(&mut self) -> anyhow::Result<()> {
        self.stop_monitoring().await
    }

    async fn pause_monitoring(&mut self) -> anyhow::Result<()> {
        self.pause_monitoring().await
    }

    async fn resume_monitoring(
        &mut self,
        output_file: Option<String>,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<GuiChatMessage>> {
        self.resume_monitoring(output_file).await
    }

    async fn get_state(&self) -> ServiceState {
        self.get_state().await
    }

    async fn update_save_config(&self, config: crate::io::SaveConfig) {
        self.update_save_config(config).await
    }

    async fn get_save_config(&self) -> crate::io::SaveConfig {
        self.get_save_config().await
    }

    async fn get_saved_response_count(&self) -> anyhow::Result<usize> {
        self.get_saved_response_count().await
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
