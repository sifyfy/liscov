//! 通知プラグイン
//!
//! 特定の条件に基づいてユーザーに通知を送信するプラグイン

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::gui::models::{GuiChatMessage, MessageType};
use crate::gui::plugin_system::{Plugin, PluginContext, PluginEvent, PluginInfo, PluginResult};
use crate::LiscovResult;

/// 通知の種類
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    /// スーパーチャット受信
    SuperChat { amount: f64 },
    /// キーワード検出
    KeywordDetected { keyword: String },
    /// 新規メンバー
    NewMember { username: String },
    /// 特定ユーザーのメッセージ
    VipUser { username: String },
    /// メッセージ数の閾値到達
    MessageThreshold { count: usize },
    /// システム通知
    System { message: String },
}

/// 通知設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// 通知を有効にするか
    pub enabled: bool,
    /// スーパーチャット通知の有効化
    pub superchat_notifications: bool,
    /// スーパーチャット通知の最小金額
    pub superchat_min_amount: f64,
    /// キーワード通知リスト
    pub keyword_notifications: Vec<String>,
    /// VIPユーザーリスト
    pub vip_users: Vec<String>,
    /// メッセージ数通知の閾値
    pub message_count_threshold: usize,
    /// 音声通知を有効にするか
    pub sound_enabled: bool,
    /// デスクトップ通知を有効にするか
    pub desktop_notifications: bool,
    /// 通知の表示時間（秒）
    pub notification_duration_seconds: u32,
    /// 通知のクールダウン時間（秒）
    pub cooldown_seconds: u32,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            superchat_notifications: true,
            superchat_min_amount: 100.0,
            keyword_notifications: vec![],
            vip_users: vec![],
            message_count_threshold: 1000,
            sound_enabled: true,
            desktop_notifications: true,
            notification_duration_seconds: 5,
            cooldown_seconds: 10,
        }
    }
}

/// 通知履歴
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHistory {
    /// 通知ID
    pub id: String,
    /// 通知タイプ
    pub notification_type: NotificationType,
    /// 通知時刻
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 通知メッセージ
    pub message: String,
    /// 関連するメッセージ（存在する場合）
    pub related_message: Option<GuiChatMessage>,
}

/// 通知プラグイン
pub struct NotificationPlugin {
    config: NotificationConfig,
    context: Option<PluginContext>,
    notification_history: Vec<NotificationHistory>,
    last_notification_time: HashMap<String, chrono::DateTime<chrono::Utc>>,
    message_count: usize,
    session_start_time: chrono::DateTime<chrono::Utc>,
}

impl NotificationPlugin {
    /// 新しいプラグインインスタンスを作成
    pub fn new() -> Self {
        Self {
            config: NotificationConfig::default(),
            context: None,
            notification_history: Vec::new(),
            last_notification_time: HashMap::new(),
            message_count: 0,
            session_start_time: chrono::Utc::now(),
        }
    }

    /// メッセージを分析して通知が必要かチェック
    async fn check_notifications(&mut self, message: &GuiChatMessage) -> Vec<NotificationType> {
        if !self.config.enabled {
            return vec![];
        }

        let mut notifications = Vec::new();

        // スーパーチャット通知
        if self.config.superchat_notifications {
            if let Some(amount) = self.extract_superchat_amount(message) {
                if amount >= self.config.superchat_min_amount {
                    notifications.push(NotificationType::SuperChat { amount });
                }
            }
        }

        // キーワード通知
        let keyword_notifications = self.config.keyword_notifications.clone();
        for keyword in &keyword_notifications {
            if message
                .content
                .to_lowercase()
                .contains(&keyword.to_lowercase())
            {
                if self.check_cooldown(&format!("keyword_{}", keyword)) {
                    notifications.push(NotificationType::KeywordDetected {
                        keyword: keyword.clone(),
                    });
                }
            }
        }

        // VIPユーザー通知
        let vip_users = self.config.vip_users.clone();
        if vip_users.contains(&message.author) {
            if self.check_cooldown(&format!("vip_{}", message.author)) {
                notifications.push(NotificationType::VipUser {
                    username: message.author.clone(),
                });
            }
        }

        // メンバーシップ通知
        if matches!(message.message_type, MessageType::Membership { .. }) {
            notifications.push(NotificationType::NewMember {
                username: message.author.clone(),
            });
        }

        // メッセージ数閾値通知
        self.message_count += 1;
        if self.message_count >= self.config.message_count_threshold {
            if self.check_cooldown("message_threshold") {
                notifications.push(NotificationType::MessageThreshold {
                    count: self.message_count,
                });
            }
        }

        notifications
    }

