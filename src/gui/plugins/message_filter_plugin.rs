//! メッセージフィルタリングプラグイン
//!
//! 特定の条件に基づいてメッセージをフィルタリングするプラグイン
//!
//! ⚠️ 注意: このプラグインはプラグインシステムのサンプル実装です。
//! YouTube Live Chatでは、YouTube側で既にスパムフィルター、禁止単語フィルター、
//! モデレーション機能などが提供されているため、実際の用途では不要です。
//! プラグイン開発の参考例として残してあります。

use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::gui::models::{GuiChatMessage, MessageType};
use crate::gui::plugin_system::{Plugin, PluginContext, PluginEvent, PluginInfo, PluginResult};
use crate::LiscovResult;

/// メッセージフィルター設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// 禁止単語リスト
    pub blocked_words: HashSet<String>,
    /// 必須単語リスト（これらの単語を含むメッセージのみ通す）
    pub required_words: HashSet<String>,
    /// スパムフィルター有効フラグ
    pub spam_filter_enabled: bool,
    /// 最小メッセージ長
    pub min_message_length: usize,
    /// 最大メッセージ長
    pub max_message_length: usize,
    /// 絵文字のみのメッセージを除外するか
    pub filter_emoji_only: bool,
    /// URLを含むメッセージを除外するか
    pub filter_urls: bool,
    /// スーパーチャットを常に通すか
    pub always_allow_superchat: bool,
    /// メンバーのメッセージを常に通すか
    pub always_allow_members: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            blocked_words: HashSet::new(),
            required_words: HashSet::new(),
            spam_filter_enabled: true,
            min_message_length: 1,
            max_message_length: 1000,
            filter_emoji_only: false,
            filter_urls: false,
            always_allow_superchat: true,
            always_allow_members: false,
        }
    }
}

/// メッセージフィルタリングプラグイン
pub struct MessageFilterPlugin {
    config: FilterConfig,
    context: Option<PluginContext>,
    url_regex: Regex,
    emoji_regex: Regex,
    stats: FilterStats,
}

/// フィルター統計
#[derive(Debug, Clone, Default)]
pub struct FilterStats {
    /// 処理したメッセージ数
    pub processed_count: usize,
    /// ブロックしたメッセージ数
    pub blocked_count: usize,
    /// 通したメッセージ数
    pub allowed_count: usize,
}

impl MessageFilterPlugin {
    /// 新しいプラグインインスタンスを作成
    pub fn new() -> Self {
        Self {
            config: FilterConfig::default(),
            context: None,
            url_regex: Regex::new(r"https?://[^\s]+").unwrap(),
            emoji_regex: Regex::new(r"[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]").unwrap(),
            stats: FilterStats::default(),
        }
    }

    /// 設定をデフォルトの禁止単語で初期化
    pub fn with_default_blocked_words() -> Self {
        let mut plugin = Self::new();

        let default_blocked = vec![
            "spam".to_string(),
            "bot".to_string(),
            "fake".to_string(),
            "scam".to_string(),
        ];

        plugin.config.blocked_words = default_blocked.into_iter().collect();
        plugin
    }

    /// メッセージがフィルター条件に合致するかチェック
    fn should_filter_message(&self, message: &GuiChatMessage) -> bool {
        // スーパーチャットは常に許可する設定の場合
        if self.config.always_allow_superchat {
            if matches!(
                message.message_type,
                MessageType::SuperChat { .. } | MessageType::SuperSticker { .. }
            ) {
                return false;
            }
        }

        // メンバーは常に許可する設定の場合
        if self.config.always_allow_members && message.is_member {
            return false;
        }

        // 長さチェック
        if message.content.len() < self.config.min_message_length
            || message.content.len() > self.config.max_message_length
        {
            return true;
        }

        // 禁止単語チェック
        let content_lower = message.content.to_lowercase();
        for blocked_word in &self.config.blocked_words {
            if content_lower.contains(&blocked_word.to_lowercase()) {
                return true;
            }
        }

        // 必須単語チェック
        if !self.config.required_words.is_empty() {
            let mut contains_required = false;
            for required_word in &self.config.required_words {
                if content_lower.contains(&required_word.to_lowercase()) {
                    contains_required = true;
                    break;
                }
            }
            if !contains_required {
                return true;
            }
        }

        // URLフィルター
        if self.config.filter_urls && self.url_regex.is_match(&message.content) {
            return true;
        }

        // 絵文字のみメッセージフィルター
        if self.config.filter_emoji_only {
            let text_without_emoji = self.emoji_regex.replace_all(&message.content, "");
            if text_without_emoji.trim().is_empty() {
                return true;
            }
        }

        // スパムフィルター（簡易実装）
        if self.config.spam_filter_enabled {
            if self.is_likely_spam(message) {
                return true;
            }
        }

        false
    }

