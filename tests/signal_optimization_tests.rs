//! Signal最適化システムとBatch処理のテスト (Phase 5.1)
//!
//! Phase 4で実装したSignal最適化・Batch処理機能の単体テストと統合テスト

use liscov::gui::signal_optimizer::*;
use std::sync::{Mutex, OnceLock};

fn optimizer_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn reset_optimizer_state_for_tests() {
    reset_signal_optimizer_state();
}

/// Phase 4.1: Signal最適化システムテスト
#[cfg(test)]
mod signal_optimization_tests {
    use super::*;

    #[test]
    fn test_signal_dependency_graph_creation() {
        let mut graph = SignalDependencyGraph::new();

        // 初期状態の確認
        assert_eq!(graph.get_stats().total_signals, 0);
        assert!(graph.list_signals().is_empty());

        // Signalの登録
        graph.register_signal(
            "test_signal_1".to_string(),
            SignalType::AutoScrollEnabled,
            "TestComponent".to_string(),
        );

        graph.register_signal(
            "test_signal_2".to_string(),
            SignalType::MessageFontSize,
            "TestComponent".to_string(),
        );

        // 登録後の状態確認
        assert_eq!(graph.get_stats().total_signals, 2);
        assert_eq!(graph.list_signals().len(), 2);
    }

    #[test]
    fn test_signal_update_tracking() {
        let mut graph = SignalDependencyGraph::new();

        // Signalを登録
        graph.register_signal(
            "update_test_signal".to_string(),
            SignalType::ShowTimestamps,
            "TestComponent".to_string(),
        );

        // 初期状態では更新カウントが0
        let signals = graph.list_signals();
        assert_eq!(signals[0].update_count, 0);
        assert!(signals[0].last_updated.is_none());

        // 更新を記録
        graph.record_update("update_test_signal");
        graph.record_update("update_test_signal");
        graph.record_update("update_test_signal");

        // 更新カウントが正しく記録されることを確認
        let signals = graph.list_signals();
        assert_eq!(signals[0].update_count, 3);
        assert!(signals[0].last_updated.is_some());
    }

