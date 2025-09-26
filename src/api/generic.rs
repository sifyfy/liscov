//! ジェネリックAPI統一システム
//!
//! Phase 3実装: 異なるAPIを統一されたインターフェースで抽象化

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::LiscovResult;

/// ジェネリックAPIリクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericRequest<T = serde_json::Value> {
    /// リクエストID（追跡用）
    pub id: String,
    /// APIエンドポイント
    pub endpoint: String,
    /// HTTPメソッド
    pub method: HttpMethod,
    /// リクエストヘッダー
    pub headers: HashMap<String, String>,
    /// リクエストボディ
    pub body: Option<T>,
    /// クエリパラメータ
    pub query_params: HashMap<String, String>,
    /// タイムアウト（ミリ秒）
    pub timeout_ms: Option<u64>,
    /// リトライ設定
    pub retry_config: Option<RetryConfig>,
}

/// ジェネリックAPIレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericResponse<T = serde_json::Value> {
    /// リクエストID
    pub request_id: String,
    /// HTTPステータスコード
    pub status_code: u16,
    /// レスポンスヘッダー
    pub headers: HashMap<String, String>,
    /// レスポンスボディ
    pub body: Option<T>,
    /// エラーメッセージ（エラー時）
    pub error: Option<String>,
    /// レスポンス時間（ミリ秒）
    pub response_time_ms: u64,
    /// メタデータ
    pub metadata: HashMap<String, serde_json::Value>,
}

/// HTTPメソッド
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

/// リトライ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大リトライ回数
    pub max_attempts: u32,
    /// 初期待機時間（ミリ秒）
    pub initial_delay_ms: u64,
    /// 指数バックオフの倍率
    pub backoff_multiplier: f64,
    /// 最大待機時間（ミリ秒）
    pub max_delay_ms: u64,
    /// リトライ対象のステータスコード
    pub retryable_status_codes: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
            retryable_status_codes: vec![500, 502, 503, 504, 429],
        }
    }
}

/// ジェネリックAPIクライアントトレイト（dyn互換版）
#[async_trait]
pub trait GenericApiClient: Send + Sync {
    /// JSONリクエストを送信（dyn互換）
    async fn send_json_request(
        &self,
        request: GenericRequest<serde_json::Value>,
    ) -> LiscovResult<GenericResponse<serde_json::Value>>;

    /// クライアント設定を取得
    fn get_config(&self) -> &ApiClientConfig;

    /// ヘルスチェック
    async fn health_check(&self) -> LiscovResult<bool>;
}

/// 型付きAPIクライアント拡張トレイト
#[async_trait]
pub trait TypedApiClient: GenericApiClient {
    /// 型付きリクエストを送信
    async fn send_request<TReq, TRes>(
        &self,
        request: GenericRequest<TReq>,
    ) -> LiscovResult<GenericResponse<TRes>>
    where
        TReq: Serialize + Send + Sync,
        TRes: DeserializeOwned + Send + Sync;

    /// GET リクエストの便利メソッド
    async fn get<TRes>(&self, endpoint: &str) -> LiscovResult<GenericResponse<TRes>>
    where
        TRes: DeserializeOwned + Send + Sync,
    {
        let request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: endpoint.to_string(),
            method: HttpMethod::GET,
            headers: HashMap::new(),
            body: None::<serde_json::Value>,
            query_params: HashMap::new(),
            timeout_ms: None,
            retry_config: None,
        };

        self.send_request(request).await
    }

    /// POST リクエストの便利メソッド
    async fn post<TReq, TRes>(
        &self,
        endpoint: &str,
        body: TReq,
    ) -> LiscovResult<GenericResponse<TRes>>
    where
        TReq: Serialize + Send + Sync,
        TRes: DeserializeOwned + Send + Sync,
    {
        let request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: endpoint.to_string(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            body: Some(body),
            query_params: HashMap::new(),
            timeout_ms: None,
            retry_config: None,
        };

        self.send_request(request).await
    }
}

/// APIクライアント設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiClientConfig {
    /// ベースURL
    pub base_url: String,
    /// デフォルトタイムアウト（ミリ秒）
    pub default_timeout_ms: u64,
    /// デフォルトヘッダー
    pub default_headers: HashMap<String, String>,
    /// リトライ設定
    pub default_retry_config: RetryConfig,
    /// レート制限設定
    pub rate_limit: Option<RateLimitConfig>,
    /// 認証設定
    pub auth_config: Option<AuthConfig>,
}

/// レート制限設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 期間（秒）
    pub window_seconds: u64,
    /// 期間内の最大リクエスト数
    pub max_requests: u32,
}

/// 認証設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// 認証方式
    pub auth_type: AuthType,
    /// APIキー（API Key認証用）
    pub api_key: Option<String>,
    /// Bearer トークン（Bearer Token認証用）
    pub bearer_token: Option<String>,
    /// カスタムヘッダー
    pub custom_headers: HashMap<String, String>,
}

