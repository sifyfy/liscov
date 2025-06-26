//! イベント駆動システム
//!
//! コンポーネント間の疎結合な通信を実現するイベントバスシステム

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, OnceLock};

pub mod chat_events;

/// イベントハンドラーのエラー
#[derive(Debug, Clone)]
pub enum EventError {
    /// ハンドラー実行エラー
    HandlerFailed(String),
    /// イベント配信エラー
    DispatchFailed(String),
    /// ハンドラー登録エラー
    RegistrationFailed(String),
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventError::HandlerFailed(msg) => write!(f, "Event handler failed: {}", msg),
            EventError::DispatchFailed(msg) => write!(f, "Event dispatch failed: {}", msg),
            EventError::RegistrationFailed(msg) => {
                write!(f, "Handler registration failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for EventError {}

/// イベントトレイト
pub trait Event: Debug + Send + Sync + 'static {
    /// イベントの名前
    fn event_name(&self) -> &'static str;

    /// イベントの優先度（低い値が高優先度）
    fn priority(&self) -> u8 {
        100
    }

    /// イベントがキャンセル可能か
    fn is_cancellable(&self) -> bool {
        false
    }
}

/// イベントハンドラートレイト
pub trait EventHandler<E: Event>: Send + Sync {
    /// イベントハンドラーの実行
    fn handle(&mut self, event: &E) -> Result<(), EventError>;

    /// ハンドラーの説明
    fn handler_name(&self) -> &'static str;
}

/// イベント配信結果
#[derive(Debug)]
pub struct EventDispatchResult {
    /// 処理されたハンドラー数
    pub handlers_executed: usize,
    /// 成功したハンドラー数
    pub handlers_succeeded: usize,
    /// 失敗したハンドラー数
    pub handlers_failed: usize,
    /// エラー詳細
    pub errors: Vec<EventError>,
    /// 実行時間（ミリ秒）
    pub total_time_ms: u64,
}

impl EventDispatchResult {
    pub fn new() -> Self {
        Self {
            handlers_executed: 0,
            handlers_succeeded: 0,
            handlers_failed: 0,
            errors: Vec::new(),
            total_time_ms: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.handlers_failed == 0
    }

    pub fn add_success(&mut self) {
        self.handlers_executed += 1;
        self.handlers_succeeded += 1;
    }

    pub fn add_failure(&mut self, error: EventError) {
        self.handlers_executed += 1;
        self.handlers_failed += 1;
        self.errors.push(error);
    }
}

/// ハンドラーコンテナ（型消去）
struct HandlerContainer {
    handler: Box<dyn Any + Send + Sync>,
    handler_name: &'static str,
    type_id: std::any::TypeId,
}

/// イベントバス
pub struct EventBus {
    /// イベント型ごとのハンドラーリスト
    handlers: HashMap<TypeId, Vec<HandlerContainer>>,
    /// イベント統計
    stats: EventStats,
}

#[derive(Debug, Default)]
struct EventStats {
    total_events_dispatched: u64,
    total_handlers_executed: u64,
    total_handlers_failed: u64,
}

impl EventBus {
    /// 新しいイベントバスを作成
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            stats: EventStats::default(),
        }
    }

    /// イベントハンドラーを登録
    pub fn register_handler<E: Event, H: EventHandler<E> + 'static>(
        &mut self,
        handler: H,
    ) -> Result<(), EventError> {
        let type_id = TypeId::of::<E>();
        let handler_name = handler.handler_name();

        let container = HandlerContainer {
            handler: Box::new(Mutex::new(handler)),
            handler_name,
            type_id: TypeId::of::<E>(),
        };

        self.handlers
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(container);

        tracing::debug!(
            "📡 [EVENT] Registered handler '{}' for event '{}'",
            handler_name,
            std::any::type_name::<E>()
        );

        Ok(())
    }

