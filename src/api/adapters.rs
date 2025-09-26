//! 既存APIのジェネリックAPI統合アダプター
//!
//! 既存のYouTube API、Database APIなどをジェネリックAPIシステムで使用できるように適応

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::analytics::data_exporter::DataExporter;
use crate::api::generic::*;
use crate::database::LiscovDatabase;
use crate::gui::models::GuiChatMessage;
use crate::LiscovResult;

/// YouTube APIアダプター
pub struct YouTubeApiAdapter {
    client: Box<dyn GenericApiClient>,
}

/// YouTube Live Chatリクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatRequest {
    pub video_id: String,
    pub continuation_token: Option<String>,
}

/// YouTube Live Chatレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatResponse {
    pub messages: Vec<GuiChatMessage>,
    pub continuation_token: Option<String>,
    pub polling_interval_ms: u64,
}

impl YouTubeApiAdapter {
    /// 新しいアダプターを作成
    pub fn new(client: Box<dyn GenericApiClient>) -> Self {
        Self { client }
    }

    /// Live Chatデータを取得
    pub async fn get_live_chat(&self, request: LiveChatRequest) -> LiscovResult<LiveChatResponse> {
        // ジェネリックAPIリクエストに変換
        let generic_request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: "/youtubei/v1/live_chat/get_live_chat_replay".to_string(),
            method: HttpMethod::POST,
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers
            },
            body: Some(serde_json::json!({
                "context": {
                    "client": {
                        "clientName": "WEB",
                        "clientVersion": "2.0"
                    }
                },
                "videoId": request.video_id,
                "continuation": request.continuation_token
            })),
            query_params: HashMap::new(),
            timeout_ms: Some(15000),
            retry_config: Some(RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 2000,
                backoff_multiplier: 1.5,
                max_delay_ms: 30000,
                retryable_status_codes: vec![429, 500, 502, 503, 504],
            }),
        };

        // APIを呼び出し
        let response: GenericResponse<serde_json::Value> =
            self.client.send_json_request(generic_request).await?;

        // レスポンスをパース（簡略化）
        if let Some(body) = response.body {
            // 実際の実装では、YouTube APIのレスポンス形式に基づいて詳細なパースを行う
            let messages = self.parse_youtube_messages(&body)?;
            let continuation_token = self.extract_continuation_token(&body);

            Ok(LiveChatResponse {
                messages,
                continuation_token,
                polling_interval_ms: 2000, // デフォルト値
            })
        } else {
            Err(crate::ApiError::InvalidFormat.into())
        }
    }

    /// YouTubeレスポンスからメッセージをパース
    fn parse_youtube_messages(
        &self,
        data: &serde_json::Value,
    ) -> LiscovResult<Vec<GuiChatMessage>> {
        // TODO: 実際のYouTube APIレスポンス形式に基づいてパース
        // ここでは簡略化された実装
        let mut messages = Vec::new();

        if let Some(actions) = data
            .get("continuationContents")
            .and_then(|c| c.get("liveChatContinuation"))
            .and_then(|l| l.get("actions"))
            .and_then(|a| a.as_array())
        {
            for action in actions {
                if let Some(message) = self.parse_single_message(action)? {
                    messages.push(message);
                }
            }
        }

        Ok(messages)
    }

    /// 単一メッセージのパース
    fn parse_single_message(
        &self,
        action: &serde_json::Value,
    ) -> LiscovResult<Option<GuiChatMessage>> {
        // YouTube APIの実際の形式に基づいてパースする必要がある
        // ここでは簡略化
        if let Some(_replay_chat_item) = action
            .get("replayChatItemAction")
            .and_then(|r| r.get("actions"))
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
        {
            // メッセージの詳細をパース
            // TODO: 実装を完成させる
            Ok(None)
        } else {
            Ok(None)
        }
    }

    /// 継続トークンを抽出
    fn extract_continuation_token(&self, data: &serde_json::Value) -> Option<String> {
        data.get("continuationContents")
            .and_then(|c| c.get("liveChatContinuation"))
            .and_then(|l| l.get("continuations"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|cont| cont.get("invalidationContinuationData"))
            .and_then(|inv| inv.get("continuation"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
    }
}

/// データベースAPIアダプター
pub struct DatabaseApiAdapter {
    client: Box<dyn GenericApiClient>,
}

impl DatabaseApiAdapter {
    /// 新しいアダプターを作成
    pub fn new(_database: LiscovDatabase, client: Box<dyn GenericApiClient>) -> Self {
        Self { client }
    }

    /// データベース操作をジェネリックAPIとして実行
    pub async fn execute_query(&self, query: DatabaseQuery) -> LiscovResult<DatabaseQueryResult> {
        match query.operation {
            DatabaseOperation::Select => self.execute_select(query).await,
            DatabaseOperation::Insert => self.execute_insert(query).await,
            DatabaseOperation::Update => self.execute_update(query).await,
            DatabaseOperation::Delete => self.execute_delete(query).await,
        }
    }

    async fn execute_select(&self, query: DatabaseQuery) -> LiscovResult<DatabaseQueryResult> {
        // TODO: 実際のSQLクエリ実行
        // ここでは簡略化

        let generic_request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: format!("/db/{}", query.table),
            method: HttpMethod::GET,
            headers: HashMap::new(),
            body: query.parameters,
            query_params: HashMap::new(),
            timeout_ms: Some(5000),
            retry_config: None,
        };

        let response: GenericResponse<serde_json::Value> =
            self.client.send_json_request(generic_request).await?;

        Ok(DatabaseQueryResult {
            affected_rows: 0,
            data: response.body.unwrap_or_default(),
            execution_time_ms: response.response_time_ms,
        })
    }

    async fn execute_insert(&self, query: DatabaseQuery) -> LiscovResult<DatabaseQueryResult> {
        let generic_request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: format!("/db/{}", query.table),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            body: query.parameters,
            query_params: HashMap::new(),
            timeout_ms: Some(5000),
            retry_config: None,
        };

        let response: GenericResponse<serde_json::Value> =
            self.client.send_json_request(generic_request).await?;

        Ok(DatabaseQueryResult {
            affected_rows: 1, // 簡略化
            data: response.body.unwrap_or_default(),
            execution_time_ms: response.response_time_ms,
        })
    }

    async fn execute_update(&self, query: DatabaseQuery) -> LiscovResult<DatabaseQueryResult> {
        let generic_request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: format!("/db/{}", query.table),
            method: HttpMethod::PUT,
            headers: HashMap::new(),
            body: query.parameters,
            query_params: HashMap::new(),
            timeout_ms: Some(5000),
            retry_config: None,
        };

        let response: GenericResponse<serde_json::Value> =
            self.client.send_json_request(generic_request).await?;

        Ok(DatabaseQueryResult {
            affected_rows: 1, // 簡略化
            data: response.body.unwrap_or_default(),
            execution_time_ms: response.response_time_ms,
        })
    }

    async fn execute_delete(&self, query: DatabaseQuery) -> LiscovResult<DatabaseQueryResult> {
        let generic_request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: format!("/db/{}", query.table),
            method: HttpMethod::DELETE,
            headers: HashMap::new(),
            body: query.parameters,
            query_params: HashMap::new(),
            timeout_ms: Some(5000),
            retry_config: None,
        };

        let response: GenericResponse<serde_json::Value> =
            self.client.send_json_request(generic_request).await?;

        Ok(DatabaseQueryResult {
            affected_rows: 1, // 簡略化
            data: response.body.unwrap_or_default(),
            execution_time_ms: response.response_time_ms,
        })
    }
}

/// データベースクエリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseQuery {
    pub table: String,
    pub operation: DatabaseOperation,
    pub parameters: Option<serde_json::Value>,
    pub conditions: Option<HashMap<String, serde_json::Value>>,
}

/// データベース操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseOperation {
    Select,
    Insert,
    Update,
    Delete,
}

/// データベースクエリ結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseQueryResult {
    pub affected_rows: u64,
    pub data: serde_json::Value,
    pub execution_time_ms: u64,
}

/// アナリティクスAPIアダプター
pub struct AnalyticsApiAdapter {
    client: Box<dyn GenericApiClient>,
}

impl AnalyticsApiAdapter {
    /// 新しいアダプターを作成
    pub fn new(_exporter: DataExporter, client: Box<dyn GenericApiClient>) -> Self {
        Self { client }
    }

    /// アナリティクスデータを取得
    pub async fn get_analytics(
        &self,
        request: AnalyticsRequest,
    ) -> LiscovResult<AnalyticsResponse> {
        let generic_request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: "/analytics/data".to_string(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            body: Some(serde_json::to_value(&request)?),
            query_params: HashMap::new(),
            timeout_ms: Some(30000),
            retry_config: Some(RetryConfig::default()),
        };

        let response: GenericResponse<serde_json::Value> =
            self.client.send_json_request(generic_request).await?;

        if let Some(body) = response.body {
            Ok(serde_json::from_value(body)?)
        } else {
            Err(crate::ApiError::InvalidFormat.into())
        }
    }

