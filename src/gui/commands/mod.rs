//! Command Pattern 実装
//!
//! チャット表示機能の各操作をコマンドオブジェクトとして実装し、
//! 実行順序制御、エラーハンドリング、取り消し機能を提供

use std::collections::VecDeque;
use std::fmt::Debug;

pub mod chat_commands;

/// コマンドエラー
#[derive(Debug, Clone)]
pub enum CommandError {
    /// 実行エラー
    ExecutionFailed(String),
    /// 取り消し不可能
    UndoNotSupported,
    /// 取り消し失敗
    UndoFailed(String),
    /// 前提条件エラー
    PreconditionFailed(String),
    /// タイムアウト
    Timeout,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::ExecutionFailed(msg) => write!(f, "Command execution failed: {}", msg),
            CommandError::UndoNotSupported => write!(f, "Undo not supported for this command"),
            CommandError::UndoFailed(msg) => write!(f, "Undo failed: {}", msg),
            CommandError::PreconditionFailed(msg) => write!(f, "Precondition failed: {}", msg),
            CommandError::Timeout => write!(f, "Command execution timed out"),
        }
    }
}

impl std::error::Error for CommandError {}

/// コマンドの実行コンテキスト（Phase 3で簡略版実装）
/// 必要最小限のSignalを含む軽量なContext
#[derive(Clone)]
pub struct CommandContext {
    // Phase 3では使用しない - Phase 2の直接操作を推奨
    // 将来のフル実装用プレースホルダー
}

impl CommandContext {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CommandContext {
    fn default() -> Self {
        Self::new()
    }
}

/// コマンドトレイト（同期版）
pub trait Command: Debug + Send + Sync {
    /// コマンドの実行
    fn execute(&self, context: &CommandContext) -> Result<(), CommandError>;

    /// コマンドの取り消し（オプション）
    fn undo(&self, _context: &CommandContext) -> Result<(), CommandError> {
        Err(CommandError::UndoNotSupported)
    }

    /// コマンドの前提条件チェック
    fn can_execute(&self, _context: &CommandContext) -> bool {
        true
    }

    /// コマンドの説明を取得
    fn description(&self) -> &str;

    /// コマンドの優先度（低い値が高優先度）
    fn priority(&self) -> u8 {
        100
    }
}

/// コマンド実行結果
#[derive(Debug)]
pub struct CommandResult {
    /// 実行成功フラグ
    pub success: bool,
    /// エラー情報
    pub error: Option<CommandError>,
    /// 実行時間（ミリ秒）
    pub execution_time_ms: u64,
    /// 追加情報
    pub details: Option<String>,
}

impl CommandResult {
    pub fn success(execution_time_ms: u64) -> Self {
        Self {
            success: true,
            error: None,
            execution_time_ms,
            details: None,
        }
    }

    pub fn success_with_details(execution_time_ms: u64, details: String) -> Self {
        Self {
            success: true,
            error: None,
            execution_time_ms,
            details: Some(details),
        }
    }

    pub fn failure(error: CommandError, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            error: Some(error),
            execution_time_ms,
            details: None,
        }
    }
}

/// コマンド実行エンジン
#[derive(Debug)]
pub struct CommandExecutor {
    /// コマンドキュー
    command_queue: VecDeque<Box<dyn Command>>,
    /// 実行中フラグ
    is_executing: bool,
    /// 実行履歴（取り消し用）
    execution_history: Vec<Box<dyn Command>>,
    /// 最大履歴数
    max_history_size: usize,
}

impl CommandExecutor {
    /// 新しいコマンド実行エンジンを作成
    pub fn new() -> Self {
        Self {
            command_queue: VecDeque::new(),
            is_executing: false,
            execution_history: Vec::new(),
            max_history_size: 100,
        }
    }

    /// コマンドをキューに追加
    pub fn enqueue(&mut self, command: Box<dyn Command>) {
        // 優先度でソート挿入
        let priority = command.priority();
        let description = command.description().to_string(); // 先に説明を取得
        let mut insert_position = None;

        for (i, existing_command) in self.command_queue.iter().enumerate() {
            if priority < existing_command.priority() {
                insert_position = Some(i);
                break;
            }
        }

        match insert_position {
            Some(pos) => {
                self.command_queue.insert(pos, command);
            }
            None => {
                self.command_queue.push_back(command);
            }
        }

        tracing::debug!(
            "📋 [COMMAND] Enqueued: {} (queue size: {})",
            description,
            self.command_queue.len()
        );
    }

    /// 単一コマンドを即座に実行
    pub fn execute_immediate(
        &mut self,
        command: Box<dyn Command>,
        context: &CommandContext,
    ) -> CommandResult {
        if self.is_executing {
            return CommandResult::failure(
                CommandError::ExecutionFailed("Another command is currently executing".to_string()),
                0,
            );
        }

        self.is_executing = true;
        let result = self.execute_single_command(command, context);
        self.is_executing = false;

        result
    }

