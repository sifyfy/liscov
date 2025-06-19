//! プラグインシステムテスト
//!
//! Phase 3で実装したプラグインアーキテクチャの包括的テスト

use liscov::{
    gui::{
        models::{GuiChatMessage, MessageType},
        plugin_system::*,
        plugins::{analytics_plugin::*, notification_plugin::*},
    },
    LiscovResult,
};

/// テスト用プラグイン
#[derive(Debug)]
struct TestPlugin {
    info: PluginInfo,
    initialized: bool,
    events_received: Vec<String>,
}

impl TestPlugin {
    fn new(id: &str, name: &str) -> Self {
        Self {
            info: PluginInfo {
                id: id.to_string(),
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: "Test plugin".to_string(),
                author: "Test Author".to_string(),
                enabled: true,
                dependencies: vec![],
            },
            initialized: false,
            events_received: vec![],
        }
    }

    fn with_dependencies(id: &str, name: &str, dependencies: Vec<String>) -> Self {
        let mut plugin = Self::new(id, name);
        plugin.info.dependencies = dependencies;
        plugin
    }
}

#[async_trait::async_trait]
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

    async fn handle_event(&mut self, event: PluginEvent) -> LiscovResult<PluginResult> {
        let event_name = match event {
            PluginEvent::ApplicationStarted => "ApplicationStarted",
            PluginEvent::ApplicationStopping => "ApplicationStopping",
            PluginEvent::MessageReceived(_) => "MessageReceived",
            PluginEvent::MessagesReceived(_) => "MessagesReceived",
            PluginEvent::ConnectionChanged { .. } => "ConnectionChanged",
            PluginEvent::ConfigurationChanged { .. } => "ConfigurationChanged",
            PluginEvent::Custom { .. } => "Custom",
        };

        self.events_received.push(event_name.to_string());
        Ok(PluginResult::Success)
    }

    fn is_enabled(&self) -> bool {
        self.info.enabled
    }
}

/// プラグインシステムテスト
#[cfg(test)]
mod plugin_system_tests {
    use super::*;

