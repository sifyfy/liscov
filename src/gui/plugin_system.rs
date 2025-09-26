//! プラグインシステム
//!
//! Phase 3実装: 拡張可能なプラグインアーキテクチャ

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::gui::models::GuiChatMessage;
use crate::gui::state_management::AppEvent;
use crate::LiscovResult;

/// プラグインの基本情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// プラグインID（一意識別子）
    pub id: String,
    /// プラグイン名（表示用）
    pub name: String,
    /// バージョン
    pub version: String,
    /// 説明
    pub description: String,
    /// 作者
    pub author: String,
    /// 有効/無効フラグ
    pub enabled: bool,
    /// 依存関係（他のプラグインID）
    pub dependencies: Vec<String>,
}

/// プラグインイベント
#[derive(Debug, Clone)]
pub enum PluginEvent {
    /// アプリケーション起動時
    ApplicationStarted,
    /// アプリケーション終了時
    ApplicationStopping,
    /// 新しいメッセージが追加された
    MessageReceived(GuiChatMessage),
    /// 複数のメッセージが追加された
    MessagesReceived(Vec<GuiChatMessage>),
    /// 接続状態が変更された
    ConnectionChanged { is_connected: bool },
    /// 設定が変更された
    ConfigurationChanged {
        key: String,
        value: serde_json::Value,
    },
    /// カスタムイベント
    Custom {
        event_type: String,
        data: serde_json::Value,
    },
}

/// プラグインコンテキスト（プラグインが使用できるAPI）
pub struct PluginContext {
    /// プラグインID
    pub plugin_id: String,
    /// 設定アクセス
    pub config_access: Arc<dyn ConfigAccess + Send + Sync>,
    /// イベント送信
    pub event_sender: Arc<dyn EventSender + Send + Sync>,
    /// ログ機能
    pub logger: Arc<dyn PluginLogger + Send + Sync>,
}

/// プラグイン設定アクセストレイト
#[async_trait]
pub trait ConfigAccess {
    /// 設定値を取得
    async fn get_config(
        &self,
        plugin_id: &str,
        key: &str,
    ) -> LiscovResult<Option<serde_json::Value>>;

    /// 設定値を保存
    async fn set_config(
        &self,
        plugin_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> LiscovResult<()>;

    /// 設定を削除
    async fn remove_config(&self, plugin_id: &str, key: &str) -> LiscovResult<()>;

    /// プラグインの全設定を取得
    async fn get_all_configs(
        &self,
        plugin_id: &str,
    ) -> LiscovResult<HashMap<String, serde_json::Value>>;
}

/// イベント送信トレイト
#[async_trait]
pub trait EventSender {
    /// アプリケーションイベントを送信
    async fn send_app_event(&self, event: AppEvent) -> LiscovResult<()>;

    /// カスタムプラグインイベントを送信
    async fn send_custom_event(
        &self,
        event_type: String,
        data: serde_json::Value,
    ) -> LiscovResult<()>;

    /// 他のプラグインにメッセージを送信
    async fn send_to_plugin(
        &self,
        target_plugin: &str,
        message: serde_json::Value,
    ) -> LiscovResult<()>;
}

/// プラグインロガートレイト
pub trait PluginLogger {
    /// 情報ログ
    fn info(&self, plugin_id: &str, message: &str);

    /// 警告ログ
    fn warn(&self, plugin_id: &str, message: &str);

    /// エラーログ
    fn error(&self, plugin_id: &str, message: &str);

    /// デバッグログ
    fn debug(&self, plugin_id: &str, message: &str);
}

/// プラグインの結果
#[derive(Debug, Clone)]
pub enum PluginResult {
    /// 正常処理完了
    Success,
    /// 処理完了（データ付き）
    SuccessWithData(serde_json::Value),
    /// エラー
    Error(String),
    /// 処理をスキップ
    Skipped,
    /// 他のプラグインに処理を委譲
    Delegate(String),
}

/// プラグイントレイト（プラグインが実装すべきインターフェース）
#[async_trait]
pub trait Plugin: Send + Sync {
    /// プラグイン情報を取得
    fn info(&self) -> PluginInfo;