    /// キューの全てのコマンドを実行
    pub fn execute_all(&mut self, context: &CommandContext) -> Vec<CommandResult> {
        if self.is_executing {
            return vec![CommandResult::failure(
                CommandError::ExecutionFailed("Executor is already running".to_string()),
                0,
            )];
        }

        self.is_executing = true;
        let mut results = Vec::new();

        tracing::info!(
            "🚀 [COMMAND] Starting batch execution: {} commands",
            self.command_queue.len()
        );

        while let Some(command) = self.command_queue.pop_front() {
            let result = self.execute_single_command(command, context);
            results.push(result);

            // 失敗した場合は残りのコマンドを停止するか判断
            if !results.last().unwrap().success {
                tracing::warn!("⚠️ [COMMAND] Command failed, stopping batch execution");
                break;
            }
        }

        self.is_executing = false;
        tracing::info!(
            "✅ [COMMAND] Batch execution completed: {}/{} successful",
            results.iter().filter(|r| r.success).count(),
            results.len()
        );

        results
    }

    /// 単一コマンドの実行
    fn execute_single_command(
        &mut self,
        command: Box<dyn Command>,
        context: &CommandContext,
    ) -> CommandResult {
        let start_time = std::time::Instant::now();
        let description = command.description().to_string();

        tracing::debug!("🔄 [COMMAND] Executing: {}", description);

        // 前提条件チェック
        if !command.can_execute(context) {
            let execution_time = start_time.elapsed().as_millis() as u64;
            tracing::warn!("❌ [COMMAND] Precondition failed: {}", description);
            return CommandResult::failure(
                CommandError::PreconditionFailed(format!(
                    "Precondition failed for: {}",
                    description
                )),
                execution_time,
            );
        }

        // コマンド実行
        match command.execute(context) {
            Ok(()) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                // 実行履歴に追加
                self.add_to_history(command);

                tracing::debug!(
                    "✅ [COMMAND] Success: {} ({}ms)",
                    description,
                    execution_time
                );
                CommandResult::success(execution_time)
            }
            Err(error) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                tracing::error!(
                    "❌ [COMMAND] Failed: {} - {} ({}ms)",
                    description,
                    error,
                    execution_time
                );
                CommandResult::failure(error, execution_time)
            }
        }
    }

    /// 履歴に追加
    fn add_to_history(&mut self, command: Box<dyn Command>) {
        self.execution_history.push(command);

        // 履歴サイズ制限
        if self.execution_history.len() > self.max_history_size {
            self.execution_history.remove(0);
        }
    }

    /// 最後に実行したコマンドを取り消し
    pub fn undo_last(&mut self, context: &CommandContext) -> Result<(), CommandError> {
        if let Some(command) = self.execution_history.pop() {
            tracing::info!("🔄 [COMMAND] Undoing: {}", command.description());
            command.undo(context)
        } else {
            Err(CommandError::UndoNotSupported)
        }
    }

    /// キューの状態を確認
    pub fn queue_status(&self) -> (usize, bool) {
        (self.command_queue.len(), self.is_executing)
    }

    /// キューをクリア
    pub fn clear_queue(&mut self) {
        let cleared_count = self.command_queue.len();
        self.command_queue.clear();
        tracing::info!("🗑️ [COMMAND] Cleared {} commands from queue", cleared_count);
    }

    /// 実行履歴をクリア
    pub fn clear_history(&mut self) {
        let cleared_count = self.execution_history.len();
        self.execution_history.clear();
        tracing::info!(
            "🗑️ [COMMAND] Cleared {} commands from history",
            cleared_count
        );
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルコマンド実行エンジン
use std::sync::{Arc, Mutex, OnceLock};

static GLOBAL_EXECUTOR: OnceLock<Arc<Mutex<CommandExecutor>>> = OnceLock::new();

/// グローバルコマンド実行エンジンを取得
pub fn get_global_executor() -> Arc<Mutex<CommandExecutor>> {
    GLOBAL_EXECUTOR
        .get_or_init(|| {
            tracing::info!("🏗️ [COMMAND] Creating global command executor");
            Arc::new(Mutex::new(CommandExecutor::new()))
        })
        .clone()
}

/// コマンド実行の便利関数
pub fn execute_command(command: Box<dyn Command>, context: &CommandContext) -> CommandResult {
    let executor = get_global_executor();
    let mut executor = executor.lock().unwrap();
    executor.execute_immediate(command, context)
}

/// コマンドをキューに追加する便利関数
pub fn enqueue_command(command: Box<dyn Command>) {
    let executor = get_global_executor();
    let mut executor = executor.lock().unwrap();
    executor.enqueue(command);
}

/// キューの全コマンドを実行する便利関数
pub fn execute_all_commands(context: &CommandContext) -> Vec<CommandResult> {
    let executor = get_global_executor();
    let mut executor = executor.lock().unwrap();
    executor.execute_all(context)
}
