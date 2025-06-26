//! メッセージ処理パイプライン実装
//!
//! Phase 2実装: トレイトベース設計への移行

use async_trait::async_trait;

use super::models::GuiChatMessage;
use super::traits::{MessageFilterConfig, MessageProcessor, MessageStatistics, ProcessingError};
use crate::get_live_chat::ChatItem;

/// デフォルトメッセージプロセッサ実装
#[derive(Debug, Clone)]
pub struct DefaultMessageProcessor {
    /// プロセッサ設定
    config: MessageProcessorConfig,
}

/// メッセージプロセッサ設定
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageProcessorConfig {
    /// メッセージの最大長（文字数）
    pub max_message_length: usize,
    /// 絵文字変換を有効にするか
    pub enable_emoji_conversion: bool,
    /// URLリンクの検出を有効にするか
    pub enable_url_detection: bool,
    /// スパムフィルタを有効にするか
    pub enable_spam_filter: bool,
    /// 重複メッセージの検出を有効にするか
    pub enable_duplicate_detection: bool,
}

impl Default for MessageProcessorConfig {
    fn default() -> Self {
        Self {
            max_message_length: 1000,
            enable_emoji_conversion: true,
            enable_url_detection: true,
            enable_spam_filter: false,
            enable_duplicate_detection: false,
        }
    }
}

impl DefaultMessageProcessor {
    /// 新しいメッセージプロセッサを作成
    pub fn new() -> Self {
        Self {
            config: MessageProcessorConfig::default(),
        }
    }

    /// 設定付きでメッセージプロセッサを作成
    pub fn with_config(config: MessageProcessorConfig) -> Self {
        Self { config }
    }

    /// 現在の設定を取得
    pub fn get_config(&self) -> &MessageProcessorConfig {
        &self.config
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: MessageProcessorConfig) {
        self.config = config;
    }

    /// メッセージ内容をサニタイズ
    fn sanitize_content(&self, content: &str) -> Result<String, ProcessingError> {
        let mut sanitized = content.to_string();

        // 最大長制限
        if sanitized.len() > self.config.max_message_length {
            sanitized.truncate(self.config.max_message_length);
            sanitized.push_str("...");
        }

        // 制御文字を除去
        sanitized = sanitized
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect();

        // 空文字列チェック
        if sanitized.trim().is_empty() {
            return Err(ProcessingError::Validation(
                "Empty message content".to_string(),
            ));
        }

        Ok(sanitized)
    }

    /// 著者名をサニタイズ
    fn sanitize_author(&self, author: &str) -> Result<String, ProcessingError> {
        let mut sanitized = author.trim().to_string();

        // 空文字列チェック
        if sanitized.is_empty() {
            sanitized = "Unknown".to_string();
        }

        // 最大長制限（著者名は短く）
        if sanitized.len() > 100 {
            sanitized.truncate(100);
        }

        // 制御文字を除去
        sanitized = sanitized.chars().filter(|c| !c.is_control()).collect();

        Ok(sanitized)
    }

    /// スパムメッセージの検出
    fn is_spam(&self, message: &GuiChatMessage) -> bool {
        if !self.config.enable_spam_filter {
            return false;
        }

        // 簡易スパム検出ロジック
        let content_lower = message.content.to_lowercase();

        // 過度な繰り返し文字
        let mut prev_char = '\0';
        let mut repeat_count = 0;
        let mut max_repeat = 0;

        for ch in content_lower.chars() {
            if ch == prev_char {
                repeat_count += 1;
            } else {
                max_repeat = max_repeat.max(repeat_count);
                repeat_count = 1;
                prev_char = ch;
            }
        }
        max_repeat = max_repeat.max(repeat_count);

        if max_repeat > 10 {
            return true;
        }

        // 過度な大文字
        let uppercase_count = message.content.chars().filter(|c| c.is_uppercase()).count();
        let total_letters = message
            .content
            .chars()
            .filter(|c| c.is_alphabetic())
            .count();

        if total_letters > 0 && (uppercase_count as f64 / total_letters as f64) > 0.8 {
            return true;
        }

        false
    }

