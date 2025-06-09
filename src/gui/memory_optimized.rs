//! メモリ効率最適化モジュール
//! 
//! Phase 2実装: メモリ効率改善

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::{RwLock, Mutex};

use super::models::GuiChatMessage;

/// 循環バッファによる効率的なメッセージ管理
#[derive(Debug)]
pub struct CircularMessageBuffer {
    /// メッセージを格納する循環バッファ
    buffer: VecDeque<GuiChatMessage>,
    /// 最大容量
    capacity: usize,
    /// 削除されたメッセージ数の累計
    dropped_count: usize,
    /// 総メッセージ数（削除されたものを含む）
    total_count: usize,
}

impl CircularMessageBuffer {
    /// 新しい循環バッファを作成
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            dropped_count: 0,
            total_count: 0,
        }
    }

    /// メッセージを追加（容量を超えた場合は古いメッセージを削除）
    pub fn push(&mut self, message: GuiChatMessage) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
            self.dropped_count += 1;
        }
        
        self.buffer.push_back(message);
        self.total_count += 1;
    }

    /// 複数のメッセージを効率的に追加
    pub fn push_batch(&mut self, messages: Vec<GuiChatMessage>) {
        let batch_size = messages.len();
        
        // バッチサイズが容量を超える場合は最新のメッセージのみを保持
        if batch_size >= self.capacity {
            self.dropped_count += self.buffer.len() + (batch_size - self.capacity);
            self.buffer.clear();
            
            // 最新のメッセージを容量分だけ保持
            let start_index = batch_size - self.capacity;
            for message in messages.into_iter().skip(start_index) {
                self.buffer.push_back(message);
            }
        } else {
            // 通常のバッチ処理
            let overflow = (self.buffer.len() + batch_size).saturating_sub(self.capacity);
            if overflow > 0 {
                self.buffer.drain(..overflow);
                self.dropped_count += overflow;
            }
            
            for message in messages {
                self.buffer.push_back(message);
            }
        }
        
        self.total_count += batch_size;
    }

    /// 現在のメッセージ一覧を取得
    pub fn messages(&self) -> &VecDeque<GuiChatMessage> {
        &self.buffer
    }

    /// メッセージ一覧のベクタを取得（互換性のため）
    pub fn to_vec(&self) -> Vec<GuiChatMessage> {
        self.buffer.iter().cloned().collect()
    }

    /// 最新のN件のメッセージを取得
    pub fn recent_messages(&self, n: usize) -> Vec<GuiChatMessage> {
        self.buffer
            .iter()
            .rev()
            .take(n)
            .rev()
            .cloned()
            .collect()
    }

    /// 現在のメッセージ数
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// バッファが空かどうか
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// バッファが満杯かどうか
    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.capacity
    }

    /// 総メッセージ数（削除されたものを含む）
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// 削除されたメッセージ数
    pub fn dropped_count(&self) -> usize {
        self.dropped_count
    }

    /// 容量を取得
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 容量を変更（既存データは保持）
    pub fn set_capacity(&mut self, new_capacity: usize) {
        if new_capacity < self.buffer.len() {
            let overflow = self.buffer.len() - new_capacity;
            self.buffer.drain(..overflow);
            self.dropped_count += overflow;
        }
        
        self.capacity = new_capacity;
        self.buffer.shrink_to_fit();
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
            self.reused_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            // メッセージをリセット
            message.timestamp = String::new();
            message.author = String::new();
            message.channel_id = String::new();
            message.content = String::new();
            message.message_type = super::models::MessageType::Text;
            message.metadata = None;
            message.is_member = false;
            
            message
        } else {
            // 新規作成
            self.created_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            created_count: self.created_count.load(std::sync::atomic::Ordering::Relaxed),
            reused_count: self.reused_count.load(std::sync::atomic::Ordering::Relaxed),
            reuse_rate: {
                let total = self.created_count.load(std::sync::atomic::Ordering::Relaxed) + 
                           self.reused_count.load(std::sync::atomic::Ordering::Relaxed);
                if total > 0 {
                    self.reused_count.load(std::sync::atomic::Ordering::Relaxed) as f64 / total as f64
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
            cache_utilization: (authors.len() + channel_ids.len()) as f64 / self.max_cache_size as f64,
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
    /// 循環バッファ
    buffer: CircularMessageBuffer,
    /// メッセージプール
    pool: MessagePool,
    /// 共有データキャッシュ
    cache: SharedDataCache,
    /// バッチ処理設定
    batch_config: BatchConfig,
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
        batch_config: BatchConfig,
    ) -> Self {
        Self {
            buffer: CircularMessageBuffer::new(buffer_capacity),
            pool: MessagePool::new(pool_size),
            cache: SharedDataCache::new(cache_size),
            batch_config,
        }
    }

    /// デフォルト設定でマネージャーを作成
    pub fn with_defaults() -> Self {
        Self::new(1000, 100, 500, BatchConfig::default())
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
            metadata: None,
            is_member: false,
        }
    }

    #[test]
    fn test_circular_buffer() {
        let mut buffer = CircularMessageBuffer::new(3);
        
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
        
        // 最初のメッセージは削除されている
        let messages = buffer.to_vec();
        assert_eq!(messages[0].author, "user2");
        assert_eq!(messages[2].author, "user4");
    }

    #[test]
    fn test_batch_processing() {
        let mut buffer = CircularMessageBuffer::new(5);
        
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

    #[test]
    fn test_memory_optimization() {
        let mut manager = OptimizedMessageManager::with_defaults();
        
        // 大量のメッセージを追加
        for i in 0..1500 {
            manager.add_message(create_test_message(&format!("user{}", i), "test"));
        }
        
        let stats_before = manager.comprehensive_stats();
        
        // メモリ最適化
        manager.optimize_memory();
        
        let stats_after = manager.comprehensive_stats();
        
        // メッセージ数は変わらないが、メモリ効率が改善される可能性
        assert_eq!(stats_before.message_count, stats_after.message_count);
        assert_eq!(stats_after.message_count, 1000); // 容量制限
        assert_eq!(stats_after.dropped_count, 500); // 500個削除
    }
}