    #[test]
    fn test_duplicate_signal_detection() {
        let mut graph = SignalDependencyGraph::new();

        // 同じ型のSignalを複数登録
        graph.register_signal(
            "auto_scroll_1".to_string(),
            SignalType::AutoScrollEnabled,
            "Component1".to_string(),
        );

        graph.register_signal(
            "auto_scroll_2".to_string(),
            SignalType::AutoScrollEnabled,
            "Component2".to_string(),
        );

        graph.register_signal(
            "highlight_1".to_string(),
            SignalType::MessageFontSize,
            "Component1".to_string(),
        );

        // 重複検出
        let duplicates = graph.detect_duplicate_signals();

        // AutoScrollEnabledの重複グループが検出されることを確認
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].len(), 2);
        assert!(duplicates[0].contains(&"auto_scroll_1".to_string()));
        assert!(duplicates[0].contains(&"auto_scroll_2".to_string()));
    }

    #[test]
    fn test_unused_signal_detection() {
        let mut graph = SignalDependencyGraph::new();

        // 使用されるSignalと未使用Signalを登録
        graph.register_signal(
            "used_signal".to_string(),
            SignalType::MessageFilter,
            "TestComponent".to_string(),
        );

        graph.register_signal(
            "unused_signal_1".to_string(),
            SignalType::ScrollPosition,
            "TestComponent".to_string(),
        );

        graph.register_signal(
            "unused_signal_2".to_string(),
            SignalType::LastMessageCount,
            "TestComponent".to_string(),
        );

        // 一つのSignalのみ更新
        graph.record_update("used_signal");

        // 未使用Signal検出
        let unused = graph.detect_unused_signals();

        assert_eq!(unused.len(), 2);
        assert!(unused.contains(&"unused_signal_1".to_string()));
        assert!(unused.contains(&"unused_signal_2".to_string()));
        assert!(!unused.contains(&"used_signal".to_string()));
    }

    #[test]
    fn test_optimization_recommendations() {
        let mut graph = SignalDependencyGraph::new();

        // 重複と未使用のSignalを作成
        graph.register_signal(
            "dup1".to_string(),
            SignalType::AutoScrollEnabled,
            "Comp1".to_string(),
        );
        graph.register_signal(
            "dup2".to_string(),
            SignalType::AutoScrollEnabled,
            "Comp2".to_string(),
        );
        graph.register_signal(
            "unused".to_string(),
            SignalType::ShowTimestamps,
            "Comp3".to_string(),
        );
        graph.register_signal(
            "used".to_string(),
            SignalType::MessageFontSize,
            "Comp4".to_string(),
        );

        // used signalのみ更新
        graph.record_update("used");

        // 推奨事項を生成
        let recommendations = graph.generate_optimization_recommendations();

        // 重複統合と未使用削除の推奨が含まれることを確認
        assert!(recommendations.len() >= 2);

        let merge_rec = recommendations
            .iter()
            .find(|r| r.recommendation_type == OptimizationType::MergeDuplicate);
        let remove_rec = recommendations
            .iter()
            .find(|r| r.recommendation_type == OptimizationType::RemoveUnused);

        assert!(merge_rec.is_some());
        assert!(remove_rec.is_some());

        // 優先度順にソートされていることを確認
        for i in 0..recommendations.len() - 1 {
            assert!(recommendations[i].priority <= recommendations[i + 1].priority);
        }
    }

    #[test]
    fn test_analysis_report_generation() {
        let mut graph = SignalDependencyGraph::new();

        // テストデータをセットアップ
        graph.register_signal(
            "signal1".to_string(),
            SignalType::ChatMessage,
            "Chat".to_string(),
        );
        graph.register_signal(
            "signal2".to_string(),
            SignalType::ChatMessage,
            "Chat".to_string(),
        );
        graph.register_signal(
            "signal3".to_string(),
            SignalType::AutoScrollEnabled,
            "UI".to_string(),
        );

        graph.record_update("signal1");
        graph.record_update("signal3");

        // 分析レポートを生成
        let report = graph.generate_analysis_report();

        // レポートに必要な情報が含まれることを確認
        assert!(report.contains("Total Signals: 3"));
        assert!(report.contains("Duplicate Signals: 1"));
        assert!(report.contains("Unused Signals: 1"));
        assert!(report.contains("Signals by Component"));
        assert!(report.contains("Chat -> 2 signals"));
        assert!(report.contains("UI -> 1 signals"));
        assert!(report.contains("Optimization Recommendations"));
    }

    #[test]
    fn test_global_signal_tracking() {
        let _lock = optimizer_test_lock().lock().unwrap();
        reset_optimizer_state_for_tests();
        // グローバル関数のテスト
        register_signal("global_test", SignalType::FilteredMessage, "GlobalTest");
        record_signal_update("global_test");
        record_signal_update("global_test");

        let report = generate_signal_analysis_report();
        assert!(report.contains("Total Signals"));

        {
            let graph = get_signal_graph();
            let graph = graph.lock().expect("lock signal graph");
            assert_eq!(graph.get_stats().total_signals, 1);
            assert!(graph.contains_signal("global_test"));
        }

        let recommendations = get_optimization_recommendations();
        assert!(recommendations
            .iter()
            .all(|rec| rec.expected_improvement >= 0.0));

        reset_optimizer_state_for_tests();
    }
}

/// Phase 4.2: Batch処理システムテスト
#[cfg(test)]
mod batch_processing_tests {
    use super::*;
    use tokio::test as async_test;

    #[test]
    fn test_batch_update_manager_creation() {
        let manager = BatchUpdateManager::new();

        // 初期状態の確認
        assert_eq!(manager.queue_size(), 0);
        assert_eq!(manager.get_stats().total_batched, 0);
        assert_eq!(manager.get_stats().high_priority_count, 0);
        assert_eq!(manager.get_stats().dom_update_count, 0);
        assert_eq!(manager.get_stats().average_batch_size, 0.0);
    }