    /// スーパーチャット金額を抽出
    fn extract_superchat_amount(&self, message: &GuiChatMessage) -> Option<f64> {
        match &message.message_type {
            MessageType::SuperChat { amount } | MessageType::SuperSticker { amount } => {
                let cleaned = amount.replace(['¥', '$', '€', '£', ','], "");
                cleaned.parse().ok()
            }
            _ => None,
        }
    }

    /// クールダウンチェック
    fn check_cooldown(&mut self, key: &str) -> bool {
        let now = chrono::Utc::now();

        if let Some(last_time) = self.last_notification_time.get(key) {
            let elapsed = now.signed_duration_since(*last_time);
            if elapsed.num_seconds() < self.config.cooldown_seconds as i64 {
                return false;
            }
        }

        self.last_notification_time.insert(key.to_string(), now);
        true
    }

    /// 通知を送信
    async fn send_notification(
        &mut self,
        notification_type: NotificationType,
        message: Option<&GuiChatMessage>,
    ) -> LiscovResult<()> {
        let notification_message = self.format_notification_message(&notification_type, message);

        // 通知履歴に追加
        let notification_history = NotificationHistory {
            id: uuid::Uuid::new_v4().to_string(),
            notification_type: notification_type.clone(),
            timestamp: chrono::Utc::now(),
            message: notification_message.clone(),
            related_message: message.cloned(),
        };

        self.notification_history.push(notification_history);

        // 履歴サイズを制限（最新100件のみ保持）
        if self.notification_history.len() > 100 {
            self.notification_history.remove(0);
        }

        // プラグインシステムにイベント送信
        if let Some(context) = &self.context {
            let notification_data = serde_json::json!({
                "type": notification_type,
                "message": notification_message,
                "timestamp": chrono::Utc::now(),
                "config": {
                    "sound_enabled": self.config.sound_enabled,
                    "desktop_notifications": self.config.desktop_notifications,
                    "duration_seconds": self.config.notification_duration_seconds
                }
            });

            context
                .event_sender
                .send_custom_event("notification".to_string(), notification_data)
                .await?;
            context.logger.info(
                &context.plugin_id,
                &format!("Notification sent: {}", notification_message),
            );
        }

        Ok(())
    }

    /// 通知メッセージをフォーマット
    fn format_notification_message(
        &self,
        notification_type: &NotificationType,
        message: Option<&GuiChatMessage>,
    ) -> String {
        match notification_type {
            NotificationType::SuperChat { amount } => {
                if let Some(msg) = message {
                    format!(
                        "💰 {}さんから¥{:.0}のスーパーチャット: \"{}\"",
                        msg.author, amount, msg.content
                    )
                } else {
                    format!("💰 ¥{:.0}のスーパーチャットが届きました", amount)
                }
            }
            NotificationType::KeywordDetected { keyword } => {
                if let Some(msg) = message {
                    format!(
                        "🔍 キーワード「{}」が検出されました - {}さん: \"{}\"",
                        keyword, msg.author, msg.content
                    )
                } else {
                    format!("🔍 キーワード「{}」が検出されました", keyword)
                }
            }
            NotificationType::NewMember { username } => {
                format!("🎉 {}さんがメンバーになりました！", username)
            }
            NotificationType::VipUser { username } => {
                if let Some(msg) = message {
                    format!(
                        "⭐ VIPユーザー {}さんのメッセージ: \"{}\"",
                        username, msg.content
                    )
                } else {
                    format!("⭐ VIPユーザー {}さんからメッセージが届きました", username)
                }
            }
            NotificationType::MessageThreshold { count } => {
                format!("📊 メッセージ数が{}件に到達しました！", count)
            }
            NotificationType::System { message } => {
                format!("🔔 システム通知: {}", message)
            }
        }
    }

    /// 通知履歴を取得
    pub fn get_notification_history(&self) -> &[NotificationHistory] {
        &self.notification_history
    }

