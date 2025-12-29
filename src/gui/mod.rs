// Core modules
pub mod auth_window; // YouTube認証ウィンドウ
pub mod config_manager;
pub mod memory_optimized; // メモリ効率最適化
pub mod message_processor; // メッセージ処理パイプライン
pub mod message_stream; // メッセージストリーミングシステム
pub mod models; // 既存のデータ構造は継続使用
pub mod plugin_system; // プラグインアーキテクチャ
pub mod plugins; // サンプルプラグイン
pub mod services; // 既存のAPIサービスは継続使用
pub mod stream_end_detector; // 配信終了検出機能
pub mod system_messages; // システムメッセージ生成機能
pub mod traits; // トレイトベース設計
pub mod unified_config; // 統一設定管理システム
pub mod utils; // ユーティリティ関数は継続使用 // 設定管理モジュール

// Dioxus UI components - 新アーキテクチャ対応
pub mod components; // 🆕 UI コンポーネント（有効化）
pub mod dom_controller; // Phase 3.2: DOM制御モジュール
pub mod hooks; // LiveChatフック有効化
pub mod performance_monitor;
pub mod signal_optimizer; // Phase 4.1: Signal最適化
pub mod styles; // スタイル有効化
pub mod timer_service; // Phase 3.3: タイマーサービス // Phase 5.2: パフォーマンス監視

// Phase 4.3: クロージャ最適化とメモリ管理
pub mod closure_optimizer;

// Core functionality exports - specific imports to avoid ambiguous glob re-exports
pub use models::{ActiveTab, GuiChatMessage, MessageType};
pub use services::*;

// Message streaming exports
pub use message_stream::{DisplayLimit, MessageStream, MessageStreamConfig, MessageStreamStats};

// New state management modules
pub mod live_chat_service;
pub mod state_broadcaster;
pub mod state_management;
pub mod ui_sync_service;

// Phase 2.1: Unified App Context (Dioxus 0.6.3準拠)
pub mod app_context;

// Phase 2.2: use_resource活用による非同期処理最適化
pub mod resource_hooks;

// Phase 2.3: 効率的なSignal構造の再設計
pub mod signal_manager;

// Phase 2.4: spawn_blocking活用による重処理の分離
pub mod blocking_processor;

// New refactored modules (Phase 3) - 段階的復活
pub mod commands; // Command Pattern - Phase 3.1で復活
pub mod events; // Event System - Phase 3.1で復活
                // pub mod state; // 統合状態管理 - Signal互換性問題で一時無効化

pub use live_chat_service::*;
pub use state_broadcaster::{get_broadcaster, StateBroadcaster, StateChange};
pub use state_management::{get_state_manager, AppEvent, StateManager};
pub use ui_sync_service::*;

// Phase 2.1: Unified App Context exports (Dioxus 0.6.3準拠)
pub use app_context::{
    send_app_event, use_app_context, use_unified_live_chat, AppContext, AppContextProvider,
    LiveChatHandle as UnifiedLiveChatHandle, LiveChatState, MessageStreamState, UiState,
};

// Phase 2.2: use_resource非同期処理最適化 exports
pub use resource_hooks::{
    use_conditional_message_fetch, use_message_resource, use_realtime_message_stream,
    MessageFetchResult, MessageLoader,
};

// Phase 2.3: 効率的なSignal構造管理システム exports
pub use signal_manager::{
    get_signal_manager, use_optimized_signals, OptimizedSignalHandle, SignalManager,
    SignalUpdateType, UpdatePriority,
};

// Phase 2.4: spawn_blocking重処理分離システム exports
pub use blocking_processor::{
    get_blocking_processor, submit_data_transformation, submit_file_export,
    submit_message_analysis, BlockingProcessor, BlockingTask, BlockingTaskResult,
    FileOperationType, FilterOptions, TransformationType,
};

// Export new architecture components - Phase 3で段階的復活
pub use commands::{enqueue_command, execute_command, Command, CommandExecutor}; // Phase 3.1で復活
pub use events::{dispatch_event, register_handler, Event, EventHandler}; // Phase 3.1で復活
                                                                         // pub use state::ChatDisplayState; // Signal互換性問題で一時無効化

// Export new modern components - 動作するもののみ
pub use components::MainWindow;

// Temporarily disable problematic state module
// pub use state::*;

pub mod state;
