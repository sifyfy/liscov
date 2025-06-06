use crate::{chat_management::QuestionCategory, gui::GuiChatMessage};
use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// メッセージフィルター構造体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageFilter {
    /// 作者フィルター（指定された作者のメッセージのみ表示）
    pub author_filter: Vec<String>,
    /// コンテンツキーワード（OR検索）
    pub content_keywords: Vec<String>,
    /// メッセージタイプフィルター
    pub message_types: HashSet<MessageType>,
    /// 金額範囲フィルター（Super Chat用）
    pub amount_range: Option<(f64, f64)>,
    /// 時間範囲フィルター
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// 質問カテゴリフィルター
    pub question_categories: HashSet<QuestionCategory>,
    /// VIP/メンバーシップフィルター
    pub membership_filter: Option<bool>,
    /// 最小メッセージ長
    pub min_message_length: Option<usize>,
    /// 最大メッセージ長
    pub max_message_length: Option<usize>,

    // パフォーマンス最適化用キャッシュ
    #[serde(skip)]
    pub(crate) lowercased_keywords: Vec<String>,
    #[serde(skip)]
    pub(crate) lowercased_authors: Vec<String>,
}

impl MessageFilter {
    /// メッセージ長範囲を取得（互換性のため）
    pub fn message_length_range(&self) -> Option<(usize, usize)> {
        match (self.min_message_length, self.max_message_length) {
            (Some(min), Some(max)) => Some((min, max)),
            (Some(min), None) => Some((min, usize::MAX)),
            (None, Some(max)) => Some((0, max)),
            (None, None) => None,
        }
    }
}

/// メッセージタイプ列挙型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    Regular,
    SuperChat,
    Membership,
    Question,
    Emoji,
    Link,
}

impl MessageType {
    /// メッセージタイプを文字列として取得
    pub fn as_string(&self) -> String {
        match self {
            MessageType::Regular => "Regular".to_string(),
            MessageType::SuperChat => "SuperChat".to_string(),
            MessageType::Membership => "Membership".to_string(),
            MessageType::Question => "Question".to_string(),
            MessageType::Emoji => "Emoji".to_string(),
            MessageType::Link => "Link".to_string(),
        }
    }
}

impl Default for MessageFilter {
    fn default() -> Self {
        let mut message_types = HashSet::new();
        message_types.insert(MessageType::Regular);
        message_types.insert(MessageType::SuperChat);
        message_types.insert(MessageType::Membership);
        message_types.insert(MessageType::Question);
        message_types.insert(MessageType::Emoji);
        message_types.insert(MessageType::Link);

        let mut question_categories = HashSet::new();
        question_categories.insert(QuestionCategory::Technical);
        question_categories.insert(QuestionCategory::General);
        question_categories.insert(QuestionCategory::Request);
        question_categories.insert(QuestionCategory::Feedback);
        question_categories.insert(QuestionCategory::Other);

        Self {
            author_filter: Vec::new(),
            content_keywords: Vec::new(),
            message_types,
            amount_range: None,
            time_range: None,
            question_categories,
            membership_filter: None,
            min_message_length: None,
            max_message_length: None,
            lowercased_keywords: Vec::new(),
            lowercased_authors: Vec::new(),
        }
    }
}

impl MessageFilter {
    /// 新しいフィルターを作成（最適化版）
    pub fn new() -> Self {
        let mut message_types = HashSet::new();
        message_types.insert(MessageType::Regular);
        message_types.insert(MessageType::SuperChat);
        message_types.insert(MessageType::Membership);
        message_types.insert(MessageType::Question);
        message_types.insert(MessageType::Emoji);
        message_types.insert(MessageType::Link);

        let mut question_categories = HashSet::new();
        question_categories.insert(QuestionCategory::Technical);
        question_categories.insert(QuestionCategory::General);
        question_categories.insert(QuestionCategory::Request);
        question_categories.insert(QuestionCategory::Feedback);
        question_categories.insert(QuestionCategory::Other);

        Self {
            author_filter: Vec::new(),
            content_keywords: Vec::new(),
            message_types,
            amount_range: None,
            time_range: None,
            question_categories,
            membership_filter: None,
            min_message_length: None,
            max_message_length: None,
            lowercased_keywords: Vec::new(),
            lowercased_authors: Vec::new(),
        }
    }

