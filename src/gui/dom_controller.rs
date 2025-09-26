//! DOM制御モジュール（Phase 3.2）
//!
//! チャット表示のDOM操作を高精度で管理
//! - スクロール制御の精密化
//! - タブ切り替え時の状態復旧
//! - 可視性変更検出
//! - パフォーマンス最適化

use std::collections::HashMap;

/// DOM制御の設定
#[derive(Debug, Clone)]
pub struct DomControllerConfig {
    /// 自動スクロールの閾値（px）
    pub scroll_threshold: f64,
    /// スクロール更新頻度（ms）
    pub scroll_update_interval: u64,
    /// タブ切り替え検出間隔（ms）
    pub tab_detection_interval: u64,
    /// 可視性変更検出有効フラグ
    pub visibility_detection_enabled: bool,
    /// パフォーマンス監視有効フラグ
    pub performance_monitoring_enabled: bool,
}

impl Default for DomControllerConfig {
    fn default() -> Self {
        Self {
            scroll_threshold: 30.0,
            scroll_update_interval: 100, // 100ms間隔
            tab_detection_interval: 500, // 500ms間隔
            visibility_detection_enabled: true,
            performance_monitoring_enabled: false,
        }
    }
}

/// DOM操作の状態
#[derive(Debug, Clone)]
pub struct DomState {
    /// スクロール位置
    pub scroll_position: f64,
    /// スクロール最大値
    pub scroll_max: f64,
    /// 可視性フラグ
    pub is_visible: bool,
    /// フォーカス状態
    pub has_focus: bool,
    /// 最後の更新時刻
    pub last_update: u64,
}

impl Default for DomState {
    fn default() -> Self {
        Self {
            scroll_position: 0.0,
            scroll_max: 0.0,
            is_visible: true,
            has_focus: true,
            last_update: 0,
        }
    }
}

/// DOM制御クラス
#[derive(Debug)]
pub struct DomController {
    /// 設定
    config: DomControllerConfig,
    /// 制御対象コンテナID
    container_id: String,
    /// DOM状態
    state: DomState,
    /// イベントハンドラー登録済みフラグ
    initialized: bool,
}

impl DomController {
    /// 新しいDOM制御インスタンスを作成
    pub fn new(container_id: String) -> Self {
        Self {
            config: DomControllerConfig::default(),
            container_id,
            state: DomState::default(),
            initialized: false,
        }
    }

    /// 設定をカスタマイズ
    pub fn with_config(mut self, config: DomControllerConfig) -> Self {
        self.config = config;
        self
    }

