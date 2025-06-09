//! 統合テスト
//! 
//! Phase 1-3で実装した機能の統合テスト

use std::collections::HashMap;
use liscov::{
    api::{
        generic::*,
        unified_client::*,
        adapters::*,
        manager::*,
    },
};

/// API統合テスト
#[cfg(test)]
mod api_integration_tests {
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
        
        // JSONシリアライゼーション/デシリアライゼーションテスト
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: GenericRequest<serde_json::Value> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, request.id);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert_eq!(config.max_delay_ms, 30000);
        assert!(config.retryable_status_codes.contains(&500));
        assert!(config.retryable_status_codes.contains(&429));
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
        
        // ページング計算の妥当性をテスト
        assert_eq!(result.total_count / result.size as u64, result.total_pages as u64);
    }

    #[test]
    fn test_api_client_config() {
        let config = ApiClientConfig {
            base_url: "https://api.example.com".to_string(),
            default_timeout_ms: 5000,
            default_headers: {
                let mut headers = HashMap::new();
                headers.insert("User-Agent".to_string(), "Test/1.0".to_string());
                headers
            },
            default_retry_config: RetryConfig::default(),
            rate_limit: Some(RateLimitConfig {
                window_seconds: 60,
                max_requests: 100,
            }),
            auth_config: None,
        };

        assert_eq!(config.base_url, "https://api.example.com");
        assert_eq!(config.default_timeout_ms, 5000);
        assert!(config.rate_limit.is_some());
        assert!(config.default_headers.contains_key("User-Agent"));
    }

    #[test]
    fn test_unified_api_client_creation() {
        let client = UnifiedApiClient::with_default_config();
        let config = client.get_config();
        
        assert_eq!(config.base_url, "https://api.example.com");
        assert_eq!(config.default_timeout_ms, 10000);
        assert!(config.default_headers.contains_key("User-Agent"));
    }

    #[test]
    fn test_youtube_api_client_config() {
        let client = UnifiedApiClient::for_youtube();
        let config = client.get_config();
        
        assert_eq!(config.base_url, "https://www.youtube.com");
        assert_eq!(config.default_timeout_ms, 15000);
        assert!(config.rate_limit.is_some());
        
        if let Some(rate_limit) = &config.rate_limit {
            assert_eq!(rate_limit.window_seconds, 60);
            assert_eq!(rate_limit.max_requests, 100);
        }
    }

    #[test]
    fn test_database_api_client_config() {
        let client = UnifiedApiClient::for_database();
        let config = client.get_config();
        
        assert_eq!(config.base_url, "file://");
        assert_eq!(config.default_timeout_ms, 5000);
        assert!(config.rate_limit.is_none());
    }

    #[test]
    fn test_api_manager_factory() {
        let manager1 = ApiManagerFactory::create();
        let manager2 = ApiManagerFactory::create();
        
        // 異なるインスタンスであることを確認
        assert!(!std::ptr::eq(&manager1, &manager2));
        
        // 初期状態での統計確認
        let metrics = manager1.get_global_metrics();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_success, 0);
        assert_eq!(metrics.total_errors, 0);
        assert!(metrics.start_time.is_some());
    }

    #[test]
    fn test_live_chat_request_serialization() {
        let request = LiveChatRequest {
            video_id: "test_video_123".to_string(),
            continuation_token: Some("continuation_456".to_string()),
        };
        
        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("test_video_123"));
        assert!(serialized.contains("continuation_456"));
        
        let deserialized: LiveChatRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.video_id, request.video_id);
        assert_eq!(deserialized.continuation_token, request.continuation_token);
    }

    #[test]
    fn test_database_query_structure() {
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
        assert!(query.parameters.is_some());
        assert!(query.conditions.is_some());
        
        if let Some(conditions) = &query.conditions {
            assert!(conditions.contains_key("session_id"));
        }
    }

    #[test]
    fn test_analytics_request_structure() {
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
        assert!(request.start_date < request.end_date);
        
        // 過去7日間の期間が正しく設定されているかテスト
        let duration = request.end_date.signed_duration_since(request.start_date);
        assert!(duration.num_days() >= 6 && duration.num_days() <= 8);
    }

    #[test]
    fn test_report_request_formats() {
        let formats = vec![
            ReportFormat::Json,
            ReportFormat::Csv,
            ReportFormat::Excel,
            ReportFormat::Pdf,
        ];
        
        for format in formats {
            let request = ReportRequest {
                report_type: "engagement_summary".to_string(),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("period".to_string(), serde_json::json!("weekly"));
                    params
                },
                format: format.clone(),
            };
            
            assert_eq!(request.report_type, "engagement_summary");
            assert!(request.parameters.contains_key("period"));
            
            // フォーマット別のシリアライゼーションテスト
            let serialized = serde_json::to_string(&request).unwrap();
            let deserialized: ReportRequest = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized.report_type, request.report_type);
        }
    }

    #[test]
    fn test_stream_config_validation() {
        let config = StreamConfig {
            endpoint: "/stream/livechat".to_string(),
            buffer_size: 1000,
            reconnect_interval_ms: 5000,
            max_reconnect_attempts: 3,
            heartbeat_interval_ms: Some(30000),
        };
        
        assert_eq!(config.endpoint, "/stream/livechat");
        assert_eq!(config.buffer_size, 1000);
        assert_eq!(config.max_reconnect_attempts, 3);
        assert!(config.heartbeat_interval_ms.is_some());
        
        // 妥当性の確認
        assert!(config.buffer_size > 0);
        assert!(config.reconnect_interval_ms > 0);
        assert!(config.max_reconnect_attempts > 0);
    }

    #[test]
    fn test_cache_stats_calculation() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            hit_rate: 80.0,
            cache_size: 500,
            last_clear_time: Some(chrono::Utc::now()),
        };
        
        assert_eq!(stats.hit_rate, 80.0);
        assert_eq!(stats.hits + stats.misses, 100);
        assert!(stats.cache_size > 0);
        assert!(stats.last_clear_time.is_some());
        
        // ヒット率の妥当性確認
        let calculated_hit_rate = (stats.hits as f64) / ((stats.hits + stats.misses) as f64) * 100.0;
        assert!((stats.hit_rate - calculated_hit_rate).abs() < 0.001);
    }

    #[test]
    fn test_error_handling_hierarchy() {
        // 各レベルのエラーが正しくLiscovErrorに変換されることをテスト
        let api_error = liscov::ApiError::NotFound;
        let liscov_error: liscov::LiscovError = api_error.into();
        assert!(matches!(liscov_error, liscov::LiscovError::Api(_)));
        
        let io_error = liscov::IoError::FileRead("test.txt".to_string());
        let liscov_error: liscov::LiscovError = io_error.into();
        assert!(matches!(liscov_error, liscov::LiscovError::Io(_)));
        
        let gui_error = liscov::GuiError::Service("test service error".to_string());
        let liscov_error: liscov::LiscovError = gui_error.into();
        assert!(matches!(liscov_error, liscov::LiscovError::Gui(_)));
        
        // エラーメッセージの確認
        assert!(liscov_error.to_string().contains("test service error"));
    }
}