    /// すべてのフィルターをクリア
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// 作者を追加（最適化版）
    pub fn add_author(&mut self, author: String) {
        self.author_filter.push(author.clone());
        self.lowercased_authors.push(author.to_lowercase());
    }

    /// 作者を削除（最適化版）
    pub fn remove_author(&mut self, author: &str) {
        if let Some(pos) = self.author_filter.iter().position(|a| a == author) {
            self.author_filter.remove(pos);
            self.lowercased_authors.remove(pos);
        }
    }

    /// キーワードを追加（最適化版）
    pub fn add_keyword(&mut self, keyword: String) {
        self.content_keywords.push(keyword.clone());
        self.lowercased_keywords.push(keyword.to_lowercase());
    }

    /// キーワードを削除（最適化版）
    pub fn remove_keyword(&mut self, keyword: &str) {
        if let Some(pos) = self.content_keywords.iter().position(|k| k == keyword) {
            self.content_keywords.remove(pos);
            self.lowercased_keywords.remove(pos);
        }
    }

    /// 金額範囲を設定（テスト用の簡易版）
    pub fn set_amount_range(&mut self, range: Option<(f64, f64)>) {
        self.amount_range = range;
    }

    /// 金額範囲を設定（詳細版）
    pub fn set_amount_range_detailed(&mut self, min: Option<f64>, max: Option<f64>) {
        match (min, max) {
            (Some(min_val), Some(max_val)) => {
                self.amount_range = Some((min_val, max_val));
            }
            (Some(min_val), None) => {
                self.amount_range = Some((min_val, f64::MAX));
            }
            (None, Some(max_val)) => {
                self.amount_range = Some((0.0, max_val));
            }
            (None, None) => {
                self.amount_range = None;
            }
        }
    }

    /// 時間範囲を設定
    pub fn set_time_range(&mut self, start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) {
        if let (Some(start_time), Some(end_time)) = (start, end) {
            self.time_range = Some((start_time, end_time));
        } else {
            self.time_range = None;
        }
    }

    /// メッセージ長範囲を設定
    pub fn set_message_length_range(&mut self, min: Option<usize>, max: Option<usize>) {
        self.min_message_length = min;
        self.max_message_length = max;
    }

    /// メッセージがフィルター条件に合致するかチェック（最適化版）
    pub fn matches(&self, message: &GuiChatMessage) -> bool {
        // 作者フィルター（最適化済み）
        if !self.lowercased_authors.is_empty() {
            let message_author_lower = message.author.to_lowercase();
            if !self.lowercased_authors.contains(&message_author_lower) {
                return false;
            }
        }

        // キーワードフィルター（最適化済み）
        if !self.lowercased_keywords.is_empty() {
            let message_content_lower = message.content.to_lowercase();
            let matches_keyword = self
                .lowercased_keywords
                .iter()
                .any(|keyword| message_content_lower.contains(keyword));
            if !matches_keyword {
                return false;
            }
        }

        // メッセージタイプフィルター（高速化）
        let message_type = self.classify_message_type_fast(message);
        if !self.message_types.contains(&message_type) {
            return false;
        }

        // 金額範囲フィルター（変更なし）
        if let Some((min_amount, max_amount)) = self.amount_range {
            if let Some(metadata) = &message.metadata {
                if let Some(amount_str) = &metadata.amount {
                    if let Ok(amount) = amount_str.replace(['$', '¥', '€', '£'], "").parse::<f64>()
                    {
                        if amount < min_amount || amount > max_amount {
                            return false;
                        }
                    } else if min_amount > 0.0 {
                        return false;
                    }
                } else if min_amount > 0.0 {
                    return false;
                }
            } else if min_amount > 0.0 {
                return false;
            }
        }

        // メンバーシップフィルター（改良版）
        if let Some(membership_required) = self.membership_filter {
            use crate::gui::models::MessageType as GuiMessageType;

            // メンバーかどうかの判定：
            // 1. メッセージのis_memberフィールド（バッジベース）
            // 2. MessageType::Membership（新規メンバー加入）
            let is_member =
                message.is_member || matches!(message.message_type, GuiMessageType::Membership);

            if membership_required != is_member {
                return false;
            }
        }

        // メッセージ長フィルター（最適化済み）
        if self.min_message_length.is_some() || self.max_message_length.is_some() {
            let message_length = message.content.chars().count();
            if let Some(min_length) = self.min_message_length {
                if message_length < min_length {
                    return false;
                }
            }
            if let Some(max_length) = self.max_message_length {
                if message_length > max_length {
                    return false;
                }
            }
        }

        true
    }