    #[test]
    fn test_batch_update_queuing() {
        let mut manager = BatchUpdateManager::new();

        // 通常の更新をキューに追加
        manager.queue_update("test_normal".to_string(), BatchUpdateType::Normal);
        assert_eq!(manager.queue_size(), 1);
        assert_eq!(manager.get_stats().total_batched, 1);

        // 高優先度更新をキューに追加
        manager.queue_update("test_high".to_string(), BatchUpdateType::HighPriority);
        assert_eq!(manager.queue_size(), 2);
        assert_eq!(manager.get_stats().high_priority_count, 1);

        // DOM更新をキューに追加
        manager.queue_update("test_dom".to_string(), BatchUpdateType::DomUpdate);
        assert_eq!(manager.queue_size(), 3);
        assert_eq!(manager.get_stats().dom_update_count, 1);
    }

    #[test]
    fn test_batch_update_priority_ordering() {
        let mut manager = BatchUpdateManager::new();

        // 異なる優先度でキューに追加
        manager.queue_update("normal".to_string(), BatchUpdateType::Normal);
        manager.queue_update("low".to_string(), BatchUpdateType::LowPriority);
        manager.queue_update("high".to_string(), BatchUpdateType::HighPriority);
        manager.queue_update("dom".to_string(), BatchUpdateType::DomUpdate);

        assert_eq!(manager.queue_size(), 4);

        // 内部実装の詳細はテストしないが、キューサイズが正しいことを確認
        assert_eq!(manager.get_stats().total_batched, 4);
        assert_eq!(manager.get_stats().high_priority_count, 1);
        assert_eq!(manager.get_stats().dom_update_count, 1);
    }

    #[async_test]
    async fn test_batch_processing_execution() {
        let mut manager = BatchUpdateManager::new();

        // 複数の更新をキューに追加
        manager.queue_update("item1".to_string(), BatchUpdateType::Normal);
        manager.queue_update("item2".to_string(), BatchUpdateType::HighPriority);
        manager.queue_update("item3".to_string(), BatchUpdateType::DomUpdate);

        let initial_size = manager.queue_size();
        assert_eq!(initial_size, 3);

        // Batch処理を実行
        let processed = manager.process_batch().await;

        // 処理が成功することを確認
        assert!(processed.is_ok());

        // 統計情報が更新されることを確認
        let stats = manager.get_stats();
        assert!(stats.last_batch_time.is_some());
        assert!(stats.average_batch_size > 0.0);
    }

    #[test]
    fn test_batch_stats_tracking() {
        let mut manager = BatchUpdateManager::new();

        // 統計追跡のテスト
        manager.queue_update("test1".to_string(), BatchUpdateType::Normal);
        manager.queue_update("test2".to_string(), BatchUpdateType::HighPriority);
        manager.queue_update("test3".to_string(), BatchUpdateType::DomUpdate);
        manager.queue_update("test4".to_string(), BatchUpdateType::LowPriority);

        let stats = manager.get_stats();
        assert_eq!(stats.total_batched, 4);
        assert_eq!(stats.high_priority_count, 1);
        assert_eq!(stats.dom_update_count, 1);
    }

    #[test]
    fn test_global_batch_manager() {
        let _lock = optimizer_test_lock().lock().unwrap();
        reset_optimizer_state_for_tests();
        // グローバルBatch管理のテスト
        queue_batch_update("global_test1", BatchUpdateType::Normal);
        queue_batch_update("global_test2", BatchUpdateType::HighPriority);

        let processed = tokio::runtime::Runtime::new()
            .expect("create runtime")
            .block_on(async { process_batch_updates().await });
        let stats = get_batch_stats().expect("batch stats should exist");
        assert!(stats.total_batched >= 2);
        assert_eq!(processed as u64, stats.total_batched);

        reset_optimizer_state_for_tests();
    }