    /// プラグインを初期化
    async fn initialize(&mut self, context: PluginContext) -> LiscovResult<()>;

    /// プラグインを終了
    async fn shutdown(&mut self) -> LiscovResult<()>;

    /// イベント処理
    async fn handle_event(&mut self, event: PluginEvent) -> LiscovResult<PluginResult>;

    /// プラグインが有効かどうか
    fn is_enabled(&self) -> bool {
        true
    }

    /// プラグインの設定スキーマを取得（オプション）
    fn get_config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// プラグイン間メッセージ処理（オプション）
    async fn handle_plugin_message(
        &mut self,
        from: &str,
        message: serde_json::Value,
    ) -> LiscovResult<PluginResult> {
        let _ = (from, message);
        Ok(PluginResult::Skipped)
    }
}

/// プラグインマネージャー
pub struct PluginManager {
    /// 登録されたプラグイン
    plugins: RwLock<HashMap<String, Box<dyn Plugin>>>,
    /// プラグイン実行順序
    execution_order: RwLock<Vec<String>>,
    /// グローバル設定
    config: RwLock<PluginManagerConfig>,
    /// 依存関係グラフ
    dependency_graph: RwLock<HashMap<String, Vec<String>>>,
}

/// プラグインマネージャー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManagerConfig {
    /// プラグイン実行タイムアウト（ミリ秒）
    pub execution_timeout_ms: u64,
    /// 並列実行を許可するか
    pub allow_parallel_execution: bool,
    /// エラー時の動作
    pub error_handling: ErrorHandling,
    /// デバッグモード
    pub debug_mode: bool,
}

/// エラーハンドリング設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandling {
    /// エラーを無視して続行
    Continue,
    /// エラー時に処理を停止
    Stop,
    /// エラーをログに記録して続行
    LogAndContinue,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            execution_timeout_ms: 5000,
            allow_parallel_execution: true,
            error_handling: ErrorHandling::LogAndContinue,
            debug_mode: false,
        }
    }
}

