//! サンプルプラグイン実装
//! 
//! プラグインシステムの使用例とテスト用プラグイン

pub mod message_filter_plugin;
pub mod analytics_plugin;
pub mod notification_plugin;

// 公開API
pub use message_filter_plugin::MessageFilterPlugin;
pub use analytics_plugin::AnalyticsPlugin;
pub use notification_plugin::NotificationPlugin;