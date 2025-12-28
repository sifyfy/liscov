//! UIコンポーネントテスト (Phase 3.2)
//!
//! Dioxusコンポーネントのレンダリングパフォーマンスとシグナル連携をテスト。

use liscov::gui::models::{GuiChatMessage, MessageType};
use liscov::gui::services::ServiceState;
use liscov::gui::signal_manager::UpdatePriority;
use liscov::gui::state_broadcaster::StateChange;
use liscov::gui::state_management::{AppEvent, StateManager};
use std::sync::Arc;
use std::time::{Duration, Instant};

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

/// Phase 3.2: シグナル更新テスト
#[cfg(test)]
mod signal_update_tests {
    use super::*;

    /// シグナル更新の優先度処理テスト
    #[test]
    fn test_signal_update_priority() {
        // 高優先度更新は低優先度より先に処理されることを確認
        let _updates = vec![
            (UpdatePriority::Low, "low_update"),
            (UpdatePriority::Medium, "medium_update"),
            (UpdatePriority::High, "high_update"),
        ];

        // 優先度の順序を確認
        let high_priority = UpdatePriority::High;
        let low_priority = UpdatePriority::Low;

        // 高優先度は低優先度より小さい数値（先に処理される）
        assert!(high_priority < low_priority);
    }

    /// StateChangeイベントの種別テスト
    #[test]
    fn test_state_change_types() {
        // 各StateChangeイベントが正しく区別されることを確認
        let events = vec![
            StateChange::MessageAdded { count: 1, latest: None },
            StateChange::MessagesCleared,
            StateChange::ConnectionChanged { is_connected: true },
            StateChange::StoppingChanged(false),
        ];

        for event in events {
            // 各イベントが有効であることを確認
            match event {
                StateChange::MessageAdded { .. } => {}
                StateChange::MessagesCleared => {}
                StateChange::ConnectionChanged { .. } => {}
                StateChange::StoppingChanged(_) => {}
                _ => {}
            }
        }
    }
}

/// Phase 3.2: メッセージストリームテスト
#[cfg(test)]
mod message_stream_tests {
    use super::*;
    use liscov::gui::message_stream::{DisplayLimit, MessageStream, MessageStreamConfig};

    /// MessageStreamの初期化テスト
    #[test]
    fn test_message_stream_initialization() {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(100),
            max_display_count: 100,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };

        let stream = MessageStream::new(config);

        assert_eq!(stream.display_count(), 0);
        assert_eq!(stream.archived_count(), 0);
        assert_eq!(stream.total_count(), 0);
    }

    /// メッセージ追加のパフォーマンステスト
    #[test]
    fn test_message_push_performance() {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(100),
            max_display_count: 100,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };

        let mut stream = MessageStream::new(config);

        // 1000メッセージの追加時間を測定
        let start = Instant::now();
        for i in 0..1000 {
            stream.push_message(create_test_message(i));
        }
        let elapsed = start.elapsed();

        // 1000メッセージの追加が100ms以内に完了すべき
        assert!(
            elapsed < Duration::from_millis(100),
            "Pushing 1000 messages took {:?}, expected < 100ms",
            elapsed
        );

        // 表示件数が上限に制限されていることを確認
        assert_eq!(stream.display_count(), 100);

        // アーカイブにメッセージが移動していることを確認
        assert_eq!(stream.archived_count(), 900);
    }

    /// 表示制限の動作テスト
    #[test]
    fn test_display_limit_enforcement() {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(50),
            max_display_count: 50,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };

        let mut stream = MessageStream::new(config);

        // 100メッセージを追加
        for i in 0..100 {
            stream.push_message(create_test_message(i));
        }

        // 表示は50件に制限
        assert_eq!(stream.display_count(), 50);

        // 残りはアーカイブに
        assert_eq!(stream.archived_count(), 50);

        // 合計は正確
        assert_eq!(stream.total_count(), 100);
    }

    /// アーカイブ検索のテスト
    #[test]
    fn test_archive_search() {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(10),
            max_display_count: 10,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };

        let mut stream = MessageStream::new(config);

        // テストメッセージを追加
        for i in 0..50 {
            let mut msg = create_test_message(i);
            if i == 25 {
                msg.content = "Special keyword message".to_string();
            }
            stream.push_message(msg);
        }

        // 内容検索
        let results = stream.search_by_content("keyword");
        assert_eq!(results.len(), 1);

        // 投稿者検索
        let author_results = stream.search_by_author("TestUser5");
        assert!(!author_results.is_empty());
    }

    /// メッセージクリアのテスト
    #[test]
    fn test_message_stream_clear() {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(100),
            max_display_count: 100,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };

        let mut stream = MessageStream::new(config);

        // メッセージを追加
        for i in 0..50 {
            stream.push_message(create_test_message(i));
        }

        assert_eq!(stream.total_count(), 50);

        // クリア
        stream.clear();

        assert_eq!(stream.display_count(), 0);
        assert_eq!(stream.archived_count(), 0);
        assert_eq!(stream.total_count(), 0);
    }

    /// 統計情報の正確性テスト
    #[test]
    fn test_message_stream_stats() {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(100),
            max_display_count: 100,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };

        let mut stream = MessageStream::new(config);

        for i in 0..150 {
            stream.push_message(create_test_message(i));
        }

        let stats = stream.stats();

        assert_eq!(stats.display_count, 100);
        assert_eq!(stats.archived_count, 50);
        assert_eq!(stats.total_count, 150);

        // 削減率の確認（150件中100件表示 = 33%削減）
        assert!(stats.effective_reduction_percent > 0);
    }
}

