//! API統合管理システム
//!
//! アプリケーション全体のAPI操作を統一的に管理

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::analytics::data_exporter::DataExporter;
use crate::api::adapters::*;
use crate::api::generic::*;
use crate::api::unified_client::UnifiedApiClientFactory;
use crate::database::LiscovDatabase;
use crate::LiscovResult;

/// API統合管理マネージャー
pub struct ApiManager {
    /// ジェネリックAPIクライアントのファクトリー
    client_factory: Arc<UnifiedApiClientFactory>,

    /// 統合APIサービス
    unified_service: Arc<RwLock<Option<UnifiedApiService>>>,

    /// クライアント設定
    configurations: Arc<RwLock<HashMap<String, ApiClientConfig>>>,

    /// アクティブなクライアント
    active_clients: Arc<RwLock<HashMap<String, Box<dyn GenericApiClient>>>>,

    /// 統計・メトリクス
    global_metrics: Arc<RwLock<GlobalApiMetrics>>,
}

/// グローバルAPIメトリクス
#[derive(Debug, Clone, Default)]
pub struct GlobalApiMetrics {
    /// API別統計
    pub api_stats: HashMap<String, ApiStats>,
    /// 全体統計
    pub total_requests: u64,
    pub total_success: u64,
    pub total_errors: u64,
    /// 開始時刻
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// API統計
#[derive(Debug, Clone, Default)]
pub struct ApiStats {
    pub requests: u64,
    pub successes: u64,
    pub errors: u64,
    pub average_latency_ms: f64,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl ApiManager {
    /// 新しいAPIマネージャーを作成
    pub fn new() -> Self {
        Self {
            client_factory: Arc::new(UnifiedApiClientFactory),
            unified_service: Arc::new(RwLock::new(None)),
            configurations: Arc::new(RwLock::new(HashMap::new())),
            active_clients: Arc::new(RwLock::new(HashMap::new())),
            global_metrics: Arc::new(RwLock::new(GlobalApiMetrics {
                start_time: Some(chrono::Utc::now()),
                ..Default::default()
            })),
        }
    }

    /// APIマネージャーを初期化
    pub async fn initialize(
        &self,
        database: LiscovDatabase,
        exporter: DataExporter,
    ) -> LiscovResult<()> {
        // デフォルト設定を登録
        self.register_default_configurations().await?;

        // 各API用のクライアントを作成
        let youtube_client = self.client_factory.create_youtube_client()?;
        let database_client = self.client_factory.create_database_client()?;
        let analytics_client = self.client_factory.create_analytics_client()?;

        // 統合サービスを作成
        let unified_service = UnifiedApiService::new(
            youtube_client,
            database_client,
            analytics_client,
            database,
            exporter,
        );

        // サービスを登録
        {
            let mut service = self.unified_service.write();
            *service = Some(unified_service);
        }

        tracing::info!("🔌 API Manager initialized with unified services");
        Ok(())
    }

    /// デフォルト設定を登録
    async fn register_default_configurations(&self) -> LiscovResult<()> {
        let mut configs = self.configurations.write();

        // YouTube API設定
        configs.insert(
            "youtube".to_string(),
            ApiClientConfig {
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
            },
        );

        // データベースAPI設定
        configs.insert(
            "database".to_string(),
            ApiClientConfig {
                base_url: "file://".to_string(),
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
            },
        );

        // アナリティクスAPI設定
        configs.insert(
            "analytics".to_string(),
            ApiClientConfig {
                base_url: "internal://analytics".to_string(),
                default_timeout_ms: 30000,
                default_headers: HashMap::new(),
                default_retry_config: RetryConfig::default(),
                rate_limit: None,
                auth_config: None,
            },
        );

        Ok(())
    }

    /// 統合サービスにアクセス
    pub fn get_unified_service(&self) -> Arc<RwLock<Option<UnifiedApiService>>> {
        Arc::clone(&self.unified_service)
    }

    /// 特定のAPIクライアントを取得
    pub async fn get_client(&self, api_name: &str) -> LiscovResult<Box<dyn GenericApiClient>> {
        // まずアクティブなクライアントをチェック
        {
            let clients = self.active_clients.read();
            if let Some(_client) = clients.get(api_name) {
                // NOTE: Boxの参照はできないため、新しいクライアントを作成
                // 実際の実装では、Arcを使用するか、クライアントプールを実装
            }
        }

        // 設定からクライアントを作成
        let configs = self.configurations.read();
        if let Some(config) = configs.get(api_name) {
            let client = self.client_factory.create_client(config.clone());

            // アクティブクライアントとして登録（簡略化）
            // 実際にはクライアントの寿命管理が必要

            Ok(client)
        } else {
            Err(crate::ApiError::NotFound.into())
        }
    }

    /// YouTube Live Chatを取得
    pub async fn get_youtube_live_chat(
        &self,
        video_id: &str,
        continuation_token: Option<String>,
    ) -> LiscovResult<LiveChatResponse> {
        let service = self.unified_service.read();
        if let Some(service) = service.as_ref() {
            let request = LiveChatRequest {
                video_id: video_id.to_string(),
                continuation_token,
            };

            self.record_api_usage("youtube").await;
            service.youtube().get_live_chat(request).await
        } else {
            Err(crate::GuiError::Service("Unified service not initialized".to_string()).into())
        }
    }

    /// データベースクエリを実行
    pub async fn execute_database_query(
        &self,
        query: DatabaseQuery,
    ) -> LiscovResult<DatabaseQueryResult> {
        let service = self.unified_service.read();
        if let Some(service) = service.as_ref() {
            self.record_api_usage("database").await;
            service.database().execute_query(query).await
        } else {
            Err(crate::GuiError::Service("Unified service not initialized".to_string()).into())
        }
    }

    /// アナリティクスデータを取得
    pub async fn get_analytics_data(
        &self,
        request: AnalyticsRequest,
    ) -> LiscovResult<AnalyticsResponse> {
        let service = self.unified_service.read();
        if let Some(service) = service.as_ref() {
            self.record_api_usage("analytics").await;
            service.analytics().get_analytics(request).await
        } else {
            Err(crate::GuiError::Service("Unified service not initialized".to_string()).into())
        }
    }

    /// アナリティクスレポートを生成
    pub async fn generate_analytics_report(
        &self,
        request: ReportRequest,
    ) -> LiscovResult<ReportResponse> {
        let service = self.unified_service.read();
        if let Some(service) = service.as_ref() {
            self.record_api_usage("analytics").await;
            service.analytics().generate_report(request).await
        } else {
            Err(crate::GuiError::Service("Unified service not initialized".to_string()).into())
        }
    }

    /// API使用状況を記録
    async fn record_api_usage(&self, api_name: &str) {
        let mut metrics = self.global_metrics.write();
        metrics.total_requests += 1;

        let stats = metrics
            .api_stats
            .entry(api_name.to_string())
            .or_insert_with(Default::default);
        stats.requests += 1;
        stats.last_used = Some(chrono::Utc::now());
    }

    /// API成功を記録
    pub async fn record_api_success(&self, api_name: &str, latency_ms: u64) {
        let mut metrics = self.global_metrics.write();
        metrics.total_success += 1;

        if let Some(stats) = metrics.api_stats.get_mut(api_name) {
            stats.successes += 1;

            // 平均レイテンシを更新
            let current_avg = stats.average_latency_ms;
            let request_count = stats.requests as f64;
            stats.average_latency_ms =
                (current_avg * (request_count - 1.0) + latency_ms as f64) / request_count;
        }
    }

    /// APIエラーを記録
    pub async fn record_api_error(&self, api_name: &str) {
        let mut metrics = self.global_metrics.write();
        metrics.total_errors += 1;

        if let Some(stats) = metrics.api_stats.get_mut(api_name) {
            stats.errors += 1;
        }
    }

    /// 全体統計を取得
    pub fn get_global_metrics(&self) -> GlobalApiMetrics {
        self.global_metrics.read().clone()
    }

    /// API統計をリセット
    pub async fn reset_metrics(&self) {
        let mut metrics = self.global_metrics.write();
        *metrics = GlobalApiMetrics {
            start_time: Some(chrono::Utc::now()),
            ..Default::default()
        };
    }

    /// 設定を更新
    pub async fn update_configuration(
        &self,
        api_name: &str,
        config: ApiClientConfig,
    ) -> LiscovResult<()> {
        let mut configs = self.configurations.write();
        configs.insert(api_name.to_string(), config);

        // アクティブなクライアントを無効化（次回使用時に新しい設定で再作成）
        let mut clients = self.active_clients.write();
        clients.remove(api_name);

        tracing::info!("🔧 API configuration updated for: {}", api_name);
        Ok(())
    }

    /// ヘルスチェック（全API）
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        let configs = self.configurations.read();
        for api_name in configs.keys() {
            match self.get_client(api_name).await {
                Ok(client) => match client.health_check().await {
                    Ok(is_healthy) => {
                        results.insert(api_name.clone(), is_healthy);
                    }
                    Err(_) => {
                        results.insert(api_name.clone(), false);
                    }
                },
                Err(_) => {
                    results.insert(api_name.clone(), false);
                }
            }
        }

        results
    }

