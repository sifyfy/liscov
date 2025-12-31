// ライブチャットサービス層
// Phase 2で実装予定

use async_trait::async_trait;
use std::sync::{Arc, OnceLock};
use tokio::sync::{mpsc, Mutex as TokioMutex};

use super::models::GuiChatMessage;
use super::stream_end_detector::{DetectionResult, StreamEndDetector};
use crate::api::auth::{CookieManager, YouTubeCookies};
use crate::api::innertube::{
    fetch_live_chat_messages, fetch_live_chat_page_with_auth,
    get_next_continuation_with_timeout, InnerTube,
};
use crate::api::youtube::{ChatMode, Continuation};
use crate::get_live_chat::Action;
use crate::gui::config_manager::get_current_config;
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
    message_sender: Arc<TokioMutex<Option<mpsc::UnboundedSender<GuiChatMessage>>>>,
    output_file: Arc<TokioMutex<Option<String>>>,
    response_saver: Arc<TokioMutex<RawResponseSaver>>,
    stream_end_detector: Arc<TokioMutex<StreamEndDetector>>,
    last_url: Option<String>,
    /// 現在のチャットモード（トップチャット or すべてのチャット）
    chat_mode: ChatMode,
    /// 認証情報（メンバー限定配信用）
    auth_cookies: Option<YouTubeCookies>,
    #[cfg(test)]
    test_fetch_live_chat_page: Option<anyhow::Result<InnerTube>>,
}

impl LiveChatService {
    pub fn new() -> Self {
        // 保存済み認証情報を読み込み
        let auth_cookies = Self::load_saved_auth();

        if auth_cookies.is_some() {
            tracing::info!("🔐 Loaded saved authentication credentials");
        }

        // 設定ファイルから保存設定を読み込み
        let save_config = if let Some(config) = get_current_config() {
            // データディレクトリの絶対パスを取得
            let file_path = if std::path::Path::new(&config.raw_response_file).is_absolute() {
                config.raw_response_file.clone()
            } else {
                // 相対パスの場合はデータディレクトリを基準にする
                directories::ProjectDirs::from("dev", "sifyfy", "liscov")
                    .map(|dirs| {
                        let data_dir = dirs.data_dir();
                        // データディレクトリを作成
                        if let Err(e) = std::fs::create_dir_all(data_dir) {
                            tracing::warn!("⚠️ Failed to create data directory: {}", e);
                        }
                        data_dir.join(&config.raw_response_file).to_string_lossy().to_string()
                    })
                    .unwrap_or_else(|| config.raw_response_file.clone())
            };

            tracing::info!(
                "📁 Loaded save config from file: enabled={}, file={}",
                config.save_raw_responses,
                file_path
            );
            SaveConfig {
                enabled: config.save_raw_responses,
                file_path,
                max_file_size_mb: config.max_raw_file_size_mb,
                enable_rotation: config.enable_file_rotation,
                max_backup_files: 5,
            }
        } else {
            tracing::warn!("⚠️ Failed to load config, using default save settings");
            SaveConfig::default()
        };

        Self {
            inner_tube: Arc::new(TokioMutex::new(None)),
            state: Arc::new(TokioMutex::new(ServiceState::Idle)),
            shutdown_sender: None,
            message_sender: Arc::new(TokioMutex::new(None)),
            output_file: Arc::new(TokioMutex::new(None)),
            response_saver: Arc::new(TokioMutex::new(RawResponseSaver::new(save_config))),
            stream_end_detector: Arc::new(TokioMutex::new(StreamEndDetector::new())),
            last_url: None,
            chat_mode: ChatMode::default(),
            auth_cookies,
            #[cfg(test)]
            test_fetch_live_chat_page: None,
        }
    }

    /// 保存済み認証情報を読み込む
    fn load_saved_auth() -> Option<YouTubeCookies> {
        tracing::info!("🔑 Checking for saved authentication credentials...");
        match CookieManager::with_default_dir() {
            Ok(manager) => {
                tracing::debug!("📁 Config path: {:?}", manager.config_path());
                if manager.exists() {
                    tracing::debug!("📄 Credentials file found");
                    match manager.load() {
                        Ok(cookies) if cookies.is_valid() => {
                            tracing::info!("✓ Valid credentials loaded");
                            Some(cookies)
                        }
                        Ok(_) => {
                            tracing::warn!("⚠️ Saved credentials are invalid");
                            None
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load credentials: {}", e);
                            None
                        }
                    }
                } else {
                    tracing::debug!("📄 No credentials file found");
                    None
                }
            }
            Err(e) => {
                tracing::warn!("Failed to initialize CookieManager: {}", e);
                None
            }
        }
    }

