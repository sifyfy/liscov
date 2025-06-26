//! Signal依存関係分析・最適化モジュール (Phase 4.1)
//!
//! Signal使用状況の分析と最適化を提供：
//! - Signal依存関係グラフの作成
//! - 重複Signal検出・統合
//! - Signal更新頻度監視
//! - 最適化推奨事項生成
//! - Phase 4.2: Batch更新機能

use std::collections::HashMap;
use std::time::Instant;

/// Signal識別子
pub type SignalId = String;

/// Phase 4.2: Batch更新システム
use std::collections::VecDeque;

/// Batch更新アイテム
#[derive(Debug, Clone)]
pub struct BatchUpdateItem {
    pub signal_id: SignalId,
    pub timestamp: Instant,
    pub update_type: BatchUpdateType,
}

/// Batch更新の種類
#[derive(Debug, Clone, PartialEq)]
pub enum BatchUpdateType {
    /// 通常の更新
    Normal,
    /// 高優先度更新（UI応答性重要）
    HighPriority,
    /// 低優先度更新（バックグラウンド処理）
    LowPriority,
    /// DOM操作伴う更新
    DomUpdate,
}

/// Batch更新管理
#[derive(Debug)]
pub struct BatchUpdateManager {
    /// 更新キュー
    queue: VecDeque<BatchUpdateItem>,
    /// 処理中フラグ
    processing: bool,
    /// 統計情報
    stats: BatchStats,
}

/// Batch統計情報
#[derive(Debug, Clone)]
pub struct BatchStats {
    pub total_batched: u64,
    pub high_priority_count: u64,
    pub dom_update_count: u64,
    pub average_batch_size: f32,
    pub last_batch_time: Option<Instant>,
}

