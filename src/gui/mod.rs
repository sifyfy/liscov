// Core modules
pub mod config_manager;
pub mod memory_optimized; // メモリ効率最適化
pub mod message_processor; // メッセージ処理パイプライン
pub mod models; // 既存のデータ構造は継続使用
pub mod plugin_system; // プラグインアーキテクチャ
pub mod plugins; // サンプルプラグイン
pub mod services; // 既存のAPIサービスは継続使用
pub mod traits; // トレイトベース設計
pub mod unified_config; // 統一設定管理システム
pub mod utils; // ユーティリティ関数は継続使用 // 設定管理モジュール

// Dioxus UI components - 新アーキテクチャ対応
pub mod components; // 🆕 UI コンポーネント（有効化）
pub mod hooks; // LiveChatフック有効化
pub mod styles; // スタイル有効化

// Core functionality exports - specific imports to avoid ambiguous glob re-exports
pub use models::{ActiveTab, GuiChatMessage, MessageType};
pub use services::*;

// New state management modules
pub mod live_chat_service;
pub mod state_management;
pub mod ui_sync_service;

// Temporarily disable problematic state module
// pub mod state;

pub use live_chat_service::*;
pub use state_management::{get_state_manager, AppEvent, StateManager};
pub use ui_sync_service::*;

// Export new modern components - 動作するもののみ
pub use components::MainWindow;

// Temporarily disable problematic state module
// pub use state::*;