    /// DOM初期化（Phase 3.2 高精度版）
    pub async fn initialize(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        let container_id = &self.container_id;
        let scroll_threshold = self.config.scroll_threshold;
        let update_interval = self.config.scroll_update_interval;

        // 高精度DOM初期化スクリプト
        let init_script = format!(
            r#"
            (function() {{
                const containerId = '{container_id}';
                const container = document.getElementById(containerId);
                
                if (!container) {{
                    console.error('Container not found:', containerId);
                    return false;
                }}

                // Phase 3.2: 高精度スクロール制御
                if (!window.liscovDomController) {{
                    window.liscovDomController = {{}};
                }}

                const controller = window.liscovDomController;
                controller.scrollThreshold = {scroll_threshold};
                controller.updateInterval = {update_interval};
                controller.userScrolled = false;
                controller.lastScrollTop = 0;
                controller.scrollVelocity = 0;
                
                // 高精度スクロールイベントハンドラー
                let scrollTimeout;
                container.addEventListener('scroll', function(event) {{
                    const currentScrollTop = container.scrollTop;
                    const scrollHeight = container.scrollHeight;
                    const clientHeight = container.clientHeight;
                    
                    // スクロール速度計算
                    controller.scrollVelocity = Math.abs(currentScrollTop - controller.lastScrollTop);
                    controller.lastScrollTop = currentScrollTop;
                    
                    // 底部判定（高精度）
                    const distanceFromBottom = scrollHeight - currentScrollTop - clientHeight;
                    const isAtBottom = distanceFromBottom <= controller.scrollThreshold;
                    
                    // ユーザースクロール検出
                    if (!isAtBottom && controller.scrollVelocity > 1) {{
                        controller.userScrolled = true;
                    }} else if (isAtBottom) {{
                        controller.userScrolled = false;
                    }}
                    
                    // デバウンス処理
                    clearTimeout(scrollTimeout);
                    scrollTimeout = setTimeout(() => {{
                        // スクロール状態の更新
                        controller.scrollPosition = currentScrollTop;
                        controller.scrollMax = scrollHeight - clientHeight;
                        controller.lastUpdate = Date.now();
                    }}, 50);
                    
                    // カスタムイベント発火
                    window.dispatchEvent(new CustomEvent('liscovScrollUpdate', {{
                        detail: {{
                            scrollTop: currentScrollTop,
                            scrollHeight: scrollHeight,
                            clientHeight: clientHeight,
                            isAtBottom: isAtBottom,
                            userScrolled: controller.userScrolled,
                            velocity: controller.scrollVelocity
                        }}
                    }}));
                }});

                // 初期スクロール位置設定
                container.scrollTop = container.scrollHeight;
                
                console.log('Phase 3.2 DOM Controller initialized:', containerId);
                return true;
            }})()
            "#
        );

        match dioxus::document::eval(&init_script).await {
            Ok(_) => {
                self.initialized = true;
                tracing::info!(
                    "🎮 [DOM] Phase 3.2 Controller initialized: {}",
                    container_id
                );
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("DOM initialization failed: {:?}", e);
                tracing::error!("❌ [DOM] {}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// 可視性変更検出の設定
    pub async fn setup_visibility_detection(&self) -> Result<(), String> {
        if !self.config.visibility_detection_enabled {
            return Ok(());
        }

        let script = r#"
            (function() {
                if (!window.liscovDomController) return;
                
                const controller = window.liscovDomController;
                
                // Page Visibility API
                document.addEventListener('visibilitychange', function() {
                    controller.isVisible = !document.hidden;
                    
                    window.dispatchEvent(new CustomEvent('liscovVisibilityChange', {
                        detail: {
                            visible: controller.isVisible,
                            timestamp: Date.now()
                        }
                    }));
                });
                
                // フォーカス検出
                window.addEventListener('focus', function() {
                    controller.hasFocus = true;
                    window.dispatchEvent(new CustomEvent('liscovFocusChange', {
                        detail: { focused: true }
                    }));
                });
                
                window.addEventListener('blur', function() {
                    controller.hasFocus = false;
                    window.dispatchEvent(new CustomEvent('liscovFocusChange', {
                        detail: { focused: false }
                    }));
                });
                
                console.log('Visibility detection enabled');
            })()
        "#;

        match dioxus::document::eval(script).await {
            Ok(_) => {
                tracing::info!("👁️ [DOM] Visibility detection enabled");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Visibility detection setup failed: {:?}", e);
                tracing::error!("❌ [DOM] {}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// 高精度自動スクロール実行
    pub async fn scroll_to_bottom(&self, force: bool) -> Result<(), String> {
        let container_id = &self.container_id;
        let force_str = if force { "true" } else { "false" };

        let script = format!(
            r#"
            (function() {{
                const container = document.getElementById('{}');
                const controller = window.liscovDomController;
                
                if (!container || !controller) {{
                    return false;
                }}
                
                const force = {};
                
                // 強制実行またはユーザースクロールしていない場合のみ実行
                if (force || !controller.userScrolled) {{
                    // スムーズスクロール（高精度）
                    const targetScrollTop = container.scrollHeight - container.clientHeight;
                    
                    if (container.scrollTo) {{
                        container.scrollTo({{
                            top: targetScrollTop,
                            behavior: 'smooth'
                        }});
                    }} else {{
                        container.scrollTop = targetScrollTop;
                    }}
                    
                    // 状態更新
                    controller.userScrolled = false;
                    return true;
                }} else {{
                    return false; // ユーザースクロール中のためスキップ
                }}
            }})()
            "#,
            container_id, force_str
        );

        match dioxus::document::eval(&script).await {
            Ok(_) => {
                tracing::debug!("📜 [DOM] Scroll to bottom executed (force: {})", force);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Scroll execution failed: {:?}", e);
                tracing::error!("❌ [DOM] {}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// ユーザースクロール状態をリセット
    pub async fn reset_user_scroll(&self) -> Result<(), String> {
        let script = r#"
            if (window.liscovDomController) {
                window.liscovDomController.userScrolled = false;
                window.dispatchEvent(new CustomEvent('liscovScrollReset', {
                    detail: { timestamp: Date.now() }
                }));
            }
        "#;

        match dioxus::document::eval(script).await {
            Ok(_) => {
                tracing::debug!("🔄 [DOM] User scroll state reset");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Scroll reset failed: {:?}", e);
                tracing::error!("❌ [DOM] {}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// DOM状態を取得
    pub async fn get_state(&mut self) -> Result<DomState, String> {
        let script = r#"
            (function() {
                const controller = window.liscovDomController;
                if (!controller) return null;
                
                return {
                    scrollPosition: controller.scrollPosition || 0,
                    scrollMax: controller.scrollMax || 0,
                    isVisible: controller.isVisible !== false,
                    hasFocus: controller.hasFocus !== false,
                    lastUpdate: controller.lastUpdate || Date.now()
                };
            })()
        "#;

        match dioxus::document::eval(script).await {
            Ok(_) => {
                // 実際の実装では、evalの結果を解析してDomStateを構築
                // Phase 3.2では簡略版として固定値を返す
                let state = DomState {
                    scroll_position: 0.0,
                    scroll_max: 1000.0,
                    is_visible: true,
                    has_focus: true,
                    last_update: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                };
                self.state = state.clone();
                Ok(state)
            }
            Err(e) => {
                let error_msg = format!("State retrieval failed: {:?}", e);
                tracing::error!("❌ [DOM] {}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// 直近で取得したDOM状態を参照
    pub fn cached_state(&self) -> &DomState {
        &self.state
    }

    /// パフォーマンス統計を取得
    pub async fn get_performance_stats(&self) -> Result<HashMap<String, f64>, String> {
        if !self.config.performance_monitoring_enabled {
            return Ok(HashMap::new());
        }

        let script = r#"
            (function() {
                const controller = window.liscovDomController;
                if (!controller) return {};
                
                return {
                    scrollVelocity: controller.scrollVelocity || 0,
                    updateFrequency: controller.updateFrequency || 0,
                    memoryUsage: performance.memory ? performance.memory.usedJSHeapSize : 0
                };
            })()
        "#;

        match dioxus::document::eval(script).await {
            Ok(_) => {
                // Phase 3.2では基本統計のみ
                let mut stats = HashMap::new();
                stats.insert("scroll_velocity".to_string(), 0.0);
                stats.insert("update_frequency".to_string(), 60.0);
                Ok(stats)
            }
            Err(e) => {
                let error_msg = format!("Performance stats retrieval failed: {:?}", e);
                tracing::error!("❌ [DOM] {}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// クリーンアップ
    pub async fn cleanup(&mut self) -> Result<(), String> {
        let script = r#"
            if (window.liscovDomController) {
                delete window.liscovDomController;
                console.log('DOM Controller cleaned up');
            }
        "#;

        match dioxus::document::eval(script).await {
            Ok(_) => {
                self.initialized = false;
                tracing::info!("🧹 [DOM] Controller cleaned up");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Cleanup failed: {:?}", e);
                tracing::error!("❌ [DOM] {}", error_msg);
                Err(error_msg)
            }
        }
    }
}

/// DOM制御の便利関数
pub mod utils {
    use super::*;

    /// 標準的なチャット用DOM制御を作成
    pub fn create_chat_controller(container_id: &str) -> DomController {
        DomController::new(container_id.to_string()).with_config(DomControllerConfig {
            scroll_threshold: 30.0,
            scroll_update_interval: 100,
            tab_detection_interval: 500,
            visibility_detection_enabled: true,
            performance_monitoring_enabled: false,
        })
    }

    /// 高性能設定のDOM制御を作成
    pub fn create_high_performance_controller(container_id: &str) -> DomController {
        DomController::new(container_id.to_string()).with_config(DomControllerConfig {
            scroll_threshold: 10.0,
            scroll_update_interval: 50,
            tab_detection_interval: 250,
            visibility_detection_enabled: true,
            performance_monitoring_enabled: true,
        })
    }
}