    /// シャットダウン
    pub async fn shutdown(&self) -> LiscovResult<()> {
        // アクティブなクライアントをクリア
        {
            let mut clients = self.active_clients.write();
            clients.clear();
        }

        // 統合サービスをクリア
        {
            let mut service = self.unified_service.write();
            *service = None;
        }

        tracing::info!("🔌 API Manager shutdown completed");
        Ok(())
    }
}

impl Default for ApiManager {
    fn default() -> Self {
        Self::new()
    }
}

/// APIマネージャーファクトリー（グローバル静的変数を避ける）
pub struct ApiManagerFactory;

impl ApiManagerFactory {
    /// 新しいAPIマネージャーを作成
    pub fn create() -> ApiManager {
        tracing::info!("🏗️ Creating API manager");
        ApiManager::new()
    }

    /// 初期化済みAPIマネージャーを作成
    pub async fn create_initialized(
        database: LiscovDatabase,
        exporter: DataExporter,
    ) -> LiscovResult<ApiManager> {
        let manager = Self::create();
        manager.initialize(database, exporter).await?;
        Ok(manager)
    }
}

// 便利関数は、APIマネージャーインスタンスを引数として受け取る形式に変更
// 例: manager.get_youtube_live_chat(video_id, continuation_token).await

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_manager_creation() {
        let manager = ApiManager::new();

        // 初期状態では統合サービスは未初期化
        let service = manager.unified_service.read();
        assert!(service.is_none());

        // メトリクスは初期化されている
        let metrics = manager.get_global_metrics();
        assert!(metrics.start_time.is_some());
    }

    #[tokio::test]
    async fn test_configuration_registration() {
        let manager = ApiManager::new();
        manager.register_default_configurations().await.unwrap();

        let configs = manager.configurations.read();
        assert!(configs.contains_key("youtube"));
        assert!(configs.contains_key("database"));
        assert!(configs.contains_key("analytics"));
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let manager = ApiManager::new();

        manager.record_api_usage("test_api").await;
        manager.record_api_success("test_api", 150).await;

        let metrics = manager.get_global_metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.total_success, 1);

        if let Some(stats) = metrics.api_stats.get("test_api") {
            assert_eq!(stats.requests, 1);
            assert_eq!(stats.successes, 1);
            assert_eq!(stats.average_latency_ms, 150.0);
        } else {
            panic!("API stats not found");
        }
    }

    #[test]
    fn test_api_manager_factory() {
        let manager1 = ApiManagerFactory::create();
        let manager2 = ApiManagerFactory::create();

        // 異なるインスタンスであることを確認
        assert!(!std::ptr::eq(&manager1, &manager2));
    }
}
