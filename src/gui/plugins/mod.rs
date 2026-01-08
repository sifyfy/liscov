//! サンプルプラグイン実装
//!
//! プラグインシステムの使用例とテスト用プラグイン

pub mod analytics_plugin;
pub mod message_filter_plugin;
pub mod notification_plugin;
pub mod tts_plugin;

// 公開API
pub use analytics_plugin::AnalyticsPlugin;
pub use message_filter_plugin::MessageFilterPlugin;
pub use notification_plugin::NotificationPlugin;
pub use tts_plugin::TtsPlugin;