    /// スパムの可能性があるかチェック（簡易実装）
    fn is_likely_spam(&self, message: &GuiChatMessage) -> bool {
        let content = &message.content;

        // 同じ文字の連続をチェック
        let mut consecutive_count = 1;
        let mut last_char = '\0';
        for ch in content.chars() {
            if ch == last_char {
                consecutive_count += 1;
                if consecutive_count > 10 {
                    return true;
                }
            } else {
                consecutive_count = 1;
                last_char = ch;
            }
        }

        // 大文字の比率をチェック
        let uppercase_count = content.chars().filter(|c| c.is_uppercase()).count();
        let total_alpha = content.chars().filter(|c| c.is_alphabetic()).count();
        if total_alpha > 5 && uppercase_count as f64 / total_alpha as f64 > 0.8 {
            return true;
        }

        false
    }

    /// フィルター統計をリセット
    pub fn reset_stats(&mut self) {
        self.stats = FilterStats::default();
    }

    /// フィルター統計を取得
    pub fn get_stats(&self) -> FilterStats {
        self.stats.clone()
    }

    /// 設定を更新
    pub async fn update_config(&mut self, config: FilterConfig) -> LiscovResult<()> {
        self.config = config;

        // 設定を永続化
        if let Some(context) = &self.context {
            let config_json = serde_json::to_value(&self.config)?;
            context
                .config_access
                .set_config(&context.plugin_id, "filter_config", config_json)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for MessageFilterPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "message-filter".to_string(),
            name: "Message Filter Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Filters chat messages based on configurable rules".to_string(),
            author: "Liscov Team".to_string(),
            enabled: true,
            dependencies: vec![],
        }
    }

    async fn initialize(&mut self, context: PluginContext) -> LiscovResult<()> {
        // 保存された設定を読み込み
        if let Ok(Some(config_value)) = context
            .config_access
            .get_config(&context.plugin_id, "filter_config")
            .await
        {
            if let Ok(config) = serde_json::from_value::<FilterConfig>(config_value) {
                self.config = config;
            }
        }

        context
            .logger
            .info(&context.plugin_id, "Message Filter Plugin initialized");
        self.context = Some(context);
        Ok(())
    }

    async fn shutdown(&mut self) -> LiscovResult<()> {
        if let Some(context) = &self.context {
            context
                .logger
                .info(&context.plugin_id, "Message Filter Plugin shutting down");
        }
        self.context = None;
        Ok(())
    }

    async fn handle_event(&mut self, event: PluginEvent) -> LiscovResult<PluginResult> {
        match event {
            PluginEvent::MessageReceived(message) => {
                self.stats.processed_count += 1;

                if self.should_filter_message(&message) {
                    self.stats.blocked_count += 1;

                    if let Some(context) = &self.context {
                        context.logger.debug(
                            &context.plugin_id,
                            &format!(
                                "Blocked message from {}: {}",
                                message.author, message.content
                            ),
                        );
                    }

                    Ok(PluginResult::SuccessWithData(serde_json::json!({
                        "action": "block",
                        "reason": "filtered",
                        "message_id": message.channel_id
                    })))
                } else {
                    self.stats.allowed_count += 1;
                    Ok(PluginResult::Success)
                }
            }

            PluginEvent::MessagesReceived(messages) => {
                let mut blocked_messages = Vec::new();

                for message in messages {
                    self.stats.processed_count += 1;

                    if self.should_filter_message(&message) {
                        self.stats.blocked_count += 1;
                        blocked_messages.push(message.channel_id.clone());
                    } else {
                        self.stats.allowed_count += 1;
                    }
                }

                if !blocked_messages.is_empty() {
                    if let Some(context) = &self.context {
                        context.logger.debug(
                            &context.plugin_id,
                            &format!("Blocked {} messages in batch", blocked_messages.len()),
                        );
                    }

                    Ok(PluginResult::SuccessWithData(serde_json::json!({
                        "action": "block_batch",
                        "blocked_messages": blocked_messages
                    })))
                } else {
                    Ok(PluginResult::Success)
                }
            }

            PluginEvent::ConfigurationChanged { key, value } => {
                if key == "filter_config" {
                    if let Ok(config) = serde_json::from_value::<FilterConfig>(value) {
                        self.config = config;

                        if let Some(context) = &self.context {
                            context
                                .logger
                                .info(&context.plugin_id, "Configuration updated");
                        }
                    }
                }
                Ok(PluginResult::Success)
            }

            _ => Ok(PluginResult::Skipped),
        }
    }

