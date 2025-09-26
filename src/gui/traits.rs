//! GUI層のトレイト定義
//!
//! Phase 2実装: トレイトベース設計への移行

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;

use super::models::GuiChatMessage;
use super::services::ServiceState;
use crate::io::SaveConfig;

/// チャットサービスの抽象インタフェース
#[async_trait]
pub trait ChatService: Send + Sync {
    /// ライブチャット監視を開始
    async fn start_monitoring(
        &mut self,
        url: &str,
        output_file: Option<String>,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<GuiChatMessage>>;

    /// ライブチャット監視を停止
    async fn stop_monitoring(&mut self) -> anyhow::Result<()>;

    /// ライブチャット監視を一時停止
    async fn pause_monitoring(&mut self) -> anyhow::Result<()>;

    /// ライブチャット監視を再開
    async fn resume_monitoring(
        &mut self,
        output_file: Option<String>,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<GuiChatMessage>>;

    /// 現在のサービス状態を取得
    async fn get_state(&self) -> ServiceState;

    /// レスポンス保存設定を更新
    async fn update_save_config(&self, config: SaveConfig);

    /// 現在の保存設定を取得
    async fn get_save_config(&self) -> SaveConfig;

    /// 保存されたレスポンス数を取得
    async fn get_saved_response_count(&self) -> anyhow::Result<usize>;
}

/// メッセージ処理パイプラインの抽象インタフェース
#[async_trait]
pub trait MessageProcessor: Send + Sync {
    /// チャットアイテムをGUIメッセージに変換
    fn process_chat_item(
        &self,
        item: &crate::get_live_chat::ChatItem,
    ) -> Result<GuiChatMessage, ProcessingError>;

    /// メッセージバッチを処理
    async fn process_message_batch(
        &self,
        items: &[crate::get_live_chat::ChatItem],
    ) -> Result<Vec<GuiChatMessage>, ProcessingError>;

    /// メッセージのフィルタリング
    fn filter_message(&self, message: &GuiChatMessage, filter_config: &MessageFilterConfig)
        -> bool;

    /// メッセージの統計情報を更新
    fn update_statistics(&self, message: &GuiChatMessage, stats: &mut MessageStatistics);
}

/// メッセージリポジトリの抽象インタフェース
#[async_trait]
pub trait MessageRepository: Send + Sync {
    /// メッセージを保存
    async fn save_message(&self, message: &GuiChatMessage) -> Result<String, RepositoryError>;

    /// メッセージバッチを保存
    async fn save_message_batch(
        &self,
        messages: &[GuiChatMessage],
    ) -> Result<Vec<String>, RepositoryError>;

    /// メッセージを取得
    async fn get_messages(
        &self,
        query: MessageQuery,
    ) -> Result<Vec<GuiChatMessage>, RepositoryError>;

    /// メッセージ数を取得
    async fn count_messages(&self, filter: Option<MessageFilter>)
        -> Result<usize, RepositoryError>;

    /// メッセージを削除
    async fn delete_message(&self, id: &str) -> Result<(), RepositoryError>;
}

/// 設定管理の抽象インタフェース
#[async_trait]
pub trait ConfigurationManager: Send + Sync {
    /// 設定をJSON値として取得
    async fn get_config_json(&self, key: &str) -> Result<Option<serde_json::Value>, ConfigError>;

    /// 設定をJSON値として保存
    async fn set_config_json(
        &self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), ConfigError>;

    /// 設定を削除
    async fn remove_config(&self, key: &str) -> Result<(), ConfigError>;

    /// 全設定を取得
    async fn get_all_configs(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError>;

    /// JSONの妥当性を検証
    fn validate_config_json(&self, value: &serde_json::Value) -> Result<(), ConfigError>;
}

/// 型安全な設定管理のためのヘルパー関数
pub struct ConfigurationHelper;

impl ConfigurationHelper {
    /// 型安全な設定取得
    pub async fn get_typed_config<T, C>(manager: &C, key: &str) -> Result<Option<T>, ConfigError>
    where
        T: serde::de::DeserializeOwned + Send,
        C: ConfigurationManager + ?Sized,
    {
        match manager.get_config_json(key).await? {
            Some(json_value) => {
                let typed_value = serde_json::from_value(json_value).map_err(|e| {
                    ConfigError::Serialization(format!("Deserialization failed: {}", e))
                })?;
                Ok(Some(typed_value))
            }
            None => Ok(None),
        }
    }

    /// 型安全な設定保存
    pub async fn set_typed_config<T, C>(
        manager: &C,
        key: &str,
        value: &T,
    ) -> Result<(), ConfigError>
    where
        T: serde::Serialize + Send + Sync,
        C: ConfigurationManager + ?Sized,
    {
        let json_value = serde_json::to_value(value)
            .map_err(|e| ConfigError::Serialization(format!("Serialization failed: {}", e)))?;
        manager.set_config_json(key, &json_value).await
    }

    /// 型安全な設定検証
    pub fn validate_typed_config<T, C>(manager: &C, value: &T) -> Result<(), ConfigError>
    where
        T: serde::Serialize,
        C: ConfigurationManager + ?Sized,
    {
        let json_value = serde_json::to_value(value).map_err(|e| {
            ConfigError::Serialization(format!("Validation serialization failed: {}", e))
        })?;
        manager.validate_config_json(&json_value)
    }
}

/// ライブチャットファクトリの抽象インタフェース
pub trait LiveChatFactory: Send + Sync {
    /// チャットサービスを作成
    fn create_chat_service(&self) -> Box<dyn ChatService>;

    /// メッセージプロセッサを作成
    fn create_message_processor(&self) -> Box<dyn MessageProcessor>;

    /// メッセージリポジトリを作成
    fn create_message_repository(&self, config: RepositoryConfig) -> Box<dyn MessageRepository>;

    /// 設定マネージャーを作成
    fn create_config_manager(&self) -> Box<dyn ConfigurationManager>;
}

// エラー型定義

/// メッセージ処理エラー
#[derive(thiserror::Error, Debug)]
pub enum ProcessingError {
    #[error("Conversion error: {0}")]
    Conversion(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Format error: {0}")]
    Format(String),

    #[error("Processing failed: {0}")]
    Processing(String),
}

/// リポジトリエラー
#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

/// 設定エラー
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("File access error: {0}")]
    FileAccess(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

// データ構造定義

/// メッセージフィルタ設定
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageFilterConfig {
    pub include_system_messages: bool,
    pub include_super_chat: bool,
    pub include_membership: bool,
    pub author_filter: Option<String>,
    pub content_filter: Option<String>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
}

impl Default for MessageFilterConfig {
    fn default() -> Self {
        Self {
            include_system_messages: false,
            include_super_chat: true,
            include_membership: true,
            author_filter: None,
            content_filter: None,
            min_amount: None,
            max_amount: None,
        }
    }
}

/// メッセージ統計情報
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MessageStatistics {
    pub total_messages: usize,
    pub unique_authors: std::collections::HashSet<String>,
    pub super_chat_count: usize,
    pub membership_count: usize,
    pub total_revenue: f64,
    pub average_message_length: f64,
    pub emoji_count: usize,
}

/// メッセージクエリ
#[derive(Debug, Clone)]
pub struct MessageQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub author: Option<String>,
    pub message_type: Option<String>,
    pub date_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
    pub sort_order: MessageSortOrder,
}

impl Default for MessageQuery {
    fn default() -> Self {
        Self {
            limit: None,
            offset: None,
            author: None,
            message_type: None,
            date_range: None,
            sort_order: MessageSortOrder::Chronological,
        }
    }
}

/// メッセージソート順序
#[derive(Debug, Clone, Copy)]
pub enum MessageSortOrder {
    Chronological,
    ReverseChronological,
    ByAuthor,
    ByAmount,
}

/// メッセージフィルタ
#[derive(Debug, Clone)]
pub struct MessageFilter {
    pub message_type: Option<String>,
    pub author: Option<String>,
    pub content_contains: Option<String>,
    pub date_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
}

/// リポジトリ設定
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepositoryConfig {
    pub repository_type: RepositoryType,
    pub connection_string: String,
    pub batch_size: usize,
    pub auto_flush: bool,
    pub compression_enabled: bool,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            repository_type: RepositoryType::Memory,
            connection_string: "memory://".to_string(),
            batch_size: 100,
            auto_flush: true,
            compression_enabled: false,
        }
    }
}

/// リポジトリタイプ
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RepositoryType {
    Memory,
    File,
    Database,
    Network,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_filter_config_default() {
        let config = MessageFilterConfig::default();
        assert!(!config.include_system_messages);
        assert!(config.include_super_chat);
        assert!(config.include_membership);
        assert!(config.author_filter.is_none());
    }

    #[test]
    fn test_message_query_default() {
        let query = MessageQuery::default();
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
        assert!(matches!(query.sort_order, MessageSortOrder::Chronological));
    }

    #[test]
    fn test_repository_config_default() {
        let config = RepositoryConfig::default();
        assert!(matches!(config.repository_type, RepositoryType::Memory));
        assert_eq!(config.batch_size, 100);
        assert!(config.auto_flush);
    }

    #[test]
    fn test_message_statistics_default() {
        let stats = MessageStatistics::default();
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.super_chat_count, 0);
        assert_eq!(stats.total_revenue, 0.0);
    }
}
