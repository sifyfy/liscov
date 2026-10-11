//! Chat monitoring commands

use crate::AppState;
use crate::commands::auth;
use crate::commands::config::ConfigState;
use crate::connection::{
    ConnectionInfo, DISCONNECT_TIMEOUT, MAX_CONNECTIONS, StreamConnection, disconnect_all,
};
use crate::core::api::InnerTubeClient;
use crate::core::chat_runtime::{
    MonitoringDeps, MonitoringOutput, MonitoringTarget, run_monitoring_loop,
};
use crate::core::models::{
    ChatMessage, ChatMode, ConnectionStatus, Platform, ReactionSummary, ReactionUpdate,
    extract_video_id,
};
use crate::core::raw_response::SaveConfig;
use crate::database::{self, ViewerAnnotation};
use crate::errors::CommandError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

/// 監視ループの通知を Tauri のイベントにする（02_chat.md「Tauriイベント」）
struct TauriMonitoringOutput {
    app: AppHandle,
    connection_id: u64,
    platform: String,
    broadcaster_name: String,
}

impl MonitoringOutput for TauriMonitoringOutput {
    fn save_config(&self) -> SaveConfig {
        self.app.state::<ConfigState>().get().raw_response
    }

    fn message(&self, message: &ChatMessage, annotation: &ViewerAnnotation) {
        let gui_msg = GuiChatMessage::from_with_connection(
            message.clone(),
            self.connection_id,
            &self.platform,
            &self.broadcaster_name,
            annotation.clone(),
        );
        let _ = self.app.emit("chat:message", &gui_msg);
    }

    fn reaction(&self, update: &ReactionUpdate) {
        let gui_update = GuiReactionUpdate {
            connection_id: self.connection_id,
            update: update.clone(),
        };
        let _ = self.app.emit("chat:reaction", &gui_update);
    }
}

/// Result of connecting to a stream
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct ConnectionResult {
    pub success: bool,
    pub stream_title: Option<String>,
    pub broadcaster_channel_id: Option<String>,
    pub broadcaster_name: Option<String>,
    pub is_replay: bool,
    pub error: Option<String>,
    pub session_id: Option<String>,
    /// この接続に割り当てられた接続ID（success=trueのときのみ有効）
    pub connection_id: u64,
}

impl From<ConnectionStatus> for ConnectionResult {
    fn from(status: ConnectionStatus) -> Self {
        Self {
            success: status.is_connected,
            stream_title: status.stream_title,
            broadcaster_channel_id: status.broadcaster_channel_id,
            broadcaster_name: status.broadcaster_name,
            is_replay: status.is_replay,
            error: status.error,
            session_id: None,
            // 呼び出し元で設定する
            connection_id: 0,
        }
    }
}

/// Message run (text or emoji)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub enum MessageRun {
    Text {
        content: String,
    },
    Emoji {
        emoji_id: String,
        image_url: String,
        alt_text: String,
    },
}

/// Badge information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct BadgeInfo {
    pub badge_type: String,
    pub label: String,
    pub tooltip: Option<String>,
    pub image_url: Option<String>,
}

/// SuperChat color scheme from YouTube
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct SuperChatColors {
    pub header_background: String,
    pub header_text: String,
    pub body_background: String,
    pub body_text: String,
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GuiMessageMetadata {
    pub amount: Option<String>,
    pub milestone_months: Option<u32>,
    pub gift_count: Option<u32>,
    // ギフト名（gift のみ）
    pub gift_name: Option<String>,
    // ギフト画像 URL（gift のみ）
    pub gift_image_url: Option<String>,
    // ジュエル数（gift で本文から読めたときのみ）
    pub jewel_count: Option<u32>,
    pub badges: Vec<String>,
    pub badge_info: Vec<BadgeInfo>,
    pub is_moderator: bool,
    pub is_verified: bool,
    pub superchat_colors: Option<SuperChatColors>,
}

/// GUI-friendly chat message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GuiChatMessage {
    pub id: String,
    pub timestamp: String,
    pub timestamp_usec: String,
    pub author: String,
    pub author_icon_url: Option<String>,
    pub channel_id: String,
    pub content: String,
    pub runs: Vec<MessageRun>,
    pub message_type: String,
    pub amount: Option<String>,
    pub is_member: bool,
    pub is_first_time_viewer: bool,
    pub in_stream_comment_count: Option<u32>,
    pub metadata: Option<GuiMessageMetadata>,
    /// この接続に割り当てられた接続ID
    pub connection_id: u64,
    /// 配信プラットフォーム（例: "youtube"）
    pub platform: String,
    /// 配信者名
    pub broadcaster_name: String,
    /// この接続の配信者のもとで登録した、投稿者の読み仮名（02_chat.md「配信中の画面」。読み上げと同じ値）
    pub author_reading: Option<String>,
    /// この接続の配信者のもとで登録した、投稿者のメモ（02_chat.md「1 行の形」。本文の下に出す。読み上げには使わない）
    pub author_notes: Option<String>,
}

