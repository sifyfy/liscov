//! 統合APIクライアント実装
//!
//! ジェネリックAPIシステムの具象実装

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::api::generic::*;
use crate::LiscovResult;

/// 統合APIクライアント
pub struct UnifiedApiClient {
    config: ApiClientConfig,
    http_client: reqwest::Client,
    metrics: Arc<RwLock<UnifiedApiMetrics>>,
}

/// 統合APIメトリクス実装
#[derive(Debug, Clone, Default)]
struct UnifiedApiMetrics {
    endpoint_stats: HashMap<String, EndpointStats>,
    global_stats: GlobalStats,
}

impl UnifiedApiClient {
    /// 新しいクライアントを作成
    pub fn new(config: ApiClientConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.default_timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            metrics: Arc::new(RwLock::new(UnifiedApiMetrics::default())),
        }
    }

    /// デフォルト設定でクライアントを作成
    pub fn with_default_config() -> Self {
        let config = ApiClientConfig {
            base_url: "https://api.example.com".to_string(),
            default_timeout_ms: 10000,
            default_headers: {
                let mut headers = HashMap::new();
                headers.insert("User-Agent".to_string(), "Liscov/1.0".to_string());
                headers.insert("Accept".to_string(), "application/json".to_string());
                headers
            },
            default_retry_config: RetryConfig::default(),
            rate_limit: None,
            auth_config: None,
        };

        Self::new(config)
    }

    /// YouTube API用の設定でクライアントを作成
    pub fn for_youtube() -> Self {
        let config = ApiClientConfig {
            base_url: "https://www.youtube.com".to_string(),
            default_timeout_ms: 15000,
            default_headers: {
                let mut headers = HashMap::new();
                headers.insert(
                    "User-Agent".to_string(),
                    "Mozilla/5.0 (compatible; Liscov/1.0)".to_string(),
                );
                headers.insert(
                    "Accept".to_string(),
                    "application/json, text/html".to_string(),
                );
                headers
            },
            default_retry_config: RetryConfig {
                max_attempts: 5,
                initial_delay_ms: 2000,
                backoff_multiplier: 1.5,
                max_delay_ms: 60000,
                retryable_status_codes: vec![429, 500, 502, 503, 504],
            },
            rate_limit: Some(RateLimitConfig {
                window_seconds: 60,
                max_requests: 100,
            }),
            auth_config: None,
        };

        Self::new(config)
    }

    /// データベース用の設定でクライアントを作成
    pub fn for_database() -> Self {
        let config = ApiClientConfig {
            base_url: "file://".to_string(), // ローカルファイルベース
            default_timeout_ms: 5000,
            default_headers: HashMap::new(),
            default_retry_config: RetryConfig {
                max_attempts: 2,
                initial_delay_ms: 500,
                backoff_multiplier: 2.0,
                max_delay_ms: 5000,
                retryable_status_codes: vec![],
            },
            rate_limit: None,
            auth_config: None,
        };

        Self::new(config)
    }

    /// アナリティクス用の設定でクライアントを作成
    pub fn for_analytics() -> Self {
        let config = ApiClientConfig {
            base_url: "internal://analytics".to_string(),
            default_timeout_ms: 30000,
            default_headers: HashMap::new(),
            default_retry_config: RetryConfig::default(),
            rate_limit: None,
            auth_config: None,
        };

        Self::new(config)
    }

    /// リクエストを実際に実行
    async fn execute_http_request<TReq, TRes>(
        &self,
        request: GenericRequest<TReq>,
    ) -> LiscovResult<GenericResponse<TRes>>
    where
        TReq: Serialize + Send + Sync,
        TRes: DeserializeOwned + Send + Sync,
    {
        let start_time = chrono::Utc::now();

        // メトリクス記録開始
        self.record_request_start(&request.endpoint, &request.method);

        // URLを構築
        let full_url = if request.endpoint.starts_with("http") {
            request.endpoint.clone()
        } else {
            format!("{}{}", self.config.base_url, request.endpoint)
        };

        // HTTPリクエストを構築
        let mut http_request = match request.method {
            HttpMethod::GET => self.http_client.get(&full_url),
            HttpMethod::POST => self.http_client.post(&full_url),
            HttpMethod::PUT => self.http_client.put(&full_url),
            HttpMethod::DELETE => self.http_client.delete(&full_url),
            HttpMethod::PATCH => self.http_client.patch(&full_url),
            HttpMethod::HEAD => self.http_client.head(&full_url),
            HttpMethod::OPTIONS => {
                // reqwestにはoptionsメソッドがないため、request()を使用
                self.http_client
                    .request(reqwest::Method::OPTIONS, &full_url)
            }
        };

        // ヘッダーを追加
        for (key, value) in &self.config.default_headers {
            http_request = http_request.header(key, value);
        }
        for (key, value) in &request.headers {
            http_request = http_request.header(key, value);
        }

        // クエリパラメータを追加
        if !request.query_params.is_empty() {
            http_request = http_request.query(&request.query_params);
        }

        // ボディを追加（POSTなどの場合）
        if let Some(body) = request.body {
            http_request = http_request.json(&body);
        }

        // タイムアウトを設定
        if let Some(timeout_ms) = request.timeout_ms {
            http_request = http_request.timeout(std::time::Duration::from_millis(timeout_ms));
        }

        // リクエストを送信（リトライ付き）
        let response = self
            .execute_with_retry(http_request, &request.retry_config)
            .await?;

        let end_time = chrono::Utc::now();
        let duration_ms = end_time
            .signed_duration_since(start_time)
            .num_milliseconds() as u64;

        // レスポンスを解析
        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let response_body = if status_code >= 200 && status_code < 300 {
            let text = response.text().await?;
            if text.is_empty() {
                None
            } else {
                Some(serde_json::from_str(&text)?)
            }
        } else {
            None
        };

        // メトリクス記録完了
        self.record_request_complete(&request.endpoint, &request.method, status_code, duration_ms);

        let generic_response = GenericResponse {
            request_id: request.id,
            status_code,
            headers,
            body: response_body,
            error: if status_code >= 400 {
                Some(format!("HTTP {}", status_code))
            } else {
                None
            },
            response_time_ms: duration_ms,
            metadata: HashMap::new(),
        };

        Ok(generic_response)
    }

    /// リトライ付きでリクエストを実行
    async fn execute_with_retry(
        &self,
        request_builder: reqwest::RequestBuilder,
        retry_config: &Option<RetryConfig>,
    ) -> LiscovResult<reqwest::Response> {
        let retry_config = retry_config
            .as_ref()
            .unwrap_or(&self.config.default_retry_config);
        let mut attempts = 0;
        let mut delay_ms = retry_config.initial_delay_ms;

        loop {
            attempts += 1;

            // リクエストを複製（RequestBuilderは消費されるため）
            let cloned_request = request_builder.try_clone().ok_or_else(|| {
                let io_error =
                    std::io::Error::new(std::io::ErrorKind::Other, "Request cloning failed");
                crate::LiscovError::StdIo(io_error)
            })?;

            match cloned_request.send().await {
                Ok(response) => {
                    if response.status().is_success()
                        || !retry_config
                            .retryable_status_codes
                            .contains(&response.status().as_u16())
                        || attempts >= retry_config.max_attempts
                    {
                        return Ok(response);
                    }

                    // リトライ可能なエラーの場合は待機
                    if attempts < retry_config.max_attempts {
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        delay_ms = ((delay_ms as f64) * retry_config.backoff_multiplier) as u64;
                        delay_ms = delay_ms.min(retry_config.max_delay_ms);
                        continue;
                    }

                    return Ok(response);
                }
                Err(e) => {
                    if attempts >= retry_config.max_attempts {
                        return Err(crate::LiscovError::Network(e));
                    }

                    // ネットワークエラーの場合もリトライ
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = ((delay_ms as f64) * retry_config.backoff_multiplier) as u64;
                    delay_ms = delay_ms.min(retry_config.max_delay_ms);
                }
            }
        }
    }
}

