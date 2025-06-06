use crate::gui::state_management::{get_state_manager, AppState};

/// UI同期サービス
/// 段階的にDioxus UI層との統合を進める
pub struct UiSyncService {
    is_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// グローバル実行状態フラグ
static GLOBAL_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

impl UiSyncService {
    pub fn new() -> Self {
        Self {
            is_running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// グローバルUI同期サービスを開始（static メソッド）
    pub fn start() -> Result<(), String> {
        // CPU使用率削減のため、グローバルUI同期を完全無効化
        tracing::debug!("🎨 Global UI sync disabled for CPU optimization");
        Ok(())
    }

    #[allow(dead_code)]
    fn start_original() -> Result<(), String> {
        if GLOBAL_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(()); // 既に実行中
        }

        GLOBAL_RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);

        // バックグラウンドでUI同期を実行
        tokio::spawn(async {
            let service = UiSyncService::new();
            let is_running = std::sync::Arc::clone(&service.is_running);
            is_running.store(true, std::sync::atomic::Ordering::Relaxed);

            let mut last_state: Option<AppState> = None;
            let mut sync_counter = 0;

            tracing::info!("🎨 Global UI sync started");

            while GLOBAL_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                sync_counter += 1;

                // 50msごとに状態をチェック
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                // 現在の状態を取得
                let current_state = get_state_manager().get_state();

                // 状態変更検出
                let needs_update = match &last_state {
                    None => true, // 初回は必ず更新
                    Some(last) => {
                        last.messages.len() != current_state.messages.len()
                            || last.service_state != current_state.service_state
                            || last.is_connected != current_state.is_connected
                            || last.is_stopping != current_state.is_stopping
                    }
                };

                if needs_update {
                    // 重要な変更をログ出力
                    if let Some(last) = &last_state {
                        if last.messages.len() != current_state.messages.len() {
                            tracing::info!(
                                "🎨 UI sync: messages {} → {}",
                                last.messages.len(),
                                current_state.messages.len()
                            );
                        }
                        if last.service_state != current_state.service_state {
                            tracing::info!(
                                "🎨 UI sync: state {:?} → {:?}",
                                last.service_state,
                                current_state.service_state
                            );
                        }
                        if last.is_connected != current_state.is_connected {
                            tracing::info!(
                                "🎨 UI sync: connected {} → {}",
                                last.is_connected,
                                current_state.is_connected
                            );
                        }
                        if last.is_stopping != current_state.is_stopping {
                            tracing::info!(
                                "🎨 UI sync: stopping {} → {}",
                                last.is_stopping,
                                current_state.is_stopping
                            );
                        }
                    }

                    last_state = Some(current_state);
                } else if sync_counter % 2000 == 0 {
                    // 100秒に1回の生存確認
                    tracing::debug!("🎨 UI sync alive - no changes ({})", sync_counter);
                }
            }

            tracing::info!("🎨 Global UI sync stopped");
        });

        Ok(())
    }

    /// グローバルUI同期サービスの実行状態を確認（static メソッド）
    pub fn is_running() -> bool {
        GLOBAL_RUNNING.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// グローバルUI同期サービスを停止（static メソッド）
    pub fn stop() {
        GLOBAL_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("🎨 Global UI sync stop requested");
    }

    /// 基本的な同期テスト
    pub fn test_sync() -> bool {
        let _state = get_state_manager().get_state();
        tracing::info!("🎨 UI sync test completed successfully");
        true
    }

    /// バックグラウンドでUI同期を開始
    pub async fn start_background_sync(&self) -> tokio::task::JoinHandle<()> {
        let is_running = std::sync::Arc::clone(&self.is_running);
        is_running.store(true, std::sync::atomic::Ordering::Relaxed);

        tokio::spawn(async move {
            let mut last_state: Option<AppState> = None;
            let mut sync_counter = 0;

            tracing::info!("🎨 Background UI sync started");

            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                sync_counter += 1;

                // 50msごとに状態をチェック
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                // 現在の状態を取得
                let current_state = get_state_manager().get_state();

                // 状態変更検出
                let needs_update = match &last_state {
                    None => true, // 初回は必ず更新
                    Some(last) => {
                        last.messages.len() != current_state.messages.len()
                            || last.service_state != current_state.service_state
                            || last.is_connected != current_state.is_connected
                            || last.is_stopping != current_state.is_stopping
                    }
                };

                if needs_update {
                    // 重要な変更をログ出力
                    if let Some(last) = &last_state {
                        if last.messages.len() != current_state.messages.len() {
                            tracing::info!(
                                "🎨 UI sync: messages {} → {}",
                                last.messages.len(),
                                current_state.messages.len()
                            );
                        }
                        if last.service_state != current_state.service_state {
                            tracing::info!(
                                "🎨 UI sync: state {:?} → {:?}",
                                last.service_state,
                                current_state.service_state
                            );
                        }
                        if last.is_connected != current_state.is_connected {
                            tracing::info!(
                                "🎨 UI sync: connected {} → {}",
                                last.is_connected,
                                current_state.is_connected
                            );
                        }
                        if last.is_stopping != current_state.is_stopping {
                            tracing::info!(
                                "🎨 UI sync: stopping {} → {}",
                                last.is_stopping,
                                current_state.is_stopping
                            );
                        }
                    }

                    // TODO: ここで実際のUI更新を行う（Dioxus統合後）

                    last_state = Some(current_state);
                } else if sync_counter % 2000 == 0 {
                    // 100秒に1回の生存確認
                    tracing::debug!("🎨 UI sync alive - no changes ({})", sync_counter);
                }
            }

            tracing::info!("🎨 Background UI sync stopped");
        })
    }

    /// UI同期を停止
    pub fn stop_sync(&self) {
        self.is_running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("🎨 UI sync stop requested");
    }
}

impl Default for UiSyncService {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルUI同期サービス
static UI_SYNC_SERVICE: std::sync::OnceLock<std::sync::Mutex<UiSyncService>> =
    std::sync::OnceLock::new();

/// グローバルUI同期サービスを取得
pub fn get_ui_sync_service() -> &'static std::sync::Mutex<UiSyncService> {
    UI_SYNC_SERVICE.get_or_init(|| {
        tracing::info!("🏗️ Creating global UI sync service");
        std::sync::Mutex::new(UiSyncService::new())
    })
}

/// UI同期操作用の公開インターフェース
pub struct UiSyncActions;

impl UiSyncActions {
    /// UI同期を開始（簡素化版）
    pub fn start_sync() -> tokio::task::JoinHandle<()> {
        tokio::spawn(async {
            let service = UiSyncService::new();
            let handle = service.start_background_sync().await;
            let _ = handle.await;
        })
    }

    /// UI同期を停止
    pub fn stop_sync() {
        if let Ok(service) = get_ui_sync_service().lock() {
            service.stop_sync();
        }
    }

    /// 同期テスト
    pub fn test() -> bool {
        UiSyncService::test_sync()
    }
}