impl From<ChatMessage> for GuiChatMessage {
    fn from(msg: ChatMessage) -> Self {
        use crate::core::models::MessageType;
        let message_type = msg.message_type.as_str().to_string();
        let amount = match &msg.message_type {
            MessageType::SuperChat { amount } | MessageType::SuperSticker { amount } => {
                Some(amount.clone())
            }
            _ => None,
        };
        let milestone_months = match &msg.message_type {
            MessageType::Membership { milestone_months } => *milestone_months,
            _ => None,
        };
        let gift_count = match &msg.message_type {
            MessageType::MembershipGift { gift_count } => Some(*gift_count),
            _ => None,
        };
        let (gift_name, gift_image_url, jewel_count) = match &msg.message_type {
            MessageType::Gift(gift) => (
                Some(gift.gift_name.clone()),
                gift.gift_image_url.clone(),
                gift.jewel_count,
            ),
            _ => (None, None, None),
        };

        // runs を core models から GUI models に変換
        let runs: Vec<MessageRun> = msg
            .runs
            .into_iter()
            .map(|run| match run {
                crate::core::models::MessageRun::Text { content } => MessageRun::Text { content },
                crate::core::models::MessageRun::Emoji {
                    emoji_id,
                    image_url,
                    alt_text,
                } => MessageRun::Emoji {
                    emoji_id,
                    image_url,
                    alt_text,
                },
            })
            .collect();

        // metadata を変換。種別に件数などがあるのに metadata が無いとき (パーサーはメンバーシップギフトに付けない) は、
        // 空の metadata に種別の値を載せる。載せないと件数が画面に届かない
        let has_type_details =
            milestone_months.is_some() || gift_count.is_some() || gift_name.is_some();
        let metadata = msg
            .metadata
            .or_else(|| has_type_details.then(crate::core::models::MessageMetadata::default))
            .map(|m| GuiMessageMetadata {
                amount: m.amount,
                milestone_months,
                gift_count,
                gift_name,
                gift_image_url,
                jewel_count,
                badges: m.badges,
                badge_info: m
                    .badge_info
                    .into_iter()
                    .map(|b| BadgeInfo {
                        badge_type: b.badge_type,
                        label: b.label.clone(),
                        tooltip: b.tooltip.or(Some(b.label)),
                        image_url: b.icon_url,
                    })
                    .collect(),
                is_moderator: m.is_moderator,
                is_verified: m.is_verified,
                superchat_colors: m.superchat_colors.map(|c| SuperChatColors {
                    header_background: c.header_background,
                    header_text: c.header_text,
                    body_background: c.body_background,
                    body_text: c.body_text,
                }),
            });

        Self {
            id: msg.id,
            timestamp: msg.timestamp,
            timestamp_usec: msg.timestamp_usec,
            author: msg.author,
            author_icon_url: msg.author_icon_url,
            channel_id: msg.channel_id,
            content: msg.content,
            runs,
            message_type,
            amount,
            is_member: msg.is_member,
            is_first_time_viewer: msg.is_first_time_viewer,
            in_stream_comment_count: msg.in_stream_comment_count,
            metadata,
            // デフォルト値（呼び出し元で from_with_connection を使うべき）
            connection_id: 0,
            platform: "youtube".to_string(),
            broadcaster_name: String::new(),
            author_reading: None,
            author_notes: None,
        }
    }
}

impl GuiChatMessage {
    /// 接続情報と投稿者の読み仮名・メモ付きで ChatMessage から GuiChatMessage を生成する
    pub fn from_with_connection(
        msg: ChatMessage,
        connection_id: u64,
        platform: &str,
        broadcaster_name: &str,
        annotation: ViewerAnnotation,
    ) -> Self {
        Self {
            connection_id,
            platform: platform.to_string(),
            broadcaster_name: broadcaster_name.to_string(),
            author_reading: annotation.reading,
            author_notes: annotation.notes,
            ..Self::from(msg)
        }
    }
}