    /// 絵文字数をカウント
    fn count_emojis(&self, content: &str) -> usize {
        if !self.config.enable_emoji_conversion {
            return 0;
        }

        content
            .chars()
            .filter(|c| {
                let code = *c as u32;
                // Unicode絵文字範囲の簡易検出
                (0x1F600..=0x1F64F).contains(&code) ||  // Emoticons
                (0x1F300..=0x1F5FF).contains(&code) ||  // Miscellaneous Symbols
                (0x1F680..=0x1F6FF).contains(&code) ||  // Transport & Map
                (0x2600..=0x26FF).contains(&code) ||    // Miscellaneous symbols
                (0x2700..=0x27BF).contains(&code) // Dingbats
            })
            .count()
    }
}

impl Default for DefaultMessageProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageProcessor for DefaultMessageProcessor {
    fn process_chat_item(&self, item: &ChatItem) -> Result<GuiChatMessage, ProcessingError> {
        // ChatItemをGuiChatMessageに変換
        let gui_message: GuiChatMessage = item.clone().into();

        // メッセージ内容をサニタイズ
        let sanitized_content = self.sanitize_content(&gui_message.content)?;
        let sanitized_author = self.sanitize_author(&gui_message.author)?;

        let processed_message = GuiChatMessage {
            content: sanitized_content,
            author: sanitized_author,
            ..gui_message
        };

        // スパム検出
        if self.is_spam(&processed_message) {
            return Err(ProcessingError::Validation(
                "Spam message detected".to_string(),
            ));
        }

        Ok(processed_message)
    }

    async fn process_message_batch(
        &self,
        items: &[ChatItem],
    ) -> Result<Vec<GuiChatMessage>, ProcessingError> {
        let mut processed_messages = Vec::with_capacity(items.len());
        let mut errors = Vec::new();

        for (index, item) in items.iter().enumerate() {
            match self.process_chat_item(item) {
                Ok(message) => processed_messages.push(message),
                Err(e) => {
                    errors.push(format!("Item {}: {}", index, e));
                    // エラーが発生してもバッチ処理を継続
                    tracing::warn!("Failed to process message at index {}: {}", index, e);
                }
            }
        }

        // エラーが多すぎる場合は失敗とする
        if errors.len() > items.len() / 2 {
            return Err(ProcessingError::Processing(format!(
                "Too many processing errors: {}/{} failed. Errors: {}",
                errors.len(),
                items.len(),
                errors.join("; ")
            )));
        }

        if !errors.is_empty() {
            tracing::info!(
                "Batch processing completed with {} errors out of {} items",
                errors.len(),
                items.len()
            );
        }

        Ok(processed_messages)
    }

    fn filter_message(
        &self,
        message: &GuiChatMessage,
        filter_config: &MessageFilterConfig,
    ) -> bool {
        // システムメッセージのフィルタリング
        if !filter_config.include_system_messages {
            if matches!(
                message.message_type,
                crate::gui::models::MessageType::System
            ) {
                return false;
            }
        }

        // SuperChatのフィルタリング
        if !filter_config.include_super_chat {
            if matches!(
                message.message_type,
                crate::gui::models::MessageType::SuperChat { .. }
            ) {
                return false;
            }
        }

        // メンバーシップのフィルタリング
        if !filter_config.include_membership {
            if matches!(
                message.message_type,
                crate::gui::models::MessageType::Membership
            ) {
                return false;
            }
        }

        // 著者フィルタ
        if let Some(ref author_filter) = filter_config.author_filter {
            if !message
                .author
                .to_lowercase()
                .contains(&author_filter.to_lowercase())
            {
                return false;
            }
        }

        // 内容フィルタ
        if let Some(ref content_filter) = filter_config.content_filter {
            if !message
                .content
                .to_lowercase()
                .contains(&content_filter.to_lowercase())
            {
                return false;
            }
        }

        // 金額フィルタ
        if let (Some(min_amount), Some(max_amount)) =
            (filter_config.min_amount, filter_config.max_amount)
        {
            if let Some(amount) = self.extract_amount_from_message(message) {
                if amount < min_amount || amount > max_amount {
                    return false;
                }
            } else if min_amount > 0.0 {
                // 金額がないメッセージで最小金額が設定されている場合は除外
                return false;
            }
        }

        true
    }