    /// 認証情報を設定
    pub fn set_auth(&mut self, cookies: YouTubeCookies) {
        tracing::info!("🔐 Authentication credentials set");
        self.auth_cookies = Some(cookies);
    }

    /// 認証情報をクリア
    pub fn clear_auth(&mut self) {
        tracing::info!("🔓 Authentication credentials cleared");
        self.auth_cookies = None;
    }

    /// 認証済みかどうかを確認
    pub fn is_authenticated(&self) -> bool {
        self.auth_cookies.is_some()
    }

    /// 認証情報を取得
    pub fn auth_cookies(&self) -> Option<&YouTubeCookies> {
        self.auth_cookies.as_ref()
    }

    /// 現在のチャットモードを取得
    pub fn get_chat_mode(&self) -> ChatMode {
        self.chat_mode
    }

    /// チャットモードを設定（監視開始前に呼び出す）
    pub fn set_chat_mode(&mut self, mode: ChatMode) {
        self.chat_mode = mode;
        tracing::info!("🔄 Chat mode set to: {}", mode);
    }

    /// チャットモードを変更（監視中でも有効）
    ///
    /// 監視中の場合はreload tokenを使ってYouTube APIにリクエストし、
    /// 新しいメッセージ取得用のcontinuation tokenを取得する。
    pub async fn change_chat_mode(&mut self, mode: ChatMode) -> anyhow::Result<bool> {
        let old_mode = self.chat_mode;

        // InnerTubeクライアントが存在する場合は非同期でモードを切り替え
        let mut inner_tube = self.inner_tube.lock().await;
        if let Some(ref mut client) = *inner_tube {
            match client.switch_chat_mode(mode).await {
                Ok(true) => {
                    self.chat_mode = mode;
                    tracing::info!("🔄 Chat mode changed from {} to {}", old_mode, mode);
                    Ok(true)
                }
                Ok(false) => {
                    tracing::warn!(
                        "⚠️ Chat mode {} not available, keeping {}",
                        mode,
                        old_mode
                    );
                    Ok(false)
                }
                Err(e) => {
                    tracing::error!("❌ Failed to switch chat mode: {}", e);
                    Err(e)
                }
            }
        } else {
            // クライアントがない場合は設定だけ変更
            self.chat_mode = mode;
            tracing::info!("🔄 Chat mode pre-set to: {} (will apply on next start)", mode);
            Ok(true)
        }
    }

    /// 利用可能なチャットモードを取得
    pub async fn available_chat_modes(&self) -> Vec<ChatMode> {
        let inner_tube = self.inner_tube.lock().await;
        if let Some(ref client) = *inner_tube {
            client.available_chat_modes()
        } else {
            vec![self.chat_mode]
        }
    }