    fn get_config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "blocked_words": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of words to block"
                },
                "required_words": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of words that must be present"
                },
                "spam_filter_enabled": {
                    "type": "boolean",
                    "description": "Enable spam filtering"
                },
                "min_message_length": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Minimum message length"
                },
                "max_message_length": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum message length"
                },
                "filter_emoji_only": {
                    "type": "boolean",
                    "description": "Filter emoji-only messages"
                },
                "filter_urls": {
                    "type": "boolean",
                    "description": "Filter messages containing URLs"
                },
                "always_allow_superchat": {
                    "type": "boolean",
                    "description": "Always allow Super Chat messages"
                },
                "always_allow_members": {
                    "type": "boolean",
                    "description": "Always allow member messages"
                }
            }
        }))
    }
}

impl Default for MessageFilterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::models::MessageType;

    fn create_test_message(content: &str) -> GuiChatMessage {
        GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            message_type: MessageType::Text,
            author: content.to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            runs: Vec::new(),
            metadata: None,
            is_member: false,
        }
    }

    #[test]
    fn test_blocked_words_filter() {
        let mut plugin = MessageFilterPlugin::new();
        plugin.config.blocked_words.insert("spam".to_string());

        let spam_message = create_test_message("This is spam content");
        let normal_message = create_test_message("This is normal content");

        assert!(plugin.should_filter_message(&spam_message));
        assert!(!plugin.should_filter_message(&normal_message));
    }

    #[test]
    fn test_message_length_filter() {
        let mut plugin = MessageFilterPlugin::new();
        plugin.config.min_message_length = 5;
        plugin.config.max_message_length = 20;

        let short_message = create_test_message("Hi");
        let long_message =
            create_test_message("This is a very long message that exceeds the limit");
        let normal_message = create_test_message("Normal message");

        assert!(plugin.should_filter_message(&short_message));
        assert!(plugin.should_filter_message(&long_message));
        assert!(!plugin.should_filter_message(&normal_message));
    }

    #[test]
    fn test_superchat_always_allowed() {
        let mut plugin = MessageFilterPlugin::new();
        plugin.config.blocked_words.insert("spam".to_string());
        plugin.config.always_allow_superchat = true;

        let mut superchat_message = create_test_message("spam content");
        superchat_message.message_type = MessageType::SuperChat {
            amount: "¥500".to_string(),
        };

        // スーパーチャットはスパム内容でもブロックされない
        assert!(!plugin.should_filter_message(&superchat_message));
    }

    #[test]
    fn test_url_filter() {
        let mut plugin = MessageFilterPlugin::new();
        plugin.config.filter_urls = true;

        let url_message = create_test_message("Check this out: https://example.com");
        let normal_message = create_test_message("Normal message without URL");

        assert!(plugin.should_filter_message(&url_message));
        assert!(!plugin.should_filter_message(&normal_message));
    }

    #[test]
    fn test_spam_detection() {
        let plugin = MessageFilterPlugin::new();

        let spam_consecutive = create_test_message("aaaaaaaaaaaaa");
        let spam_uppercase = create_test_message("SPAMMMMMMM");
        let normal_message = create_test_message("Normal Message");

        assert!(plugin.is_likely_spam(&spam_consecutive));
        assert!(plugin.is_likely_spam(&spam_uppercase));
        assert!(!plugin.is_likely_spam(&normal_message));
    }
}
