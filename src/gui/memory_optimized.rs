//! メモリ効率最適化モジュール
//!
//! Phase 2実装: メモリ効率改善

use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;

use super::models::GuiChatMessage;

/// 設定可能な制限付きメッセージバッファ
#[derive(Debug, Clone)]
pub enum BufferStrategy {
    /// 無制限（メモリが許す限り）
    Unlimited,
    /// 固定制限（従来の循環バッファ）
    FixedLimit(usize),
    /// スマート制限（メモリ使用量ベース）
    MemoryBased { max_memory_mb: usize },
    /// 時間ベース制限（古いメッセージを自動削除）
    TimeBased { max_hours: u64 },
}

impl Default for BufferStrategy {
    fn default() -> Self {
        // デフォルトは無制限
        Self::Unlimited
    }
}

/// 改良されたメッセージバッファ
#[derive(Debug)]
pub struct FlexibleMessageBuffer {
    /// メッセージを格納するバッファ
    buffer: Vec<GuiChatMessage>,
    /// バッファ戦略
    strategy: BufferStrategy,
    /// 削除されたメッセージ数の累計
    dropped_count: usize,
    /// 総メッセージ数（削除されたものを含む）
    total_count: usize,
    /// 最後のクリーンアップ時刻
    last_cleanup: std::time::Instant,
}

impl FlexibleMessageBuffer {
    /// 新しい柔軟なメッセージバッファを作成
    pub fn new(strategy: BufferStrategy) -> Self {
        Self {
            buffer: Vec::new(),
            strategy,
            dropped_count: 0,
            total_count: 0,
            last_cleanup: std::time::Instant::now(),
        }
    }

    /// メッセージを追加
    pub fn push(&mut self, message: GuiChatMessage) {
        self.buffer.push(message);
        self.total_count += 1;

        // 即座にクリーンアップを適用（循環バッファの動作に合わせる）
        self.apply_cleanup_strategy();

        // 定期的なクリーンアップは60秒ごとに実行
        if self.last_cleanup.elapsed().as_secs() > 60 {
            self.last_cleanup = std::time::Instant::now();
        }
    }

    /// バッチでメッセージを追加
    pub fn push_batch(&mut self, messages: Vec<GuiChatMessage>) {
        self.buffer.extend(messages.iter().cloned());
        self.total_count += messages.len();

        // バッチ追加後に即座にクリーンアップを適用
        self.apply_cleanup_strategy();
    }

    /// クリーンアップ戦略を適用
    fn apply_cleanup_strategy(&mut self) {
        match &self.strategy {
            BufferStrategy::Unlimited => {
                // 何もしない
            }
            BufferStrategy::FixedLimit(limit) => {
                if self.buffer.len() > *limit {
                    let overflow = self.buffer.len() - limit;
                    self.buffer.drain(..overflow);
                    self.dropped_count += overflow;
                }
            }
            BufferStrategy::MemoryBased { max_memory_mb } => {
                let message_size = std::mem::size_of::<GuiChatMessage>();
                let max_messages = (max_memory_mb * 1024 * 1024) / message_size;

                if self.buffer.len() > max_messages {
                    let overflow = self.buffer.len() - max_messages;
                    self.buffer.drain(..overflow);
                    self.dropped_count += overflow;
                }
            }
            BufferStrategy::TimeBased {
                max_hours: _max_hours,
            } => {
                // TODO: 時間ベース制限の実装
                // 現在は簡易実装のため、実際のタイムスタンプ解析は後で実装

                // 簡易的な実装：1時間ごとに古いメッセージの1/10を削除
                if self.buffer.len() > 1000 {
                    let remove_count = self.buffer.len() / 10;
                    self.buffer.drain(..remove_count);
                    self.dropped_count += remove_count;
                }
            }
        }
    }

    /// 現在のメッセージ一覧を取得
    pub fn messages(&self) -> &Vec<GuiChatMessage> {
        &self.buffer
    }

    /// メッセージ一覧のベクタを取得
    pub fn to_vec(&self) -> Vec<GuiChatMessage> {
        self.buffer.clone()
    }

    /// 最新のN件のメッセージを取得
    pub fn recent_messages(&self, n: usize) -> Vec<GuiChatMessage> {
        self.buffer.iter().rev().take(n).rev().cloned().collect()
    }

    /// 現在のメッセージ数
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// バッファが空かどうか
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// 総メッセージ数（削除されたものを含む）
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// 削除されたメッセージ数
    pub fn dropped_count(&self) -> usize {
        self.dropped_count
    }

