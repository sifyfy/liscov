//! 配信接続の管理

use crate::core::models::{ChatMode, Platform};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{RwLock, watch};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

/// 同時接続数の上限
pub const MAX_CONNECTIONS: usize = 32;

/// 切断・アプリ終了で監視タスクの終了処理（セッションを閉じる）を待つ上限
pub const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// 個別の配信接続を表す
///
/// InnerTube クライアントは監視タスク内の Arc<RwLock> で管理され、
/// この構造体には含まれない（ライフタイムが異なるため）。
/// チャットモードの変更は `chat_mode_tx` (watch チャネル) を通じて
/// 監視タスクに非同期で伝達される。
pub struct StreamConnection {
    pub id: u64,
    pub platform: Platform,
    pub stream_url: String,
    pub stream_title: String,
    pub broadcaster_name: String,
    pub broadcaster_channel_id: String,
    pub is_monitoring: bool,
    pub session_id: Option<String>,
    pub cancellation_token: CancellationToken,
    pub task_handle: Option<JoinHandle<()>>,
    /// チャットモード変更要求を監視タスクに伝達する watch チャネル
    pub chat_mode_tx: watch::Sender<ChatMode>,
}

/// フロントエンドに公開する接続情報（シリアライズ可能）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct ConnectionInfo {
    pub id: u64,
    pub platform: Platform,
    pub stream_url: String,
    pub stream_title: String,
    pub broadcaster_name: String,
    pub broadcaster_channel_id: String,
    pub is_monitoring: bool,
    pub is_cancelling: bool,
}

impl From<&StreamConnection> for ConnectionInfo {
    fn from(conn: &StreamConnection) -> Self {
        Self {
            id: conn.id,
            platform: conn.platform,
            stream_url: conn.stream_url.clone(),
            stream_title: conn.stream_title.clone(),
            broadcaster_name: conn.broadcaster_name.clone(),
            broadcaster_channel_id: conn.broadcaster_channel_id.clone(),
            is_monitoring: conn.is_monitoring,
            // キャンセル済みかどうかをCancellationTokenから取得
            is_cancelling: conn.cancellation_token.is_cancelled(),
        }
    }
}

/// 今ある全接続を止め、監視タスクの終了処理（セッションを閉じる）を最大 `timeout` 待って一覧から消す
///
/// 消すのは止めた接続だけ。待っている間に成立した接続は止めずに残す（02_chat.md「多接続」）。
pub async fn disconnect_all(
    connections: &RwLock<HashMap<u64, StreamConnection>>,
    timeout: Duration,
) {
    let (ids, handles): (Vec<u64>, Vec<Option<JoinHandle<()>>>) = {
        let mut connections = connections.write().await;
        connections
            .iter_mut()
            .map(|(id, conn)| {
                conn.cancellation_token.cancel();
                (*id, conn.task_handle.take())
            })
            .unzip()
    };

    // 並列に待つ（直列だと N × timeout になるため）
    let waits = ids
        .iter()
        .zip(handles)
        .filter_map(|(id, handle)| handle.map(|handle| (*id, handle)))
        .map(|(id, handle)| async move {
            match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(())) => tracing::debug!("disconnect_all: task {} completed", id),
                Ok(Err(e)) => tracing::warn!("disconnect_all: task {} panicked: {}", id, e),
                Err(_) => tracing::warn!("disconnect_all: task {} timed out", id),
            }
        });
    futures_util::future::join_all(waits).await;

    let mut connections = connections.write().await;
    for id in &ids {
        connections.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用のStreamConnectionを作成するヘルパー
    fn make_connection(id: u64) -> StreamConnection {
        let (chat_mode_tx, _) = watch::channel(ChatMode::TopChat);
        StreamConnection {
            id,
            platform: Platform::YouTube,
            stream_url: "https://youtube.com/watch?v=test123".to_string(),
            stream_title: "テスト配信".to_string(),
            broadcaster_name: "テスト配信者".to_string(),
            broadcaster_channel_id: "UCtest123".to_string(),
            is_monitoring: false,
            session_id: None,
            cancellation_token: CancellationToken::new(),
            task_handle: None,
            chat_mode_tx,
        }
    }

    #[test]
    fn connection_info_from_stream_connection() {
        // StreamConnectionからConnectionInfoへの変換でフィールドが正しくコピーされる
        let conn = make_connection(42);
        let info = ConnectionInfo::from(&conn);

        assert_eq!(info.id, 42);
        assert_eq!(info.platform, Platform::YouTube);
        assert_eq!(info.stream_url, "https://youtube.com/watch?v=test123");
        assert_eq!(info.stream_title, "テスト配信");
        assert_eq!(info.broadcaster_name, "テスト配信者");
        assert_eq!(info.broadcaster_channel_id, "UCtest123");
        assert!(!info.is_monitoring);
        assert!(!info.is_cancelling);
    }

    #[test]
    fn connection_info_shows_cancelling_state() {
        // CancellationTokenをキャンセルするとis_cancellingがtrueになる
        let conn = make_connection(1);
        conn.cancellation_token.cancel();

        let info = ConnectionInfo::from(&conn);
        assert!(info.is_cancelling);
    }

    // 02_chat.md 多接続: 全切断は押した時点の接続だけを切り、終了処理を待つ。切断の途中で成立した接続は残る
    #[tokio::test]
    async fn disconnect_all_keeps_connection_established_during_disconnect() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let connections: Arc<RwLock<HashMap<u64, StreamConnection>>> = Arc::default();
        let finished = Arc::new(AtomicBool::new(false));
        let mut first = make_connection(1);
        first.task_handle = Some({
            let token = first.cancellation_token.clone();
            let connections = Arc::clone(&connections);
            let finished = Arc::clone(&finished);
            tokio::spawn(async move {
                token.cancelled().await;
                // 切断を待っている間に別の接続が成立した
                connections.write().await.insert(2, make_connection(2));
                // 監視タスクの終了処理（finish_session）まで終わった
                finished.store(true, Ordering::SeqCst);
            })
        });
        connections.write().await.insert(1, first);

        disconnect_all(&connections, std::time::Duration::from_secs(1)).await;

        assert!(finished.load(Ordering::SeqCst), "終了処理を待たずに戻った");
        let remaining = connections.read().await;
        assert_eq!(remaining.keys().copied().collect::<Vec<_>>(), vec![2]);
        assert!(!remaining[&2].cancellation_token.is_cancelled());
    }

    #[test]
    fn max_connections_constant() {
        // MAX_CONNECTIONSは32
        assert_eq!(MAX_CONNECTIONS, 32);
    }
}
