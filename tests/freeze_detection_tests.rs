//! フリーズ検出テスト (Phase 3.1)
//!
//! UIスレッドがブロックされないことを確認するテスト群。
//! 状態アクセス、ブロードキャスト、高スループット処理の各シナリオでテスト。

use liscov::gui::models::{GuiChatMessage, MessageType};
use liscov::gui::services::ServiceState;
use liscov::gui::state_broadcaster::{StateBroadcaster, StateChange};
use liscov::gui::state_management::{AppEvent, ChatStats, StateManager};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;

/// 1フレームの時間（60FPSベース）
const ONE_FRAME_MS: u64 = 16;

/// テスト用のダミーメッセージを生成
fn create_test_message(id: usize) -> GuiChatMessage {
    GuiChatMessage {
        author: format!("TestUser{}", id % 10),
        content: format!("Test message content #{}", id),
        timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
        channel_id: "test_channel".to_string(),
        is_member: id % 3 == 0,
        author_icon_url: None,
        runs: vec![],
        metadata: None,
        message_type: MessageType::Text,
        comment_count: None,
    }
}

/// Phase 3.1: 状態アクセスレイテンシテスト
#[cfg(test)]
mod state_access_latency_tests {
    use super::*;

    /// get_state_async()が16ms以内に完了することを確認
    #[tokio::test]
    async fn test_state_async_access_within_one_frame() {
        let manager = StateManager::new();

        // 複数回アクセスしてレイテンシを測定
        let mut max_latency = Duration::ZERO;
        for _ in 0..100 {
            let start = Instant::now();
            let _state = manager.get_state_async().await;
            let elapsed = start.elapsed();
            if elapsed > max_latency {
                max_latency = elapsed;
            }
        }

        assert!(
            max_latency < Duration::from_millis(ONE_FRAME_MS),
            "Max state access latency {:?} exceeded 16ms threshold",
            max_latency
        );
    }

    /// get_state()（レガシー）が適切にフォールバックすることを確認
    #[tokio::test]
    async fn test_legacy_state_access_fallback() {
        let manager = StateManager::new();

        // レガシーメソッドでも状態取得が成功することを確認
        let result = manager.get_state();
        assert!(result.is_ok(), "Legacy get_state() should succeed");

        let state = result.unwrap();
        assert_eq!(state.service_state, ServiceState::Idle);
    }

    /// 書き込み中でも読み取りがブロックされないことを確認
    #[tokio::test]
    async fn test_read_during_write_non_blocking() {
        let manager = Arc::new(StateManager::new());
        let manager_clone = Arc::clone(&manager);

        // 複数のメッセージを送信（書き込み処理を開始）
        for i in 0..50 {
            let msg = create_test_message(i);
            let _ = manager.send_event(AppEvent::MessageAdded(msg));
        }

        // 少し待機してイベント処理を開始させる
        tokio::time::sleep(Duration::from_millis(5)).await;

        // 書き込み中でも読み取りがブロックされないことを確認
        let start = Instant::now();
        let _ = manager_clone.get_state_async().await;
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(ONE_FRAME_MS * 2),
            "Read during write took {:?}, expected < 32ms",
            elapsed
        );
    }
}

/// Phase 3.1: ブロードキャスト非ブロッキングテスト
#[cfg(test)]
mod broadcast_non_blocking_tests {
    use super::*;

    /// ブロードキャストが即座に完了することを確認
    #[tokio::test]
    async fn test_broadcast_is_non_blocking() {
        let broadcaster = StateBroadcaster::new();

        // サブスクライバーなしでもブロードキャストが即座に完了すること
        let start = Instant::now();
        for i in 0..1000 {
            broadcaster.broadcast(StateChange::MessageAdded {
                count: i,
                latest: None,
            });
        }
        let elapsed = start.elapsed();

        // 1000件のブロードキャストが10ms以内に完了すべき
        assert!(
            elapsed < Duration::from_millis(10),
            "1000 broadcasts took {:?}, expected < 10ms",
            elapsed
        );
    }

