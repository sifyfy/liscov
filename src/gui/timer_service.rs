//! タイマーサービス（Phase 3.3）
//!
//! ハイライト自動クリアなどの時間ベース機能を精密制御
//! - タイマーライフサイクル管理
//! - メモリリーク完全防止
//! - 動的設定変更対応
//! - タイマーキャンセル機能

use dioxus::prelude::spawn; // Phase 3.3: Dioxus async spawn関数
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

/// タイマータスクのID
pub type TimerId = String;

/// タイマータスクの種類
#[derive(Debug, Clone, PartialEq)]
pub enum TimerTaskType {
    /// ハイライト自動クリア
    HighlightClear,
    /// 定期的なDOM更新
    DomUpdate,
    /// 統計更新
    StatsUpdate,
    /// カスタムタスク
    Custom(String),
}

/// タイマータスクの設定
#[derive(Debug, Clone)]
pub struct TimerConfig {
    /// 実行までの遅延
    pub delay: Duration,
    /// 繰り返し間隔（Noneで単発実行）
    pub interval: Option<Duration>,
    /// 最大実行回数（Noneで無制限）
    pub max_executions: Option<u32>,
    /// 優先度（低い値が高優先度）
    pub priority: u8,
    /// 自動キャンセル条件
    pub auto_cancel_after: Option<Duration>,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            delay: Duration::from_secs(5),
            interval: None,
            max_executions: Some(1),
            priority: 100,
            auto_cancel_after: Some(Duration::from_secs(300)), // 5分でタイムアウト
        }
    }
}

impl TimerConfig {
    /// ハイライト自動クリア用設定
    pub fn highlight_clear(delay_secs: u64) -> Self {
        Self {
            delay: Duration::from_secs(delay_secs),
            interval: None,
            max_executions: Some(1),
            priority: 50,
            auto_cancel_after: Some(Duration::from_secs(60)),
        }
    }

    /// 定期実行用設定
    pub fn periodic(interval_secs: u64) -> Self {
        Self {
            delay: Duration::from_secs(0),
            interval: Some(Duration::from_secs(interval_secs)),
            max_executions: None,
            priority: 100,
            auto_cancel_after: None,
        }
    }

    /// 単発実行用設定
    pub fn once(delay_secs: u64) -> Self {
        Self {
            delay: Duration::from_secs(delay_secs),
            interval: None,
            max_executions: Some(1),
            priority: 75,
            auto_cancel_after: Some(Duration::from_secs(30)),
        }
    }
}

/// タイマータスクの実行コンテキスト
#[derive(Debug, Clone)]
pub struct TimerContext {
    /// タスクID
    pub task_id: TimerId,
    /// タスクタイプ
    pub task_type: TimerTaskType,
    /// 実行回数
    pub execution_count: u32,
    /// 開始時刻
    pub started_at: Instant,
    /// 最後の実行時刻
    pub last_executed: Option<Instant>,
}

/// タイマータスクの実行結果
#[derive(Debug)]
pub enum TimerResult {
    /// 継続実行
    Continue,
    /// 完了（タスク終了）
    Complete,
    /// エラー（タスク停止）
    Error(String),
    /// キャンセル要求
    Cancel,
}

/// タイマータスクのハンドラー
pub type TimerHandler = Box<dyn Fn(TimerContext) -> TimerResult + Send + Sync>;

/// タイマータスクの内部状態（Phase 3.3 簡略版）
#[derive(Debug)]
struct TimerTask {
    id: TimerId,
    task_type: TimerTaskType,
    #[allow(dead_code)] // 設定UI統合時に活用予定のタイマー設定なのだ
    config: TimerConfig,
    cancel_sender: Option<oneshot::Sender<()>>,
    #[allow(dead_code)] // 追加メトリクス整備で使用予定のコンテキストなのだ
    context: TimerContext,
}

/// タイマーサービス
#[derive(Debug)]
pub struct TimerService {
    /// アクティブなタスク
    active_tasks: Arc<Mutex<HashMap<TimerId, TimerTask>>>,
    /// 統計情報
    stats: Arc<Mutex<TimerStats>>,
    /// サービス開始時刻
    started_at: Instant,
}

/// タイマー統計情報
#[derive(Debug, Clone)]
pub struct TimerStats {
    /// 総実行タスク数
    pub total_tasks: u64,
    /// アクティブタスク数
    pub active_tasks: u64,
    /// 完了タスク数
    pub completed_tasks: u64,
    /// キャンセルタスク数
    pub cancelled_tasks: u64,
    /// エラータスク数
    pub error_tasks: u64,
    /// 最後の更新時刻
    pub last_updated: Instant,
}