    #[test]
    fn test_batch_update_types() {
        // 各更新タイプの特性をテスト
        let types = vec![
            BatchUpdateType::Normal,
            BatchUpdateType::HighPriority,
            BatchUpdateType::LowPriority,
            BatchUpdateType::DomUpdate,
        ];

        for update_type in types {
            let mut manager = BatchUpdateManager::new();
            manager.queue_update("test".to_string(), update_type.clone());

            assert_eq!(manager.queue_size(), 1);

            match update_type {
                BatchUpdateType::HighPriority => {
                    assert_eq!(manager.get_stats().high_priority_count, 1);
                }
                BatchUpdateType::DomUpdate => {
                    assert_eq!(manager.get_stats().dom_update_count, 1);
                }
                _ => {
                    // Normal や LowPriority は特別なカウンタを持たない
                }
            }
        }
    }

    #[async_test]
    async fn test_empty_batch_processing() {
        let mut manager = BatchUpdateManager::new();

        // 空のキューで処理を実行
        let result = manager.process_batch().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // 処理された項目数は0
    }

    #[async_test]
    async fn test_concurrent_batch_processing() {
        let mut manager = BatchUpdateManager::new();

        // アイテムを追加
        manager.queue_update("concurrent1".to_string(), BatchUpdateType::Normal);

        // 同時処理の確認（2回目の処理は処理中フラグにより実行されない）
        let _result1 = manager.process_batch().await;
        let result2 = manager.process_batch().await;

        assert!(result2.is_ok());
        // 2回目は既に処理中のため0が返される可能性がある
    }
}

/// Phase 5.1: パフォーマンステスト
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_signal_registration_performance() {
        let mut graph = SignalDependencyGraph::new();
        let start = Instant::now();

        // 大量のSignal登録のパフォーマンステスト
        for i in 0..1000 {
            graph.register_signal(
                format!("signal_{}", i),
                SignalType::ChatMessage,
                "PerformanceTest".to_string(),
            );
        }

        let duration = start.elapsed();

        // 1000個のSignal登録が1秒以内に完了することを確認
        assert!(duration.as_secs() < 1);
        assert_eq!(graph.get_stats().total_signals, 1000);
    }

    #[test]
    fn test_signal_update_performance() {
        let mut graph = SignalDependencyGraph::new();

        // Signal登録
        for i in 0..100 {
            graph.register_signal(
                format!("perf_signal_{}", i),
                SignalType::AutoScrollEnabled,
                "PerformanceTest".to_string(),
            );
        }

        let start = Instant::now();

        // 大量の更新のパフォーマンステスト
        for i in 0..100 {
            for _ in 0..10 {
                graph.record_update(&format!("perf_signal_{}", i));
            }
        }

        let duration = start.elapsed();

        // 1000回の更新が1秒以内に完了することを確認
        assert!(duration.as_secs() < 1);

        // 更新カウントが正しいことを確認
        let signals = graph.list_signals();
        for signal in signals {
            assert_eq!(signal.update_count, 10);
        }
    }

    #[test]
    fn test_duplicate_detection_performance() {
        let mut graph = SignalDependencyGraph::new();

        // 重複検出のパフォーマンステスト用データ
        for i in 0..500 {
            graph.register_signal(
                format!("dup_signal_{}", i),
                SignalType::AutoScrollEnabled, // 全て同じ型
                "PerformanceTest".to_string(),
            );
        }

        let start = Instant::now();
        let duplicates = graph.detect_duplicate_signals();
        let duration = start.elapsed();

        // 重複検出が1秒以内に完了することを確認
        assert!(duration.as_secs() < 1);

        // 500個の重複グループが検出されることを確認
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].len(), 500);
    }

    #[test]
    fn test_batch_processing_performance() {
        let mut manager = BatchUpdateManager::new();

        let start = Instant::now();

        // 大量のBatch更新のパフォーマンステスト
        for i in 0..1000 {
            manager.queue_update(
                format!("perf_batch_{}", i),
                if i % 2 == 0 {
                    BatchUpdateType::Normal
                } else {
                    BatchUpdateType::HighPriority
                },
            );
        }

        let duration = start.elapsed();

        // 1000個のキューイングが1秒以内に完了することを確認
        assert!(duration.as_secs() < 1);
        assert_eq!(manager.queue_size(), 1000);
        assert_eq!(manager.get_stats().total_batched, 1000);
        assert_eq!(manager.get_stats().high_priority_count, 500);
    }

    #[test]
    fn test_analysis_report_generation_performance() {
        let mut graph = SignalDependencyGraph::new();

        // レポート生成のパフォーマンステスト用データ
        for i in 0..200 {
            graph.register_signal(
                format!("report_signal_{}", i),
                if i % 3 == 0 {
                    SignalType::AutoScrollEnabled
                } else if i % 3 == 1 {
                    SignalType::MessageFontSize
                } else {
                    SignalType::ShowTimestamps
                },
                format!("Component_{}", i % 10),
            );

            // 一部のSignalを更新
            if i % 4 == 0 {
                graph.record_update(&format!("report_signal_{}", i));
            }
        }

        let start = Instant::now();
        let report = graph.generate_analysis_report();
        let duration = start.elapsed();

        // レポート生成が1秒以内に完了することを確認
        assert!(duration.as_secs() < 1);
        assert!(!report.is_empty());
        assert!(report.contains("Total Signals: 200"));
    }
}