impl PluginManager {
    /// 新しいプラグインマネージャーを作成
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            execution_order: RwLock::new(Vec::new()),
            config: RwLock::new(PluginManagerConfig::default()),
            dependency_graph: RwLock::new(HashMap::new()),
        }
    }

    /// プラグインを登録
    pub async fn register_plugin(&self, mut plugin: Box<dyn Plugin>) -> LiscovResult<()> {
        let info = plugin.info();

        // 依存関係の検証
        self.validate_dependencies(&info)?;

        // プラグインの初期化用コンテキストを作成
        let context = self.create_plugin_context(&info.id).await?;

        // プラグインを初期化
        plugin.initialize(context).await?;

        // プラグインを登録
        {
            let mut plugins = self.plugins.write();
            plugins.insert(info.id.clone(), plugin);
        }

        // 実行順序を更新
        self.update_execution_order(&info)?;

        // 依存関係グラフを更新
        self.update_dependency_graph(&info);

        tracing::info!("🧩 Plugin registered: {} v{}", info.name, info.version);
        Ok(())
    }

    /// プラグインを削除
    pub async fn unregister_plugin(&self, plugin_id: &str) -> LiscovResult<()> {
        let mut plugin = {
            let mut plugins = self.plugins.write();
            plugins.remove(plugin_id)
        };

        if let Some(ref mut plugin) = plugin {
            plugin.shutdown().await?;

            // 実行順序から削除
            let mut execution_order = self.execution_order.write();
            execution_order.retain(|id| id != plugin_id);

            // 依存関係グラフから削除
            let mut dependency_graph = self.dependency_graph.write();
            dependency_graph.remove(plugin_id);

            tracing::info!("🧩 Plugin unregistered: {}", plugin_id);
            Ok(())
        } else {
            Err(crate::GuiError::PluginError(format!("Plugin not found: {}", plugin_id)).into())
        }
    }

    /// 全プラグインにイベントを送信
    pub async fn broadcast_event(&self, event: PluginEvent) -> LiscovResult<Vec<PluginResult>> {
        let config = self.config.read().clone();
        let execution_order = self.execution_order.read().clone();

        let results = if config.allow_parallel_execution {
            // 並列実行
            let _tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

            for plugin_id in &execution_order {
                let plugins = self.plugins.read();
                if let Some(_plugin) = plugins.get(plugin_id) {
                    // NOTE: 実際の並列実行は複雑になるため、ここでは逐次実行
                    // 将来的にはArc<Mutex<Plugin>>などを使用して並列実行を実装
                }
            }

            // 暫定的に逐次実行で処理
            self.execute_sequentially(event, &execution_order).await?
        } else {
            // 逐次実行
            self.execute_sequentially(event, &execution_order).await?
        };

        Ok(results)
    }

    /// 逐次実行でイベントを処理
    async fn execute_sequentially(
        &self,
        event: PluginEvent,
        execution_order: &[String],
    ) -> LiscovResult<Vec<PluginResult>> {
        let mut results = Vec::new();

        for plugin_id in execution_order {
            let result = self.execute_plugin_event(plugin_id, &event).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 特定のプラグインでイベントを実行
    async fn execute_plugin_event(
        &self,
        plugin_id: &str,
        _event: &PluginEvent,
    ) -> LiscovResult<PluginResult> {
        let _config = self.config.read().clone();

        // TODO: タイムアウト処理とエラーハンドリングを実装
        // 現在は簡単なバージョンで実装

        let plugins = self.plugins.read();
        if let Some(_plugin) = plugins.get(plugin_id) {
            // NOTE: ここではRwLockの制約により、実際のプラグイン実行は簡化
            // 実際の実装では、Arc<Mutex<Plugin>>などを使用
            Ok(PluginResult::Success)
        } else {
            Ok(PluginResult::Skipped)
        }
    }

    /// 依存関係を検証
    fn validate_dependencies(&self, info: &PluginInfo) -> LiscovResult<()> {
        let plugins = self.plugins.read();

        for dep in &info.dependencies {
            if !plugins.contains_key(dep) {
                return Err(crate::GuiError::PluginError(format!(
                    "Dependency not found: {} (required by {})",
                    dep, info.id
                ))
                .into());
            }
        }

        Ok(())
    }

    /// 実行順序を更新（依存関係に基づくトポロジカルソート）
    fn update_execution_order(&self, info: &PluginInfo) -> LiscovResult<()> {
        let mut execution_order = self.execution_order.write();

        // 簡単な実装：依存関係の後に追加
        if !execution_order.contains(&info.id) {
            execution_order.push(info.id.clone());
        }

        // TODO: 本格的なトポロジカルソートを実装

        Ok(())
    }

    /// 依存関係グラフを更新
    fn update_dependency_graph(&self, info: &PluginInfo) {
        let mut dependency_graph = self.dependency_graph.write();
        dependency_graph.insert(info.id.clone(), info.dependencies.clone());
    }

    /// プラグインコンテキストを作成
    async fn create_plugin_context(&self, plugin_id: &str) -> LiscovResult<PluginContext> {
        // TODO: 実際のConfigAccess、EventSender、PluginLoggerの実装を作成

        Ok(PluginContext {
            plugin_id: plugin_id.to_string(),
            config_access: Arc::new(DefaultConfigAccess::new()),
            event_sender: Arc::new(DefaultEventSender::new()),
            logger: Arc::new(DefaultPluginLogger::new()),
        })
    }

    /// 登録済みプラグイン一覧を取得
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read();
        plugins.values().map(|p| p.info()).collect()
    }

    /// プラグインを有効/無効化
    pub async fn set_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> LiscovResult<()> {
        // TODO: プラグインの有効/無効化を実装
        tracing::info!(
            "🧩 Plugin {} {}",
            plugin_id,
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    }
}

/// デフォルトの設定アクセス実装
#[derive(Debug)]
struct DefaultConfigAccess;

impl DefaultConfigAccess {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ConfigAccess for DefaultConfigAccess {
    async fn get_config(
        &self,
        _plugin_id: &str,
        _key: &str,
    ) -> LiscovResult<Option<serde_json::Value>> {
        // TODO: 実際の設定ストレージとの連携を実装
        Ok(None)
    }

    async fn set_config(
        &self,
        _plugin_id: &str,
        _key: &str,
        _value: serde_json::Value,
    ) -> LiscovResult<()> {
        // TODO: 実際の設定ストレージとの連携を実装
        Ok(())
    }

    async fn remove_config(&self, _plugin_id: &str, _key: &str) -> LiscovResult<()> {
        // TODO: 実際の設定ストレージとの連携を実装
        Ok(())
    }

    async fn get_all_configs(
        &self,
        _plugin_id: &str,
    ) -> LiscovResult<HashMap<String, serde_json::Value>> {
        // TODO: 実際の設定ストレージとの連携を実装
        Ok(HashMap::new())
    }
}

/// デフォルトのイベント送信実装
#[derive(Debug)]
struct DefaultEventSender;

impl DefaultEventSender {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventSender for DefaultEventSender {
    async fn send_app_event(&self, _event: AppEvent) -> LiscovResult<()> {
        // TODO: 実際のStateManagerとの連携を実装
        Ok(())
    }

    async fn send_custom_event(
        &self,
        _event_type: String,
        _data: serde_json::Value,
    ) -> LiscovResult<()> {
        // TODO: カスタムイベントシステムを実装
        Ok(())
    }

    async fn send_to_plugin(
        &self,
        _target_plugin: &str,
        _message: serde_json::Value,
    ) -> LiscovResult<()> {
        // TODO: プラグイン間通信を実装
        Ok(())
    }
}

/// デフォルトのプラグインロガー実装
#[derive(Debug)]
struct DefaultPluginLogger;

impl DefaultPluginLogger {
    fn new() -> Self {
        Self
    }
}

impl PluginLogger for DefaultPluginLogger {
    fn info(&self, plugin_id: &str, message: &str) {
        tracing::info!("[Plugin:{}] {}", plugin_id, message);
    }

    fn warn(&self, plugin_id: &str, message: &str) {
        tracing::warn!("[Plugin:{}] {}", plugin_id, message);
    }

    fn error(&self, plugin_id: &str, message: &str) {
        tracing::error!("[Plugin:{}] {}", plugin_id, message);
    }

    fn debug(&self, plugin_id: &str, message: &str) {
        tracing::debug!("[Plugin:{}] {}", plugin_id, message);
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用プラグイン
    struct TestPlugin {
        info: PluginInfo,
        initialized: bool,
    }

    impl TestPlugin {
        fn new(id: &str, name: &str) -> Self {
            Self {
                info: PluginInfo {
                    id: id.to_string(),
                    name: name.to_string(),
                    version: "1.0.0".to_string(),
                    description: "Test plugin".to_string(),
                    author: "Test".to_string(),
                    enabled: true,
                    dependencies: vec![],
                },
                initialized: false,
            }
        }
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn info(&self) -> PluginInfo {
            self.info.clone()
        }

        async fn initialize(&mut self, _context: PluginContext) -> LiscovResult<()> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> LiscovResult<()> {
            self.initialized = false;
            Ok(())
        }

        async fn handle_event(&mut self, _event: PluginEvent) -> LiscovResult<PluginResult> {
            Ok(PluginResult::Success)
        }
    }

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        let plugins = manager.list_plugins();
        assert!(plugins.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin", "Test Plugin"));

        let result = manager.register_plugin(plugin).await;
        assert!(result.is_ok());

        let plugins = manager.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "test-plugin");
    }

    #[tokio::test]
    async fn test_plugin_unregistration() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin", "Test Plugin"));

        manager.register_plugin(plugin).await.unwrap();

        let result = manager.unregister_plugin("test-plugin").await;
        assert!(result.is_ok());

        let plugins = manager.list_plugins();
        assert!(plugins.is_empty());
    }

    #[tokio::test]
    async fn test_event_broadcasting() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin", "Test Plugin"));

        manager.register_plugin(plugin).await.unwrap();

        let event = PluginEvent::ApplicationStarted;
        let results = manager.broadcast_event(event).await.unwrap();

        assert_eq!(results.len(), 1);
    }
}