    /// 遅いサブスクライバーがあってもブロードキャストがブロックされないことを確認
    #[tokio::test]
    async fn test_slow_subscriber_does_not_block_broadcast() {
        let broadcaster = Arc::new(StateBroadcaster::new());
        let broadcaster_clone = Arc::clone(&broadcaster);

        // 遅いサブスクライバーを開始
        let mut rx = broadcaster.subscribe();
        let _slow_task = tokio::spawn(async move {
            while let Ok(_change) = rx.recv().await {
                // 意図的に遅延させる
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        // 少し待機
        tokio::time::sleep(Duration::from_millis(10)).await;

        // ブロードキャストが遅いサブスクライバーにブロックされないことを確認
        let start = Instant::now();
        for i in 0..100 {
            broadcaster_clone.broadcast(StateChange::MessageAdded {
                count: i,
                latest: None,
            });
        }
        let elapsed = start.elapsed();

        // 100件のブロードキャストが10ms以内に完了すべき
        assert!(
            elapsed < Duration::from_millis(10),
            "Broadcasts blocked by slow subscriber: {:?}",
            elapsed
        );
    }

    /// 複数のサブスクライバーが同じイベントを受信することを確認
    #[tokio::test]
    async fn test_multiple_subscribers_receive_events() {
        let broadcaster = StateBroadcaster::new();
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();
        let mut rx3 = broadcaster.subscribe();

        // イベントを送信
        broadcaster.broadcast(StateChange::ConnectionChanged { is_connected: true });

        // すべてのサブスクライバーがイベントを受信することを確認
        let timeout = Duration::from_millis(100);

        let r1 = tokio::time::timeout(timeout, rx1.recv()).await;
        let r2 = tokio::time::timeout(timeout, rx2.recv()).await;
        let r3 = tokio::time::timeout(timeout, rx3.recv()).await;

        assert!(r1.is_ok(), "Subscriber 1 should receive event");
        assert!(r2.is_ok(), "Subscriber 2 should receive event");
        assert!(r3.is_ok(), "Subscriber 3 should receive event");
    }

    /// サブスクライバーがドロップされてもブロードキャストが継続することを確認
    #[tokio::test]
    async fn test_broadcast_continues_after_subscriber_drop() {
        let broadcaster = StateBroadcaster::new();

        {
            let _rx = broadcaster.subscribe();
            assert_eq!(broadcaster.subscriber_count(), 1);
        } // rxがドロップされる

        // サブスクライバーがドロップされた後もブロードキャストは成功すべき
        broadcaster.broadcast(StateChange::MessagesCleared);

        // 新しいサブスクライバーを追加できることを確認
        let mut rx = broadcaster.subscribe();
        broadcaster.broadcast(StateChange::ConnectionChanged { is_connected: false });

        let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok(), "New subscriber should receive events");
    }
}

/// Phase 3.1: 高スループット処理テスト
#[cfg(test)]
mod high_throughput_tests {
    use super::*;

    /// 500メッセージの高速処理でフリーズしないことを確認
    #[tokio::test]
    async fn test_500_messages_no_freeze() {
        let manager = StateManager::new();

        // 500メッセージを送信
        let start = Instant::now();
        for i in 0..500 {
            let msg = create_test_message(i);
            let _ = manager.send_event(AppEvent::MessageAdded(msg));
        }
        let send_elapsed = start.elapsed();

        // 送信自体は非同期なので即座に完了すべき
        assert!(
            send_elapsed < Duration::from_millis(100),
            "Sending 500 messages took {:?}, expected < 100ms",
            send_elapsed
        );

        // 処理完了を待機
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 状態取得がブロックされないことを確認
        let read_start = Instant::now();
        let state = manager.get_state_async().await;
        let read_elapsed = read_start.elapsed();

        assert!(
            read_elapsed < Duration::from_millis(ONE_FRAME_MS),
            "State read after 500 messages took {:?}",
            read_elapsed
        );

        // メッセージが処理されていることを確認
        assert!(
            state.message_count() > 0,
            "Messages should have been processed"
        );
    }

    /// 並行アクセスでフリーズしないことを確認
    #[tokio::test]
    async fn test_concurrent_access_no_freeze() {
        let manager = Arc::new(StateManager::new());
        let barrier = Arc::new(Barrier::new(10));
        let max_latency = Arc::new(AtomicUsize::new(0));

        // 10タスクが同時にアクセス
        let mut handles = vec![];
        for task_id in 0..10 {
            let manager = Arc::clone(&manager);
            let barrier = Arc::clone(&barrier);
            let max_latency = Arc::clone(&max_latency);

            handles.push(tokio::spawn(async move {
                // 全タスクが揃うまで待機
                barrier.wait().await;

                // 状態アクセスとイベント送信を交互に実行
                for i in 0..50 {
                    if i % 2 == 0 {
                        let start = Instant::now();
                        let _state = manager.get_state_async().await;
                        let elapsed = start.elapsed().as_micros() as usize;
                        max_latency.fetch_max(elapsed, Ordering::Relaxed);
                    } else {
                        let msg = create_test_message(task_id * 100 + i);
                        let _ = manager.send_event(AppEvent::MessageAdded(msg));
                    }
                }
            }));
        }

        // すべてのタスクの完了を待機
        for handle in handles {
            handle.await.unwrap();
        }

        let max_latency_us = max_latency.load(Ordering::Relaxed);
        let max_latency_ms = max_latency_us / 1000;

        // 最大レイテンシが32ms以下であることを確認
        assert!(
            max_latency_ms < 32,
            "Max latency during concurrent access: {}ms, expected < 32ms",
            max_latency_ms
        );
    }

    /// イベント処理が順序を保証することを確認
    #[tokio::test]
    async fn test_event_ordering_preserved() {
        let broadcaster = StateBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        // 順番にイベントを送信
        for i in 0..10 {
            broadcaster.broadcast(StateChange::MessageAdded {
                count: i,
                latest: None,
            });
        }

        // 受信順序が送信順序と一致することを確認
        for expected in 0..10 {
            let received =
                tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;

            match received {
                Ok(Ok(StateChange::MessageAdded { count, .. })) => {
                    assert_eq!(
                        count, expected,
                        "Event order mismatch: expected {}, got {}",
                        expected, count
                    );
                }
                _ => panic!("Failed to receive event {}", expected),
            }
        }
    }

    /// 統計更新がパフォーマンスに影響しないことを確認
    #[tokio::test]
    async fn test_stats_update_performance() {
        let manager = StateManager::new();

        // 多数の統計更新イベントを送信
        let start = Instant::now();
        for _ in 0..100 {
            let stats = ChatStats {
                total_messages: 1000,
                messages_per_minute: 10.0,
                uptime_seconds: 3600,
                last_message_time: Some(chrono::Utc::now()),
                start_time: Some(chrono::Utc::now()),
            };
            let _ = manager.send_event(AppEvent::StatsUpdated(stats));
        }
        let elapsed = start.elapsed();

        // 100件の統計更新が10ms以内に送信完了すべき
        assert!(
            elapsed < Duration::from_millis(10),
            "Sending 100 stats updates took {:?}",
            elapsed
        );
    }

    /// サービス状態変更が即座に反映されることを確認
    #[tokio::test]
    async fn test_service_state_change_latency() {
        let manager = StateManager::new();
        let mut rx = manager.subscribe();

        // サービス状態変更を送信
        let _ = manager.send_event(AppEvent::ServiceStateChanged(ServiceState::Connected));

        // ブロードキャスト受信を待機
        let timeout = Duration::from_millis(100);
        let received = tokio::time::timeout(timeout, rx.recv()).await;

        match received {
            Ok(Ok(StateChange::ServiceStateChanged(state))) => {
                assert_eq!(
                    state,
                    ServiceState::Connected,
                    "Service state mismatch"
                );
            }
            _ => panic!("Failed to receive service state change"),
        }
    }
}

/// 統合フリーズ検出テスト
#[cfg(test)]
mod integration_freeze_tests {
    use super::*;

    /// リアルワールドシナリオ: メッセージ受信中のUI操作をシミュレート
    #[tokio::test]
    async fn test_realworld_scenario_message_reception() {
        let manager = Arc::new(StateManager::new());
        let manager_write = Arc::clone(&manager);
        let manager_read = Arc::clone(&manager);

        // メッセージ受信をシミュレート（バックグラウンドで継続的に送信）
        let write_task = tokio::spawn(async move {
            for i in 0..200 {
                let msg = create_test_message(i);
                let _ = manager_write.send_event(AppEvent::MessageAdded(msg));
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        // UI読み取りをシミュレート（定期的に状態取得）
        let mut read_latencies = vec![];
        for _ in 0..50 {
            let start = Instant::now();
            let _state = manager_read.get_state_async().await;
            read_latencies.push(start.elapsed());
            tokio::time::sleep(Duration::from_millis(32)).await; // 約30FPS
        }

        // 書き込みタスクの完了を待機
        let _ = write_task.await;

        // すべての読み取りが16ms以内に完了していることを確認
        let max_latency = read_latencies.iter().max().unwrap();
        let avg_latency: Duration =
            read_latencies.iter().sum::<Duration>() / read_latencies.len() as u32;

        println!(
            "Read latencies - Max: {:?}, Avg: {:?}",
            max_latency, avg_latency
        );

        assert!(
            *max_latency < Duration::from_millis(ONE_FRAME_MS * 2),
            "Max read latency {:?} exceeded threshold during message reception",
            max_latency
        );
    }

    /// ストレステスト: 極端な負荷でもフリーズしないことを確認
    #[tokio::test]
    async fn test_stress_no_freeze_under_extreme_load() {
        let manager = Arc::new(StateManager::new());
        let broadcaster = Arc::new(StateBroadcaster::new());

        // 複数のサブスクライバーを作成
        let mut subscribers = vec![];
        for _ in 0..5 {
            subscribers.push(broadcaster.subscribe());
        }

        // バースト送信（一度に大量のイベント）
        let start = Instant::now();
        for i in 0..1000 {
            let msg = create_test_message(i);
            let msg_clone = msg.clone();
            let _ = manager.send_event(AppEvent::MessageAdded(msg));
            broadcaster.broadcast(StateChange::MessageAdded {
                count: i,
                latest: Some(msg_clone),
            });
        }
        let burst_elapsed = start.elapsed();

        // バースト送信が高速であることを確認
        assert!(
            burst_elapsed < Duration::from_millis(200),
            "Burst send of 1000 events took {:?}",
            burst_elapsed
        );

        // バースト後も状態アクセスが正常であることを確認
        let read_start = Instant::now();
        let _state = manager.get_state_async().await;
        let read_elapsed = read_start.elapsed();

        assert!(
            read_elapsed < Duration::from_millis(ONE_FRAME_MS * 2),
            "State read after burst took {:?}",
            read_elapsed
        );
    }
}
