use crate::gui::models::GuiChatMessage;
use crate::gui::services::{LiveChatService, ServiceState};
use crate::gui::state_management::{get_state_manager, AppEvent};
use dioxus::prelude::*;

/// イベント駆動ライブチャットハンドル
pub struct EventDrivenLiveChatHandle {
    service: LiveChatService,
}

impl EventDrivenLiveChatHandle {
    pub fn new() -> Self {
        Self {
            service: LiveChatService::new(),
        }
    }

    /// 監視を開始
    pub async fn start_monitoring(
        &mut self,
        url: String,
        output_file: Option<String>,
    ) -> anyhow::Result<()> {
        // 停止状態をリセット
        let _ =
            get_state_manager().send_event(AppEvent::StoppingStateChanged { is_stopping: false });

        // 接続開始状態に設定
        let _ =
            get_state_manager().send_event(AppEvent::ServiceStateChanged(ServiceState::Connecting));

        tracing::info!("▶️ Starting live chat monitoring for URL: {}", url);

        // サービスを開始
        match self.service.start_monitoring(&url, output_file).await {
            Ok(_receiver) => {
                // 接続成功
                let _ = get_state_manager()
                    .send_event(AppEvent::ConnectionChanged { is_connected: true });
                let _ = get_state_manager()
                    .send_event(AppEvent::ServiceStateChanged(ServiceState::Connected));

                tracing::info!("✅ Live chat monitoring started successfully");
                Ok(())
            }
            Err(e) => {
                // 接続失敗
                let error_message = self.format_user_friendly_error(&e);
                let _ = get_state_manager().send_event(AppEvent::ServiceStateChanged(
                    ServiceState::Error(error_message.clone()),
                ));
                let _ = get_state_manager().send_event(AppEvent::ConnectionChanged {
                    is_connected: false,
                });

                tracing::error!("❌ Failed to start monitoring: {}", e);
                Err(e)
            }
        }
    }

    /// 監視を停止
    pub async fn stop_monitoring(&mut self) -> anyhow::Result<()> {
        // 即座に停止処理中フラグを設定
        let _ =
            get_state_manager().send_event(AppEvent::StoppingStateChanged { is_stopping: true });

        tracing::info!("⏹️ Stopping live chat monitoring");

        // サービスを停止
        match self.service.stop_monitoring().await {
            Ok(_) => {
                // 停止成功
                let _ = get_state_manager().send_event(AppEvent::ConnectionChanged {
                    is_connected: false,
                });
                let _ = get_state_manager()
                    .send_event(AppEvent::ServiceStateChanged(ServiceState::Idle));
                let _ = get_state_manager()
                    .send_event(AppEvent::StoppingStateChanged { is_stopping: false });

                tracing::info!("✅ Live chat monitoring stopped successfully");
                Ok(())
            }
            Err(e) => {
                // 停止失敗（まれ）
                let _ = get_state_manager()
                    .send_event(AppEvent::StoppingStateChanged { is_stopping: false });
                tracing::error!("❌ Error stopping monitoring: {}", e);
                Err(e)
            }
        }
    }

    /// メッセージをクリア
    pub fn clear_messages(&self) {
        let _ = get_state_manager().send_event(AppEvent::MessagesCleared);
        tracing::info!("🗑️ Messages cleared");
    }

    /// テストメッセージを追加
    pub fn add_test_message(
        &self,
        author: &str,
        content: &str,
        message_type: crate::gui::models::MessageType,
    ) {
        let message = GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            message_type,
            author: author.to_string(),
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            metadata: None,
            is_member: false,
        };

        let _ = get_state_manager().send_event(AppEvent::MessageAdded(message));
    }

    /// ユーザーフレンドリーなエラーメッセージに変換
    fn format_user_friendly_error(&self, error: &anyhow::Error) -> String {
        let error_string = error.to_string();

        if error_string.contains("continuation not found") {
            "❌ YouTubeライブ配信が見つかりません。\n\n考えられる原因:\n• 配信が終了している\n• URLが間違っている\n• 配信がプライベートまたは制限されている\n• チャットが無効になっている\n\n✅ 解決方法:\n• 現在進行中のライブ配信URLを使用してください\n• URLが正確であることを確認してください".to_string()
        } else if error_string.contains("network") || error_string.contains("timeout") {
            "❌ ネットワーク接続エラー\n\n• インターネット接続を確認してください\n• ファイアウォールがブロックしていないか確認してください".to_string()
        } else if error_string.contains("rate limit") {
            "❌ API制限に達しました\n\n• しばらく待ってから再試行してください\n• 短時間での連続アクセスを避けてください".to_string()
        } else {
            format!("❌ 監視開始エラー: {}", error_string)
        }
    }
}

/// グローバルライブチャットサービスのインスタンス
static LIVE_CHAT_HANDLE: std::sync::OnceLock<std::sync::Mutex<EventDrivenLiveChatHandle>> =
    std::sync::OnceLock::new();

/// グローバルライブチャットハンドルを取得
pub fn get_live_chat_handle() -> &'static std::sync::Mutex<EventDrivenLiveChatHandle> {
    LIVE_CHAT_HANDLE.get_or_init(|| {
        tracing::info!("🏗️ Creating global live chat handle");
        std::sync::Mutex::new(EventDrivenLiveChatHandle::new())
    })
}

/// ライブチャット操作用の公開インターフェース（簡易版）
pub struct LiveChatActions;

impl LiveChatActions {
    /// ライブチャット監視を開始
    pub fn start_monitoring(url: String, output_file: Option<String>) {
        let handle = get_live_chat_handle();
        spawn(async move {
            if let Ok(mut service) = handle.lock() {
                match service.start_monitoring(url, output_file).await {
                    Ok(_) => {
                        tracing::info!("✅ Live chat monitoring started via LiveChatActions");
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to start monitoring via LiveChatActions: {}", e);
                    }
                }
            } else {
                tracing::error!("❌ Failed to acquire service lock for start_monitoring");
            }
        });
    }

    /// ライブチャット監視を停止
    pub fn stop_monitoring() {
        let handle = get_live_chat_handle();
        spawn(async move {
            if let Ok(mut service) = handle.lock() {
                match service.stop_monitoring().await {
                    Ok(_) => {
                        tracing::info!("✅ Live chat monitoring stopped via LiveChatActions");
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to stop monitoring via LiveChatActions: {}", e);
                    }
                }
            } else {
                tracing::error!("❌ Failed to acquire service lock for stop_monitoring");
            }
        });
    }

    /// メッセージをクリア
    pub fn clear_messages() {
        if let Ok(service) = get_live_chat_handle().lock() {
            service.clear_messages();
        }
    }

    /// テストメッセージを追加
    pub fn add_test_message(
        author: &str,
        content: &str,
        message_type: crate::gui::models::MessageType,
    ) {
        if let Ok(service) = get_live_chat_handle().lock() {
            service.add_test_message(author, content, message_type);
        }
    }
}