    /// メッセージのタイプを分類（高速版）
    fn classify_message_type_fast(&self, message: &GuiChatMessage) -> MessageType {
        use crate::gui::models::MessageType as GuiMessageType;

        // Super Chat（最優先）
        if matches!(message.message_type, GuiMessageType::SuperChat { .. }) {
            return MessageType::SuperChat;
        }

        // メンバーシップ
        if matches!(message.message_type, GuiMessageType::Membership) {
            return MessageType::Membership;
        }

        // 短いメッセージは高速パスで判定
        let content_len = message.content.len();
        if content_len <= 3 {
            // 絵文字のみの可能性が高い
            if self.is_mostly_emoji_fast(&message.content) {
                return MessageType::Emoji;
            }
            return MessageType::Regular;
        }

        // リンク判定（軽量）
        if message.content.contains("http") || message.content.contains("www.") {
            return MessageType::Link;
        }

        // 質問判定（最適化済み）
        if self.looks_like_question_fast(&message.content) {
            return MessageType::Question;
        }

        MessageType::Regular
    }

    /// 質問らしいかどうかの高速判定
    fn looks_like_question_fast(&self, content: &str) -> bool {
        // 簡単な文字ベース判定（正規表現なし）
        content.contains('？')
            || content.contains('?')
            || content.contains("どう")
            || content.contains("なん")
            || content.contains("何")
            || content.contains("いつ")
            || content.contains("どこ")
            || content.contains("どれ")
            || content.contains("なぜ")
            || content.contains("教え")
            || content.contains("知り")
            || content.contains("わから")
    }

    /// 主に絵文字からなるメッセージかどうか（高速版）
    fn is_mostly_emoji_fast(&self, content: &str) -> bool {
        if content.len() > 10 {
            return false; // 長いメッセージは絵文字のみではない
        }

        // 絵文字の簡易判定（Unicodeブロック範囲チェック）
        content.chars().any(|c| {
            let code = c as u32;
            (code >= 0x1F600 && code <= 0x1F64F) || // 絵文字ブロック
            (code >= 0x1F300 && code <= 0x1F5FF) || // その他シンボル
            (code >= 0x1F680 && code <= 0x1F6FF) || // 交通・地図
            (code >= 0x2600 && code <= 0x26FF) // その他シンボル
        })
    }

    /// フィルター適用してメッセージリストを取得
    pub fn filter_messages(&self, messages: &[GuiChatMessage]) -> Vec<GuiChatMessage> {
        messages
            .iter()
            .filter(|message| self.matches(message))
            .cloned()
            .collect()
    }

    /// アクティブなフィルター数を取得
    pub fn active_filter_count(&self) -> usize {
        let mut count = 0;

        if !self.author_filter.is_empty() {
            count += 1;
        }
        if !self.content_keywords.is_empty() {
            count += 1;
        }
        if self.message_types.len() < 6 {
            // デフォルトは全6種類
            count += 1;
        }
        if self.amount_range.is_some() {
            count += 1;
        }
        if self.time_range.is_some() {
            count += 1;
        }
        if self.membership_filter.is_some() {
            count += 1;
        }
        if self.min_message_length.is_some() || self.max_message_length.is_some() {
            count += 1;
        }

        count
    }