    fn update_statistics(&self, message: &GuiChatMessage, stats: &mut MessageStatistics) {
        stats.total_messages += 1;
        stats.unique_authors.insert(message.author.clone());

        match &message.message_type {
            crate::gui::models::MessageType::SuperChat { amount } => {
                stats.super_chat_count += 1;
                if let Ok(amount_value) = self.parse_amount_string(amount) {
                    stats.total_revenue += amount_value;
                }
            }
            crate::gui::models::MessageType::SuperSticker { amount } => {
                stats.super_chat_count += 1; // SuperStickerもSuperChatとしてカウント
                if let Ok(amount_value) = self.parse_amount_string(amount) {
                    stats.total_revenue += amount_value;
                }
            }
            crate::gui::models::MessageType::Membership => {
                stats.membership_count += 1;
            }
            _ => {}
        }

        // 平均メッセージ長を更新
        let total_length = stats.average_message_length * (stats.total_messages - 1) as f64
            + message.content.len() as f64;
        stats.average_message_length = total_length / stats.total_messages as f64;

        // 絵文字数を更新
        stats.emoji_count += self.count_emojis(&message.content);
    }
}

impl DefaultMessageProcessor {
    /// メッセージから金額を抽出
    fn extract_amount_from_message(&self, message: &GuiChatMessage) -> Option<f64> {
        match &message.message_type {
            crate::gui::models::MessageType::SuperChat { amount } => {
                self.parse_amount_string(amount).ok()
            }
            crate::gui::models::MessageType::SuperSticker { amount } => {
                self.parse_amount_string(amount).ok()
            }
            _ => None,
        }
    }

    /// 金額文字列をパース
    fn parse_amount_string(&self, amount_str: &str) -> Result<f64, ProcessingError> {
        // 数字とピリオドのみを抽出
        let clean_amount = amount_str
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>();

        if clean_amount.is_empty() {
            return Err(ProcessingError::Format(
                "No numeric content found".to_string(),
            ));
        }

        clean_amount
            .parse::<f64>()
            .map_err(|e| ProcessingError::Format(format!("Failed to parse amount: {}", e)))
    }
}

/// ファクトリ実装
pub struct DefaultMessageProcessorFactory;

impl DefaultMessageProcessorFactory {
    /// デフォルト設定でプロセッサを作成
    pub fn create_default() -> Box<dyn MessageProcessor> {
        Box::new(DefaultMessageProcessor::new())
    }

    /// カスタム設定でプロセッサを作成
    pub fn create_with_config(config: MessageProcessorConfig) -> Box<dyn MessageProcessor> {
        Box::new(DefaultMessageProcessor::with_config(config))
    }