    /// イベントを配信
    pub fn dispatch<E: Event>(&mut self, event: &E) -> EventDispatchResult {
        let start_time = std::time::Instant::now();
        let mut result = EventDispatchResult::new();

        let type_id = TypeId::of::<E>();

        tracing::debug!("📡 [EVENT] Dispatching event: {}", event.event_name());

        // TODO: Phase 2で完全な型安全イベントハンドラーを実装
        // 現在は基本的なログ出力のみ（型消去問題の回避）
        if let Some(handlers) = self.handlers.get(&type_id) {
            tracing::debug!(
                "📡 [EVENT] Found {} handlers for event: {} (Phase 1 placeholder)",
                handlers.len(),
                event.event_name()
            );
        } else {
            tracing::debug!(
                "📡 [EVENT] No handlers registered for event: {}",
                event.event_name()
            );
        }

        result.total_time_ms = start_time.elapsed().as_millis() as u64;

        // 統計更新
        self.stats.total_events_dispatched += 1;
        self.stats.total_handlers_executed += result.handlers_executed as u64;
        self.stats.total_handlers_failed += result.handlers_failed as u64;

        if result.handlers_executed > 0 {
            tracing::debug!(
                "📡 [EVENT] Dispatch completed: {}/{} handlers succeeded ({}ms)",
                result.handlers_succeeded,
                result.handlers_executed,
                result.total_time_ms
            );
        }

        result
    }

    /// 登録されているハンドラー数を取得
    pub fn handler_count<E: Event>(&self) -> usize {
        let type_id = TypeId::of::<E>();
        self.handlers.get(&type_id).map(|v| v.len()).unwrap_or(0)
    }

    /// 全ハンドラーをクリア
    pub fn clear_handlers(&mut self) {
        let total_handlers: usize = self.handlers.values().map(|v| v.len()).sum();
        self.handlers.clear();
        tracing::info!("🗑️ [EVENT] Cleared {} handlers", total_handlers);
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> &EventStats {
        &self.stats
    }

    /// 統計情報をリセット
    pub fn reset_stats(&mut self) {
        self.stats = EventStats::default();
        tracing::debug!("📊 [EVENT] Statistics reset");
    }

    /// デバッグ情報を出力
    pub fn debug_info(&self) {
        tracing::info!(
            "📊 [EVENT] Bus stats: {} events dispatched, {} handlers executed, {} failed",
            self.stats.total_events_dispatched,
            self.stats.total_handlers_executed,
            self.stats.total_handlers_failed
        );

        for (type_id, handlers) in &self.handlers {
            tracing::debug!("📊 [EVENT] Type {:?}: {} handlers", type_id, handlers.len());
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ハンドラーの型消去のための unsafe な実装
// これはコンパイル時の型安全性を保ちつつ、実行時の柔軟性を提供するため
unsafe impl Send for HandlerContainer {}
unsafe impl Sync for HandlerContainer {}

/// グローバルイベントバス
static GLOBAL_EVENT_BUS: OnceLock<Arc<Mutex<EventBus>>> = OnceLock::new();

/// グローバルイベントバスを取得
pub fn get_global_event_bus() -> Arc<Mutex<EventBus>> {
    GLOBAL_EVENT_BUS
        .get_or_init(|| {
            tracing::info!("🏗️ [EVENT] Creating global event bus");
            Arc::new(Mutex::new(EventBus::new()))
        })
        .clone()
}

/// イベント配信の便利関数
pub fn dispatch_event<E: Event>(event: &E) -> EventDispatchResult {
    let bus = get_global_event_bus();
    let mut bus = bus.lock().unwrap();
    bus.dispatch(event)
}

/// ハンドラー登録の便利関数
pub fn register_handler<E: Event, H: EventHandler<E> + 'static>(
    handler: H,
) -> Result<(), EventError> {
    let bus = get_global_event_bus();
    let mut bus = bus.lock().unwrap();
    bus.register_handler(handler)
}

/// 統計情報取得の便利関数
pub fn get_event_stats() -> (u64, u64, u64) {
    let bus = get_global_event_bus();
    let bus = bus.lock().unwrap();
    let stats = bus.get_stats();
    (
        stats.total_events_dispatched,
        stats.total_handlers_executed,
        stats.total_handlers_failed,
    )
}

/// デバッグ情報出力の便利関数
pub fn debug_event_bus() {
    let bus = get_global_event_bus();
    let bus = bus.lock().unwrap();
    bus.debug_info();
}