    /// レポートを生成
    pub async fn generate_report(&self, request: ReportRequest) -> LiscovResult<ReportResponse> {
        let generic_request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: "/analytics/report".to_string(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            body: Some(serde_json::to_value(&request)?),
            query_params: HashMap::new(),
            timeout_ms: Some(60000), // レポート生成は時間がかかる可能性がある
            retry_config: Some(RetryConfig::default()),
        };

        let response: GenericResponse<serde_json::Value> =
            self.client.send_json_request(generic_request).await?;

        if let Some(body) = response.body {
            Ok(serde_json::from_value(body)?)
        } else {
            Err(crate::ApiError::InvalidFormat.into())
        }
    }
}

/// アナリティクスリクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRequest {
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: chrono::DateTime<chrono::Utc>,
    pub metrics: Vec<String>,
    pub filters: HashMap<String, serde_json::Value>,
}

/// アナリティクスレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub data: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// レポートリクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub report_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub format: ReportFormat,
}

/// レポート形式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFormat {
    Json,
    Csv,
    Excel,
    Pdf,
}

/// レポートレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResponse {
    pub report_id: String,
    pub download_url: Option<String>,
    pub data: Option<serde_json::Value>,
    pub status: ReportStatus,
}

/// レポート状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportStatus {
    Pending,
    Processing,
    Completed,
    Failed(String),
}

/// 統合APIサービス - 全てのアダプターを管理
pub struct UnifiedApiService {
    youtube_adapter: YouTubeApiAdapter,
    database_adapter: DatabaseApiAdapter,
    analytics_adapter: AnalyticsApiAdapter,
}

impl UnifiedApiService {
    /// 新しいサービスを作成
    pub fn new(
        youtube_client: Box<dyn GenericApiClient>,
        database_client: Box<dyn GenericApiClient>,
        analytics_client: Box<dyn GenericApiClient>,
        database: LiscovDatabase,
        exporter: DataExporter,
    ) -> Self {
        Self {
            youtube_adapter: YouTubeApiAdapter::new(youtube_client),
            database_adapter: DatabaseApiAdapter::new(database, database_client),
            analytics_adapter: AnalyticsApiAdapter::new(exporter, analytics_client),
        }
    }

    /// YouTube APIアダプターにアクセス
    pub fn youtube(&self) -> &YouTubeApiAdapter {
        &self.youtube_adapter
    }

    /// データベースAPIアダプターにアクセス
    pub fn database(&self) -> &DatabaseApiAdapter {
        &self.database_adapter
    }

    /// アナリティクスAPIアダプターにアクセス
    pub fn analytics(&self) -> &AnalyticsApiAdapter {
        &self.analytics_adapter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::api::unified_client::UnifiedApiClient; // 未使用のため一時的にコメントアウト

    #[test]
    fn test_live_chat_request_serialization() {
        let request = LiveChatRequest {
            video_id: "test_video_123".to_string(),
            continuation_token: Some("continuation_123".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test_video_123"));
        assert!(json.contains("continuation_123"));
    }

    #[test]
    fn test_database_query_creation() {
        let query = DatabaseQuery {
            table: "messages".to_string(),
            operation: DatabaseOperation::Select,
            parameters: Some(serde_json::json!({"limit": 100})),
            conditions: Some({
                let mut conditions = HashMap::new();
                conditions.insert("session_id".to_string(), serde_json::json!("session_123"));
                conditions
            }),
        };

        assert_eq!(query.table, "messages");
        assert!(matches!(query.operation, DatabaseOperation::Select));
    }

    #[test]
    fn test_analytics_request() {
        let request = AnalyticsRequest {
            start_date: chrono::Utc::now() - chrono::Duration::days(7),
            end_date: chrono::Utc::now(),
            metrics: vec!["message_count".to_string(), "engagement_rate".to_string()],
            filters: {
                let mut filters = HashMap::new();
                filters.insert("channel_id".to_string(), serde_json::json!("UC123"));
                filters
            },
        };

        assert_eq!(request.metrics.len(), 2);
        assert!(request.filters.contains_key("channel_id"));
    }
}