impl Default for TimerStats {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            active_tasks: 0,
            completed_tasks: 0,
            cancelled_tasks: 0,
            error_tasks: 0,
            last_updated: Instant::now(),
        }
    }
}

impl TimerService {
    /// 新しいタイマーサービスを作成
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(TimerStats::default())),
            started_at: Instant::now(),
        }
    }

    /// タイマータスクを開始（Phase 3.3 簡略版）
    pub fn start_task<F>(
        &self,
        id: TimerId,
        task_type: TimerTaskType,
        config: TimerConfig,
        handler: F,
    ) -> Result<(), String>
    where
        F: Fn(TimerContext) -> TimerResult + Send + Sync + 'static,
    {
        // 既存タスクのキャンセル
        self.cancel_task(&id);

        let (cancel_sender, cancel_receiver) = oneshot::channel();

        let context = TimerContext {
            task_id: id.clone(),
            task_type: task_type.clone(),
            execution_count: 0,
            started_at: Instant::now(),
            last_executed: None,
        };

        let task = TimerTask {
            id: id.clone(),
            task_type: task_type.clone(),
            config: config.clone(),
            cancel_sender: Some(cancel_sender),
            context: context.clone(),
        };

        // タスクを登録
        {
            let mut tasks = self.active_tasks.lock().unwrap();
            tasks.insert(id.clone(), task);
        }

        // 統計更新
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_tasks += 1;
            stats.active_tasks += 1;
            stats.last_updated = Instant::now();
        }

        // タスク実行を開始
        let active_tasks = self.active_tasks.clone();
        let stats = self.stats.clone();
        let task_id = id.clone();
        let task_type_for_async = task_type.clone();

        spawn(async move {
            Self::execute_task(
                task_id,
                task_type_for_async,
                config,
                Box::new(handler),
                context,
                cancel_receiver,
                active_tasks,
                stats,
            )
            .await;
        });

        tracing::info!("⏱️ [TIMER] Started task: {} ({:?})", id, task_type);
        Ok(())
    }

    /// タスクの実行処理
    async fn execute_task(
        task_id: TimerId,
        _task_type: TimerTaskType,
        config: TimerConfig,
        handler: TimerHandler,
        mut context: TimerContext,
        mut cancel_receiver: oneshot::Receiver<()>,
        active_tasks: Arc<Mutex<HashMap<TimerId, TimerTask>>>,
        stats: Arc<Mutex<TimerStats>>,
    ) {
        let mut execution_count = 0u32;
        let start_time = Instant::now();

        // 初回遅延
        tokio::select! {
            _ = tokio::time::sleep(config.delay) => {},
            _ = &mut cancel_receiver => {
                Self::complete_task(&task_id, "cancelled", &active_tasks, &stats);
                return;
            }
        }

        loop {
            // 自動キャンセル条件チェック
            if let Some(timeout) = config.auto_cancel_after {
                if start_time.elapsed() > timeout {
                    tracing::warn!("⏱️ [TIMER] Task timeout: {}", task_id);
                    Self::complete_task(&task_id, "timeout", &active_tasks, &stats);
                    return;
                }
            }

            // 最大実行回数チェック
            if let Some(max) = config.max_executions {
                if execution_count >= max {
                    Self::complete_task(&task_id, "completed", &active_tasks, &stats);
                    return;
                }
            }

            // コンテキスト更新
            context.execution_count = execution_count;
            context.last_executed = Some(Instant::now());

            // ハンドラー実行
            let result = handler(context.clone());

            execution_count += 1;

            match result {
                TimerResult::Continue => {
                    // 継続実行
                }
                TimerResult::Complete => {
                    Self::complete_task(&task_id, "completed", &active_tasks, &stats);
                    return;
                }
                TimerResult::Error(msg) => {
                    tracing::error!("⏱️ [TIMER] Task error: {} - {}", task_id, msg);
                    Self::complete_task(&task_id, "error", &active_tasks, &stats);
                    return;
                }
                TimerResult::Cancel => {
                    Self::complete_task(&task_id, "cancelled", &active_tasks, &stats);
                    return;
                }
            }

            // 繰り返し間隔の処理
            if let Some(interval) = config.interval {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {},
                    _ = &mut cancel_receiver => {
                        Self::complete_task(&task_id, "cancelled", &active_tasks, &stats);
                        return;
                    }
                }
            } else {
                // 単発実行の場合は終了
                Self::complete_task(&task_id, "completed", &active_tasks, &stats);
                return;
            }
        }
    }

    /// タスクの完了処理
    fn complete_task(
        task_id: &str,
        reason: &str,
        active_tasks: &Arc<Mutex<HashMap<TimerId, TimerTask>>>,
        stats: &Arc<Mutex<TimerStats>>,
    ) {
        // アクティブタスクから削除
        let removed = {
            let mut tasks = active_tasks.lock().unwrap();
            tasks.remove(task_id).is_some()
        };

        if removed {
            // 統計更新
            let mut stats = stats.lock().unwrap();
            stats.active_tasks = stats.active_tasks.saturating_sub(1);

            match reason {
                "completed" => stats.completed_tasks += 1,
                "cancelled" => stats.cancelled_tasks += 1,
                "error" | "timeout" => stats.error_tasks += 1,
                _ => {}
            }

            stats.last_updated = Instant::now();

            tracing::debug!("⏱️ [TIMER] Task completed: {} ({})", task_id, reason);
        }
    }

    /// タスクをキャンセル
    pub fn cancel_task(&self, task_id: &str) -> bool {
        let sender = {
            let mut tasks = self.active_tasks.lock().unwrap();
            tasks
                .remove(task_id)
                .and_then(|mut task| task.cancel_sender.take())
        };

        if let Some(sender) = sender {
            let _ = sender.send(());
            tracing::info!("⏱️ [TIMER] Cancelled task: {}", task_id);
            true
        } else {
            false
        }
    }

    /// 特定タイプのタスクをすべてキャンセル
    pub fn cancel_tasks_by_type(&self, task_type: &TimerTaskType) -> u32 {
        let task_ids: Vec<String> = {
            let tasks = self.active_tasks.lock().unwrap();
            tasks
                .values()
                .filter(|task| &task.task_type == task_type)
                .map(|task| task.id.clone())
                .collect()
        };

        let mut cancelled = 0;
        for task_id in task_ids {
            if self.cancel_task(&task_id) {
                cancelled += 1;
            }
        }

        if cancelled > 0 {
            tracing::info!(
                "⏱️ [TIMER] Cancelled {} tasks of type {:?}",
                cancelled,
                task_type
            );
        }

        cancelled
    }

    /// 全タスクをキャンセル
    pub fn cancel_all_tasks(&self) -> u32 {
        let task_ids: Vec<String> = {
            let tasks = self.active_tasks.lock().unwrap();
            tasks.keys().cloned().collect()
        };

        let mut cancelled = 0;
        for task_id in task_ids {
            if self.cancel_task(&task_id) {
                cancelled += 1;
            }
        }

        if cancelled > 0 {
            tracing::info!("⏱️ [TIMER] Cancelled all {} tasks", cancelled);
        }

        cancelled
    }

    /// アクティブなタスク一覧を取得
    pub fn get_active_tasks(&self) -> Vec<(TimerId, TimerTaskType)> {
        let tasks = self.active_tasks.lock().unwrap();
        tasks
            .values()
            .map(|task| (task.id.clone(), task.task_type.clone()))
            .collect()
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> TimerStats {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }

    /// サービスの稼働時間を取得
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}

