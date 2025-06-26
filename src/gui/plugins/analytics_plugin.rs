//! アナリティクスプラグイン
//!
//! メッセージの統計情報を収集し、詳細な分析を提供するプラグイン

use async_trait::async_trait;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::gui::models::{GuiChatMessage, MessageType};
use crate::gui::plugin_system::{Plugin, PluginContext, PluginEvent, PluginInfo, PluginResult};
use crate::LiscovResult;

/// アナリティクス設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// 統計収集を有効にするか
    pub enabled: bool,
    /// エンゲージメント分析を有効にするか
    pub engagement_analysis: bool,
    /// 感情分析を有効にするか
    pub sentiment_analysis: bool,
    /// レポート生成間隔（秒）
    pub report_interval_seconds: u64,
    /// データ保持期間（日）
    pub data_retention_days: u32,
    /// 詳細ログを有効にするか
    pub verbose_logging: bool,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            engagement_analysis: true,
            sentiment_analysis: false,
            report_interval_seconds: 300, // 5分
            data_retention_days: 30,
            verbose_logging: false,
        }
    }
}

/// アナリティクス統計
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyticsStats {
    /// 総メッセージ数
    pub total_messages: usize,
    /// ユニーク視聴者数
    pub unique_viewers: usize,
    /// メッセージタイプ別統計
    pub message_types: HashMap<String, usize>,
    /// 時間帯別メッセージ数
    pub hourly_distribution: HashMap<u32, usize>,
    /// エンゲージメント率
    pub engagement_rate: f64,
    /// 平均メッセージ長
    pub average_message_length: f64,
    /// スーパーチャット総額
    pub total_superchat_amount: f64,
    /// 最初のメッセージ時刻
    pub first_message_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 最後のメッセージ時刻
    pub last_message_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 期間中のピーク時間
    pub peak_hour: Option<u32>,
    /// 最も活発なユーザー
    pub top_chatters: Vec<(String, usize)>,
}

