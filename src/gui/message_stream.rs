//! メッセージストリーミングシステム
//!
//! 大量メッセージの効率的な表示とメモリ管理を提供

use crate::gui::models::GuiChatMessage;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// 表示制限の設定
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DisplayLimit {
    /// 制限なし（現在の動作、デバッグ用）
    Unlimited,
    /// 固定件数制限
    Fixed(usize),
    /// メモリ上限ベース（MB単位）
    Memory(usize),
    /// パフォーマンス重視（目標FPS維持）
    Performance(u32),
}

impl Default for DisplayLimit {
    fn default() -> Self {
        Self::Fixed(100) // デフォルトは100件制限
    }
}

/// メッセージストリーミング設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStreamConfig {
    /// 表示制限方式
    pub display_limit: DisplayLimit,
    /// 最大表示件数（固定制限時）
    pub max_display_count: usize,
    /// 仮想スクロール有効
    pub enable_virtual_scroll: bool,
    /// 目標フレームレート
    pub target_fps: u32,
    /// アーカイブ機能有効
    pub enable_archive: bool,
    /// 検索機能有効
    pub archive_search_enabled: bool,
}

impl Default for MessageStreamConfig {
    fn default() -> Self {
        Self {
            display_limit: DisplayLimit::default(),
            max_display_count: 100,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        }
    }
}

/// メッセージストリーミングの中核構造体
#[derive(Debug)]
pub struct MessageStream {
    /// 現在表示されているメッセージウィンドウ
    display_window: VecDeque<GuiChatMessage>,
    /// アーカイブされたメッセージ
    archive: Vec<GuiChatMessage>,
    /// 🚀 IDベース更新システム: メッセージID → インデックスマッピング
    message_id_map: HashMap<String, usize>,
    /// 🚀 メッセージIDの順序リスト（効率的な順序管理）
    message_id_order: VecDeque<String>,
    /// 設定
    config: MessageStreamConfig,
    /// 総メッセージ数（削除されたものを含む）
    total_count: usize,
    /// アーカイブ済みメッセージ数
    archived_count: usize,
    /// 最後のクリーンアップ時刻
    last_cleanup: std::time::Instant,
}

impl MessageStream {
    /// 新しいメッセージストリームを作成
    pub fn new(config: MessageStreamConfig) -> Self {
        let capacity = match &config.display_limit {
            DisplayLimit::Fixed(count) => *count,
            DisplayLimit::Memory(mb) => {
                // GuiChatMessage 1件あたり約300バイトと仮定
                (mb * 1024 * 1024) / 300
            }
            _ => 100, // デフォルト
        };

        Self {
            display_window: VecDeque::with_capacity(capacity),
            archive: Vec::new(),
            message_id_map: HashMap::new(),    // 🚀 IDマッピング初期化
            message_id_order: VecDeque::new(), // 🚀 ID順序リスト初期化
            config,
            total_count: 0,
            archived_count: 0,
            last_cleanup: std::time::Instant::now(),
        }
    }

    /// デフォルト設定で作成
    pub fn with_defaults() -> Self {
        Self::new(MessageStreamConfig::default())
    }