/// メモリ効率最適化テスト
#[cfg(test)]
mod memory_optimization_tests {
    use liscov::gui::{
        memory_optimized::*,
        models::{GuiChatMessage, MessageType},
    };

    fn create_test_message(content: &str, author: &str) -> GuiChatMessage {
        GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            message_type: MessageType::Text,
            author: author.to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            metadata: None,
            is_member: false,
        }
    }

    #[test]
    fn test_circular_message_buffer() {
        let mut buffer = CircularMessageBuffer::new(3);
        
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), 3);
        assert!(buffer.is_empty());
        
        // メッセージを追加
        buffer.push(create_test_message("msg1", "user1"));
        buffer.push(create_test_message("msg2", "user2"));
        buffer.push(create_test_message("msg3", "user3"));
        
        assert_eq!(buffer.len(), 3);
        assert!(!buffer.is_empty());
        assert!(buffer.is_full());
        
        // 容量を超えて追加（古いメッセージが削除される）
        buffer.push(create_test_message("msg4", "user4"));
        
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.dropped_count(), 1);
        assert_eq!(buffer.total_count(), 4);
        
        // 最新のメッセージが保持されていることを確認
        let messages = buffer.messages();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[2].content, "msg4");
    }

    #[test]
    fn test_message_pool_reuse() {
        let pool = MessagePool::new(2);
        
        // 初期状態
        let stats = pool.stats();
        assert_eq!(stats.pool_size, 0);
        assert_eq!(stats.created_count, 0);
        assert_eq!(stats.reused_count, 0);
        
        // メッセージを取得
        let msg1 = pool.acquire();
        let msg2 = pool.acquire();
        
        let stats = pool.stats();
        assert_eq!(stats.created_count, 2);
        
        // プールに返却
        pool.release(msg1);
        pool.release(msg2);
        
        let stats = pool.stats();
        assert_eq!(stats.pool_size, 2);
        
        // 再利用
        let _msg3 = pool.acquire();
        let _msg4 = pool.acquire();
        
        let stats = pool.stats();
        assert_eq!(stats.reused_count, 2);
        assert_eq!(stats.pool_size, 0);
    }

    #[test]
    fn test_shared_data_cache() {
        let cache = SharedDataCache::new(100);
        
        // 初期状態
        let stats = cache.cache_stats();
        assert_eq!(stats.author_cache_size, 0);
        assert_eq!(stats.channel_id_cache_size, 0);
        
        // データを追加
        let author1 = cache.get_shared_author("user1");
        let author2 = cache.get_shared_author("user1"); // 同じユーザー
        let author3 = cache.get_shared_author("user2"); // 異なるユーザー
        
        // 同じユーザーに対して同じインスタンスが返されることを確認
        assert!(std::ptr::eq(&*author1, &*author2));
        assert!(!std::ptr::eq(&*author1, &*author3));
        
        let stats = cache.cache_stats();
        assert_eq!(stats.author_cache_size, 2);
        
        // チャンネルIDも同様にテスト
        let channel1 = cache.get_shared_channel_id("channel1");
        let channel2 = cache.get_shared_channel_id("channel1");
        assert!(std::ptr::eq(&*channel1, &*channel2));
        
        let stats = cache.cache_stats();
        assert_eq!(stats.channel_id_cache_size, 1);
    }

    #[test]
    fn test_optimized_message_manager() {
        use liscov::gui::memory_optimized::BatchConfig;
        let mut manager = OptimizedMessageManager::new(5, 2, 100, BatchConfig::default());
        
        // 初期状態
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());
        
        // メッセージを追加
        for i in 1..=3 {
            manager.add_message(create_test_message(&format!("msg{}", i), "user"));
        }
        
        assert_eq!(manager.len(), 3);
        assert!(!manager.is_empty());
        
        // 統計情報をテスト
        let stats = manager.comprehensive_stats();
        assert_eq!(stats.message_count, 3);
        assert!(stats.cache_stats.author_cache_size > 0);
        
        // メッセージ取得
        let messages = manager.messages();
        assert_eq!(messages.len(), 3);
        
        // クリア機能
        manager.clear();
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());
    }
}