/// 認証方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    ApiKey { header_name: String },
    BearerToken,
    Custom { headers: HashMap<String, String> },
}

/// CRUD操作用のジェネリックリポジトリトレイト
#[async_trait]
pub trait GenericRepository<T, K>: Send + Sync
where
    T: Serialize + DeserializeOwned + Send + Sync,
    K: Serialize + Send + Sync,
{
    /// エンティティを作成
    async fn create(&self, entity: T) -> LiscovResult<T>;

    /// IDでエンティティを取得
    async fn get_by_id(&self, id: K) -> LiscovResult<Option<T>>;

    /// 全エンティティを取得
    async fn get_all(&self) -> LiscovResult<Vec<T>>;

    /// エンティティを更新
    async fn update(&self, id: K, entity: T) -> LiscovResult<T>;

    /// エンティティを削除
    async fn delete(&self, id: K) -> LiscovResult<bool>;

    /// 条件に基づくクエリ
    async fn query(&self, filters: HashMap<String, serde_json::Value>) -> LiscovResult<Vec<T>>;

    /// ページング付きクエリ
    async fn query_paged(
        &self,
        filters: HashMap<String, serde_json::Value>,
        page: u32,
        size: u32,
    ) -> LiscovResult<PagedResult<T>>;
}

/// ページング結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResult<T> {
    /// データ
    pub data: Vec<T>,
    /// 現在のページ
    pub page: u32,
    /// ページサイズ
    pub size: u32,
    /// 総件数
    pub total_count: u64,
    /// 総ページ数
    pub total_pages: u32,
    /// 前のページが存在するか
    pub has_previous: bool,
    /// 次のページが存在するか
    pub has_next: bool,
}

/// リアルタイムデータストリーム用トレイト
#[async_trait]
pub trait GenericDataStream<T>: Send + Sync
where
    T: DeserializeOwned + Send + Sync,
{
    /// ストリームを開始
    async fn start_stream(&mut self, config: StreamConfig) -> LiscovResult<()>;

    /// ストリームを停止
    async fn stop_stream(&mut self) -> LiscovResult<()>;

    /// 次のデータを取得（非ブロッキング）
    async fn next_data(&mut self) -> LiscovResult<Option<T>>;

    /// ストリームの状態を取得
    fn get_stream_status(&self) -> StreamStatus;

    /// ストリーム統計を取得
    fn get_stream_stats(&self) -> StreamStats;
}

/// ストリーム設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// エンドポイント
    pub endpoint: String,
    /// バッファサイズ
    pub buffer_size: usize,
    /// 再接続間隔（ミリ秒）
    pub reconnect_interval_ms: u64,
    /// 最大再接続試行回数
    pub max_reconnect_attempts: u32,
    /// ハートビート間隔（ミリ秒）
    pub heartbeat_interval_ms: Option<u64>,
}

/// ストリーム状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// ストリーム統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    /// 接続開始時刻
    pub connection_start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 受信メッセージ数
    pub messages_received: u64,
    /// バイト受信数
    pub bytes_received: u64,
    /// 再接続回数
    pub reconnect_count: u32,
    /// 最後のエラー
    pub last_error: Option<String>,
    /// 平均レイテンシ（ミリ秒）
    pub average_latency_ms: f64,
}

/// キャッシュ機能付きAPIクライアント用トレイト
#[async_trait]
pub trait CachedApiClient<K, V>: GenericApiClient
where
    K: Serialize + DeserializeOwned + Send + Sync + Clone + std::hash::Hash + Eq,
    V: Serialize + DeserializeOwned + Send + Sync + Clone,
{
    /// キャッシュからデータを取得（ミスした場合はAPIから取得）
    async fn get_cached(&self, key: K) -> LiscovResult<V>;

    /// キャッシュを手動で更新
    async fn refresh_cache(&self, key: K) -> LiscovResult<V>;

    /// キャッシュをクリア
    async fn clear_cache(&self) -> LiscovResult<()>;

    /// キャッシュ統計を取得
    fn get_cache_stats(&self) -> CacheStats;
}

/// キャッシュ統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// ヒット数
    pub hits: u64,
    /// ミス数
    pub misses: u64,
    /// ヒット率
    pub hit_rate: f64,
    /// キャッシュサイズ
    pub cache_size: usize,
    /// 最後のクリア時刻
    pub last_clear_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// バッチ操作用トレイト
#[async_trait]
pub trait BatchApiClient: GenericApiClient {
    /// 複数のリクエストをバッチで実行
    async fn send_batch<TReq, TRes>(
        &self,
        requests: Vec<GenericRequest<TReq>>,
    ) -> LiscovResult<Vec<GenericResponse<TRes>>>
    where
        TReq: Serialize + Send + Sync,
        TRes: DeserializeOwned + Send + Sync;