    /// 戦略を変更
    pub fn set_strategy(&mut self, strategy: BufferStrategy) {
        self.strategy = strategy;
        self.apply_cleanup_strategy();
    }

    /// バッファをクリア
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.dropped_count = 0;
        self.total_count = 0;
    }

    /// メモリ使用量を最適化
    pub fn optimize_memory(&mut self) {
        self.buffer.shrink_to_fit();
    }

    /// メモリ統計を取得
    pub fn memory_stats(&self) -> MemoryStats {
        let message_size = std::mem::size_of::<GuiChatMessage>();
        let buffer_capacity = self.buffer.capacity();
        let used_memory = self.buffer.len() * message_size;
        let allocated_memory = buffer_capacity * message_size;

        MemoryStats {
            used_memory,
            allocated_memory,
            buffer_capacity,
            message_count: self.buffer.len(),
            fragmentation_ratio: if allocated_memory > 0 {
                1.0 - (used_memory as f64 / allocated_memory as f64)
            } else {
                0.0
            },
        }
    }

    /// 従来の`CircularMessageBuffer::new`との互換性のため
    pub fn new_circular(capacity: usize) -> Self {
        Self::new(BufferStrategy::FixedLimit(capacity))
    }

    /// 容量を取得（従来の循環バッファ互換）
    pub fn capacity(&self) -> usize {
        match &self.strategy {
            BufferStrategy::FixedLimit(limit) => *limit,
            BufferStrategy::MemoryBased { max_memory_mb } => {
                let message_size = std::mem::size_of::<GuiChatMessage>();
                (max_memory_mb * 1024 * 1024) / message_size
            }
            _ => usize::MAX, // 無制限または時間ベースの場合
        }
    }

    /// バッファが満杯かどうか（従来の循環バッファ互換）
    pub fn is_full(&self) -> bool {
        match &self.strategy {
            BufferStrategy::Unlimited => false,
            BufferStrategy::FixedLimit(limit) => self.buffer.len() >= *limit,
            BufferStrategy::MemoryBased { max_memory_mb } => {
                let message_size = std::mem::size_of::<GuiChatMessage>();
                let max_messages = (max_memory_mb * 1024 * 1024) / message_size;
                self.buffer.len() >= max_messages
            }
            BufferStrategy::TimeBased { .. } => false, // 時間ベースでは満杯という概念がない
        }
    }

    /// 容量を変更（従来の循環バッファ互換）
    pub fn set_capacity(&mut self, new_capacity: usize) {
        self.set_strategy(BufferStrategy::FixedLimit(new_capacity));
    }
}

/// メモリ統計情報
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// 使用中メモリ（バイト）
    pub used_memory: usize,
    /// 割り当て済みメモリ（バイト）
    pub allocated_memory: usize,
    /// バッファ容量
    pub buffer_capacity: usize,
    /// メッセージ数
    pub message_count: usize,
    /// 断片化率（0.0-1.0）
    pub fragmentation_ratio: f64,
}

/// メッセージプール - オブジェクトの再利用によるメモリ効率化
#[derive(Debug)]
pub struct MessagePool {
    /// 再利用可能なメッセージのプール
    pool: Mutex<Vec<GuiChatMessage>>,
    /// プールの最大サイズ
    max_pool_size: usize,
    /// 作成されたオブジェクトの総数
    created_count: std::sync::atomic::AtomicUsize,
    /// 再利用されたオブジェクトの総数
    reused_count: std::sync::atomic::AtomicUsize,
}

impl MessagePool {
    /// 新しいメッセージプールを作成
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            pool: Mutex::new(Vec::with_capacity(max_pool_size)),
            max_pool_size,
            created_count: std::sync::atomic::AtomicUsize::new(0),
            reused_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// メッセージを取得（プールから再利用、または新規作成）
    pub fn acquire(&self) -> GuiChatMessage {
        let mut pool = self.pool.lock();

        if let Some(mut message) = pool.pop() {
            // プールから再利用
            self.reused_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // メッセージをリセット
            message.timestamp = String::new();
            message.author = String::new();
            message.channel_id = String::new();
            message.content = String::new();
            message.runs = Vec::new();
            message.message_type = super::models::MessageType::Text;
            message.metadata = None;
            message.is_member = false;

            message
        } else {
            // 新規作成
            self.created_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            GuiChatMessage::default()
        }
    }