/// Tauri イベント `chat:reaction` のペイロード（02_chat.md GuiReactionUpdate）
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GuiReactionUpdate {
    pub connection_id: u64,
    #[serde(flatten)]
    #[ts(flatten)]
    pub update: ReactionUpdate,
}

/// Connect to a YouTube live stream and start monitoring chat
#[tauri::command]
pub async fn connect_to_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    config_state: State<'_, ConfigState>,
    url: String,
    chat_mode: Option<String>,
) -> Result<ConnectionResult, CommandError> {
    // 同時接続の枠を取る（接続中も数える。接続に失敗したら drop で空く。02_chat.md「多接続」）
    let slot = state.connection_slots.try_reserve().ok_or_else(|| {
        CommandError::InvalidInput(format!(
            "同時接続数の上限（{}）に達しています",
            MAX_CONNECTIONS
        ))
    })?;

    // 新しい接続IDを採番
    let connection_id = state.next_connection_id.fetch_add(1, Ordering::SeqCst) + 1;
    tracing::info!(
        "connect_to_stream called with url: {}, chat_mode: {:?}, connection_id: {}",
        url,
        chat_mode,
        connection_id
    );

    // URLからビデオIDを抽出
    let video_id = extract_video_id(&url)
        .ok_or_else(|| CommandError::InvalidInput("Invalid YouTube URL".to_string()))?;

    // チャットモードをパース
    let mode = match chat_mode.as_deref() {
        Some("all") | Some("AllChat") => ChatMode::AllChat,
        _ => ChatMode::TopChat,
    };

    // InnerTube クライアントを作成・初期化
    let mut client = InnerTubeClient::new(&video_id);

    // 認証クッキーをストレージから読み込んでクライアントに設定（メンバー限定配信用）
    let config = config_state.get();
    if let Ok(cookies) = auth::load_cookies(&config.storage.mode) {
        tracing::info!("Auth cookies loaded, setting on InnerTube client");
        client.set_auth(cookies);
    } else {
        tracing::debug!("No auth cookies available, connecting without authentication");
    }

    let status = client
        .initialize()
        .await
        .map_err(|e| CommandError::ConnectionFailed(format!("Failed to connect: {}", e)))?;

    // 初期化後にチャットモードを設定（continuation token が必要）
    if status.is_connected && !client.set_chat_mode(mode) {
        tracing::warn!("Failed to set chat mode to {:?}, using default", mode);
    }

    tracing::info!(
        "Connection status: is_connected={}, stream_title={:?}, broadcaster_channel_id={:?}, broadcaster_name={:?}",
        status.is_connected,
        status.stream_title,
        status.broadcaster_channel_id,
        status.broadcaster_name
    );

    let mut result = ConnectionResult::from(status.clone());
    result.connection_id = connection_id;

    if result.success {
        // データベースセッションを作成
        let session_id = {
            let db_guard = state.database.read().await;
            if let Some(db) = db_guard.as_ref() {
                let conn = db.connection().await;
                match database::create_session(
                    &conn,
                    Some(&url),
                    result.stream_title.as_deref(),
                    result.broadcaster_channel_id.as_deref(),
                    result.broadcaster_name.as_deref(),
                ) {
                    Ok(id) => {
                        tracing::info!("Created session: {}", id);
                        Some(id)
                    }
                    Err(e) => {
                        tracing::error!("Failed to create session: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        };

        result.session_id = session_id.clone();

        // キャンセレーショントークンを生成
        let cancellation_token = CancellationToken::new();

        // チャットモード制御用の watch チャネルを生成
        let (chat_mode_tx, chat_mode_rx) = watch::channel(mode);

        // 監視タスクの共有依存を構築
        let deps = MonitoringDeps::from_state(&state);

        let conn_id = connection_id;
        let output = TauriMonitoringOutput {
            app: app.clone(),
            connection_id,
            platform: Platform::YouTube.as_str().to_string(),
            broadcaster_name: result.broadcaster_name.clone().unwrap_or_default(),
        };
        let target = MonitoringTarget {
            connection_id,
            video_id,
            session_id: session_id.clone(),
            broadcaster_id: result.broadcaster_channel_id.clone(),
            cancellation_token: cancellation_token.clone(),
            chat_mode_rx,
        };

        // StreamConnection を生成して connections マップに追加
        let stream_conn = StreamConnection {
            id: connection_id,
            platform: Platform::YouTube,
            stream_url: url.clone(),
            stream_title: result.stream_title.clone().unwrap_or_default(),
            broadcaster_name: result.broadcaster_name.clone().unwrap_or_default(),
            broadcaster_channel_id: result.broadcaster_channel_id.clone().unwrap_or_default(),
            is_monitoring: true,
            session_id: session_id.clone(),
            cancellation_token: cancellation_token.clone(),
            task_handle: None, // spawn後に設定
            chat_mode_tx,
            slot,
        };

        {
            let mut connections = state.connections.write().await;
            connections.insert(connection_id, stream_conn);
        }

        // 監視タスク終了後のクリーンアップ用に connections Arc をキャプチャ
        let connections_for_cleanup = Arc::clone(&state.connections);
        let app_for_cleanup = app.clone();

        // 監視タスクをスポーン
        let handle = tokio::spawn(async move {
            let end = run_monitoring_loop(deps, client, target, output).await;

            // 監視タスク終了後: connections マップに残っている場合はクリーンアップ
            // （disconnect_stream 経由で既に削除済みの場合はスキップ）
            let was_present = {
                let mut connections = connections_for_cleanup.write().await;
                connections.remove(&conn_id).is_some()
            };
            if was_present {
                tracing::info!(
                    "監視タスクが自律終了 — フロントエンドに切断を通知 connection_id: {}",
                    conn_id
                );
                let _ = app_for_cleanup.emit(
                    "chat:connection",
                    ConnectionResult {
                        success: false,
                        stream_title: None,
                        broadcaster_channel_id: None,
                        broadcaster_name: None,
                        is_replay: false,
                        error: Some(end.error_message()),
                        session_id: None,
                        connection_id: conn_id,
                    },
                );
            }
        });

        // JoinHandle を StreamConnection に格納
        {
            let mut connections = state.connections.write().await;
            if let Some(conn) = connections.get_mut(&connection_id) {
                conn.task_handle = Some(handle);
            }
        }

        // 接続イベントを emit
        let _ = app.emit("chat:connection", &result);
    }

    Ok(result)
}

/// 特定の配信への接続を切断する
#[tauri::command]
pub async fn disconnect_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    connection_id: u64,
) -> Result<(), CommandError> {
    tracing::info!(
        "disconnect_stream called for connection_id: {}",
        connection_id
    );

    // 接続のキャンセレーショントークンを取得してキャンセル
    let task_handle = {
        let mut connections = state.connections.write().await;
        let conn = connections.get_mut(&connection_id).ok_or_else(|| {
            CommandError::NotConnected(format!("接続 {} が見つかりません", connection_id))
        })?;

        // トークンをキャンセル
        conn.cancellation_token.cancel();
        tracing::debug!("disconnect_stream: connection {} cancelled", connection_id);

        // JoinHandle を取り出す
        conn.task_handle.take()
    };

    // 切断イベントを emit
    let _ = app.emit(
        "chat:connection",
        ConnectionResult {
            success: false,
            stream_title: None,
            broadcaster_channel_id: None,
            broadcaster_name: None,
            is_replay: false,
            error: None,
            session_id: None,
            connection_id,
        },
    );

    // JoinHandle を待機（タイムアウト付き）
    if let Some(handle) = task_handle {
        match tokio::time::timeout(DISCONNECT_TIMEOUT, handle).await {
            Ok(Ok(())) => tracing::debug!("disconnect_stream: task {} completed", connection_id),
            Ok(Err(e)) => {
                tracing::warn!("disconnect_stream: task {} panicked: {}", connection_id, e)
            }
            Err(_) => tracing::warn!("disconnect_stream: task {} timed out", connection_id),
        }
    }

    // connections マップから削除
    {
        let mut connections = state.connections.write().await;
        connections.remove(&connection_id);
    }

    Ok(())
}

/// 全接続を一括切断する
#[tauri::command]
pub async fn disconnect_all_streams(state: State<'_, AppState>) -> Result<(), CommandError> {
    tracing::info!("disconnect_all_streams called");
    disconnect_all(&state.connections, DISCONNECT_TIMEOUT).await;
    Ok(())
}

/// 現在アクティブな全接続情報を取得する
#[tauri::command]
pub async fn get_connections(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionInfo>, CommandError> {
    let connections = state.connections.read().await;
    Ok(connections.values().map(ConnectionInfo::from).collect())
}

/// その接続の配信のリアクション累計を返す（接続直後・F5 リロード後の復元用）
#[tauri::command]
pub async fn get_connection_reactions(
    state: State<'_, AppState>,
    connection_id: u64,
) -> Result<ReactionSummary, CommandError> {
    let stream_url = {
        let connections = state.connections.read().await;
        connections
            .get(&connection_id)
            .map(|conn| conn.stream_url.clone())
            .ok_or_else(|| {
                CommandError::NotConnected(format!("接続 {} が見つかりません", connection_id))
            })?
    };
    let video_id = extract_video_id(&stream_url)
        .ok_or_else(|| CommandError::InvalidInput("Invalid YouTube URL".to_string()))?;

    let conn = state.db_connection().await?;
    database::get_reaction_summary(&conn, &video_id)
        .map_err(|e| CommandError::DatabaseError(e.to_string()))
}

/// チャットモード（TopChat/AllChat）を変更する
///
/// watch チャネル経由で監視タスクにモード変更要求を送信する。
/// 実際の適用は次回ポーリング時に行われる（非同期）。
#[tauri::command]
pub async fn set_chat_mode(
    state: State<'_, AppState>,
    connection_id: u64,
    mode: String,
) -> Result<bool, CommandError> {
    let chat_mode = match mode.as_str() {
        "all" | "AllChat" => ChatMode::AllChat,
        _ => ChatMode::TopChat,
    };

    // connections ロックを短時間で解放するため、send() 完了後すぐ drop
    let connections = state.connections.read().await;
    let conn = connections.get(&connection_id).ok_or_else(|| {
        CommandError::NotConnected(format!("接続 {} が見つかりません", connection_id))
    })?;

    conn.chat_mode_tx
        .send(chat_mode)
        .map_err(|_| CommandError::Internal("監視タスクが既に終了しています".to_string()))?;

    tracing::info!(
        "チャットモード変更要求を送信: connection_id={}, mode={:?}",
        connection_id,
        chat_mode
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::MessageType;

    fn text_message() -> ChatMessage {
        ChatMessage {
            id: "m1".to_string(),
            timestamp: "12:00:00".to_string(),
            timestamp_usec: "1000000".to_string(),
            message_type: MessageType::Text,
            author: "@田中-abc".to_string(),
            author_icon_url: None,
            channel_id: "UCxxx".to_string(),
            content: "こんにちは".to_string(),
            runs: vec![],
            metadata: None,
            is_member: false,
            is_first_time_viewer: false,
            in_stream_comment_count: None,
        }
    }

    // 02_chat.md「メッセージ表示仕様」優先順位 1 の面: メンバーシップギフトは「メンバーシップを N 件ギフト」。
    // パーサーはメンバーシップギフトに metadata を付けないが、件数は画面に届く
    #[test]
    fn membership_gift_count_reaches_gui_without_metadata() {
        let msg = ChatMessage {
            message_type: MessageType::MembershipGift { gift_count: 5 },
            content: "Sent 5 Channel gift memberships".to_string(),
            ..text_message()
        };
        let gui = GuiChatMessage::from(msg);
        assert_eq!(gui.metadata.and_then(|m| m.gift_count), Some(5));
    }

    // 02_chat.md「配信中の画面」1 行の形: 読み仮名が登録されている視聴者は、名前の横に小さく出す
    #[test]
    fn gui_message_carries_reading_of_the_viewer() {
        let gui = GuiChatMessage::from_with_connection(
            text_message(),
            1,
            "youtube",
            "配信者A",
            ViewerAnnotation {
                reading: Some("たなか".to_string()),
                notes: Some("赤スパの人".to_string()),
            },
        );
        assert_eq!(gui.author_reading.as_deref(), Some("たなか"));
        assert_eq!(gui.author_notes.as_deref(), Some("赤スパの人"));
        assert_eq!(gui.connection_id, 1);
        assert_eq!(gui.broadcaster_name, "配信者A");
    }

    #[test]
    fn gui_message_without_reading() {
        let gui = GuiChatMessage::from_with_connection(
            text_message(),
            1,
            "youtube",
            "配信者A",
            ViewerAnnotation::default(),
        );
        assert_eq!(gui.author_reading, None);
        assert_eq!(gui.author_notes, None);
    }
}
