// Dioxus GUI Components Module
// Phase 2: Core Component Migration

// 一時的に無効化 - 依存関係問題を解決後に段階的に有効化
pub mod auth_panel; // メンバー限定配信認証パネル
pub mod chat_display;
pub mod chat_header;
pub mod export_panel;
pub mod filter_panel;
pub mod input_section;
pub mod main_window;
pub mod raw_response_settings;
pub mod revenue_dashboard;
pub mod signal_analyzer; // Phase 4.1: Signal分析パネル
pub mod status_panel;
pub mod tab_navigation;
pub mod tts_settings; // TTS読み上げ設定
pub mod viewer_info_panel; // 視聴者情報管理パネル
pub mod viewer_management; // 視聴者管理タブ

// Re-exports for convenience - 新アーキテクチャのみ
pub use auth_panel::{AuthContext, AuthPanel};
pub use chat_display::ChatDisplay;
pub use export_panel::ExportPanel;
pub use filter_panel::FilterPanel;
pub use input_section::{CompactInputSection, InputSection};
pub use main_window::MainWindow;
pub use revenue_dashboard::RevenueDashboard;
pub use status_panel::{CompactStatusPanel, StatusPanel};
pub use tab_navigation::{TabContent, TabNavigation};
pub use viewer_info_panel::ViewerInfoPanel;
pub use viewer_management::ViewerManagementTab;
