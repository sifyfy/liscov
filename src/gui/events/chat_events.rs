//! チャット機能のイベント定義
//!
//! チャット表示に関連する各種イベントを定義

use super::{Event, EventError, EventHandler};
use crate::chat_management::MessageFilter;
use crate::gui::models::{ActiveTab, GuiChatMessage};

/// メッセージ追加イベント
#[derive(Debug, Clone)]
pub struct MessageAddedEvent {
    /// 追加されたメッセージ
    pub message: GuiChatMessage,
    /// 追加前のメッセージ数
    pub previous_count: usize,
    /// 追加後のメッセージ数
    pub new_count: usize,
}

impl Event for MessageAddedEvent {
    fn event_name(&self) -> &'static str {
        "MessageAdded"
    }

    fn priority(&self) -> u8 {
        20 // 高優先度（新着メッセージは重要）
    }
}

/// 複数メッセージ追加イベント
#[derive(Debug, Clone)]
pub struct MessagesAddedEvent {
    /// 追加されたメッセージリスト
    pub messages: Vec<GuiChatMessage>,
    /// 追加前のメッセージ数
    pub previous_count: usize,
    /// 追加後のメッセージ数
    pub new_count: usize,
}

impl Event for MessagesAddedEvent {
    fn event_name(&self) -> &'static str {
        "MessagesAdded"
    }

    fn priority(&self) -> u8 {
        20 // 高優先度
    }
}

/// タブ変更イベント
#[derive(Debug, Clone)]
pub struct TabChangedEvent {
    /// 前のタブ
    pub previous_tab: ActiveTab,
    /// 新しいタブ
    pub new_tab: ActiveTab,
    /// 変更タイムスタンプ
    pub timestamp: std::time::Instant,
}

impl Event for TabChangedEvent {
    fn event_name(&self) -> &'static str {
        "TabChanged"
    }

    fn priority(&self) -> u8 {
        30 // 中優先度
    }
}

/// スクロール状態変更イベント
#[derive(Debug, Clone)]
pub struct ScrollStateChangedEvent {
    /// ユーザーがスクロールしたかフラグ
    pub user_has_scrolled: bool,
    /// 自動スクロール有効フラグ
    pub auto_scroll_enabled: bool,
    /// 現在のスクロール位置
    pub scroll_position: f64,
    /// イベント発生時刻
    pub timestamp: std::time::Instant,
}

impl Event for ScrollStateChangedEvent {
    fn event_name(&self) -> &'static str {
        "ScrollStateChanged"
    }

    fn priority(&self) -> u8 {
        40 // 中優先度
    }
}

/// ハイライト設定変更イベント
#[derive(Debug, Clone)]
pub struct HighlightConfigChangedEvent {
    /// ハイライト有効フラグ
    pub enabled: bool,
    /// ハイライト継続時間（秒）
    pub duration_seconds: u64,
    /// 最大ハイライト数
    pub max_messages: usize,
    /// 変更タイムスタンプ
    pub timestamp: std::time::Instant,
}

impl Event for HighlightConfigChangedEvent {
    fn event_name(&self) -> &'static str {
        "HighlightConfigChanged"
    }

    fn priority(&self) -> u8 {
        50 // 中低優先度
    }
}

/// フィルタ変更イベント
#[derive(Debug, Clone)]
pub struct FilterChangedEvent {
    /// 前のフィルタ設定
    pub previous_filter: MessageFilter,
    /// 新しいフィルタ設定
    pub new_filter: MessageFilter,
    /// フィルタ適用前のメッセージ数
    pub unfiltered_count: usize,
    /// フィルタ適用後のメッセージ数
    pub filtered_count: usize,
}

impl Event for FilterChangedEvent {
    fn event_name(&self) -> &'static str {
        "FilterChanged"
    }

    fn priority(&self) -> u8 {
        25 // 高優先度（表示に直結）
    }
}

/// 接続状態変更イベント
#[derive(Debug, Clone)]
pub struct ConnectionStateChangedEvent {
    /// 接続状態
    pub is_connected: bool,
    /// 前の接続状態
    pub previous_state: bool,
    /// 変更理由
    pub reason: String,
    /// 変更タイムスタンプ
    pub timestamp: std::time::Instant,
}