/// アナリティクスプラグイン
pub struct AnalyticsPlugin {
    config: AnalyticsConfig,
    context: Option<PluginContext>,
    stats: AnalyticsStats,
    user_message_counts: HashMap<String, usize>,
    session_start_time: Option<chrono::DateTime<chrono::Utc>>,
    last_report_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl AnalyticsPlugin {
    /// 新しいプラグインインスタンスを作成
    pub fn new() -> Self {
        Self {
            config: AnalyticsConfig::default(),
            context: None,
            stats: AnalyticsStats::default(),
            user_message_counts: HashMap::new(),
            session_start_time: None,
            last_report_time: None,
        }
    }

    /// メッセージを分析して統計を更新
    fn analyze_message(&mut self, message: &GuiChatMessage) {
        if !self.config.enabled {
            return;
        }

        // 基本統計更新
        self.stats.total_messages += 1;

        // ユニーク視聴者数更新
        *self
            .user_message_counts
            .entry(message.author.clone())
            .or_insert(0) += 1;
        self.stats.unique_viewers = self.user_message_counts.len();

        // メッセージタイプ統計
        let type_key = message.message_type.as_string();
        *self.stats.message_types.entry(type_key).or_insert(0) += 1;

        // 時間帯分析
        let now = chrono::Utc::now();
        let hour = now.hour();
        *self.stats.hourly_distribution.entry(hour).or_insert(0) += 1;

        // 平均メッセージ長更新
        let current_total_length =
            self.stats.average_message_length * (self.stats.total_messages - 1) as f64;
        self.stats.average_message_length = (current_total_length + message.content.len() as f64)
            / self.stats.total_messages as f64;

        // スーパーチャット金額集計
        if let MessageType::SuperChat { amount } | MessageType::SuperSticker { amount } =
            &message.message_type
        {
            if let Some(parsed_amount) = self.parse_amount(amount) {
                self.stats.total_superchat_amount += parsed_amount;
            }
        }

        // 時刻情報更新
        if self.stats.first_message_time.is_none() {
            self.stats.first_message_time = Some(now);
        }
        self.stats.last_message_time = Some(now);

        // エンゲージメント率計算
        self.calculate_engagement_rate();

        // ピーク時間更新
        self.update_peak_hour();

        // トップチャッター更新
        self.update_top_chatters();
    }

    /// 金額文字列をパース
    fn parse_amount(&self, amount_str: &str) -> Option<f64> {
        let cleaned = amount_str.replace(['¥', '$', '€', '£', ','], "");
        cleaned.parse().ok()
    }

    /// エンゲージメント率を計算
    fn calculate_engagement_rate(&mut self) {
        if self.stats.unique_viewers == 0 {
            self.stats.engagement_rate = 0.0;
            return;
        }

        // 簡易エンゲージメント率: アクティブユーザー数 / 総視聴者数
        let active_users = self
            .user_message_counts
            .values()
            .filter(|&&count| count > 1)
            .count();
        self.stats.engagement_rate =
            (active_users as f64 / self.stats.unique_viewers as f64) * 100.0;
    }

    /// ピーク時間を更新
    fn update_peak_hour(&mut self) {
        if let Some((peak_hour, _max_count)) = self
            .stats
            .hourly_distribution
            .iter()
            .max_by_key(|(_, &count)| count)
        {
            self.stats.peak_hour = Some(*peak_hour);
        }
    }

    /// トップチャッターリストを更新
    fn update_top_chatters(&mut self) {
        let mut chatters: Vec<(String, usize)> = self
            .user_message_counts
            .iter()
            .map(|(user, &count)| (user.clone(), count))
            .collect();

        chatters.sort_by(|a, b| b.1.cmp(&a.1));
        self.stats.top_chatters = chatters.into_iter().take(10).collect();
    }

    /// 統計レポートを生成
    async fn generate_report(&self) -> serde_json::Value {
        let session_duration = if let (Some(start), Some(end)) =
            (self.stats.first_message_time, self.stats.last_message_time)
        {
            end.signed_duration_since(start).num_seconds()
        } else {
            0
        };

        serde_json::json!({
            "timestamp": chrono::Utc::now(),
            "session_duration_seconds": session_duration,
            "stats": self.stats,
            "messages_per_minute": if session_duration > 0 {
                (self.stats.total_messages as f64) / (session_duration as f64 / 60.0)
            } else {
                0.0
            },
            "superchat_per_message": if self.stats.total_messages > 0 {
                self.stats.total_superchat_amount / self.stats.total_messages as f64
            } else {
                0.0
            }
        })
    }

    /// 統計をリセット
    pub fn reset_stats(&mut self) {
        self.stats = AnalyticsStats::default();
        self.user_message_counts.clear();
        self.session_start_time = Some(chrono::Utc::now());
        self.last_report_time = None;
    }

    /// 統計データを取得
    pub fn get_stats(&self) -> &AnalyticsStats {
        &self.stats
    }

    /// 設定を更新
    pub async fn update_config(&mut self, config: AnalyticsConfig) -> LiscovResult<()> {
        self.config = config;

        // 設定を永続化
        if let Some(context) = &self.context {
            let config_json = serde_json::to_value(&self.config)?;
            context
                .config_access
                .set_config(&context.plugin_id, "analytics_config", config_json)
                .await?;
        }

        Ok(())
    }

    /// 定期レポートの送信が必要かチェック
    fn should_send_report(&self) -> bool {
        if let Some(last_report) = self.last_report_time {
            let elapsed = chrono::Utc::now().signed_duration_since(last_report);
            elapsed.num_seconds() >= self.config.report_interval_seconds as i64
        } else {
            true
        }
    }
}

#[async_trait]
impl Plugin for AnalyticsPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "analytics".to_string(),
            name: "Analytics Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Collects and analyzes chat message statistics".to_string(),
            author: "Liscov Team".to_string(),
            enabled: true,
            dependencies: vec![],
        }
    }

    async fn initialize(&mut self, context: PluginContext) -> LiscovResult<()> {
        // 保存された設定を読み込み
        if let Ok(Some(config_value)) = context
            .config_access
            .get_config(&context.plugin_id, "analytics_config")
            .await
        {
            if let Ok(config) = serde_json::from_value::<AnalyticsConfig>(config_value) {
                self.config = config;
            }
        }

        self.session_start_time = Some(chrono::Utc::now());
        context
            .logger
            .info(&context.plugin_id, "Analytics Plugin initialized");
        self.context = Some(context);
        Ok(())
    }

    async fn shutdown(&mut self) -> LiscovResult<()> {
        if let Some(context) = &self.context {
            // 最終レポートを生成
            let final_report = self.generate_report().await;
            context.logger.info(
                &context.plugin_id,
                &format!("Final analytics report: {}", final_report),
            );
            context
                .logger
                .info(&context.plugin_id, "Analytics Plugin shutting down");
        }

        self.context = None;
        Ok(())
    }

    async fn handle_event(&mut self, event: PluginEvent) -> LiscovResult<PluginResult> {
        match event {
            PluginEvent::ApplicationStarted => {
                self.reset_stats();
                if let Some(context) = &self.context {
                    context
                        .logger
                        .info(&context.plugin_id, "Analytics session started");
                }
                Ok(PluginResult::Success)
            }

            PluginEvent::MessageReceived(message) => {
                self.analyze_message(&message);

                if self.config.verbose_logging {
                    if let Some(context) = &self.context {
                        context.logger.debug(
                            &context.plugin_id,
                            &format!(
                                "Analyzed message from {}: {} chars",
                                message.author,
                                message.content.len()
                            ),
                        );
                    }
                }

                // 定期レポート送信
                if self.should_send_report() {
                    let report = self.generate_report().await;
                    self.last_report_time = Some(chrono::Utc::now());

                    if let Some(context) = &self.context {
                        context
                            .event_sender
                            .send_custom_event("analytics_report".to_string(), report.clone())
                            .await?;
                        context
                            .logger
                            .info(&context.plugin_id, "Analytics report generated");
                    }

                    Ok(PluginResult::SuccessWithData(report))
                } else {
                    Ok(PluginResult::Success)
                }
            }

            PluginEvent::MessagesReceived(messages) => {
                for message in &messages {
                    self.analyze_message(message);
                }

                if let Some(context) = &self.context {
                    context.logger.debug(
                        &context.plugin_id,
                        &format!("Analyzed batch of {} messages", messages.len()),
                    );
                }

                Ok(PluginResult::Success)
            }

            PluginEvent::ConfigurationChanged { key, value } => {
                if key == "analytics_config" {
                    if let Ok(config) = serde_json::from_value::<AnalyticsConfig>(value) {
                        self.config = config;

                        if let Some(context) = &self.context {
                            context
                                .logger
                                .info(&context.plugin_id, "Analytics configuration updated");
                        }
                    }
                }
                Ok(PluginResult::Success)
            }

            PluginEvent::ApplicationStopping => {
                // 最終レポートを生成
                let final_report = self.generate_report().await;

                if let Some(context) = &self.context {
                    context
                        .event_sender
                        .send_custom_event(
                            "final_analytics_report".to_string(),
                            final_report.clone(),
                        )
                        .await?;
                    context
                        .logger
                        .info(&context.plugin_id, "Final analytics report sent");
                }

                Ok(PluginResult::SuccessWithData(final_report))
            }

            _ => Ok(PluginResult::Skipped),
        }
    }

    async fn handle_plugin_message(
        &mut self,
        from: &str,
        message: serde_json::Value,
    ) -> LiscovResult<PluginResult> {
        if from == "analytics-request" {
            if let Some(command) = message.get("command").and_then(|c| c.as_str()) {
                match command {
                    "get_stats" => {
                        let stats_json = serde_json::to_value(&self.stats)?;
                        Ok(PluginResult::SuccessWithData(stats_json))
                    }
                    "reset_stats" => {
                        self.reset_stats();
                        Ok(PluginResult::Success)
                    }
                    "generate_report" => {
                        let report = self.generate_report().await;
                        Ok(PluginResult::SuccessWithData(report))
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
                    "description": "Enable analytics collection"
                },
                "engagement_analysis": {
                    "type": "boolean",
                    "description": "Enable engagement analysis"
                },
                "sentiment_analysis": {
                    "type": "boolean",
                    "description": "Enable sentiment analysis"
                },
                "report_interval_seconds": {
                    "type": "integer",
                    "minimum": 60,
                    "description": "Report generation interval in seconds"
                },
                "data_retention_days": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Data retention period in days"
                },
                "verbose_logging": {
                    "type": "boolean",
                    "description": "Enable verbose logging"
                }
            }
        }))
    }
}