impl BatchUpdateManager {
    /// 新しいBatch更新管理を作成
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            processing: false,
            stats: BatchStats {
                total_batched: 0,
                high_priority_count: 0,
                dom_update_count: 0,
                average_batch_size: 0.0,
                last_batch_time: None,
            },
        }
    }

    /// 更新をキューに追加
    pub fn queue_update(&mut self, signal_id: SignalId, update_type: BatchUpdateType) {
        let item = BatchUpdateItem {
            signal_id: signal_id.clone(),
            timestamp: Instant::now(),
            update_type: update_type.clone(),
        };

        // 優先度に基づいてキューに挿入
        match item.update_type {
            BatchUpdateType::HighPriority => {
                self.queue.push_front(item);
                self.stats.high_priority_count += 1;
            }
            BatchUpdateType::DomUpdate => {
                // DOM更新は特別な処理順序
                let insert_pos = self
                    .queue
                    .iter()
                    .position(|existing| {
                        !matches!(existing.update_type, BatchUpdateType::HighPriority)
                    })
                    .unwrap_or(self.queue.len());
                self.queue.insert(insert_pos, item);
                self.stats.dom_update_count += 1;
            }
            _ => {
                self.queue.push_back(item);
            }
        }

        self.stats.total_batched += 1;

        let queue_len = self.queue.len();

        tracing::debug!(
            "📦 [BATCH] Queued {:?} update for {} (queue size: {})",
            update_type,
            signal_id,
            queue_len
        );
    }

    /// Batch処理を実行 - タイムアウト保護付き
    pub async fn process_batch(&mut self) -> Result<usize, String> {
        if self.processing || self.queue.is_empty() {
            return Ok(0);
        }

        self.processing = true;
        let batch_start = Instant::now();
        let batch_size = self.queue.len();

        tracing::info!("🚀 [BATCH] Processing batch of {} updates", batch_size);

        // 100msタイムアウト保護
        let processed = match tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            self.process_with_animation_frame(),
        )
        .await
        {
            Ok(result) => result?,
            Err(_) => {
                tracing::warn!(
                    "⚠️ [BATCH] Processing timeout (>100ms), processed some items and stopping. Queue size: {}",
                    self.queue.len()
                );
                // タイムアウト時は残りのキューをクリアしてデッドロック防止
                let remaining = self.queue.len();
                self.queue.clear();
                batch_size - remaining
            }
        };

        // 統計更新
        self.stats.average_batch_size = (self.stats.average_batch_size + batch_size as f32) / 2.0;
        self.stats.last_batch_time = Some(batch_start);

        self.processing = false;

        tracing::info!(
            "✅ [BATCH] Processed {} updates in {:.2}ms",
            processed,
            batch_start.elapsed().as_secs_f32() * 1000.0
        );

        Ok(processed)
    }

    /// requestAnimationFrameベースの処理
    async fn process_with_animation_frame(&mut self) -> Result<usize, String> {
        let mut processed = 0;
        let batch_size = self.queue.len();

        while !self.queue.is_empty() {
            // フレーム単位でprocessing
            let frame_items = self.collect_frame_items();

            if frame_items.is_empty() {
                break;
            }

            // フレーム処理の実行
            self.execute_frame_updates(&frame_items).await?;
            processed += frame_items.len();

            // 次のフレームまで待機
            if processed < batch_size {
                self.wait_for_next_frame().await;
            }
        }

        Ok(processed)
    }

    /// フレーム単位のアイテム収集
    fn collect_frame_items(&mut self) -> Vec<BatchUpdateItem> {
        let max_per_frame = 5; // フレーム辺りの最大処理数
        let mut frame_items = Vec::new();

        for _ in 0..max_per_frame.min(self.queue.len()) {
            if let Some(item) = self.queue.pop_front() {
                frame_items.push(item);
            }
        }

        frame_items
    }

    /// フレーム更新の実行
    async fn execute_frame_updates(&self, items: &[BatchUpdateItem]) -> Result<(), String> {
        // DOM更新とSignal更新を分離
        let mut dom_updates = Vec::new();
        let mut signal_updates = Vec::new();

        for item in items {
            match item.update_type {
                BatchUpdateType::DomUpdate => dom_updates.push(item),
                _ => signal_updates.push(item),
            }
        }

        // DOM更新を先に実行
        if !dom_updates.is_empty() {
            self.execute_dom_updates(&dom_updates).await?;
        }

        // Signal更新を後に実行
        if !signal_updates.is_empty() {
            self.execute_signal_updates(&signal_updates).await?;
        }

        Ok(())
    }

    /// DOM更新の実行
    async fn execute_dom_updates(&self, items: &[&BatchUpdateItem]) -> Result<(), String> {
        tracing::debug!("🎨 [BATCH] Executing {} DOM updates", items.len());

        for item in items {
            // DOM操作のbatch処理
            match item.signal_id.as_str() {
                "chat_scroll" => {
                    // スクロール処理のbatch化
                    let _ = dioxus::document::eval(
                        r#"
                        if (!window.liscovBatchScrollPending) {
                            window.liscovBatchScrollPending = true;
                            requestAnimationFrame(() => {
                                const container = document.getElementById('liscov-message-list');
                                if (container) {
                                    container.scrollTop = container.scrollHeight;
                                }
                                window.liscovBatchScrollPending = false;
                            });
                        }
                    "#,
                    )
                    .await;
                }
                "highlight_update" => {
                    // ハイライト処理のbatch化 - 強化版（エラーハンドリング・タイムアウト付き）
                    let _ = dioxus::document::eval(r#"
                        if (!window.liscovBatchHighlightPending) {
                            window.liscovBatchHighlightPending = true;
                            
                            // タイムアウト保護（100ms以内に完了）
                            const timeout = setTimeout(() => {
                                console.warn('🚨 [BATCH] Highlight update timeout, resetting flag');
                                window.liscovBatchHighlightPending = false;
                            }, 100);
                            
                            requestAnimationFrame(() => {
                                try {
                                    // ハイライト処理をbatch実行
                                    const highlighted = document.querySelectorAll('.liscov-highlight-animation');
                                    if (highlighted.length > 0) {
                                        highlighted.forEach(el => {
                                            el.style.animation = 'highlight-pulse 2s ease-in-out';
                                        });
                                    }
                                } catch (error) {
                                    console.error('🚨 [BATCH] Highlight update error:', error);
                                } finally {
                                    clearTimeout(timeout);
                                    window.liscovBatchHighlightPending = false;
                                }
                            });
                        }
                    "#).await;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Signal更新の実行
    async fn execute_signal_updates(&self, items: &[&BatchUpdateItem]) -> Result<(), String> {
        tracing::debug!("📊 [BATCH] Executing {} Signal updates", items.len());

        // Signal更新はグループ化してメモリ効率を向上
        let mut signal_groups: HashMap<String, Vec<&BatchUpdateItem>> = HashMap::new();

        for item in items {
            signal_groups
                .entry(item.signal_id.clone())
                .or_insert_with(Vec::new)
                .push(item);
        }

        // グループ毎に処理
        for (signal_id, group_items) in signal_groups {
            tracing::debug!(
                "🔧 [BATCH] Processing {} updates for signal: {}",
                group_items.len(),
                signal_id
            );

            // 最新の更新のみ適用（重複削除）
            if let Some(latest_item) = group_items.last() {
                // 実際のSignal更新処理は呼び出し側で実装
                tracing::debug!("✅ [BATCH] Applied update for: {}", latest_item.signal_id);
            }
        }

        Ok(())
    }

    /// 次のフレームまで待機
    async fn wait_for_next_frame(&self) {
        // 16ms ≈ 60fps
        tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
    }

    /// キューサイズを取得
    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> &BatchStats {
        &self.stats
    }
}

impl Default for BatchUpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルBatch更新管理
static GLOBAL_BATCH_MANAGER: OnceLock<Arc<Mutex<BatchUpdateManager>>> = OnceLock::new();

/// グローバルBatch管理を取得
pub fn get_batch_manager() -> Arc<Mutex<BatchUpdateManager>> {
    GLOBAL_BATCH_MANAGER
        .get_or_init(|| {
            tracing::info!("📦 [BATCH] Creating global batch update manager");
            Arc::new(Mutex::new(BatchUpdateManager::new()))
        })
        .clone()
}

/// Phase 4.2: Batch更新便利関数
pub fn queue_batch_update(signal_id: &str, update_type: BatchUpdateType) {
    if let Ok(mut manager) = get_batch_manager().lock() {
        manager.queue_update(signal_id.to_string(), update_type);
    }
}

/// Phase 4.2: Batch処理実行便利関数
pub async fn process_batch_updates() -> usize {
    if let Ok(mut manager) = get_batch_manager().lock() {
        manager.process_batch().await.unwrap_or(0)
    } else {
        0
    }
}

/// Phase 4.2: Batch統計取得便利関数
pub fn get_batch_stats() -> Option<BatchStats> {
    if let Ok(manager) = get_batch_manager().lock() {
        Some(manager.get_stats().clone())
    } else {
        None
    }
}

/// Signalの種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignalType {
    // チャット表示関連
    ChatMessage,
    FilteredMessage,
    MessageFilter,

    // UI状態
    AutoScrollEnabled,
    UserHasScrolled,
    ShowFilterPanel,
    ShowTimestamps,
    MessageFontSize,

    // ハイライト
    HighlightEnabled,
    HighlightDuration,
    HighlightedMessageIds,

    // 内部制御
    LastMessageCount,
    ScrollPosition,

    // カスタム
    Custom(String),
}

/// Signal情報
#[derive(Debug, Clone)]
pub struct SignalInfo {
    pub id: SignalId,
    pub signal_type: SignalType,
    pub component: String,
    pub created_at: Instant,
    pub update_count: u64,
    pub last_updated: Option<Instant>,
}

/// Signal最適化の推奨事項
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    pub recommendation_type: OptimizationType,
    pub signal_ids: Vec<SignalId>,
    pub expected_improvement: f32,
    pub description: String,
    pub priority: u8, // 1が最高優先度
}

/// 最適化の種類
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationType {
    /// 重複Signal統合
    MergeDuplicate,
    /// Signal削除
    RemoveUnused,
    /// Batch更新
    BatchUpdate,
    /// 依存関係簡素化
    SimplifyDependency,
}

/// Signal依存関係グラフ
#[derive(Debug)]
pub struct SignalDependencyGraph {
    /// Signal情報
    signals: HashMap<SignalId, SignalInfo>,
    /// 統計情報
    stats: GraphStats,
}

/// グラフ統計情報
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub total_signals: usize,
    pub duplicate_signals: usize,
    pub unused_signals: usize,
    pub memory_usage: usize,
    pub last_analyzed: Instant,
}

impl SignalDependencyGraph {
    /// 新しい依存関係グラフを作成
    pub fn new() -> Self {
        Self {
            signals: HashMap::new(),
            stats: GraphStats {
                total_signals: 0,
                duplicate_signals: 0,
                unused_signals: 0,
                memory_usage: 0,
                last_analyzed: Instant::now(),
            },
        }
    }

    /// Signalを登録
    pub fn register_signal(&mut self, id: SignalId, signal_type: SignalType, component: String) {
        let signal_info = SignalInfo {
            id: id.clone(),
            signal_type,
            component,
            created_at: Instant::now(),
            update_count: 0,
            last_updated: None,
        };

        self.signals.insert(id, signal_info);
        self.stats.total_signals = self.signals.len();

        tracing::debug!(
            "📊 [SIGNAL] Registered: {} signals total",
            self.stats.total_signals
        );
    }

    /// Signal更新を記録
    pub fn record_update(&mut self, signal_id: &str) {
        if let Some(signal) = self.signals.get_mut(signal_id) {
            signal.update_count += 1;
            signal.last_updated = Some(Instant::now());
        }
    }

    /// 重複Signal検出
    pub fn detect_duplicate_signals(&self) -> Vec<Vec<SignalId>> {
        let mut duplicates = Vec::new();
        let mut type_groups: HashMap<SignalType, Vec<SignalId>> = HashMap::new();

        // 型別にグループ化
        for (id, info) in &self.signals {
            type_groups
                .entry(info.signal_type.clone())
                .or_insert_with(Vec::new)
                .push(id.clone());
        }

        // 同じ型で複数のSignalがある場合は重複候補
        for (_signal_type, ids) in type_groups {
            if ids.len() > 1 {
                duplicates.push(ids);
            }
        }

        duplicates
    }

    /// 未使用Signal検出
    pub fn detect_unused_signals(&self) -> Vec<SignalId> {
        self.signals
            .iter()
            .filter(|(_, info)| info.update_count == 0)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 最適化推奨事項を生成
    pub fn generate_optimization_recommendations(&mut self) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // 1. 重複Signal統合
        let duplicates = self.detect_duplicate_signals();
        for duplicate_group in duplicates.iter() {
            if duplicate_group.len() > 1 {
                recommendations.push(OptimizationRecommendation {
                    recommendation_type: OptimizationType::MergeDuplicate,
                    signal_ids: duplicate_group.clone(),
                    expected_improvement: (duplicate_group.len() - 1) as f32 * 0.2,
                    description: format!(
                        "Merge {} duplicate signals of same type",
                        duplicate_group.len()
                    ),
                    priority: 1,
                });
            }
        }

        // 2. 未使用Signal削除
        let unused = self.detect_unused_signals();
        if !unused.is_empty() {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: OptimizationType::RemoveUnused,
                signal_ids: unused.clone(),
                expected_improvement: unused.len() as f32 * 0.1,
                description: format!("Remove {} unused signals", unused.len()),
                priority: 2,
            });
        }

        // 優先度順にソート
        recommendations.sort_by_key(|r| r.priority);
        recommendations
    }

    /// 統計情報を更新
    pub fn update_stats(&mut self) {
        self.stats.total_signals = self.signals.len();
        self.stats.duplicate_signals = self
            .detect_duplicate_signals()
            .iter()
            .map(|g| g.len() - 1)
            .sum();
        self.stats.unused_signals = self.detect_unused_signals().len();
        self.stats.last_analyzed = Instant::now();
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> &GraphStats {
        &self.stats
    }

    /// 分析レポート生成
    pub fn generate_analysis_report(&mut self) -> String {
        self.update_stats();

        let mut report = String::new();
        report.push_str("=== Signal Optimization Analysis Report ===\n\n");

        // 基本統計
        report.push_str(&format!("📊 Total Signals: {}\n", self.stats.total_signals));
        report.push_str(&format!(
            "🔄 Duplicate Signals: {}\n",
            self.stats.duplicate_signals
        ));
        report.push_str(&format!(
            "🗑️ Unused Signals: {}\n",
            self.stats.unused_signals
        ));
        report.push_str(&format!(
            "💾 Memory Usage: {} bytes\n\n",
            self.stats.memory_usage
        ));

        // Component別統計
        let mut component_stats: HashMap<String, usize> = HashMap::new();
        for signal in self.signals.values() {
            *component_stats.entry(signal.component.clone()).or_insert(0) += 1;
        }

        report.push_str("📦 Signals by Component:\n");
        for (component, count) in &component_stats {
            report.push_str(&format!("  {} -> {} signals\n", component, count));
        }
        report.push_str("\n");

        // 重複Signal詳細
        let duplicates = self.detect_duplicate_signals();
        if !duplicates.is_empty() {
            report.push_str("🔍 Duplicate Signal Groups:\n");
            for (i, duplicate_group) in duplicates.iter().enumerate() {
                let signal_type = self
                    .signals
                    .get(&duplicate_group[0])
                    .map(|s| format!("{:?}", s.signal_type))
                    .unwrap_or_else(|| "Unknown".to_string());
                report.push_str(&format!(
                    "  Group {}: {} ({} signals)\n",
                    i + 1,
                    signal_type,
                    duplicate_group.len()
                ));
                for signal_id in duplicate_group {
                    if let Some(signal) = self.signals.get(signal_id) {
                        report.push_str(&format!("    - {} ({})\n", signal_id, signal.component));
                    }
                }
            }
            report.push_str("\n");
        }

        // 最適化推奨事項
        let recommendations = self.generate_optimization_recommendations();
        if !recommendations.is_empty() {
            report.push_str("💡 Optimization Recommendations:\n");
            for (i, rec) in recommendations.iter().enumerate() {
                report.push_str(&format!(
                    "  {}. [Priority {}] {}\n",
                    i + 1,
                    rec.priority,
                    rec.description
                ));
                report.push_str(&format!(
                    "     Expected improvement: {:.1}%\n",
                    rec.expected_improvement * 100.0
                ));
                report.push_str(&format!(
                    "     Affected signals: {}\n",
                    rec.signal_ids.len()
                ));
            }
        } else {
            report.push_str("✅ No optimization recommendations at this time.\n");
        }

        report
    }

    /// 現在のSignal一覧を取得
    pub fn list_signals(&self) -> Vec<&SignalInfo> {
        self.signals.values().collect()
    }
}