impl Event for ConnectionStateChangedEvent {
    fn event_name(&self) -> &'static str {
        "ConnectionStateChanged"
    }

    fn priority(&self) -> u8 {
        10 // 最高優先度（接続状態は最重要）
    }
}

/// メッセージクリアイベント
#[derive(Debug, Clone)]
pub struct MessagesClearedEvent {
    /// クリア前のメッセージ数
    pub previous_count: usize,
    /// クリア理由
    pub reason: String,
    /// クリアタイムスタンプ
    pub timestamp: std::time::Instant,
}

impl Event for MessagesClearedEvent {
    fn event_name(&self) -> &'static str {
        "MessagesCleared"
    }

    fn priority(&self) -> u8 {
        15 // 高優先度（状態リセット）
    }
}

/// UI状態リセットイベント
#[derive(Debug, Clone)]
pub struct UiStateResetEvent {
    /// リセット理由
    pub reason: String,
    /// リセット範囲
    pub scope: UiResetScope,
    /// リセットタイムスタンプ
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum UiResetScope {
    /// 全て
    All,
    /// ハイライト関連のみ
    HighlightOnly,
    /// スクロール関連のみ
    ScrollOnly,
    /// フィルター関連のみ
    FilterOnly,
}

impl Event for UiStateResetEvent {
    fn event_name(&self) -> &'static str {
        "UiStateReset"
    }

    fn priority(&self) -> u8 {
        10 // 最高優先度（状態整合性確保）
    }
}

/// エラーイベント
#[derive(Debug, Clone)]
pub struct ChatErrorEvent {
    /// エラーメッセージ
    pub error_message: String,
    /// エラーの種類
    pub error_type: ChatErrorType,
    /// 発生コンポーネント
    pub component: String,
    /// エラー発生時刻
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum ChatErrorType {
    /// コマンド実行エラー
    CommandError,
    /// DOM操作エラー
    DomError,
    /// 状態同期エラー
    SyncError,
    /// フィルタリングエラー
    FilterError,
    /// その他
    Other,
}

impl Event for ChatErrorEvent {
    fn event_name(&self) -> &'static str {
        "ChatError"
    }

    fn priority(&self) -> u8 {
        5 // 最高優先度（エラーは最重要）
    }
}

// === イベントハンドラーの例 ===

/// ログ出力用のイベントハンドラー
pub struct LoggingEventHandler;

impl<E: Event> EventHandler<E> for LoggingEventHandler {
    fn handle(&mut self, event: &E) -> Result<(), EventError> {
        tracing::info!("📡 [EVENT_LOG] {}: {:?}", event.event_name(), event);
        Ok(())
    }

    fn handler_name(&self) -> &'static str {
        "LoggingEventHandler"
    }
}

/// 統計収集用のイベントハンドラー
pub struct StatsCollectorHandler {
    /// イベント発生回数
    pub event_counts: std::collections::HashMap<&'static str, u64>,
}

impl StatsCollectorHandler {
    pub fn new() -> Self {
        Self {
            event_counts: std::collections::HashMap::new(),
        }
    }

    pub fn get_count(&self, event_name: &str) -> u64 {
        *self.event_counts.get(event_name).unwrap_or(&0)
    }
}

impl<E: Event> EventHandler<E> for StatsCollectorHandler {
    fn handle(&mut self, event: &E) -> Result<(), EventError> {
        let event_name = event.event_name();
        *self.event_counts.entry(event_name).or_insert(0) += 1;
        Ok(())
    }

    fn handler_name(&self) -> &'static str {
        "StatsCollectorHandler"
    }
}

/// デバッグ用のイベントハンドラー
pub struct DebugEventHandler {
    /// 詳細ログを出力するかフラグ
    pub verbose: bool,
}

impl DebugEventHandler {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl<E: Event> EventHandler<E> for DebugEventHandler {
    fn handle(&mut self, event: &E) -> Result<(), EventError> {
        if self.verbose {
            tracing::debug!(
                "🔍 [EVENT_DEBUG] {} (priority: {}): {:#?}",
                event.event_name(),
                event.priority(),
                event
            );
        } else {
            tracing::debug!(
                "🔍 [EVENT_DEBUG] {} (priority: {})",
                event.event_name(),
                event.priority()
            );
        }
        Ok(())
    }

    fn handler_name(&self) -> &'static str {
        "DebugEventHandler"
    }
}