    /// フィルターが有効かどうか
    pub fn is_active(&self) -> bool {
        self.active_filter_count() > 0
    }

    /// 作者フィルターのリストを取得
    pub fn get_authors(&self) -> &Vec<String> {
        &self.author_filter
    }

    /// キーワードフィルターのリストを取得
    pub fn get_keywords(&self) -> &Vec<String> {
        &self.content_keywords
    }

    /// 金額範囲フィルターを取得
    pub fn get_amount_range(&self) -> Option<(f64, f64)> {
        self.amount_range
    }

    /// メッセージタイプフィルターを取得
    pub fn get_message_types(&self) -> &HashSet<MessageType> {
        &self.message_types
    }

    /// メンバーシップフィルターを取得
    pub fn get_membership_filter(&self) -> Option<bool> {
        self.membership_filter
    }

    /// メッセージ長範囲を取得
    pub fn get_message_length_range(&self) -> Option<(usize, usize)> {
        match (self.min_message_length, self.max_message_length) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    /// 時間範囲フィルターを取得
    pub fn get_time_range(&self) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        self.time_range
    }

    /// フィルターがアクティブかどうかを判定（メソッド名を統一）
    pub fn is_filter_active(&self) -> bool {
        self.is_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message(author: &str, content: &str, amount: Option<f64>) -> GuiChatMessage {
        use crate::gui::models::{MessageMetadata, MessageType};

        let message_type = if let Some(amt) = amount {
            MessageType::SuperChat {
                amount: amt.to_string(),
            }
        } else {
            MessageType::Text
        };

        GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type,
            author: author.to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            metadata: amount.map(|amt| MessageMetadata {
                amount: Some(amt.to_string()),
                badges: vec![],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
            is_member: false,
        }
    }

    fn create_membership_message(author: &str, content: &str) -> GuiChatMessage {
        use crate::gui::models::{MessageMetadata, MessageType};

        GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type: MessageType::Membership,
            author: author.to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            metadata: Some(MessageMetadata {
                amount: None,
                badges: vec![],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
            is_member: true,
        }
    }

    fn create_member_text_message(author: &str, content: &str) -> GuiChatMessage {
        use crate::gui::models::{MessageMetadata, MessageType};

        GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type: MessageType::Text,
            author: author.to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            metadata: Some(MessageMetadata {
                amount: None,
                badges: vec!["メンバー（1年）".to_string()],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
            is_member: true,
        }
    }

    #[test]
    fn test_default_filter() {
        let filter = MessageFilter::default();
        assert_eq!(filter.author_filter.len(), 0);
        assert_eq!(filter.content_keywords.len(), 0);
        assert_eq!(filter.message_types.len(), 6); // 全タイプが有効
        assert_eq!(filter.amount_range, None);
        assert_eq!(filter.time_range, None);
        assert_eq!(filter.membership_filter, None);
        assert_eq!(filter.min_message_length, None);
        assert_eq!(filter.max_message_length, None);
        assert!(!filter.is_active()); // デフォルトはフィルター非適用
    }

    #[test]
    fn test_author_filter() {
        let mut filter = MessageFilter::new();
        filter.add_author("TestUser".to_string());

        let msg1 = create_test_message("TestUser", "Hello", None);
        let msg2 = create_test_message("OtherUser", "Hi", None);

        assert!(filter.matches(&msg1));
        assert!(!filter.matches(&msg2));
        assert_eq!(filter.active_filter_count(), 1);
        assert!(filter.is_active());
    }

    #[test]
    fn test_multiple_authors() {
        let mut filter = MessageFilter::new();
        filter.add_author("User1".to_string());
        filter.add_author("User2".to_string());

        let msg1 = create_test_message("User1", "Hello", None);
        let msg2 = create_test_message("User2", "Hi", None);
        let msg3 = create_test_message("User3", "Hey", None);

        assert!(filter.matches(&msg1));
        assert!(filter.matches(&msg2));
        assert!(!filter.matches(&msg3));
    }

    #[test]
    fn test_author_removal() {
        let mut filter = MessageFilter::new();
        filter.add_author("User1".to_string());
        filter.add_author("User2".to_string());
        filter.remove_author("User1");

        let msg1 = create_test_message("User1", "Hello", None);
        let msg2 = create_test_message("User2", "Hi", None);

        assert!(!filter.matches(&msg1));
        assert!(filter.matches(&msg2));
    }

    #[test]
    fn test_keyword_filter() {
        let mut filter = MessageFilter::new();
        filter.add_keyword("hello".to_string());

        let msg1 = create_test_message("User", "Hello world", None);
        let msg2 = create_test_message("User", "HELLO everyone", None);
        let msg3 = create_test_message("User", "Goodbye", None);

        assert!(filter.matches(&msg1)); // 大小文字無視
        assert!(filter.matches(&msg2)); // 大小文字無視
        assert!(!filter.matches(&msg3));
    }

    #[test]
    fn test_multiple_keywords_or_search() {
        let mut filter = MessageFilter::new();
        filter.add_keyword("hello".to_string());
        filter.add_keyword("world".to_string());

        let msg1 = create_test_message("User", "Hello everyone", None);
        let msg2 = create_test_message("User", "Beautiful world", None);
        let msg3 = create_test_message("User", "How are you?", None);

        assert!(filter.matches(&msg1)); // helloが含まれている
        assert!(filter.matches(&msg2)); // worldが含まれている
        assert!(!filter.matches(&msg3)); // どちらも含まれていない
    }

    #[test]
    fn test_keyword_removal() {
        let mut filter = MessageFilter::new();
        filter.add_keyword("hello".to_string());
        filter.add_keyword("world".to_string());
        filter.remove_keyword("hello");

        let msg1 = create_test_message("User", "Hello everyone", None);
        let msg2 = create_test_message("User", "Beautiful world", None);

        assert!(!filter.matches(&msg1));
        assert!(filter.matches(&msg2));
    }

    #[test]
    fn test_amount_filter() {
        let mut filter = MessageFilter::new();
        filter.set_amount_range(Some((100.0, 500.0)));

        let msg1 = create_test_message("User", "Thanks!", Some(200.0));
        let msg2 = create_test_message("User", "Thanks!", Some(50.0));
        let msg3 = create_test_message("User", "Thanks!", Some(600.0));
        let msg4 = create_test_message("User", "Regular message", None);

        assert!(filter.matches(&msg1)); // 範囲内
        assert!(!filter.matches(&msg2)); // 範囲外（下限未満）
        assert!(!filter.matches(&msg3)); // 範囲外（上限超過）
        assert!(!filter.matches(&msg4)); // 金額なし
    }

    #[test]
    fn test_amount_filter_detailed() {
        let mut filter = MessageFilter::new();

        // 最小値のみ指定
        filter.set_amount_range_detailed(Some(100.0), None);
        let msg1 = create_test_message("User", "Thanks!", Some(200.0));
        let msg2 = create_test_message("User", "Thanks!", Some(50.0));
        assert!(filter.matches(&msg1));
        assert!(!filter.matches(&msg2));

        // 最大値のみ指定
        filter.set_amount_range_detailed(None, Some(500.0));
        let msg3 = create_test_message("User", "Thanks!", Some(300.0));
        let msg4 = create_test_message("User", "Thanks!", Some(600.0));
        assert!(filter.matches(&msg3));
        assert!(!filter.matches(&msg4));

        // 両方指定なし
        filter.set_amount_range_detailed(None, None);
        assert_eq!(filter.amount_range, None);
    }

    #[test]
    fn test_message_type_filter() {
        let mut filter = MessageFilter::new();

        // SuperChatのみを許可
        filter.message_types.clear();
        filter.message_types.insert(MessageType::SuperChat);

        let msg1 = create_test_message("User", "Thanks!", Some(100.0));
        let msg2 = create_test_message("User", "Regular message", None);

        assert!(filter.matches(&msg1)); // SuperChat
        assert!(!filter.matches(&msg2)); // Regular message
    }

    #[test]
    fn test_membership_filter() {
        let mut filter = MessageFilter::new();

        // テストメッセージ作成
        let member_msg = create_membership_message("Member", "Hello as member");
        let member_text_msg = create_member_text_message("MemberUser", "メンバーからのテキスト");
        let regular_msg = create_test_message("Regular", "Hello", None);

        // メンバーのみフィルター
        filter.membership_filter = Some(true);
        assert!(filter.matches(&member_msg)); // メンバーシップアイテム
        assert!(filter.matches(&member_text_msg)); // バッジベースメンバー
        assert!(!filter.matches(&regular_msg)); // 一般ユーザー

        // 非メンバーのみフィルター
        filter.membership_filter = Some(false);
        assert!(!filter.matches(&member_msg)); // メンバーシップアイテム
        assert!(!filter.matches(&member_text_msg)); // バッジベースメンバー
        assert!(filter.matches(&regular_msg)); // 一般ユーザー

        // フィルターなし
        filter.membership_filter = None;
        assert!(filter.matches(&member_msg)); // 全て通す
        assert!(filter.matches(&member_text_msg)); // 全て通す
        assert!(filter.matches(&regular_msg)); // 全て通す
    }

    #[test]
    fn test_message_length_filter() {
        let mut filter = MessageFilter::new();
        filter.set_message_length_range(Some(5), Some(20));

        let msg1 = create_test_message("User", "Hello", None); // 5文字
        let msg2 = create_test_message("User", "Hi", None); // 2文字
        let msg3 = create_test_message("User", "This is a very long message", None); // 27文字

        assert!(filter.matches(&msg1)); // 範囲内
        assert!(!filter.matches(&msg2)); // 短すぎる
        assert!(!filter.matches(&msg3)); // 長すぎる
    }

    #[test]
    fn test_question_detection() {
        let filter = MessageFilter::new();

        // 質問文
        assert!(filter.looks_like_question("これはどうやって使うんですか？"));
        assert!(filter.looks_like_question("何時からですか?"));
        assert!(filter.looks_like_question("教えてください"));
        assert!(filter.looks_like_question("わからないです"));
        assert!(filter.looks_like_question("いつ始まりますか？"));
        assert!(filter.looks_like_question("どこで買えますか？"));
        assert!(filter.looks_like_question("なぜですか？"));

        // 質問ではない文
        assert!(!filter.looks_like_question("ありがとうございます"));
        assert!(!filter.looks_like_question("こんにちは"));
        assert!(!filter.looks_like_question("良い配信でした"));
    }

    #[test]
    fn test_emoji_detection() {
        let filter = MessageFilter::new();

        // 絵文字メッセージ（短い）
        assert!(filter.is_mostly_emoji("😀"));
        assert!(filter.is_mostly_emoji("🎉"));

        // 長いメッセージは絵文字判定しない
        assert!(!filter.is_mostly_emoji("Hello 😀 world"));
        assert!(!filter.is_mostly_emoji("こんにちは"));
    }

    #[test]
    fn test_link_detection() {
        let filter = MessageFilter::new();

        assert!(filter.contains_link("Check this out: https://example.com"));
        assert!(filter.contains_link("Visit http://test.org"));
        assert!(filter.contains_link("Go to www.example.com"));
        assert!(!filter.contains_link("No links here"));
    }

    #[test]
    fn test_message_classification() {
        let filter = MessageFilter::new();

        // SuperChat
        let superchat = create_test_message("User", "Thanks!", Some(100.0));
        assert_eq!(
            filter.classify_message_type_fast(&superchat),
            MessageType::SuperChat
        );

        // Membership
        let membership = create_membership_message("User", "Joined");
        assert_eq!(
            filter.classify_message_type_fast(&membership),
            MessageType::Membership
        );

        // Question
        let question = create_test_message("User", "これはどうですか？", None);
        assert_eq!(
            filter.classify_message_type_fast(&question),
            MessageType::Question
        );

        // Link
        let link = create_test_message("User", "Check https://example.com", None);
        assert_eq!(filter.classify_message_type_fast(&link), MessageType::Link);

        // Regular
        let regular = create_test_message("User", "Hello everyone", None);
        assert_eq!(
            filter.classify_message_type_fast(&regular),
            MessageType::Regular
        );
    }

    #[test]
    fn test_filter_combination() {
        let mut filter = MessageFilter::new();
        filter.add_author("TestUser".to_string());
        filter.add_keyword("hello".to_string());

        let msg1 = create_test_message("TestUser", "Hello world", None);
        let msg2 = create_test_message("TestUser", "Goodbye", None);
        let msg3 = create_test_message("OtherUser", "Hello", None);

        assert!(filter.matches(&msg1)); // 両方の条件を満たす
        assert!(!filter.matches(&msg2)); // keywordが一致しない
        assert!(!filter.matches(&msg3)); // authorが一致しない
    }

    #[test]
    fn test_complex_filter_combination() {
        let mut filter = MessageFilter::new();
        filter.add_author("VIP".to_string());
        filter.add_keyword("question".to_string());
        filter.set_amount_range(Some((100.0, 1000.0)));
        filter.set_message_length_range(Some(10), Some(100));

        let msg1 = create_test_message("VIP", "I have a question about the stream", Some(500.0));
        let msg2 = create_test_message("VIP", "Question", Some(500.0)); // 短すぎる
        let msg3 = create_test_message("Regular", "I have a question", Some(500.0)); // 作者が違う
        let msg4 = create_test_message("VIP", "I have a question", Some(50.0)); // 金額が低い

        assert!(filter.matches(&msg1)); // 全条件満たす
        assert!(!filter.matches(&msg2)); // 文字数不足
        assert!(!filter.matches(&msg3)); // 作者不一致
        assert!(!filter.matches(&msg4)); // 金額不足
    }

    #[test]
    fn test_filter_messages_function() {
        let mut filter = MessageFilter::new();
        filter.add_author("Alice".to_string());

        let messages = vec![
            create_test_message("Alice", "Hello", None),
            create_test_message("Bob", "Hi", None),
            create_test_message("Alice", "How are you?", None),
            create_test_message("Charlie", "Good", None),
        ];

        let filtered = filter.filter_messages(&messages);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|msg| msg.author == "Alice"));
    }

    #[test]
    fn test_active_filter_count() {
        let mut filter = MessageFilter::new();
        assert_eq!(filter.active_filter_count(), 0);

        filter.add_author("User".to_string());
        assert_eq!(filter.active_filter_count(), 1);

        filter.add_keyword("test".to_string());
        assert_eq!(filter.active_filter_count(), 2);

        filter.set_amount_range(Some((100.0, 500.0)));
        assert_eq!(filter.active_filter_count(), 3);

        filter.set_message_length_range(Some(5), Some(100));
        assert_eq!(filter.active_filter_count(), 4);

        filter.membership_filter = Some(true);
        assert_eq!(filter.active_filter_count(), 5);
    }

    #[test]
    fn test_filter_clear() {
        let mut filter = MessageFilter::new();
        filter.add_author("User".to_string());
        filter.add_keyword("test".to_string());
        filter.set_amount_range(Some((100.0, 500.0)));

        assert!(filter.is_active());

        filter.clear();

        assert!(!filter.is_active());
        assert_eq!(filter.active_filter_count(), 0);
        assert!(filter.author_filter.is_empty());
        assert!(filter.content_keywords.is_empty());
        assert_eq!(filter.message_types.len(), 6); // デフォルトの6種類
    }

    #[test]
    fn test_getter_methods() {
        let mut filter = MessageFilter::new();
        filter.add_author("User1".to_string());
        filter.add_keyword("keyword1".to_string());
        filter.set_amount_range(Some((100.0, 500.0)));
        filter.membership_filter = Some(true);

        assert_eq!(filter.get_authors(), &vec!["User1".to_string()]);
        assert_eq!(filter.get_keywords(), &vec!["keyword1".to_string()]);
        assert_eq!(filter.get_amount_range(), Some((100.0, 500.0)));
        assert_eq!(filter.get_membership_filter(), Some(true));
        assert_eq!(filter.get_message_types().len(), 6);
    }

    #[test]
    fn test_message_length_range_compatibility() {
        let mut filter = MessageFilter::new();

        // 両方設定
        filter.set_message_length_range(Some(5), Some(50));
        assert_eq!(filter.message_length_range(), Some((5, 50)));

        // 最小のみ
        filter.set_message_length_range(Some(10), None);
        assert_eq!(filter.message_length_range(), Some((10, usize::MAX)));

        // 最大のみ
        filter.set_message_length_range(None, Some(100));
        assert_eq!(filter.message_length_range(), Some((0, 100)));

        // 両方なし
        filter.set_message_length_range(None, None);
        assert_eq!(filter.message_length_range(), None);
    }

    #[test]
    fn test_currency_parsing_in_amount_filter() {
        let mut filter = MessageFilter::new();
        filter.set_amount_range(Some((100.0, 500.0)));

        // 様々な通貨記号でテスト
        let msg_yen = GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type: crate::gui::models::MessageType::SuperChat {
                amount: "¥300".to_string(),
            },
            author: "User".to_string(),
            channel_id: "test".to_string(),
            content: "Thanks!".to_string(),
            metadata: Some(crate::gui::models::MessageMetadata {
                amount: Some("¥300".to_string()),
                badges: vec![],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
            is_member: false,
        };

        let msg_dollar = GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type: crate::gui::models::MessageType::SuperChat {
                amount: "$200".to_string(),
            },
            author: "User".to_string(),
            channel_id: "test".to_string(),
            content: "Thanks!".to_string(),
            metadata: Some(crate::gui::models::MessageMetadata {
                amount: Some("$200".to_string()),
                badges: vec![],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
            is_member: false,
        };

        assert!(filter.matches(&msg_yen)); // ¥300は範囲内
        assert!(filter.matches(&msg_dollar)); // $200は範囲内
    }

    #[test]
    fn test_japanese_content_filtering() {
        let mut filter = MessageFilter::new();
        filter.add_keyword("配信".to_string());

        let msg1 = create_test_message("User", "今日の配信ありがとうございました", None);
        let msg2 = create_test_message("User", "Thank you for streaming", None);

        assert!(filter.matches(&msg1)); // 日本語キーワードマッチ
        assert!(!filter.matches(&msg2)); // 英語のみ
    }

    #[test]
    fn test_empty_filter_matches_all() {
        let filter = MessageFilter::new();

        let messages = vec![
            create_test_message("User1", "Hello", None),
            create_test_message("User2", "World", Some(100.0)),
            create_membership_message("Member", "Joined"),
        ];

        // デフォルトフィルター（非アクティブ）は全メッセージにマッチ
        for msg in &messages {
            assert!(filter.matches(msg));
        }
    }

    #[test]
    fn test_edge_cases() {
        let filter = MessageFilter::new();

        // 空文字列コンテンツ
        let empty_msg = create_test_message("User", "", None);
        assert!(filter.matches(&empty_msg));

        // 非常に長いコンテンツ
        let long_content = "a".repeat(10000);
        let long_msg = create_test_message("User", &long_content, None);
        assert!(filter.matches(&long_msg));

        // 特殊文字
        let special_msg = create_test_message("User", "!@#$%^&*()", None);
        assert!(filter.matches(&special_msg));
    }
}
