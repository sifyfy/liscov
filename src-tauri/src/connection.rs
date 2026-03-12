//! 配信接続の管理

use crate::core::api::InnerTubeClient;
use crate::core::models::Platform;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

/// 同時接続数の上限
pub const MAX_CONNECTIONS: usize = 32;

/// 個別の配信接続を表す
pub struct StreamConnection {
    pub id: u64,
    pub platform: Platform,
    pub stream_url: String,
    pub stream_title: String,
    pub broadcaster_name: String,
    pub broadcaster_channel_id: String,
    pub is_monitoring: bool,
    pub innertube_client: Option<InnerTubeClient>,
    pub session_id: Option<String>,
    pub cancellation_token: CancellationToken,
    pub task_handle: Option<JoinHandle<()>>,
}

/// フロントエンドに公開する接続情報（シリアライズ可能）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用のStreamConnectionを作成するヘルパー
    fn make_connection(id: u64) -> StreamConnection {
        StreamConnection {
            id,
            platform: Platform::YouTube,
            stream_url: "https://youtube.com/watch?v=test123".to_string(),
            stream_title: "テスト配信".to_string(),
            broadcaster_name: "テスト配信者".to_string(),
            broadcaster_channel_id: "UCtest123".to_string(),
            is_monitoring: false,
            innertube_client: None,
            session_id: None,
            cancellation_token: CancellationToken::new(),
            task_handle: None,
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

    #[test]
    fn max_connections_constant() {
        // MAX_CONNECTIONSは32
        assert_eq!(MAX_CONNECTIONS, 32);
    }
}