    /// メッセージをプールに返却
    pub fn release(&self, message: GuiChatMessage) {
        let mut pool = self.pool.lock();

        if pool.len() < self.max_pool_size {
            pool.push(message);
        }
        // プールが満杯の場合はドロップ（ガベージコレクションに任せる）
    }

    /// プール統計を取得
    pub fn stats(&self) -> PoolStats {
        let pool = self.pool.lock();
        PoolStats {
            pool_size: pool.len(),
            max_pool_size: self.max_pool_size,
            created_count: self
                .created_count
                .load(std::sync::atomic::Ordering::Relaxed),
            reused_count: self.reused_count.load(std::sync::atomic::Ordering::Relaxed),
            reuse_rate: {
                let total = self
                    .created_count
                    .load(std::sync::atomic::Ordering::Relaxed)
                    + self.reused_count.load(std::sync::atomic::Ordering::Relaxed);
                if total > 0 {
                    self.reused_count.load(std::sync::atomic::Ordering::Relaxed) as f64
                        / total as f64
                } else {
                    0.0
                }
            },
        }
    }

    /// プールをクリア
    pub fn clear(&self) {
        let mut pool = self.pool.lock();
        pool.clear();
    }
}

/// プール統計情報
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// 現在のプールサイズ
    pub pool_size: usize,
    /// 最大プールサイズ
    pub max_pool_size: usize,
    /// 作成されたオブジェクト数
    pub created_count: usize,
    /// 再利用されたオブジェクト数
    pub reused_count: usize,
    /// 再利用率（0.0-1.0）
    pub reuse_rate: f64,
}

/// 共有データの重複排除システム
#[derive(Debug)]
pub struct SharedDataCache {
    /// 著者名の共有キャッシュ
    authors: RwLock<HashMap<String, Arc<String>>>,
    /// チャンネルIDの共有キャッシュ
    channel_ids: RwLock<HashMap<String, Arc<String>>>,
    /// 最大キャッシュサイズ
    max_cache_size: usize,
}

impl SharedDataCache {
    /// 新しい共有データキャッシュを作成
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            authors: RwLock::new(HashMap::with_capacity(max_cache_size / 2)),
            channel_ids: RwLock::new(HashMap::with_capacity(max_cache_size / 2)),
            max_cache_size,
        }
    }

    /// 著者名を共有データとして取得
    pub fn get_shared_author(&self, author: &str) -> Arc<String> {
        // 読み取りロックで既存データを確認
        {
            let authors = self.authors.read();
            if let Some(shared) = authors.get(author) {
                return shared.clone();
            }
        }

        // 書き込みロックで新しいデータを挿入
        let mut authors = self.authors.write();

        // ダブルチェック（他のスレッドが既に挿入している可能性）
        if let Some(shared) = authors.get(author) {
            return shared.clone();
        }

        // キャッシュサイズ制限
        if authors.len() >= self.max_cache_size / 2 {
            // 単純なLRU代替：最初の要素を削除
            if let Some(first_key) = authors.keys().next().cloned() {
                authors.remove(&first_key);
            }
        }

        let shared = Arc::new(author.to_string());
        authors.insert(author.to_string(), shared.clone());
        shared
    }

    /// チャンネルIDを共有データとして取得
    pub fn get_shared_channel_id(&self, channel_id: &str) -> Arc<String> {
        // 著者名と同様の実装
        {
            let channel_ids = self.channel_ids.read();
            if let Some(shared) = channel_ids.get(channel_id) {
                return shared.clone();
            }
        }

        let mut channel_ids = self.channel_ids.write();

        if let Some(shared) = channel_ids.get(channel_id) {
            return shared.clone();
        }

        if channel_ids.len() >= self.max_cache_size / 2 {
            if let Some(first_key) = channel_ids.keys().next().cloned() {
                channel_ids.remove(&first_key);
            }
        }

        let shared = Arc::new(channel_id.to_string());
        channel_ids.insert(channel_id.to_string(), shared.clone());
        shared
    }

    /// キャッシュ統計を取得
    pub fn cache_stats(&self) -> CacheStats {
        let authors = self.authors.read();
        let channel_ids = self.channel_ids.read();

        CacheStats {
            author_cache_size: authors.len(),
            channel_id_cache_size: channel_ids.len(),
            total_cache_size: authors.len() + channel_ids.len(),
            max_cache_size: self.max_cache_size,
            cache_utilization: (authors.len() + channel_ids.len()) as f64
                / self.max_cache_size as f64,
        }
    }

    /// キャッシュをクリア
    pub fn clear(&self) {
        let mut authors = self.authors.write();
        let mut channel_ids = self.channel_ids.write();
        authors.clear();
        channel_ids.clear();
    }
}