/// Phase 3.2: ブロードキャスト連携テスト
/// 注: StateManagerのイベントループはDioxusランタイム内で動作するため、
/// 直接的なブロードキャストテストはfreeze_detection_testsで実施している。
/// ここではStateBroadcaster単体の機能をテストする。
#[cfg(test)]
mod broadcast_integration_tests {
    use super::*;
    use liscov::gui::state_broadcaster::StateBroadcaster;

    /// StateBroadcasterの直接送受信テスト
    #[tokio::test]
    async fn test_broadcaster_send_receive() {
        let broadcaster = StateBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        // メッセージ追加イベントを送信
        broadcaster.broadcast(StateChange::MessageAdded {
            count: 1,
            latest: Some(create_test_message(1)),
        });

        // ブロードキャストを受信（タイムアウト付き）
        let timeout = Duration::from_millis(100);
        let received = tokio::time::timeout(timeout, rx.recv()).await;

        assert!(
            received.is_ok(),
            "Should receive broadcast within 100ms"
        );

        match received.unwrap() {
            Ok(StateChange::MessageAdded { count, .. }) => {
                assert_eq!(count, 1);
            }
            _ => panic!("Expected MessageAdded event"),
        }
    }

    /// 複数イベントの順序保証テスト
    #[tokio::test]
    async fn test_event_order_preservation() {
        let broadcaster = StateBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        // 複数のイベントを順番に送信
        for i in 0..5 {
            broadcaster.broadcast(StateChange::MessageAdded {
                count: i + 1,
                latest: Some(create_test_message(i)),
            });
        }

        // イベントの順序を確認
        let mut received_count = 0;
        let timeout = Duration::from_millis(100);

        for expected in 1..=5 {
            match tokio::time::timeout(timeout, rx.recv()).await {
                Ok(Ok(StateChange::MessageAdded { count, .. })) => {
                    assert_eq!(count, expected, "Event order mismatch");
                    received_count += 1;
                }
                Ok(Err(_)) | Err(_) => break,
                _ => {}
            }
        }

        assert_eq!(received_count, 5, "Should receive all 5 events");
    }

    /// 接続状態変更のブロードキャストテスト
    #[tokio::test]
    async fn test_connection_state_broadcast() {
        let broadcaster = StateBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        // 接続状態変更を送信
        broadcaster.broadcast(StateChange::ConnectionChanged { is_connected: true });

        let timeout = Duration::from_millis(100);
        let received = tokio::time::timeout(timeout, rx.recv()).await;

        match received {
            Ok(Ok(StateChange::ConnectionChanged { is_connected })) => {
                assert!(is_connected);
            }
            _ => panic!("Expected ConnectionChanged event"),
        }
    }

    /// サービス状態変更のブロードキャストテスト
    #[tokio::test]
    async fn test_service_state_broadcast() {
        let broadcaster = StateBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        // サービス状態変更を送信
        broadcaster.broadcast(StateChange::ServiceStateChanged(ServiceState::Connected));

        let timeout = Duration::from_millis(100);
        let received = tokio::time::timeout(timeout, rx.recv()).await;

        match received {
            Ok(Ok(StateChange::ServiceStateChanged(state))) => {
                assert_eq!(state, ServiceState::Connected);
            }
            _ => panic!("Expected ServiceStateChanged event"),
        }
    }
}

/// Phase 3.2: パフォーマンス統合テスト
#[cfg(test)]
mod performance_integration_tests {
    use super::*;

    /// 高負荷下でのシステム全体の応答性テスト
    #[tokio::test]
    async fn test_system_responsiveness_under_load() {
        let manager = Arc::new(StateManager::new());
        let _rx = manager.subscribe(); // サブスクライバーを保持してブロードキャストを有効化

        // バックグラウンドで大量のメッセージを送信
        let manager_clone = Arc::clone(&manager);
        let sender_task = tokio::spawn(async move {
            for i in 0..100 {
                let msg = create_test_message(i);
                let _ = manager_clone.send_event(AppEvent::MessageAdded(msg));
                // 少し間隔を空ける
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });

        // 同時に状態読み取りを行い、レスポンス時間を測定
        let mut max_latency = Duration::ZERO;
        for _ in 0..20 {
            let start = Instant::now();
            let _state = manager.get_state_async().await;
            let latency = start.elapsed();

            if latency > max_latency {
                max_latency = latency;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // タスクの完了を待機
        let _ = sender_task.await;

        // 負荷下でも応答時間が許容範囲内であることを確認
        assert!(
            max_latency < Duration::from_millis(50),
            "Max latency under load: {:?}, expected < 50ms",
            max_latency
        );
    }

    /// メモリ効率テスト
    #[test]
    fn test_memory_efficiency() {
        use liscov::gui::message_stream::{DisplayLimit, MessageStream, MessageStreamConfig};

        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(100),
            max_display_count: 100,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };

        let mut stream = MessageStream::new(config);

        // 1000メッセージを追加
        for i in 0..1000 {
            stream.push_message(create_test_message(i));
        }

        let stats = stream.stats();

        // メモリ使用量が合理的な範囲内であることを確認
        // 表示100件のみがフルデータ、残り900件はアーカイブ（軽量）
        assert!(
            stats.display_memory_mb() < 10.0,
            "Display memory: {} MB, expected < 10 MB",
            stats.display_memory_mb()
        );

        // 削減率が正しく計算されていることを確認
        assert!(
            stats.effective_reduction_percent >= 90,
            "Reduction: {}%, expected >= 90%",
            stats.effective_reduction_percent
        );
    }
}