impl Default for SignalDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルSignal依存関係グラフ
use std::sync::{Arc, Mutex, OnceLock};

static GLOBAL_SIGNAL_GRAPH: OnceLock<Arc<Mutex<SignalDependencyGraph>>> = OnceLock::new();

/// グローバルSignalグラフを取得
pub fn get_signal_graph() -> Arc<Mutex<SignalDependencyGraph>> {
    GLOBAL_SIGNAL_GRAPH
        .get_or_init(|| {
            tracing::info!("📊 [SIGNAL] Creating global signal dependency graph");
            Arc::new(Mutex::new(SignalDependencyGraph::new()))
        })
        .clone()
}

/// Signal登録便利関数
pub fn register_signal(id: &str, signal_type: SignalType, component: &str) {
    if let Ok(mut graph) = get_signal_graph().lock() {
        graph.register_signal(id.to_string(), signal_type, component.to_string());
    }
}

/// Signal更新記録便利関数
pub fn record_signal_update(signal_id: &str) {
    if let Ok(mut graph) = get_signal_graph().lock() {
        graph.record_update(signal_id);
    }
}

/// 分析レポート生成便利関数
pub fn generate_signal_analysis_report() -> String {
    if let Ok(mut graph) = get_signal_graph().lock() {
        graph.generate_analysis_report()
    } else {
        "Error: Could not access signal graph".to_string()
    }
}

/// 最適化推奨事項取得便利関数
pub fn get_optimization_recommendations() -> Vec<OptimizationRecommendation> {
    if let Ok(mut graph) = get_signal_graph().lock() {
        graph.generate_optimization_recommendations()
    } else {
        Vec::new()
    }
}