    /// バッチサイズの制限を取得
    fn get_max_batch_size(&self) -> usize;
}

/// API呼び出しの統計・メトリクス用トレイト
pub trait ApiMetrics: Send + Sync {
    /// リクエスト開始を記録
    fn record_request_start(&self, endpoint: &str, method: &HttpMethod);

    /// リクエスト完了を記録
    fn record_request_complete(
        &self,
        endpoint: &str,
        method: &HttpMethod,
        status_code: u16,
        duration_ms: u64,
    );

    /// エラーを記録
    fn record_error(&self, endpoint: &str, method: &HttpMethod, error_type: &str);

    /// メトリクスを取得
    fn get_metrics(&self) -> ApiMetricsSnapshot;
}

/// APIメトリクスのスナップショット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetricsSnapshot {
    /// エンドポイント別統計
    pub endpoint_stats: HashMap<String, EndpointStats>,
    /// 全体統計
    pub global_stats: GlobalStats,
    /// 生成時刻
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// エンドポイント別統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStats {
    /// リクエスト数
    pub request_count: u64,
    /// 成功数
    pub success_count: u64,
    /// エラー数
    pub error_count: u64,
    /// 平均レスポンス時間（ミリ秒）
    pub average_response_time_ms: f64,
    /// 最小レスポンス時間（ミリ秒）
    pub min_response_time_ms: u64,
    /// 最大レスポンス時間（ミリ秒）
    pub max_response_time_ms: u64,
}

/// 全体統計
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalStats {
    /// 総リクエスト数
    pub total_requests: u64,
    /// 総成功数
    pub total_success: u64,
    /// 総エラー数
    pub total_errors: u64,
    /// 全体成功率
    pub success_rate: f64,
    /// 平均レスポンス時間（ミリ秒）
    pub average_response_time_ms: f64,
}

/// レスポンス変換用のユーティリティ関数
pub struct ResponseMapper;

impl ResponseMapper {
    /// JSONレスポンスを特定の型に変換
    pub fn map_json_response<T>(
        response: GenericResponse<serde_json::Value>,
    ) -> LiscovResult<GenericResponse<T>>
    where
        T: DeserializeOwned,
    {
        let mapped_body = if let Some(body) = response.body {
            Some(serde_json::from_value(body)?)
        } else {
            None
        };

        Ok(GenericResponse {
            request_id: response.request_id,
            status_code: response.status_code,
            headers: response.headers,
            body: mapped_body,
            error: response.error,
            response_time_ms: response.response_time_ms,
            metadata: response.metadata,
        })
    }

    /// エラーレスポンスを作成
    pub fn create_error_response<T>(
        request_id: String,
        error_message: String,
    ) -> GenericResponse<T> {
        GenericResponse {
            request_id,
            status_code: 500,
            headers: HashMap::new(),
            body: None,
            error: Some(error_message),
            response_time_ms: 0,
            metadata: HashMap::new(),
        }
    }
}

/// 非同期タスクのユーティリティ
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// APIクライアントファクトリー
pub trait ApiClientFactory: Send + Sync {
    /// 設定からクライアントを作成
    fn create_client(&self, config: ApiClientConfig) -> Box<dyn GenericApiClient>;

    /// 特定の用途向けのクライアントを作成
    fn create_youtube_client(&self) -> LiscovResult<Box<dyn GenericApiClient>>;
    fn create_database_client(&self) -> LiscovResult<Box<dyn GenericApiClient>>;
    fn create_analytics_client(&self) -> LiscovResult<Box<dyn GenericApiClient>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_request_creation() {
        let request = GenericRequest {
            id: "test-123".to_string(),
            endpoint: "/api/test".to_string(),
            method: HttpMethod::GET,
            headers: HashMap::new(),
            body: Some(serde_json::json!({"test": "data"})),
            query_params: HashMap::new(),
            timeout_ms: Some(5000),
            retry_config: Some(RetryConfig::default()),
        };

        assert_eq!(request.id, "test-123");
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.timeout_ms, Some(5000));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_paged_result() {
        let result = PagedResult {
            data: vec![1, 2, 3],
            page: 1,
            size: 10,
            total_count: 100,
            total_pages: 10,
            has_previous: false,
            has_next: true,
        };

        assert_eq!(result.data.len(), 3);
        assert_eq!(result.total_pages, 10);
        assert!(!result.has_previous);
        assert!(result.has_next);
    }

    #[test]
    fn test_response_mapper_error_creation() {
        let error_response: GenericResponse<String> =
            ResponseMapper::create_error_response("req-123".to_string(), "Test error".to_string());

        assert_eq!(error_response.request_id, "req-123");
        assert_eq!(error_response.status_code, 500);
        assert_eq!(error_response.error, Some("Test error".to_string()));
        assert!(error_response.body.is_none());
    }
}