impl Default for TimerService {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルタイマーサービス
use std::sync::OnceLock;

static GLOBAL_TIMER_SERVICE: OnceLock<Arc<TimerService>> = OnceLock::new();

/// グローバルタイマーサービスを取得
pub fn get_timer_service() -> Arc<TimerService> {
    GLOBAL_TIMER_SERVICE
        .get_or_init(|| {
            tracing::info!("⏱️ [TIMER] Creating global timer service");
            Arc::new(TimerService::new())
        })
        .clone()
}

/// ハイライト自動クリア便利関数（Phase 3.3 簡略版）
pub fn schedule_highlight_clear<F>(
    highlight_ids: std::collections::HashSet<String>,
    delay_secs: u64,
    clear_callback: F,
) -> Result<TimerId, String>
where
    F: Fn() + Send + Sync + 'static,
{
    let timer_service = get_timer_service();
    let task_id = format!(
        "highlight_clear_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let config = TimerConfig::highlight_clear(delay_secs);

    timer_service.start_task(
        task_id.clone(),
        TimerTaskType::HighlightClear,
        config,
        move |_context| {
            clear_callback();
            TimerResult::Complete
        },
    )?;

    tracing::info!(
        "⏱️ [TIMER] Scheduled highlight clear: {} IDs in {}s",
        highlight_ids.len(),
        delay_secs
    );

    Ok(task_id)
}

/// 便利関数: 既存のハイライトクリアタスクをキャンセル
pub fn cancel_highlight_clear_tasks() -> u32 {
    let timer_service = get_timer_service();
    timer_service.cancel_tasks_by_type(&TimerTaskType::HighlightClear)
}