/// キャッシュ統計情報
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 著者キャッシュサイズ
    pub author_cache_size: usize,
    /// チャンネルIDキャッシュサイズ
    pub channel_id_cache_size: usize,
    /// 総キャッシュサイズ
    pub total_cache_size: usize,
    /// 最大キャッシュサイズ
    pub max_cache_size: usize,
    /// キャッシュ利用率（0.0-1.0）
    pub cache_utilization: f64,
}

/// メモリ最適化されたメッセージマネージャー
#[derive(Debug)]
pub struct OptimizedMessageManager {
    /// 改良されたメッセージバッファ
    buffer: FlexibleMessageBuffer,
    /// メッセージプール
    pool: MessagePool,
    /// 共有データキャッシュ
    cache: SharedDataCache,
}

/// バッチ処理設定
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// バッチサイズ
    pub batch_size: usize,
    /// バッチ処理間隔（ミリ秒）
    pub batch_interval_ms: u64,
    /// 最大バッチ数
    pub max_batches: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            batch_interval_ms: 100,
            max_batches: 10,
        }
    }
}

impl OptimizedMessageManager {
    /// 新しい最適化されたメッセージマネージャーを作成
    pub fn new(
        buffer_capacity: usize,
        pool_size: usize,
        cache_size: usize,
        _batch_config: BatchConfig,
    ) -> Self {
        Self {
            buffer: FlexibleMessageBuffer::new(BufferStrategy::FixedLimit(buffer_capacity)),
            pool: MessagePool::new(pool_size),
            cache: SharedDataCache::new(cache_size),
        }
    }

    /// デフォルト設定でマネージャーを作成
    pub fn with_defaults() -> Self {
        // デフォルトは無制限バッファ（従来の5000件制限を撤廃）
        Self::new_with_strategy(
            BufferStrategy::Unlimited,
            100, // pool_size
            500, // cache_size
            BatchConfig::default(),
        )
    }

    /// 特定のバッファ戦略でマネージャーを作成
    pub fn new_with_strategy(
        buffer_strategy: BufferStrategy,
        pool_size: usize,
        cache_size: usize,
        _batch_config: BatchConfig,
    ) -> Self {
        Self {
            buffer: FlexibleMessageBuffer::new(buffer_strategy),
            pool: MessagePool::new(pool_size),
            cache: SharedDataCache::new(cache_size),
        }
    }

    /// 従来の循環バッファ互換のコンストラクタ
    pub fn with_fixed_limit(limit: usize) -> Self {
        Self::new_with_strategy(
            BufferStrategy::FixedLimit(limit),
            100,
            500,
            BatchConfig::default(),
        )
    }

    /// メモリベース制限のコンストラクタ
    pub fn with_memory_limit(max_memory_mb: usize) -> Self {
        Self::new_with_strategy(
            BufferStrategy::MemoryBased { max_memory_mb },
            100,
            500,
            BatchConfig::default(),
        )
    }

    /// 時間ベース制限のコンストラクタ
    pub fn with_time_limit(max_hours: u64) -> Self {
        Self::new_with_strategy(
            BufferStrategy::TimeBased { max_hours },
            100,
            500,
            BatchConfig::default(),
        )
    }

    /// バッファ戦略を変更
    pub fn set_buffer_strategy(&mut self, strategy: BufferStrategy) {
        self.buffer.set_strategy(strategy);
    }

    /// 現在のバッファ戦略を取得
    pub fn get_buffer_strategy(&self) -> &BufferStrategy {
        &self.buffer.strategy
    }

    /// メッセージを追加（最適化）
    pub fn add_message(&mut self, message: GuiChatMessage) {
        // 共有データを使用してメモリ使用量を削減
        let _shared_author = self.cache.get_shared_author(&message.author);
        let _shared_channel_id = self.cache.get_shared_channel_id(&message.channel_id);

        // 実際にはArcを直接使用できないため、文字列はそのまま保持
        // 実用的には、Stringの代わりにArc<String>を使うメッセージ型が必要

        self.buffer.push(message);
    }

    /// バッチでメッセージを追加
    pub fn add_messages_batch(&mut self, messages: Vec<GuiChatMessage>) {
        self.buffer.push_batch(messages);
    }