    /// 通知履歴をクリア
    pub fn clear_notification_history(&mut self) {
        self.notification_history.clear();
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> serde_json::Value {
        let mut type_counts = HashMap::new();
        for notification in &self.notification_history {
            let type_key = match &notification.notification_type {
                NotificationType::SuperChat { .. } => "superchat",
                NotificationType::KeywordDetected { .. } => "keyword",
                NotificationType::NewMember { .. } => "new_member",
                NotificationType::VipUser { .. } => "vip_user",
                NotificationType::MessageThreshold { .. } => "message_threshold",
                NotificationType::System { .. } => "system",
            };
            *type_counts.entry(type_key).or_insert(0) += 1;
        }

        serde_json::json!({
            "total_notifications": self.notification_history.len(),
            "notification_types": type_counts,
            "session_start_time": self.session_start_time,
            "current_message_count": self.message_count
        })
    }

    /// 設定を更新
    pub async fn update_config(&mut self, config: NotificationConfig) -> LiscovResult<()> {
        self.config = config;

        // 設定を永続化
        if let Some(context) = &self.context {
            let config_json = serde_json::to_value(&self.config)?;
            context
                .config_access
                .set_config(&context.plugin_id, "notification_config", config_json)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for NotificationPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "notification".to_string(),
            name: "Notification Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Sends notifications based on chat events and conditions".to_string(),
            author: "Liscov Team".to_string(),
            enabled: true,
            dependencies: vec![],
        }
    }

    async fn initialize(&mut self, context: PluginContext) -> LiscovResult<()> {
        // 保存された設定を読み込み
        if let Ok(Some(config_value)) = context
            .config_access
            .get_config(&context.plugin_id, "notification_config")
            .await
        {
            if let Ok(config) = serde_json::from_value::<NotificationConfig>(config_value) {
                self.config = config;
            }
        }

        self.session_start_time = chrono::Utc::now();
        context
            .logger
            .info(&context.plugin_id, "Notification Plugin initialized");
        self.context = Some(context);
        Ok(())
    }

    async fn shutdown(&mut self) -> LiscovResult<()> {
        if let Some(context) = &self.context {
            context
                .logger
                .info(&context.plugin_id, "Notification Plugin shutting down");
        }
        self.context = None;
        Ok(())
    }

    async fn handle_event(&mut self, event: PluginEvent) -> LiscovResult<PluginResult> {
        match event {
            PluginEvent::MessageReceived(message) => {
                let notifications = self.check_notifications(&message).await;

                for notification_type in notifications {
                    self.send_notification(notification_type, Some(&message))
                        .await?;
                }

                Ok(PluginResult::Success)
            }

            PluginEvent::MessagesReceived(messages) => {
                for message in &messages {
                    let notifications = self.check_notifications(message).await;

                    for notification_type in notifications {
                        self.send_notification(notification_type, Some(message))
                            .await?;
                    }
                }

                Ok(PluginResult::Success)
            }

            PluginEvent::ApplicationStarted => {
                let system_notification = NotificationType::System {
                    message: "ライブチャット監視を開始しました".to_string(),
                };
                self.send_notification(system_notification, None).await?;
                Ok(PluginResult::Success)
            }

            PluginEvent::ApplicationStopping => {
                let system_notification = NotificationType::System {
                    message: "ライブチャット監視を停止します".to_string(),
                };
                self.send_notification(system_notification, None).await?;
                Ok(PluginResult::Success)
            }

            PluginEvent::ConfigurationChanged { key, value } => {
                if key == "notification_config" {
                    if let Ok(config) = serde_json::from_value::<NotificationConfig>(value) {
                        self.config = config;

                        if let Some(context) = &self.context {
                            context
                                .logger
                                .info(&context.plugin_id, "Notification configuration updated");
                        }
                    }
                }
                Ok(PluginResult::Success)
            }

            _ => Ok(PluginResult::Skipped),
        }
    }

    async fn handle_plugin_message(
        &mut self,
        from: &str,
        message: serde_json::Value,
    ) -> LiscovResult<PluginResult> {
        if from == "notification-request" {
            if let Some(command) = message.get("command").and_then(|c| c.as_str()) {
                match command {
                    "get_history" => {
                        let history_json = serde_json::to_value(&self.notification_history)?;
                        Ok(PluginResult::SuccessWithData(history_json))
                    }
                    "clear_history" => {
                        self.clear_notification_history();
                        Ok(PluginResult::Success)
                    }
                    "get_stats" => {
                        let stats = self.get_stats();
                        Ok(PluginResult::SuccessWithData(stats))
                    }
                    "send_custom" => {
                        if let Some(custom_message) =
                            message.get("message").and_then(|m| m.as_str())
                        {
                            let system_notification = NotificationType::System {
                                message: custom_message.to_string(),
                            };
                            self.send_notification(system_notification, None).await?;
                            Ok(PluginResult::Success)
                        } else {
                            Ok(PluginResult::Error("Missing message parameter".to_string()))
                        }
                    }
                    _ => Ok(PluginResult::Skipped),
                }
            } else {
                Ok(PluginResult::Skipped)
            }
        } else {
            Ok(PluginResult::Skipped)
        }
    }

    fn get_config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "Enable notifications"
                },
                "superchat_notifications": {
                    "type": "boolean",
                    "description": "Enable Super Chat notifications"
                },
                "superchat_min_amount": {
                    "type": "number",
                    "minimum": 0,
                    "description": "Minimum Super Chat amount for notifications"
                },
                "keyword_notifications": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Keywords to watch for notifications"
                },
                "vip_users": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "VIP users to notify for"
                },
                "message_count_threshold": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Message count threshold for notifications"
                },
                "sound_enabled": {
                    "type": "boolean",
                    "description": "Enable sound notifications"
                },
                "desktop_notifications": {
                    "type": "boolean",
                    "description": "Enable desktop notifications"
                },
                "notification_duration_seconds": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Notification display duration in seconds"
                },
                "cooldown_seconds": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Cooldown period between notifications"
                }
            }
        }))
    }
}

