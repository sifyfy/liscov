//! 状態変更のブロードキャストシステム
//!
//! ポーリングベースの状態同期を、プッシュ型のイベント通知に置き換える。
//! これにより、UIスレッドのブロッキングを排除し、フリーズを防止する。

use std::sync::{Arc, OnceLock};
use tokio::sync::broadcast;

use crate::gui::models::GuiChatMessage;
use crate::gui::services::ServiceState;
use crate::gui::state_management::ChatStats;

/// 状態変更イベント
///
/// StateManagerで発生した変更をサブスクライバーに通知する。
/// 各イベントは必要最小限のデータのみを含み、フルクローンを回避する。
#[derive(Clone, Debug)]
pub enum StateChange {
    /// 新しいメッセージが追加された
    MessageAdded {
        /// 現在のメッセージ数
        count: usize,
        /// 最新のメッセージ（オプション）
        latest: Option<GuiChatMessage>,
    },

    /// 複数のメッセージが追加された
    MessagesAdded {
        /// 現在のメッセージ数
        count: usize,
        /// 追加されたメッセージ数
        added_count: usize,
    },

    /// メッセージがクリアされた
    MessagesCleared,

    /// 接続状態が変更された
    ConnectionChanged {
        /// 接続中かどうか
        is_connected: bool,
    },

    /// サービス状態が変更された
    ServiceStateChanged(ServiceState),

    /// 停止処理状態が変更された
    StoppingChanged(bool),

    /// 統計情報が更新された
    StatsUpdated(ChatStats),

    /// 継続トークンが更新された
    ContinuationTokenUpdated(Option<String>),

    /// 現在のURLが更新された
    CurrentUrlUpdated(Option<String>),
}

/// 状態変更のブロードキャスター
///
/// tokio::sync::broadcastを使用して、複数のサブスクライバーに
/// 状態変更を非同期で通知する。
pub struct StateBroadcaster {
    /// ブロードキャスト送信者
    sender: broadcast::Sender<StateChange>,
}

impl StateBroadcaster {
    /// 新しいブロードキャスターを作成
    ///
    /// バッファサイズは256に設定。これにより、遅いサブスクライバーが
    /// いても256件までのイベントをバッファリングできる。
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self { sender }
    }

    /// 新しいサブスクリプションを作成
    ///
    /// 返されたReceiverで状態変更を受信できる。
    /// サブスクライバーが遅延すると、古いイベントは破棄される（lagged error）。
    pub fn subscribe(&self) -> broadcast::Receiver<StateChange> {
        self.sender.subscribe()
    }

    /// 状態変更をブロードキャスト
    ///
    /// すべてのサブスクライバーに状態変更を通知する。
    /// サブスクライバーがいない場合はイベントは破棄される。
    /// この操作は非ブロッキングで、即座に完了する。
    pub fn broadcast(&self, change: StateChange) {
        // send()はResultを返すが、受信者がいない場合のエラーは無視する
        let _ = self.sender.send(change);
    }

    /// 現在のサブスクライバー数を取得
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for StateBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルブロードキャスターのインスタンス
static GLOBAL_BROADCASTER: OnceLock<Arc<StateBroadcaster>> = OnceLock::new();

/// グローバルブロードキャスターを取得
pub fn get_broadcaster() -> &'static Arc<StateBroadcaster> {
    GLOBAL_BROADCASTER.get_or_init(|| {
        tracing::info!("🔊 [BROADCASTER] Global StateBroadcaster initialized");
        Arc::new(StateBroadcaster::new())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_broadcaster_creation() {
        let broadcaster = StateBroadcaster::new();
        assert_eq!(broadcaster.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_subscription() {
        let broadcaster = StateBroadcaster::new();
        let _rx1 = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 1);

        let _rx2 = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 2);
    }

    #[tokio::test]
    async fn test_broadcast_message_added() {
        let broadcaster = StateBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        broadcaster.broadcast(StateChange::MessageAdded {
            count: 1,
            latest: None,
        });

        let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("timeout")
            .expect("receive error");

        match received {
            StateChange::MessageAdded { count, .. } => assert_eq!(count, 1),
            _ => panic!("unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_connection_changed() {
        let broadcaster = StateBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        broadcaster.broadcast(StateChange::ConnectionChanged { is_connected: true });

        let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("timeout")
            .expect("receive error");

        match received {
            StateChange::ConnectionChanged { is_connected } => assert!(is_connected),
            _ => panic!("unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broadcaster = StateBroadcaster::new();
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();

        broadcaster.broadcast(StateChange::MessagesCleared);

        // 両方のサブスクライバーが同じイベントを受信
        let r1 = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
            .await
            .expect("timeout")
            .expect("receive error");

        let r2 = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
            .await
            .expect("timeout")
            .expect("receive error");

        assert!(matches!(r1, StateChange::MessagesCleared));
        assert!(matches!(r2, StateChange::MessagesCleared));
    }

    #[tokio::test]
    async fn test_broadcast_is_non_blocking() {
        let broadcaster = StateBroadcaster::new();

        // サブスクライバーなしでもブロードキャストは即座に完了
        let start = std::time::Instant::now();
        for i in 0..1000 {
            broadcaster.broadcast(StateChange::MessageAdded {
                count: i,
                latest: None,
            });
        }
        let elapsed = start.elapsed();

        // 1000件のブロードキャストが1ms以内に完了すべき
        assert!(
            elapsed < Duration::from_millis(10),
            "broadcast took {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_global_broadcaster() {
        let broadcaster1 = get_broadcaster();
        let broadcaster2 = get_broadcaster();

        // 同じインスタンスを返す
        assert!(Arc::ptr_eq(broadcaster1, broadcaster2));
    }
}