#[async_trait]
impl GenericApiClient for UnifiedApiClient {
    async fn send_json_request(
        &self,
        request: GenericRequest<serde_json::Value>,
    ) -> LiscovResult<GenericResponse<serde_json::Value>> {
        self.execute_http_request(request).await
    }

    fn get_config(&self) -> &ApiClientConfig {
        &self.config
    }

    async fn health_check(&self) -> LiscovResult<bool> {
        // 簡単なヘルスチェック実装
        let health_endpoint = format!("{}/health", self.config.base_url);
        let request = GenericRequest {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint: health_endpoint,
            method: HttpMethod::GET,
            headers: HashMap::new(),
            body: None,
            query_params: HashMap::new(),
            timeout_ms: Some(5000),
            retry_config: None,
        };
        match self.send_json_request(request).await {
            Ok(response) => Ok(response.status_code >= 200 && response.status_code < 300),
            Err(_) => Ok(false),
        }
    }
}

#[async_trait]
impl TypedApiClient for UnifiedApiClient {
    async fn send_request<TReq, TRes>(
        &self,
        request: GenericRequest<TReq>,
    ) -> LiscovResult<GenericResponse<TRes>>
    where
        TReq: Serialize + Send + Sync,
        TRes: DeserializeOwned + Send + Sync,
    {
        self.execute_http_request(request).await
    }
}

impl ApiMetrics for UnifiedApiClient {
    fn record_request_start(&self, endpoint: &str, _method: &HttpMethod) {
        let mut metrics = self.metrics.write();

        // エンドポイント統計を初期化（存在しない場合）
        metrics
            .endpoint_stats
            .entry(endpoint.to_string())
            .or_insert_with(|| EndpointStats {
                request_count: 0,
                success_count: 0,
                error_count: 0,
                average_response_time_ms: 0.0,
                min_response_time_ms: u64::MAX,
                max_response_time_ms: 0,
            });

        // グローバル統計を更新
        metrics.global_stats.total_requests += 1;
    }