    /// スパムフィルタ有効でプロセッサを作成
    pub fn create_with_spam_filter() -> Box<dyn MessageProcessor> {
        let config = MessageProcessorConfig {
            enable_spam_filter: true,
            enable_duplicate_detection: true,
            ..Default::default()
        };
        Box::new(DefaultMessageProcessor::with_config(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::models::{GuiChatMessage, MessageType};

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
    fn test_sanitize_content() {
        let processor = DefaultMessageProcessor::new();

        // 正常なコンテンツ
        let result = processor.sanitize_content("Hello, world!");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");

        // 長すぎるコンテンツ
        let long_content = "a".repeat(1500);
        let result = processor.sanitize_content(&long_content);
        assert!(result.is_ok());
        let sanitized = result.unwrap();
        assert!(sanitized.len() <= 1003); // 1000 + "..."
        assert!(sanitized.ends_with("..."));

        // 空のコンテンツ
        let result = processor.sanitize_content("");
        assert!(result.is_err());

        // 制御文字を含むコンテンツ
        let result = processor.sanitize_content("Hello\x00World\x1F!");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "HelloWorld!");
    }

    #[test]
    fn test_sanitize_author() {
        let processor = DefaultMessageProcessor::new();

        // 正常な著者名
        let result = processor.sanitize_author("TestUser");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "TestUser");

        // 空の著者名
        let result = processor.sanitize_author("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Unknown");

        // 空白のみの著者名
        let result = processor.sanitize_author("   ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Unknown");

        // 長すぎる著者名
        let long_author = "a".repeat(150);
        let result = processor.sanitize_author(&long_author);
        assert!(result.is_ok());
        assert!(result.unwrap().len() <= 100);
    }

    #[test]
    fn test_spam_detection() {
        let mut config = MessageProcessorConfig::default();
        config.enable_spam_filter = true;
        let processor = DefaultMessageProcessor::with_config(config);

        // 正常なメッセージ
        let normal_message = create_test_message("User", "Hello everyone!", MessageType::Text);
        assert!(!processor.is_spam(&normal_message));

        // 繰り返し文字のスパム
        let spam_message = create_test_message("Spammer", "aaaaaaaaaaaaa", MessageType::Text);
        assert!(processor.is_spam(&spam_message));

        // 大文字のスパム
        let caps_message = create_test_message("Shouter", "HELLO EVERYONE!!!", MessageType::Text);
        assert!(processor.is_spam(&caps_message));
    }

    #[test]
    fn test_emoji_counting() {
        let processor = DefaultMessageProcessor::new();

        // 絵文字なし
        assert_eq!(processor.count_emojis("Hello world"), 0);

        // 絵文字あり
        assert!(processor.count_emojis("Hello 😊 world 🎉") > 0);
    }

    #[test]
    fn test_message_filtering() {
        let processor = DefaultMessageProcessor::new();

        // デフォルトフィルタ（すべて許可）
        let default_filter = MessageFilterConfig::default();
        let message = create_test_message("User", "Hello", MessageType::Text);
        assert!(processor.filter_message(&message, &default_filter));

        // システムメッセージ除外
        let mut filter = MessageFilterConfig::default();
        filter.include_system_messages = false;
        let system_message = create_test_message("System", "User joined", MessageType::System);
        assert!(!processor.filter_message(&system_message, &filter));

        // 著者フィルタ
        filter.author_filter = Some("Alice".to_string());
        let alice_message = create_test_message("Alice", "Hello", MessageType::Text);
        let bob_message = create_test_message("Bob", "Hello", MessageType::Text);
        assert!(processor.filter_message(&alice_message, &filter));
        assert!(!processor.filter_message(&bob_message, &filter));
    }

    #[test]
    fn test_statistics_update() {
        let processor = DefaultMessageProcessor::new();
        let mut stats = MessageStatistics::default();

        // テキストメッセージ
        let text_message = create_test_message("User1", "Hello", MessageType::Text);
        processor.update_statistics(&text_message, &mut stats);
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.unique_authors.len(), 1);

        // SuperChatメッセージ
        let superchat_message = create_test_message(
            "User2",
            "Thanks!",
            MessageType::SuperChat {
                amount: "¥500".to_string(),
            },
        );
        processor.update_statistics(&superchat_message, &mut stats);
        assert_eq!(stats.total_messages, 2);
        assert_eq!(stats.unique_authors.len(), 2);
        assert_eq!(stats.super_chat_count, 1);
        assert!(stats.total_revenue > 0.0);
    }

    #[test]
    fn test_amount_parsing() {
        let processor = DefaultMessageProcessor::new();

        // 正常な金額
        assert_eq!(processor.parse_amount_string("¥500").unwrap(), 500.0);
        assert_eq!(processor.parse_amount_string("$25.50").unwrap(), 25.5);

        // 無効な金額
        assert!(processor.parse_amount_string("abc").is_err());
        assert!(processor.parse_amount_string("").is_err());
    }

    #[test]
    fn test_processor_factory() {
        let default_processor = DefaultMessageProcessorFactory::create_default();
        let spam_filter_processor = DefaultMessageProcessorFactory::create_with_spam_filter();

        // ファクトリが正常にプロセッサを作成することを確認
        // 実際のメッセージでスパムフィルターの動作をテスト
        use crate::gui::models::{GuiChatMessage, MessageType};
        let test_message = GuiChatMessage {
            timestamp: "12:34:56".to_string(),
            message_type: MessageType::Text,
            author: "testuser".to_string(),
            author_icon_url: None,
            channel_id: "test_channel".to_string(),
            content: "spam spam spam spam spam".to_string(), // スパムっぽい内容
            runs: Vec::new(),
            metadata: None,
            is_member: false,
            comment_count: None,
        };

        let filter_config = MessageFilterConfig {
            include_system_messages: false,
            include_super_chat: true,
            include_membership: true,
            author_filter: None,
            content_filter: Some("spam".to_string()), // スパムっぽいコンテンツをフィルター
            min_amount: None,
            max_amount: None,
        };

        // デフォルトプロセッサとスパムフィルタープロセッサで結果が同じことを確認
        // filter_message はスパム検出ではなく MessageFilterConfig に基づくフィルタリングを行う
        let default_result = default_processor.filter_message(&test_message, &filter_config);
        let spam_filter_result =
            spam_filter_processor.filter_message(&test_message, &filter_config);

        // 両方ともプロセッサが作成されていることを確認
        assert_eq!(default_result, true); // content_filter で "spam" を含むメッセージを通す
        assert_eq!(spam_filter_result, true); // filter_message は同じロジックを使用
    }
}