    /// 固定件数制限で作成
    pub fn with_fixed_limit(max_count: usize) -> Self {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(max_count),
            max_display_count: max_count,
            ..Default::default()
        };
        Self::new(config)
    }

    /// メッセージを追加
    pub fn push_message(&mut self, message: GuiChatMessage) {
        // 🚀 IDベース更新システム: ユニークIDを生成
        let message_id = self.generate_message_id(&message);

        // 重複チェック（O(1)）
        if self.message_id_map.contains_key(&message_id) {
            // 既存メッセージの場合は更新をスキップ
            return;
        }

        self.total_count += 1;
        let index = self.display_window.len();

        // 🚀 IDマッピングを更新（O(1)アクセス用）
        self.message_id_map.insert(message_id.clone(), index);
        self.message_id_order.push_back(message_id);

        // 表示ウィンドウに追加
        self.display_window.push_back(message);

        // 容量チェックとアーカイブ処理
        self.apply_display_limit();

        // 定期的なクリーンアップ
        if self.last_cleanup.elapsed().as_secs() > 60 {
            self.cleanup();
        }
    }

    /// 🚀 メッセージのユニークIDを生成
    fn generate_message_id(&self, message: &GuiChatMessage) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        message.timestamp.hash(&mut hasher);
        message.author.hash(&mut hasher);
        message.content.hash(&mut hasher);

        format!("msg_{:x}", hasher.finish())
    }

    /// 複数メッセージをバッチ追加
    pub fn push_messages(&mut self, messages: Vec<GuiChatMessage>) {
        for message in messages {
            self.push_message(message);
        }
    }

    /// 現在表示中のメッセージを取得
    pub fn display_messages(&self) -> Vec<GuiChatMessage> {
        self.display_window.iter().cloned().collect()
    }

    /// 表示中メッセージの参照を取得
    pub fn display_messages_ref(&self) -> impl Iterator<Item = &GuiChatMessage> {
        self.display_window.iter()
    }

    /// 最新のN件のメッセージを取得
    pub fn recent_messages(&self, n: usize) -> Vec<GuiChatMessage> {
        self.display_window
            .iter()
            .rev()
            .take(n)
            .rev()
            .cloned()
            .collect()
    }

    /// 表示中メッセージ数
    pub fn display_count(&self) -> usize {
        self.display_window.len()
    }

    /// 総メッセージ数
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// アーカイブ済みメッセージ数
    pub fn archived_count(&self) -> usize {
        self.archived_count
    }

    /// 表示制限を適用（双方向復帰対応）
    fn apply_display_limit(&mut self) {
        let limit = match &self.config.display_limit {
            DisplayLimit::Unlimited => {
                // 無制限の場合：全てのアーカイブを復帰
                self.restore_from_archive(usize::MAX);
                return;
            }
            DisplayLimit::Fixed(count) => *count,
            DisplayLimit::Memory(mb) => {
                // GuiChatMessage 1件あたり約300バイトと仮定
                (mb * 1024 * 1024) / 300
            }
            DisplayLimit::Performance(_target_fps) => {
                // パフォーマンス重視の場合は動的調整
                // 現状では固定100件とする（将来的にFPS監視で調整）
                100
            }
        };

        if self.display_window.len() < limit {
            // 表示制限が増加：アーカイブから復帰
            let restore_count = limit - self.display_window.len();
            self.restore_from_archive(restore_count);
        } else if self.display_window.len() > limit {
            // 表示制限が減少：アーカイブに移動
            while self.display_window.len() > limit {
                if let Some(old_message) = self.display_window.pop_front() {
                    if self.config.enable_archive {
                        self.archive.push(old_message);
                    }
                    self.archived_count += 1;
                }
            }
        }
    }

    /// アーカイブから表示ウィンドウに復帰
    fn restore_from_archive(&mut self, max_count: usize) {
        if !self.config.enable_archive || self.archive.is_empty() {
            return;
        }

        let restore_count = max_count.min(self.archive.len());
        if restore_count == 0 {
            return;
        }

        tracing::info!(
            "🔄 [MessageStream] Restoring {} messages from archive",
            restore_count
        );

        // アーカイブの末尾（最新）から復帰
        for _ in 0..restore_count {
            if let Some(message) = self.archive.pop() {
                self.display_window.push_front(message);
                self.archived_count = self.archived_count.saturating_sub(1);
            }
        }
    }

    /// 定期的なクリーンアップ
    fn cleanup(&mut self) {
        // アーカイブのメモリ最適化
        if self.config.enable_archive {
            self.archive.shrink_to_fit();
        }

        // 表示ウィンドウのメモリ最適化
        self.display_window.shrink_to_fit();

        self.last_cleanup = std::time::Instant::now();

        tracing::debug!(
            "🧹 MessageStream cleanup: display={}, archived={}, total={}",
            self.display_count(),
            self.archived_count(),
            self.total_count()
        );
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: MessageStreamConfig) {
        self.config = config;
        self.apply_display_limit(); // 新しい制限を即座に適用
    }

    /// 現在の設定を取得
    pub fn config(&self) -> &MessageStreamConfig {
        &self.config
    }

    /// 統計情報を取得
    pub fn stats(&self) -> MessageStreamStats {
        let display_memory = self.display_window.len() * std::mem::size_of::<GuiChatMessage>();
        let archive_memory = if self.config.enable_archive {
            self.archive.len() * std::mem::size_of::<GuiChatMessage>()
        } else {
            0
        };

        MessageStreamStats {
            display_count: self.display_count(),
            archived_count: self.archived_count(),
            total_count: self.total_count(),
            display_memory_bytes: display_memory,
            archive_memory_bytes: archive_memory,
            total_memory_bytes: display_memory + archive_memory,
            effective_reduction_percent: if self.total_count > 0 {
                ((self.archived_count as f64 / self.total_count as f64) * 100.0) as u32
            } else {
                0
            },
        }
    }

    /// 全メッセージをクリア
    pub fn clear(&mut self) {
        self.display_window.clear();
        self.archive.clear();
        self.total_count = 0;
        self.archived_count = 0;
    }

    /// アーカイブから検索（投稿者別）
    pub fn search_by_author(&self, author: &str) -> Vec<&GuiChatMessage> {
        if !self.config.archive_search_enabled {
            return Vec::new();
        }

        self.archive
            .iter()
            .filter(|msg| msg.author == author)
            .collect()
    }

    /// アーカイブから検索（内容別）
    pub fn search_by_content(&self, keyword: &str) -> Vec<&GuiChatMessage> {
        if !self.config.archive_search_enabled {
            return Vec::new();
        }

        self.archive
            .iter()
            .filter(|msg| msg.content.contains(keyword))
            .collect()
    }
}