impl Default for AnalyticsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::models::MessageType;

    fn create_test_message(
        author: &str,
        content: &str,
        message_type: MessageType,
    ) -> GuiChatMessage {
        GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
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

    #[test]
    fn test_basic_stats_collection() {
        let mut plugin = AnalyticsPlugin::new();

        let message1 = create_test_message("user1", "Hello world!", MessageType::Text);
        let message2 = create_test_message("user2", "Hi there!", MessageType::Text);
        let message3 = create_test_message("user1", "Another message", MessageType::Text);

        plugin.analyze_message(&message1);
        plugin.analyze_message(&message2);
        plugin.analyze_message(&message3);

        assert_eq!(plugin.stats.total_messages, 3);
        assert_eq!(plugin.stats.unique_viewers, 2);
        assert_eq!(plugin.user_message_counts.get("user1"), Some(&2));
        assert_eq!(plugin.user_message_counts.get("user2"), Some(&1));
    }

    #[test]
    fn test_superchat_amount_parsing() {
        let plugin = AnalyticsPlugin::new();

        assert_eq!(plugin.parse_amount("¥500"), Some(500.0));
        assert_eq!(plugin.parse_amount("$12.34"), Some(12.34));
        assert_eq!(plugin.parse_amount("€1,000"), Some(1000.0));
        assert_eq!(plugin.parse_amount("invalid"), None);
    }

    #[test]
    fn test_superchat_stats() {
        let mut plugin = AnalyticsPlugin::new();

        let superchat = create_test_message(
            "supporter",
            "Thank you!",
            MessageType::SuperChat {
                amount: "¥500".to_string(),
            },
        );

        plugin.analyze_message(&superchat);

        assert_eq!(plugin.stats.total_superchat_amount, 500.0);
        assert_eq!(plugin.stats.message_types.get("super-chat"), Some(&1));
    }

    #[test]
    fn test_average_message_length() {
        let mut plugin = AnalyticsPlugin::new();

        let short_msg = create_test_message("user1", "Hi", MessageType::Text); // 2 chars
        let long_msg = create_test_message("user2", "Hello world!", MessageType::Text); // 12 chars

        plugin.analyze_message(&short_msg);
        plugin.analyze_message(&long_msg);

        assert_eq!(plugin.stats.average_message_length, 7.0); // (2 + 12) / 2
    }

    #[test]
    fn test_engagement_rate_calculation() {
        let mut plugin = AnalyticsPlugin::new();

        // ユーザー1: 3メッセージ（アクティブ）
        plugin.analyze_message(&create_test_message("user1", "msg1", MessageType::Text));
        plugin.analyze_message(&create_test_message("user1", "msg2", MessageType::Text));
        plugin.analyze_message(&create_test_message("user1", "msg3", MessageType::Text));

        // ユーザー2: 1メッセージ（非アクティブ）
        plugin.analyze_message(&create_test_message("user2", "single", MessageType::Text));

        // ユーザー3: 2メッセージ（アクティブ）
        plugin.analyze_message(&create_test_message("user3", "first", MessageType::Text));
        plugin.analyze_message(&create_test_message("user3", "second", MessageType::Text));

        // エンゲージメント率 = アクティブユーザー数(2) / 総ユーザー数(3) * 100 = 66.67%
        assert!((plugin.stats.engagement_rate - 66.66666666666667).abs() < 0.001);
    }
}