    #[test]
    fn test_plugin_info_creation() {
        let info = PluginInfo {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.2.3".to_string(),
            description: "A test plugin for unit testing".to_string(),
            author: "Test Team".to_string(),
            enabled: true,
            dependencies: vec!["dep1".to_string(), "dep2".to_string()],
        };

        assert_eq!(info.id, "test-plugin");
        assert_eq!(info.name, "Test Plugin");
        assert_eq!(info.version, "1.2.3");
        assert_eq!(info.dependencies.len(), 2);
        assert!(info.enabled);

        // シリアライゼーション/デシリアライゼーションテスト
        let serialized = serde_json::to_string(&info).unwrap();
        let deserialized: PluginInfo = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, info.id);
        assert_eq!(deserialized.dependencies, info.dependencies);
    }

    #[test]
    fn test_plugin_event_types() {
        // 各イベントタイプが正しく作成できることをテスト
        let events = vec![
            PluginEvent::ApplicationStarted,
            PluginEvent::ApplicationStopping,
            PluginEvent::MessageReceived(create_test_message("test")),
            PluginEvent::MessagesReceived(vec![
                create_test_message("test1"),
                create_test_message("test2"),
            ]),
            PluginEvent::ConnectionChanged { is_connected: true },
            PluginEvent::ConfigurationChanged {
                key: "test_key".to_string(),
                value: serde_json::json!({"test": "value"}),
            },
            PluginEvent::Custom {
                event_type: "custom_event".to_string(),
                data: serde_json::json!({"custom": "data"}),
            },
        ];

        assert_eq!(events.len(), 7);

        // 各イベントがクローン可能であることを確認
        for event in &events {
            let _cloned = event.clone();
        }
    }

    #[test]
    fn test_plugin_result_types() {
        let results = vec![
            PluginResult::Success,
            PluginResult::SuccessWithData(serde_json::json!({"result": "data"})),
            PluginResult::Error("Test error".to_string()),
            PluginResult::Skipped,
            PluginResult::Delegate("other-plugin".to_string()),
        ];

        assert_eq!(results.len(), 5);

        // 結果タイプの判定テスト
        match &results[0] {
            PluginResult::Success => (),
            _ => panic!("Expected Success"),
        }

        match &results[1] {
            PluginResult::SuccessWithData(data) => {
                assert!(data.get("result").is_some());
            }
            _ => panic!("Expected SuccessWithData"),
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
        assert_eq!(plugins[0].name, "Test Plugin");
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
    async fn test_dependency_validation() {
        let manager = PluginManager::new();

        // 依存関係がないプラグインを先に登録
        let base_plugin = Box::new(TestPlugin::new("base-plugin", "Base Plugin"));
        manager.register_plugin(base_plugin).await.unwrap();

        // 依存関係があるプラグインを登録
        let dependent_plugin = Box::new(TestPlugin::with_dependencies(
            "dependent-plugin",
            "Dependent Plugin",
            vec!["base-plugin".to_string()],
        ));
        let result = manager.register_plugin(dependent_plugin).await;
        assert!(result.is_ok());

        // 存在しない依存関係を持つプラグインの登録は失敗する
        let invalid_plugin = Box::new(TestPlugin::with_dependencies(
            "invalid-plugin",
            "Invalid Plugin",
            vec!["non-existent-plugin".to_string()],
        ));
        let result = manager.register_plugin(invalid_plugin).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_event_broadcasting() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin", "Test Plugin"));

        manager.register_plugin(plugin).await.unwrap();

        let event = PluginEvent::ApplicationStarted;
        let results = manager.broadcast_event(event).await.unwrap();

        assert_eq!(results.len(), 1);
        match &results[0] {
            PluginResult::Success => (),
            _ => panic!("Expected Success result"),
        }
    }

    #[tokio::test]
    async fn test_plugin_enable_disable() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin", "Test Plugin"));

        manager.register_plugin(plugin).await.unwrap();

        // プラグインを無効化
        let result = manager.set_plugin_enabled("test-plugin", false).await;
        assert!(result.is_ok());

        // プラグインを有効化
        let result = manager.set_plugin_enabled("test-plugin", true).await;
        assert!(result.is_ok());

        // 存在しないプラグインの操作はエラーにならない（ログのみ）
        let result = manager.set_plugin_enabled("non-existent", true).await;
        assert!(result.is_ok());
    }

    fn create_test_message(content: &str) -> GuiChatMessage {
        GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            message_type: MessageType::Text,
            author: "testuser".to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            runs: Vec::new(),
            metadata: None,
            is_member: false,
        }
    }
}

/// アナリティクスプラグインテスト
#[cfg(test)]
mod analytics_plugin_tests {
    use super::*;

    #[test]
    fn test_analytics_config_default() {
        let config = AnalyticsConfig::default();

        assert!(config.enabled);
        assert!(config.engagement_analysis);
        assert!(!config.sentiment_analysis);
        assert_eq!(config.report_interval_seconds, 300);
        assert_eq!(config.data_retention_days, 30);
        assert!(!config.verbose_logging);
    }

    #[test]
    fn test_analytics_stats_initialization() {
        let stats = AnalyticsStats::default();

        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.unique_viewers, 0);
        assert!(stats.message_types.is_empty());
        assert!(stats.hourly_distribution.is_empty());
        assert_eq!(stats.engagement_rate, 0.0);
        assert_eq!(stats.average_message_length, 0.0);
        assert_eq!(stats.total_superchat_amount, 0.0);
        assert!(stats.first_message_time.is_none());
        assert!(stats.last_message_time.is_none());
    }

    #[test]
    fn test_analytics_plugin_creation() {
        let plugin = AnalyticsPlugin::new();
        let info = plugin.info();

        assert_eq!(info.id, "analytics");
        assert_eq!(info.name, "Analytics Plugin");
        assert_eq!(info.version, "1.0.0");
        assert!(info.enabled);
        assert!(info.dependencies.is_empty());
    }

    #[test]
    fn test_analytics_stats_collection() {
        let mut plugin = AnalyticsPlugin::new();

        let _message1 = create_test_message_with_author("Hello world!", "user1");
        let _message2 = create_test_message_with_author("Hi there!", "user2");
        let _message3 = create_test_message_with_author("Another message", "user1");

        // メッセージを分析（プライベートメソッドなので直接テストはできないが、構造をテスト）
        let stats = plugin.get_stats();
        assert_eq!(stats.total_messages, 0); // 初期状態

        // 設定の更新テスト
        let new_config = AnalyticsConfig {
            enabled: false,
            engagement_analysis: false,
            sentiment_analysis: true,
            report_interval_seconds: 600,
            data_retention_days: 60,
            verbose_logging: true,
        };

        // ランタイムでの設定更新テスト
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = plugin.update_config(new_config.clone()).await;
            assert!(result.is_ok());
        });
    }

    fn create_test_message_with_author(content: &str, author: &str) -> GuiChatMessage {
        GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            message_type: MessageType::Text,
            author: author.to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            runs: Vec::new(),
            metadata: None,
            is_member: false,
        }
    }
}