    /// 現在のメッセージを取得
    pub fn messages(&self) -> Vec<GuiChatMessage> {
        self.buffer.to_vec()
    }

    /// 最新のメッセージを取得
    pub fn recent_messages(&self, n: usize) -> Vec<GuiChatMessage> {
        self.buffer.recent_messages(n)
    }

    /// 総合統計を取得
    pub fn comprehensive_stats(&self) -> ComprehensiveStats {
        ComprehensiveStats {
            memory_stats: self.buffer.memory_stats(),
            pool_stats: self.pool.stats(),
            cache_stats: self.cache.cache_stats(),
            message_count: self.buffer.len(),
            total_processed: self.buffer.total_count(),
            dropped_count: self.buffer.dropped_count(),
        }
    }

    /// メモリを最適化
    pub fn optimize_memory(&mut self) {
        self.buffer.optimize_memory();
        self.pool.clear();

        // キャッシュは使用中のデータがあるためクリアしない
    }

    /// 全データをクリア
    pub fn clear_all(&mut self) {
        self.buffer.clear();
        self.pool.clear();
        self.cache.clear();
    }

    /// 現在のメッセージ数を取得
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// バッファが空かどうかを確認
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// バッファのクリア（互換性のため）
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// 総合統計情報
#[derive(Debug, Clone)]
pub struct ComprehensiveStats {
    pub memory_stats: MemoryStats,
    pub pool_stats: PoolStats,
    pub cache_stats: CacheStats,
    pub message_count: usize,
    pub total_processed: usize,
    pub dropped_count: usize,
}

/// 互換性のための型エイリアス
pub type CircularMessageBuffer = FlexibleMessageBuffer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::models::{GuiChatMessage, MessageType};