impl Default for NotificationPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::models::MessageType;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

    fn create_test_message(
        author: &str,
        content: &str,
        message_type: MessageType,
    ) -> GuiChatMessage {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        GuiChatMessage {
            id: format!("test_{}", counter),
            timestamp: "00:00:00".to_string(),
            timestamp_usec: counter.to_string(),
            message_type,
            author: author.to_string(),
            author_icon_url: None,
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            runs: Vec::new(),
            metadata: None,
            is_member: false,
            comment_count: None,
        }
    }

    #[tokio::test]
    async fn test_superchat_notification() {
        let mut plugin = NotificationPlugin::new();
        plugin.config.superchat_notifications = true;
        plugin.config.superchat_min_amount = 500.0;

        let superchat_message = create_test_message(
            "donor",
            "Thank you!",
            MessageType::SuperChat {
                amount: "¥1000".to_string(),
            },
        );

        let notifications = plugin.check_notifications(&superchat_message).await;

        assert_eq!(notifications.len(), 1);
        if let NotificationType::SuperChat { amount } = &notifications[0] {
            assert_eq!(*amount, 1000.0);
        } else {
            panic!("Expected SuperChat notification");
        }
    }

    #[tokio::test]
    async fn test_keyword_notification() {
        let mut plugin = NotificationPlugin::new();
        plugin.config.keyword_notifications.push("test".to_string());

        let keyword_message =
            create_test_message("user", "This is a test message", MessageType::Text);
        let normal_message = create_test_message("user", "Normal message", MessageType::Text);

        let notifications1 = plugin.check_notifications(&keyword_message).await;
        let notifications2 = plugin.check_notifications(&normal_message).await;

        assert_eq!(notifications1.len(), 1);
        assert_eq!(notifications2.len(), 0);

        if let NotificationType::KeywordDetected { keyword } = &notifications1[0] {
            assert_eq!(keyword, "test");
        } else {
            panic!("Expected KeywordDetected notification");
        }
    }

    #[tokio::test]
    async fn test_vip_user_notification() {
        let mut plugin = NotificationPlugin::new();
        plugin.config.vip_users.push("vipuser".to_string());

        let vip_message = create_test_message("vipuser", "Hello from VIP", MessageType::Text);
        let normal_message =
            create_test_message("normaluser", "Hello from normal", MessageType::Text);

        let notifications1 = plugin.check_notifications(&vip_message).await;
        let notifications2 = plugin.check_notifications(&normal_message).await;

        assert_eq!(notifications1.len(), 1);
        assert_eq!(notifications2.len(), 0);

        if let NotificationType::VipUser { username } = &notifications1[0] {
            assert_eq!(username, "vipuser");
        } else {
            panic!("Expected VipUser notification");
        }
    }

    #[tokio::test]
    async fn test_cooldown_mechanism() {
        let mut plugin = NotificationPlugin::new();
        plugin.config.cooldown_seconds = 5;
        plugin.config.keyword_notifications.push("test".to_string());

        let message = create_test_message("user", "test message", MessageType::Text);

        // 最初の通知は送信される
        let notifications1 = plugin.check_notifications(&message).await;
        assert_eq!(notifications1.len(), 1);

        // すぐに同じキーワードでもクールダウン中なので通知されない
        let notifications2 = plugin.check_notifications(&message).await;
        assert_eq!(notifications2.len(), 0);
    }

    #[test]
    fn test_superchat_amount_extraction() {
        let plugin = NotificationPlugin::new();

        let superchat = create_test_message(
            "user",
            "Thank you!",
            MessageType::SuperChat {
                amount: "¥1,500".to_string(),
            },
        );
        let normal = create_test_message("user", "Normal message", MessageType::Text);

        assert_eq!(plugin.extract_superchat_amount(&superchat), Some(1500.0));
        assert_eq!(plugin.extract_superchat_amount(&normal), None);
    }

    #[test]
    fn test_notification_message_formatting() {
        let plugin = NotificationPlugin::new();

        let message = create_test_message("testuser", "Hello world", MessageType::Text);

        let superchat_notification = NotificationType::SuperChat { amount: 500.0 };
        let keyword_notification = NotificationType::KeywordDetected {
            keyword: "test".to_string(),
        };

        let superchat_msg =
            plugin.format_notification_message(&superchat_notification, Some(&message));
        let keyword_msg = plugin.format_notification_message(&keyword_notification, Some(&message));

        assert!(superchat_msg.contains("testuser"));
        assert!(superchat_msg.contains("500"));
        assert!(keyword_msg.contains("test"));
        assert!(keyword_msg.contains("Hello world"));
    }
}