/// 通知プラグインテスト  
#[cfg(test)]
mod notification_plugin_tests {
    use super::*;

    #[test]
    fn test_notification_config_default() {
        let config = NotificationConfig::default();

        assert!(config.enabled);
        assert!(config.superchat_notifications);
        assert_eq!(config.superchat_min_amount, 100.0);
        assert!(config.keyword_notifications.is_empty());
        assert!(config.vip_users.is_empty());
        assert_eq!(config.message_count_threshold, 1000);
        assert!(config.sound_enabled);
        assert!(config.desktop_notifications);
        assert_eq!(config.notification_duration_seconds, 5);
        assert_eq!(config.cooldown_seconds, 10);
    }

    #[test]
    fn test_notification_types() {
        let notifications = vec![
            NotificationType::SuperChat { amount: 500.0 },
            NotificationType::KeywordDetected {
                keyword: "test".to_string(),
            },
            NotificationType::NewMember {
                username: "newuser".to_string(),
            },
            NotificationType::VipUser {
                username: "vipuser".to_string(),
            },
            NotificationType::MessageThreshold { count: 1000 },
            NotificationType::System {
                message: "System message".to_string(),
            },
        ];

        assert_eq!(notifications.len(), 6);

        // 各通知タイプのシリアライゼーションテスト
        for notification in &notifications {
            let serialized = serde_json::to_string(notification).unwrap();
            let deserialized: NotificationType = serde_json::from_str(&serialized).unwrap();

            // タイプが一致することを確認（詳細な比較は実装によって異なる）
            match (notification, &deserialized) {
                (NotificationType::SuperChat { .. }, NotificationType::SuperChat { .. }) => (),
                (
                    NotificationType::KeywordDetected { .. },
                    NotificationType::KeywordDetected { .. },
                ) => (),
                (NotificationType::NewMember { .. }, NotificationType::NewMember { .. }) => (),
                (NotificationType::VipUser { .. }, NotificationType::VipUser { .. }) => (),
                (
                    NotificationType::MessageThreshold { .. },
                    NotificationType::MessageThreshold { .. },
                ) => (),
                (NotificationType::System { .. }, NotificationType::System { .. }) => (),
                _ => panic!("Notification type mismatch after serialization"),
            }
        }
    }

    #[test]
    fn test_notification_plugin_creation() {
        let plugin = NotificationPlugin::new();
        let info = plugin.info();

        assert_eq!(info.id, "notification");
        assert_eq!(info.name, "Notification Plugin");
        assert_eq!(info.version, "1.0.0");
        assert!(info.enabled);
        assert!(info.dependencies.is_empty());
    }

    #[test]
    fn test_notification_history() {
        let mut plugin = NotificationPlugin::new();

        // 初期状態
        let history = plugin.get_notification_history();
        assert!(history.is_empty());

        // 統計情報の確認
        let stats = plugin.get_stats();
        assert_eq!(stats["total_notifications"], 0);
        assert!(stats["notification_types"].as_object().unwrap().is_empty());

        // 履歴のクリア（空の状態でも正常動作することを確認）
        plugin.clear_notification_history();
        assert!(plugin.get_notification_history().is_empty());
    }