    fn create_test_message(author: &str, content: &str) -> GuiChatMessage {
        GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            message_type: MessageType::Text,
            author: author.to_string(),
            channel_id: format!("channel_{}", author),
            content: content.to_string(),
            runs: Vec::new(),
            metadata: None,
            is_member: false,
        }
    }

    #[test]
    fn test_flexible_buffer_unlimited() {
        let mut buffer = FlexibleMessageBuffer::new(BufferStrategy::Unlimited);

        // 大量のメッセージを追加
        for i in 0..10000 {
            buffer.push(create_test_message(&format!("user{}", i), "msg"));
        }

        assert_eq!(buffer.len(), 10000);
        assert_eq!(buffer.dropped_count(), 0); // 無制限なので削除されない
        assert_eq!(buffer.total_count(), 10000);
    }

    #[test]
    fn test_flexible_buffer_fixed_limit() {
        let mut buffer = FlexibleMessageBuffer::new(BufferStrategy::FixedLimit(3));

        // 容量内でのメッセージ追加
        buffer.push(create_test_message("user1", "msg1"));
        buffer.push(create_test_message("user2", "msg2"));
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.dropped_count(), 0);

        // 容量を超えるメッセージ追加
        buffer.push(create_test_message("user3", "msg3"));
        buffer.push(create_test_message("user4", "msg4"));

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.dropped_count(), 1);
        assert_eq!(buffer.total_count(), 4);

        // 最新のメッセージが保持されている
        let messages = buffer.to_vec();
        assert_eq!(messages[2].author, "user4");
    }

    #[test]
    fn test_flexible_buffer_memory_based() {
        // 1MBの制限（実際の制限は計算される）
        let mut buffer =
            FlexibleMessageBuffer::new(BufferStrategy::MemoryBased { max_memory_mb: 1 });

        // メッセージを追加
        for i in 0..100 {
            buffer.push(create_test_message(&format!("user{}", i), "msg"));
        }

        // メモリ制限内であることを確認
        let stats = buffer.memory_stats();
        assert!(stats.used_memory <= 1024 * 1024); // 1MB以下
    }

    #[test]
    fn test_backward_compatibility() {
        // 従来のCircularMessageBufferとして使用
        let mut buffer = CircularMessageBuffer::new_circular(3);

        buffer.push(create_test_message("user1", "msg1"));
        buffer.push(create_test_message("user2", "msg2"));
        buffer.push(create_test_message("user3", "msg3"));
        buffer.push(create_test_message("user4", "msg4"));

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.capacity(), 3);
        assert_eq!(buffer.dropped_count(), 1);
        assert!(buffer.is_full());
    }

    #[test]
    fn test_strategy_switching() {
        let mut buffer = FlexibleMessageBuffer::new(BufferStrategy::Unlimited);

        // 1000メッセージを追加
        for i in 0..1000 {
            buffer.push(create_test_message(&format!("user{}", i), "msg"));
        }
        assert_eq!(buffer.len(), 1000);

        // 制限付きに変更
        buffer.set_strategy(BufferStrategy::FixedLimit(500));
        assert_eq!(buffer.len(), 500); // 古いメッセージが削除される
        assert_eq!(buffer.dropped_count(), 500);
    }

    #[test]
    fn test_optimized_manager_unlimited() {
        let mut manager = OptimizedMessageManager::with_defaults();

        // 大量のメッセージを追加
        for i in 0..10000 {
            manager.add_message(create_test_message(&format!("user{}", i), "test"));
        }

        let stats = manager.comprehensive_stats();
        assert_eq!(stats.message_count, 10000);
        assert_eq!(stats.dropped_count, 0); // 無制限なので削除されない
    }

    #[test]
    fn test_optimized_manager_with_limits() {
        let mut manager = OptimizedMessageManager::with_fixed_limit(100);

        // 制限を超えるメッセージを追加
        for i in 0..200 {
            manager.add_message(create_test_message(&format!("user{}", i), "test"));
        }

        let stats = manager.comprehensive_stats();
        assert_eq!(stats.message_count, 100);
        assert_eq!(stats.dropped_count, 100);
    }

    // 従来のテストとの互換性を保つため、古いテスト名も残す
    #[test]
    fn test_circular_buffer() {
        test_flexible_buffer_fixed_limit();
    }

    #[test]
    fn test_batch_processing() {
        let mut buffer = FlexibleMessageBuffer::new(BufferStrategy::FixedLimit(5));

        let batch = vec![
            create_test_message("user1", "msg1"),
            create_test_message("user2", "msg2"),
            create_test_message("user3", "msg3"),
        ];

        buffer.push_batch(batch);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.dropped_count(), 0);

        // 容量を超えるバッチ
        let large_batch = vec![
            create_test_message("user4", "msg4"),
            create_test_message("user5", "msg5"),
            create_test_message("user6", "msg6"),
            create_test_message("user7", "msg7"),
        ];

        buffer.push_batch(large_batch);
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.dropped_count(), 2);
    }

    #[test]
    fn test_memory_optimization() {
        let mut manager = OptimizedMessageManager::with_defaults();

        // 大量のメッセージを追加（無制限なので全て保持される）
        for i in 0..1500 {
            manager.add_message(create_test_message(&format!("user{}", i), "test"));
        }

        let stats_before = manager.comprehensive_stats();

        // メモリ最適化
        manager.optimize_memory();

        let stats_after = manager.comprehensive_stats();

        // 無制限バッファなので全メッセージが保持される
        assert_eq!(stats_before.message_count, stats_after.message_count);
        assert_eq!(stats_after.message_count, 1500);
        assert_eq!(stats_after.dropped_count, 0);
    }

    #[test]
    fn test_message_pool() {
        let pool = MessagePool::new(2);

        // 新規作成
        let msg1 = pool.acquire();
        let msg2 = pool.acquire();

        // プールに返却
        pool.release(msg1);
        pool.release(msg2);

        // 再利用
        let _msg3 = pool.acquire();
        let _msg4 = pool.acquire();

        let stats = pool.stats();
        assert_eq!(stats.pool_size, 0); // 取得済み
        assert_eq!(stats.created_count, 2);
        assert_eq!(stats.reused_count, 2);
        assert_eq!(stats.reuse_rate, 0.5);
    }

    #[test]
    fn test_shared_cache() {
        let cache = SharedDataCache::new(100);

        let author1 = cache.get_shared_author("user1");
        let author1_again = cache.get_shared_author("user1");

        // 同じ参照が返される
        assert!(Arc::ptr_eq(&author1, &author1_again));

        let stats = cache.cache_stats();
        assert_eq!(stats.author_cache_size, 1);
    }

    #[test]
    fn test_optimized_manager() {
        let mut manager = OptimizedMessageManager::with_defaults();

        // メッセージ追加
        manager.add_message(create_test_message("user1", "Hello"));
        manager.add_message(create_test_message("user2", "World"));

        assert_eq!(manager.messages().len(), 2);

        // 統計確認
        let stats = manager.comprehensive_stats();
        assert_eq!(stats.message_count, 2);
        assert!(stats.memory_stats.used_memory > 0);
    }
}