    fn record_request_complete(
        &self,
        endpoint: &str,
        _method: &HttpMethod,
        status_code: u16,
        duration_ms: u64,
    ) {
        let mut metrics = self.metrics.write();

        // エンドポイント統計を先に更新
        let (success_count_delta, error_count_delta) = {
            if let Some(endpoint_stats) = metrics.endpoint_stats.get_mut(endpoint) {
                endpoint_stats.request_count += 1;

                let (success_delta, error_delta) = if status_code >= 200 && status_code < 300 {
                    endpoint_stats.success_count += 1;
                    (1, 0)
                } else {
                    endpoint_stats.error_count += 1;
                    (0, 1)
                };

                // レスポンス時間統計を更新
                let current_avg = endpoint_stats.average_response_time_ms;
                let count = endpoint_stats.request_count as f64;
                endpoint_stats.average_response_time_ms =
                    (current_avg * (count - 1.0) + duration_ms as f64) / count;

                endpoint_stats.min_response_time_ms =
                    endpoint_stats.min_response_time_ms.min(duration_ms);
                endpoint_stats.max_response_time_ms =
                    endpoint_stats.max_response_time_ms.max(duration_ms);

                (success_delta, error_delta)
            } else {
                (0, 0)
            }
        };

        // グローバル統計を更新
        metrics.global_stats.total_success += success_count_delta;
        metrics.global_stats.total_errors += error_count_delta;

        let total_requests = metrics.global_stats.total_requests as f64;
        if total_requests > 0.0 {
            metrics.global_stats.success_rate =
                (metrics.global_stats.total_success as f64) / total_requests * 100.0;

            let current_global_avg = metrics.global_stats.average_response_time_ms;
            metrics.global_stats.average_response_time_ms =
                (current_global_avg * (total_requests - 1.0) + duration_ms as f64) / total_requests;
        }
    }

    fn record_error(&self, endpoint: &str, method: &HttpMethod, error_type: &str) {
        let mut metrics = self.metrics.write();

        if let Some(endpoint_stats) = metrics.endpoint_stats.get_mut(endpoint) {
            endpoint_stats.error_count += 1;
        }

        metrics.global_stats.total_errors += 1;

        // エラータイプ別の統計も将来的に追加可能
        tracing::warn!(
            "API error recorded: {} {} - {}",
            endpoint,
            method.to_string(),
            error_type
        );
    }

    fn get_metrics(&self) -> ApiMetricsSnapshot {
        let metrics = self.metrics.read();
        ApiMetricsSnapshot {
            endpoint_stats: metrics.endpoint_stats.clone(),
            global_stats: metrics.global_stats.clone(),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl HttpMethod {
    fn to_string(&self) -> String {
        match self {
            HttpMethod::GET => "GET".to_string(),
            HttpMethod::POST => "POST".to_string(),
            HttpMethod::PUT => "PUT".to_string(),
            HttpMethod::DELETE => "DELETE".to_string(),
            HttpMethod::PATCH => "PATCH".to_string(),
            HttpMethod::HEAD => "HEAD".to_string(),
            HttpMethod::OPTIONS => "OPTIONS".to_string(),
        }
    }
}

/// APIクライアントファクトリーの実装
pub struct UnifiedApiClientFactory;

impl ApiClientFactory for UnifiedApiClientFactory {
    fn create_client(&self, config: ApiClientConfig) -> Box<dyn GenericApiClient> {
        Box::new(UnifiedApiClient::new(config))
    }

    fn create_youtube_client(&self) -> LiscovResult<Box<dyn GenericApiClient>> {
        Ok(Box::new(UnifiedApiClient::for_youtube()))
    }

    fn create_database_client(&self) -> LiscovResult<Box<dyn GenericApiClient>> {
        Ok(Box::new(UnifiedApiClient::for_database()))
    }

    fn create_analytics_client(&self) -> LiscovResult<Box<dyn GenericApiClient>> {
        Ok(Box::new(UnifiedApiClient::for_analytics()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_client_creation() {
        let client = UnifiedApiClient::with_default_config();
        assert_eq!(client.config.base_url, "https://api.example.com");
        assert_eq!(client.config.default_timeout_ms, 10000);
    }

    #[test]
    fn test_youtube_client_config() {
        let client = UnifiedApiClient::for_youtube();
        assert_eq!(client.config.base_url, "https://www.youtube.com");
        assert_eq!(client.config.default_timeout_ms, 15000);
        assert!(client.config.rate_limit.is_some());
    }

    #[test]
    fn test_metrics_recording() {
        let client = UnifiedApiClient::with_default_config();

        client.record_request_start("/test", &HttpMethod::GET);
        client.record_request_complete("/test", &HttpMethod::GET, 200, 150);

        let metrics = client.get_metrics();
        assert_eq!(metrics.global_stats.total_requests, 1);
        assert_eq!(metrics.global_stats.total_success, 1);
        assert!(metrics.endpoint_stats.contains_key("/test"));
    }

    #[tokio::test]
    async fn test_client_factory() {
        let factory = UnifiedApiClientFactory;

        let youtube_client = factory.create_youtube_client().unwrap();
        assert_eq!(
            youtube_client.get_config().base_url,
            "https://www.youtube.com"
        );

        let db_client = factory.create_database_client().unwrap();
        assert_eq!(db_client.get_config().base_url, "file://");
    }
}