/// メッセージストリーム統計情報
#[derive(Debug, Clone)]
pub struct MessageStreamStats {
    /// 表示中メッセージ数
    pub display_count: usize,
    /// アーカイブ済みメッセージ数
    pub archived_count: usize,
    /// 総メッセージ数
    pub total_count: usize,
    /// 表示部分のメモリ使用量（バイト）
    pub display_memory_bytes: usize,
    /// アーカイブ部分のメモリ使用量（バイト）
    pub archive_memory_bytes: usize,
    /// 総メモリ使用量（バイト）
    pub total_memory_bytes: usize,
    /// 効果的な削減率（パーセント）
    pub effective_reduction_percent: u32,
}

impl MessageStreamStats {
    /// メモリ使用量をMB単位で取得
    pub fn memory_mb(&self) -> f64 {
        self.total_memory_bytes as f64 / 1024.0 / 1024.0
    }

    /// 表示メモリをMB単位で取得
    pub fn display_memory_mb(&self) -> f64 {
        self.display_memory_bytes as f64 / 1024.0 / 1024.0
    }

    /// アーカイブメモリをMB単位で取得
    pub fn archive_memory_mb(&self) -> f64 {
        self.archive_memory_bytes as f64 / 1024.0 / 1024.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message(id: usize) -> GuiChatMessage {
        GuiChatMessage {
            timestamp: format!("12:00:{:02}", id % 60),
            author: format!("User{}", id),
            content: format!("Test message {}", id),
            ..Default::default()
        }
    }

    #[test]
    fn test_basic_functionality() {
        let mut stream = MessageStream::with_fixed_limit(3);

        // メッセージ追加
        stream.push_message(create_test_message(1));
        stream.push_message(create_test_message(2));
        stream.push_message(create_test_message(3));

        assert_eq!(stream.display_count(), 3);
        assert_eq!(stream.total_count(), 3);
        assert_eq!(stream.archived_count(), 0);
    }

    #[test]
    fn test_archiving() {
        let mut stream = MessageStream::with_fixed_limit(2);

        // 容量超過でアーカイブが発生
        stream.push_message(create_test_message(1));
        stream.push_message(create_test_message(2));
        stream.push_message(create_test_message(3)); // これで1番目がアーカイブされる

        assert_eq!(stream.display_count(), 2);
        assert_eq!(stream.total_count(), 3);
        assert_eq!(stream.archived_count(), 1);

        // 表示中は2番目と3番目
        let display = stream.display_messages();
        assert_eq!(display[0].author, "User2");
        assert_eq!(display[1].author, "User3");
    }

    #[test]
    fn test_batch_add() {
        let mut stream = MessageStream::with_fixed_limit(2);

        let messages = vec![
            create_test_message(1),
            create_test_message(2),
            create_test_message(3),
        ];

        stream.push_messages(messages);

        assert_eq!(stream.display_count(), 2);
        assert_eq!(stream.total_count(), 3);
        assert_eq!(stream.archived_count(), 1);
    }

    #[test]
    fn test_search() {
        let mut stream = MessageStream::with_fixed_limit(1);

        // User1のメッセージがアーカイブされる
        stream.push_message(create_test_message(1));
        stream.push_message(create_test_message(2));

        let results = stream.search_by_author("User1");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].author, "User1");
    }

    #[test]
    fn test_stats() {
        let mut stream = MessageStream::with_fixed_limit(2);

        stream.push_message(create_test_message(1));
        stream.push_message(create_test_message(2));
        stream.push_message(create_test_message(3));

        let stats = stream.stats();
        assert_eq!(stats.display_count, 2);
        assert_eq!(stats.archived_count, 1);
        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.effective_reduction_percent, 33); // 1/3 = 33%
    }

    #[test]
    fn test_bidirectional_limit_changes() {
        let mut stream = MessageStream::with_fixed_limit(2);

        // 初期メッセージ追加
        stream.push_message(create_test_message(1));
        stream.push_message(create_test_message(2));
        stream.push_message(create_test_message(3)); // Message 1 がアーカイブされる

        assert_eq!(stream.display_count(), 2);
        assert_eq!(stream.archived_count(), 1);

        // 表示件数を50件に増加
        let new_config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(50),
            max_display_count: 50,
            ..Default::default()
        };
        stream.update_config(new_config);

        // アーカイブからメッセージが復帰
        assert_eq!(stream.display_count(), 3); // 2 + 1 (復帰)
        assert_eq!(stream.archived_count(), 0); // アーカイブがクリア

        // 表示メッセージの順序確認（時系列順）
        let display = stream.display_messages();
        assert_eq!(display[0].author, "User1"); // 復帰したメッセージ
        assert_eq!(display[1].author, "User2");
        assert_eq!(display[2].author, "User3");

        // 再度制限を減少
        let new_config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(1),
            max_display_count: 1,
            ..Default::default()
        };
        stream.update_config(new_config);

        // 最新以外がアーカイブ
        assert_eq!(stream.display_count(), 1);
        assert_eq!(stream.archived_count(), 2);

        let display = stream.display_messages();
        assert_eq!(display[0].author, "User3"); // 最新のメッセージのみ
    }

    #[test]
    fn test_unlimited_restore() {
        let mut stream = MessageStream::with_fixed_limit(1);

        // 大量メッセージでアーカイブを作成
        for i in 1..=10 {
            stream.push_message(create_test_message(i));
        }

        assert_eq!(stream.display_count(), 1);
        assert_eq!(stream.archived_count(), 9);

        // 無制限に変更
        let new_config = MessageStreamConfig {
            display_limit: DisplayLimit::Unlimited,
            max_display_count: usize::MAX,
            ..Default::default()
        };
        stream.update_config(new_config);

        // 全てのメッセージが復帰
        assert_eq!(stream.display_count(), 10);
        assert_eq!(stream.archived_count(), 0);

        // 順序確認
        let display = stream.display_messages();
        assert_eq!(display[0].author, "User1"); // 最古
        assert_eq!(display[9].author, "User10"); // 最新
    }

    #[test]
    fn test_partial_restore() {
        let mut stream = MessageStream::with_fixed_limit(1);

        // アーカイブを作成
        for i in 1..=5 {
            stream.push_message(create_test_message(i));
        }

        assert_eq!(stream.display_count(), 1);
        assert_eq!(stream.archived_count(), 4);

        // 部分的に復帰（3件制限）
        let new_config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(3),
            max_display_count: 3,
            ..Default::default()
        };
        stream.update_config(new_config);

        // 最新2件がアーカイブから復帰
        assert_eq!(stream.display_count(), 3);
        assert_eq!(stream.archived_count(), 2);

        let display = stream.display_messages();
        assert_eq!(display[0].author, "User3"); // アーカイブから復帰
        assert_eq!(display[1].author, "User4"); // アーカイブから復帰
        assert_eq!(display[2].author, "User5"); // 元々表示中
    }
}