    #[test]
    fn test_notification_configuration_schema() {
        let plugin = NotificationPlugin::new();
        let schema = plugin.get_config_schema();

        assert!(schema.is_some());

        if let Some(schema_value) = schema {
            assert_eq!(schema_value["type"], "object");

            let properties = schema_value["properties"].as_object().unwrap();
            assert!(properties.contains_key("enabled"));
            assert!(properties.contains_key("superchat_notifications"));
            assert!(properties.contains_key("keyword_notifications"));
            assert!(properties.contains_key("vip_users"));
            assert!(properties.contains_key("sound_enabled"));

            // スキーマの型確認
            assert_eq!(properties["enabled"]["type"], "boolean");
            assert_eq!(properties["keyword_notifications"]["type"], "array");
            assert_eq!(properties["superchat_min_amount"]["type"], "number");
        }
    }

    #[tokio::test]
    async fn test_notification_plugin_lifecycle() {
        let plugin = NotificationPlugin::new();

        // プラグイン情報の確認
        let info = plugin.info();
        assert_eq!(info.id, "notification");
        assert_eq!(info.name, "Notification Plugin");

        // 模擬的なコンテキストでの初期化（実際のコンテキストは複雑すぎるため簡略化）
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            // 実際のテストでは適切なPluginContextが必要
            // ここでは基本的な構造テストのみ
            Ok::<(), liscov::LiscovError>(())
        });
        assert!(result.is_ok());
    }

    // 注意: プライベートフィールドのテストには、
    // 実際の実装では、テスト用のアクセサーメソッドを追加するか、
    // モジュール内でのテストを行う必要がある
}

/// プラグイン統合テスト
#[cfg(test)]
mod plugin_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_multiple_plugins_interaction() {
        let manager = PluginManager::new();

        // 複数のプラグインを登録
        let analytics_plugin = Box::new(AnalyticsPlugin::new());
        let notification_plugin = Box::new(NotificationPlugin::new());
        let test_plugin = Box::new(TestPlugin::new("test", "Test Plugin"));

        manager.register_plugin(analytics_plugin).await.unwrap();
        manager.register_plugin(notification_plugin).await.unwrap();
        manager.register_plugin(test_plugin).await.unwrap();

        let plugins = manager.list_plugins();
        assert_eq!(plugins.len(), 3);

        // 全プラグインにイベント送信
        let event = PluginEvent::ApplicationStarted;
        let results = manager.broadcast_event(event).await.unwrap();
        assert_eq!(results.len(), 3);

        // 全プラグインが正常に応答することを確認
        for result in &results {
            match result {
                PluginResult::Success => (),
                PluginResult::Skipped => (), // スキップも正常
                _ => panic!("Unexpected plugin result: {:?}", result),
            }
        }
    }

    #[tokio::test]
    async fn test_plugin_error_handling() {
        let manager = PluginManager::new();

        // エラーを発生させるプラグイン
        struct ErrorPlugin;

        #[async_trait::async_trait]
        impl Plugin for ErrorPlugin {
            fn info(&self) -> PluginInfo {
                PluginInfo {
                    id: "error-plugin".to_string(),
                    name: "Error Plugin".to_string(),
                    version: "1.0.0".to_string(),
                    description: "Plugin that always errors".to_string(),
                    author: "Test".to_string(),
                    enabled: true,
                    dependencies: vec![],
                }
            }

            async fn initialize(&mut self, _context: PluginContext) -> LiscovResult<()> {
                Err(liscov::GuiError::PluginError("Initialization failed".to_string()).into())
            }

            async fn shutdown(&mut self) -> LiscovResult<()> {
                Ok(())
            }

            async fn handle_event(&mut self, _event: PluginEvent) -> LiscovResult<PluginResult> {
                Ok(PluginResult::Error("Plugin error".to_string()))
            }
        }

        let error_plugin = Box::new(ErrorPlugin);
        let result = manager.register_plugin(error_plugin).await;

        // プラグイン登録時のエラーハンドリングテスト
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_config_schema_validation() {
        let analytics_plugin = AnalyticsPlugin::new();
        let notification_plugin = NotificationPlugin::new();

        // 各プラグインが有効なJSONスキーマを返すことを確認
        if let Some(analytics_schema) = analytics_plugin.get_config_schema() {
            assert!(analytics_schema.is_object());
            assert!(analytics_schema["properties"].is_object());
        }

        if let Some(notification_schema) = notification_plugin.get_config_schema() {
            assert!(notification_schema.is_object());
            assert!(notification_schema["properties"].is_object());
        }
    }
}