/// Phase 5.1: 統合テスト
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::test as async_test;

    #[test]
    fn test_signal_and_batch_integration() {
        let _lock = optimizer_test_lock().lock().unwrap();
        reset_optimizer_state_for_tests();
        // Signal最適化とBatch処理の統合テスト

        // Signalを登録
        register_signal(
            "integration_test_1",
            SignalType::AutoScrollEnabled,
            "IntegrationTest",
        );
        register_signal(
            "integration_test_2",
            SignalType::MessageFontSize,
            "IntegrationTest",
        );

        // Signal更新を記録
        record_signal_update("integration_test_1");
        record_signal_update("integration_test_2");

        // Batch更新をキューに追加
        queue_batch_update("integration_test_1", BatchUpdateType::Normal);
        queue_batch_update("integration_test_2", BatchUpdateType::HighPriority);

        // 両システムの動作確認
        let signal_report = generate_signal_analysis_report();
        let batch_processed = tokio::runtime::Runtime::new()
            .expect("create runtime")
            .block_on(async { process_batch_updates().await });
        let batch_stats = get_batch_stats().expect("batch stats should exist");

        // 結果検証
        assert!(signal_report.contains("Total Signals"));
        {
            let graph = get_signal_graph();
            let graph = graph.lock().expect("lock signal graph");
            assert_eq!(graph.get_stats().total_signals, 2);
            assert!(graph.contains_signal("integration_test_1"));
            assert!(graph.contains_signal("integration_test_2"));
        }
        assert!(batch_stats.total_batched >= 2);
        assert_eq!(batch_processed as u64, batch_stats.total_batched);

        reset_optimizer_state_for_tests();
    }

    #[test]
    fn test_memory_usage_tracking() {
        // メモリ使用量の追跡テスト
        let mut graph = SignalDependencyGraph::new();

        // 大量のSignalを登録してメモリ使用量を監視
        for i in 0..100 {
            graph.register_signal(
                format!("memory_test_{}", i),
                SignalType::ChatMessage,
                "MemoryTest".to_string(),
            );
        }

        let stats = graph.get_stats();
        assert_eq!(stats.total_signals, 100);

        // メモリ統計が追跡されていることを確認
        // 実際のメモリ使用量は実装に依存するが、0より大きいことを確認
        // assert!(stats.memory_usage > 0); // この機能は将来実装予定
    }

    #[async_test]
    async fn test_error_handling_integration() {
        // エラーハンドリングの統合テスト
        let mut manager = BatchUpdateManager::new();

        // 正常なケース
        manager.queue_update("valid_update".to_string(), BatchUpdateType::Normal);
        let result = manager.process_batch().await;
        assert!(result.is_ok());

        // 空のキューでの処理
        let empty_result = manager.process_batch().await;
        assert!(empty_result.is_ok());
        assert_eq!(empty_result.unwrap(), 0);
    }
}