    #[cfg(test)]
    pub fn set_test_fetch_live_chat_page(&mut self, result: anyhow::Result<InnerTube>) {
        self.test_fetch_live_chat_page = Some(result);
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

        // InnerTubeクライアントを初期化（チャットモードを指定、認証情報付き）
        let chat_mode = self.chat_mode;
        let auth_cookies_ref = self.auth_cookies.as_ref();
        tracing::info!("🎯 Starting with chat mode: {}", chat_mode);
        if auth_cookies_ref.is_some() {
            tracing::info!("🔐 Using authentication for initial page fetch");
        }

        #[cfg(test)]
        let fetch_result = if let Some(result) = self.test_fetch_live_chat_page.take() {
            result
        } else {
            fetch_live_chat_page_with_auth(url, chat_mode, auth_cookies_ref).await
        };
        #[cfg(not(test))]
        let fetch_result = fetch_live_chat_page_with_auth(url, chat_mode, auth_cookies_ref).await;

        match fetch_result {
            Ok(mut inner_tube) => {
                // 認証情報を設定（後続のAPIリクエスト用）
                if let Some(ref cookies) = self.auth_cookies {
                    inner_tube.set_auth(cookies.clone());
                    tracing::info!("🔐 Authentication applied to InnerTube client for API requests");
                }

                let mut inner_tube_guard = self.inner_tube.lock().await;
                *inner_tube_guard = Some(inner_tube);
                drop(inner_tube_guard);

                // ダミーレシーバー（互換性のため）
                let (message_tx, message_rx) = mpsc::unbounded_channel();
                self.set_message_sender(Some(message_tx)).await;
                let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();

                self.shutdown_sender = Some(shutdown_tx);

                self.reset_stream_end_detector().await;
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

        self.set_message_sender(None).await;
        self.reset_stream_end_detector().await;

        tracing::info!("Live chat monitoring stopped");
        Ok(())
    }

    async fn handle_detection_result_internal(
        state: &Arc<TokioMutex<ServiceState>>,
        detection_result: DetectionResult,
        error_state_message: String,
        consecutive_errors: usize,
        wait_duration_secs: Option<u64>,
        warning_context: &str,
    ) -> bool {
        match detection_result {
            DetectionResult::StreamEnded | DetectionResult::AlreadyEnded => {
                let mut state_guard = state.lock().await;
                *state_guard = ServiceState::Idle;
                true
            }
            DetectionResult::Warning | DetectionResult::Continue => {
                {
                    let mut state_guard = state.lock().await;
                    *state_guard = ServiceState::Error(error_state_message);
                }

                if let Some(wait_secs) = wait_duration_secs {
                    if wait_secs > 0 {
                        tracing::warn!(
                            "⏳ [API_SERVICE] Waiting {} seconds before next attempt",
                            wait_secs
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                    }
                }

                tracing::warn!(
                    "⚠️ [API_SERVICE] Continuing despite error (attempt {}): {}",
                    consecutive_errors,
                    warning_context
                );
                false
            }
        }
    }

    #[cfg(test)]
    pub async fn test_handle_detection_result(
        &self,
        detection_result: DetectionResult,
        error_state_message: String,
        consecutive_errors: usize,
        wait_duration_secs: Option<u64>,
        warning_context: &str,
    ) -> bool {
        Self::handle_detection_result_internal(
            &self.state,
            detection_result,
            error_state_message,
            consecutive_errors,
            wait_duration_secs,
            warning_context,
        )
        .await
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

        self.set_message_sender(None).await;
        self.reset_stream_end_detector().await;

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
            // 新しいクライアントを作成（チャットモードを指定、認証情報付き）
            let chat_mode = self.chat_mode;
            let auth_cookies_ref = self.auth_cookies.as_ref();
            tracing::info!("🎯 Resuming with chat mode: {}", chat_mode);
            let mut client = fetch_live_chat_page_with_auth(&url, chat_mode, auth_cookies_ref).await?;
            // 認証情報を設定（後続のAPIリクエスト用）
            if let Some(ref cookies) = self.auth_cookies {
                client.set_auth(cookies.clone());
            }
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
        let (message_tx, message_receiver) = mpsc::unbounded_channel();
        self.set_message_sender(Some(message_tx)).await;
        let (shutdown_sender, shutdown_receiver) = mpsc::unbounded_channel();

        self.shutdown_sender = Some(shutdown_sender);

        self.reset_stream_end_detector().await;
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

    async fn set_message_sender(&self, sender: Option<mpsc::UnboundedSender<GuiChatMessage>>) {
        let mut guard = self.message_sender.lock().await;
        *guard = sender;
    }

    async fn broadcast_to_receivers(
        message_sender: &Arc<TokioMutex<Option<mpsc::UnboundedSender<GuiChatMessage>>>>,
        message: &GuiChatMessage,
    ) -> bool {
        let sender_option = {
            let guard = message_sender.lock().await;
            guard.clone()
        };

        if let Some(sender) = sender_option {
            if sender.send(message.clone()).is_err() {
                tracing::warn!("?? [API_SERVICE] Dropping message sender because receiver hung up");
                let mut guard = message_sender.lock().await;
                guard.take();
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// 現在の状態を取得
    async fn reset_stream_end_detector(&self) {
        let mut detector = self.stream_end_detector.lock().await;
        detector.reset();
    }

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
        let stream_end_detector = Arc::clone(&self.stream_end_detector);
        let message_sender = Arc::clone(&self.message_sender);

        tokio::spawn(async move {
            let mut request_count = 0;
            let mut consecutive_errors = 0;
            let mut last_successful_request = std::time::Instant::now();
            let _start_time = std::time::Instant::now();
            const HEALTH_CHECK_INTERVAL_SECS: u64 = 30;
            const DEFAULT_POLL_INTERVAL_MS: u64 = 1500; // デフォルト1.5秒
            const MIN_POLL_INTERVAL_MS: u64 = 300;      // 最小300ms（高速チャット対応）
            const MAX_POLL_INTERVAL_MS: u64 = 1500;     // 最大1.5秒（取りこぼし防止）

            // 次のポーリングまでの待機時間（初回は即座に実行）
            let mut next_poll_delay_ms: u64 = 0;

            tracing::info!("🚀 Message receiver task started (dynamic polling enabled)");

            loop {
                // 動的な待機時間でスリープ（シャットダウンシグナルも監視）
                let sleep_future = tokio::time::sleep(tokio::time::Duration::from_millis(next_poll_delay_ms));
                tokio::select! {
                    _ = shutdown_receiver.recv() => {
                        tracing::info!("🛑 Shutdown signal received, stopping message receiver");
                        break;
                    }
                    _ = sleep_future => {
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

                                    // StreamEndDetectorに成功を通知
                                    {
                                        let mut detector = stream_end_detector.lock().await;
                                        detector.on_success();
                                    }

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

                                    // 継続トークンとポーリング間隔を更新
                                    if let Some(continuation_info) = get_next_continuation_with_timeout(&response) {
                                        // 動的ポーリング間隔を設定（処理時間を差し引く）
                                        let target_interval_ms = continuation_info
                                            .timeout_ms
                                            .map(|ms| ms.clamp(MIN_POLL_INTERVAL_MS, MAX_POLL_INTERVAL_MS))
                                            .unwrap_or(DEFAULT_POLL_INTERVAL_MS);

                                        // 処理時間を差し引いて実際の待機時間を計算
                                        let elapsed_ms = request_start.elapsed().as_millis() as u64;
                                        next_poll_delay_ms = target_interval_ms.saturating_sub(elapsed_ms);

                                        // 最小でも100ms待機（CPUビジーループ防止）
                                        if next_poll_delay_ms < 100 {
                                            next_poll_delay_ms = 100;
                                        }

                                        if should_log_request {
                                            tracing::debug!(
                                                "🔄 Updating continuation token (next poll in {}ms, target {}ms, elapsed {}ms)",
                                                next_poll_delay_ms,
                                                target_interval_ms,
                                                elapsed_ms
                                            );
                                        }

                                        inner_tube_client.continuation = Continuation(continuation_info.continuation.clone());

                                        // StateManagerにも継続トークンを保存
                                        use crate::gui::state_management::{get_state_manager, AppEvent};
                                        let _ = get_state_manager().send_event(AppEvent::ContinuationTokenUpdated(Some(continuation_info.continuation)));
                                    } else {
                                        tracing::warn!("⚠️ No next continuation token found in response #{}", request_count);
                                        // 継続トークンがない場合はデフォルト間隔を使用
                                        next_poll_delay_ms = DEFAULT_POLL_INTERVAL_MS;
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

                                            // 🚀 最適化: ChatItemをGuiChatMessageに変換（最小クローン）
                                            let conversion_start = std::time::Instant::now();
                                            let gui_message: GuiChatMessage = chat_item.clone().into(); // 必要最小限のクローン
                                            let conversion_duration = conversion_start.elapsed();

                                            tracing::info!(
                                                "📝 [API_SERVICE] New message converted in {:?}: {} - '{}'",
                                                conversion_duration,
                                                gui_message.author,
                                                gui_message.content.chars().take(50).collect::<String>()
                                            );

                                            // 新しい状態管理システム（StateManager）のみを使用

                                            // 🚀 最適化: ログ用データをmove前に取得
                                            let author_for_log = gui_message.author.clone();
                                            let content_preview = gui_message.content.chars().take(30).collect::<String>();

                                            // 🚀 最適化: イベント駆動状態管理にメッセージを送信（move使用）
                                            use crate::gui::state_management::{get_state_manager, AppEvent};
                                            let state_send_start = std::time::Instant::now();
                                            let send_result = get_state_manager().send_event(AppEvent::MessageAdded(gui_message.clone())); // 一時的にクローン保持（ファイル保存でも使用のため）
                                            let state_send_duration = state_send_start.elapsed();

                                            match send_result {
                                                Ok(()) => {
                                                    tracing::info!(
                                                        "📤 [API_SERVICE] Message sent to StateManager in {:?}: {} - {}",
                                                        state_send_duration,
                                                        author_for_log,
                                                        content_preview
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
                                            if !Self::broadcast_to_receivers(&message_sender, &gui_message).await {
                                                tracing::trace!("?? [API_SERVICE] No external message receiver registered");
                                            }

                                            // WebSocket APIにブロードキャスト
                                            {
                                                let ws_server = crate::api::websocket_server::get_websocket_server();
                                                if ws_server.is_running().await {
                                                    ws_server.broadcast_message(&gui_message).await;
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
                                Ok(Err(e)) => {
                                    consecutive_errors += 1;
                                    let request_duration = request_start.elapsed();
                                    let error_str = e.to_string();

                                    // エラー時はデフォルト間隔を使用
                                    next_poll_delay_ms = DEFAULT_POLL_INTERVAL_MS;

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

                                    // StreamEndDetectorでエラーを分析
                                    let detection_result = {
                                        let mut detector = stream_end_detector.lock().await;
                                        detector.on_error(&error_str)
                                    };

                                    let wait_duration_secs = if error_str.contains("403") || error_str.contains("Forbidden") {
                                        if consecutive_errors >= 3 {
                                            Some(std::cmp::min(consecutive_errors * 3, 30) as u64)
                                        } else {
                                            None
                                        }
                                    } else if consecutive_errors >= 3 {
                                        Some(std::cmp::min(consecutive_errors * 2, 20) as u64)
                                    } else {
                                        None
                                    };

                                    let should_break = LiveChatService::handle_detection_result_internal(
                                        &state,
                                        detection_result,
                                        format!("API Error ({}): {}", consecutive_errors, e),
                                        consecutive_errors,
                                        wait_duration_secs,
                                        &error_str,
                                    )
                                    .await;

                                    if should_break {
                                        break;
                                    }
                                }
                                Err(_timeout_error) => {
                                    consecutive_errors += 1;
                                    let request_duration = request_start.elapsed();

                                    // タイムアウト時はデフォルト間隔を使用
                                    next_poll_delay_ms = DEFAULT_POLL_INTERVAL_MS;

                                    tracing::error!(
                                        "⏰ [API_SERVICE] Request #{} timed out after {:?} (consecutive timeouts: {})",
                                        request_count,
                                        request_duration,
                                        consecutive_errors
                                    );

                                    // StreamEndDetectorでタイムアウトエラーを分析
                                    let detection_result = {
                                        let mut detector = stream_end_detector.lock().await;
                                        detector.on_error("timeout")
                                    };

                                    let wait_duration_secs = if consecutive_errors >= 3 {
                                        Some(std::cmp::min(consecutive_errors * 2, 20) as u64)
                                    } else {
                                        None
                                    };

                                    let should_break = LiveChatService::handle_detection_result_internal(
                                        &state,
                                        detection_result,
                                        format!("Timeout ({})", consecutive_errors),
                                        consecutive_errors,
                                        wait_duration_secs,
                                        "timeout",
                                    )
                                        .await;

                                    if should_break {
                                        break;
                                    }
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

            let mut sender_guard = message_sender.lock().await;
            sender_guard.take();
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

        file.write_all(
            format!(
                "{}
",
                json_line
            )
            .as_bytes(),
        )
        .await?;
        file.flush().await?;

        Ok(())
    }

    /// Phase 2.2: use_resource統合用バッチメッセージ取得
    ///
    /// 現在のメッセージバッファから最新のメッセージをバッチで取得
    pub async fn get_recent_messages_batch(&mut self) -> anyhow::Result<Vec<GuiChatMessage>> {
        // StateManagerから現在のメッセージを取得
        use crate::gui::state_management::get_state_manager;

        let current_state = get_state_manager().get_state_unchecked();
        let messages = current_state.messages();

        tracing::debug!(
            "🚀 [BATCH_FETCH] Retrieved {} messages from state manager",
            messages.len()
        );

        Ok(messages)
    }

    /// Phase 2.2: 最新N件のメッセージを取得（use_resource用）
    pub async fn get_latest_messages(
        &mut self,
        count: usize,
    ) -> anyhow::Result<Vec<GuiChatMessage>> {
        use crate::gui::state_management::get_state_manager;

        let current_state = get_state_manager().get_state_unchecked();
        let recent_messages = current_state.recent_messages(count);

        tracing::debug!(
            "🚀 [LATEST_FETCH] Retrieved {} latest messages (requested: {})",
            recent_messages.len(),
            count
        );

        Ok(recent_messages)
    }

    /// Phase 2.2: 差分メッセージ取得（効率的な更新用）
    pub async fn get_new_messages_since(
        &mut self,
        last_count: usize,
    ) -> anyhow::Result<Vec<GuiChatMessage>> {
        use crate::gui::state_management::get_state_manager;

        let current_state = get_state_manager().get_state_unchecked();
        let all_messages = current_state.messages();
        let current_count = all_messages.len();

        if current_count > last_count {
            let new_messages = all_messages.iter().skip(last_count).cloned().collect();

            tracing::info!(
                "🚀 [DIFF_FETCH] Retrieved {} new messages (total: {} → {})",
                current_count - last_count,
                last_count,
                current_count
            );

            Ok(new_messages)
        } else {
            tracing::debug!(
                "🚀 [DIFF_FETCH] No new messages (current: {}, last: {})",
                current_count,
                last_count
            );
            Ok(Vec::new())
        }
    }
}

impl Default for LiveChatService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_service() -> LiveChatService {
        LiveChatService::new()
    }

    #[tokio::test]
    async fn reconnect_resets_consecutive_errors() {
        let mut service = init_test_service();

        service.set_test_fetch_live_chat_page(Err(anyhow::anyhow!("forced error")));
        assert!(service.start_monitoring("test_url", None).await.is_err());
        service.stop_monitoring().await.unwrap();

        let state_guard = service.state.lock().await;
        assert_eq!(*state_guard, ServiceState::Idle);
    }

    #[tokio::test]
    async fn detection_stream_end_sets_idle_and_breaks() {
        let service = init_test_service();
        let result = service
            .test_handle_detection_result(
                DetectionResult::StreamEnded,
                "Stream ended".to_string(),
                1,
                None,
                "stream end",
            )
            .await;
        assert!(result);
        assert_eq!(*service.state.lock().await, ServiceState::Idle);
    }

    #[tokio::test]
    async fn detection_warning_sets_error_and_continues() {
        let service = init_test_service();
        let result = service
            .test_handle_detection_result(
                DetectionResult::Warning,
                "API Error (2): temporary".to_string(),
                2,
                None,
                "temporary warning",
            )
            .await;
        assert!(!result);
        let state_value = { service.state.lock().await.clone() };
        match state_value {
            ServiceState::Error(message) => assert!(message.contains("API Error")),
            other => panic!("expected error state, found {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_path_notifies_stream_end_detector_and_sets_state() {
        let mut service = init_test_service();

        {
            let mut guard = service.state.lock().await;
            *guard = ServiceState::Error("API Error".to_string());
        }

        service.stop_monitoring().await.unwrap();
        let state_guard = service.state.lock().await;
        assert_eq!(*state_guard, ServiceState::Idle);
    }

    #[tokio::test]
    async fn broadcast_sends_message_to_registered_receiver() {
        let service = LiveChatService::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        {
            let mut sender_guard = service.message_sender.lock().await;
            *sender_guard = Some(tx);
        }

        let message = GuiChatMessage {
            author: "tester".to_string(),
            content: "hello".to_string(),
            ..GuiChatMessage::default()
        };

        let delivered =
            LiveChatService::broadcast_to_receivers(&service.message_sender, &message).await;
        assert!(
            delivered,
            "expected message to be delivered to registered receiver"
        );

        let received = rx.recv().await.expect("receiver should obtain a message");
        assert_eq!(received, message);
    }

    #[tokio::test]
    async fn stop_monitoring_closes_message_channel() {
        let mut service = LiveChatService::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        {
            let mut sender_guard = service.message_sender.lock().await;
            *sender_guard = Some(tx);
        }

        service
            .stop_monitoring()
            .await
            .expect("stop_monitoring should succeed");

        assert!(
            rx.recv().await.is_none(),
            "channel should close after stop_monitoring"
        );
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
