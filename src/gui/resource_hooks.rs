//! Dioxus 0.6.3 use_resource活用による非同期処理最適化
//!
//! Phase 2.2実装: APIサービスのuse_resource統合
//! - 自動依存関係管理
//! - Suspense対応
//! - バッチ処理最適化

use dioxus::prelude::*;
use std::time::Duration;
use tokio::time::interval;

use crate::gui::{
    app_context::{send_app_event, use_app_context},
    models::GuiChatMessage,
    services::{get_global_service, ServiceState},
    state_management::AppEvent,
};

/// use_resource統合メッセージ取得結果
#[derive(Debug, Clone)]
pub struct MessageFetchResult {
    pub messages: Vec<GuiChatMessage>,
    pub fetch_count: usize,
    pub error: Option<String>,
    pub last_fetch_time: std::time::Instant,
}

impl Default for MessageFetchResult {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            fetch_count: 0,
            error: None,
            last_fetch_time: std::time::Instant::now(),
        }
    }
}

/// Phase 2.2: use_resource活用によるメッセージ取得フック
/// 
/// Dioxus推奨パターン:
/// - 自動依存関係管理
/// - エラーハンドリング統合
/// - Suspenseコンポーネント対応
pub fn use_message_resource() -> Resource<MessageFetchResult> {
    let app_context = use_app_context();
    
    // 依存関係: 接続状態とURL
    let live_chat_state = app_context.live_chat;
    let is_connected = live_chat_state.read().is_connected;
    let current_url = live_chat_state.read().current_url.clone();
    let service_state = live_chat_state.read().service_state.clone();

    tracing::debug!(
        "🚀 [USE_RESOURCE] Initializing message resource: connected={}, state={:?}",
        is_connected,
        service_state
    );

    use_resource(move || {
        let current_url_captured = current_url.clone();
        let service_state_captured = service_state.clone();
        
        async move {
            tracing::info!(
                "🚀 [USE_RESOURCE] Starting message fetch resource for URL: {:?}",
                current_url_captured
            );

            // 接続されていない場合は空の結果を返す
            if !is_connected || !matches!(service_state_captured, ServiceState::Connected) {
                tracing::debug!("⏸️ [USE_RESOURCE] Not connected, returning empty result");
                return MessageFetchResult::default();
            }

            // 🚀 Dioxus推奨: spawn_blockingで重い処理を分離
            let fetch_result = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async {
                    fetch_messages_batch().await
                })
            }).await;

            match fetch_result {
                Ok(Ok(result)) => {
                    tracing::info!(
                        "✅ [USE_RESOURCE] Message fetch completed: {} messages",
                        result.messages.len()
                    );
                    result
                }
                Ok(Err(e)) => {
                    tracing::error!("❌ [USE_RESOURCE] Message fetch error: {}", e);
                    MessageFetchResult {
                        error: Some(e),
                        ..MessageFetchResult::default()
                    }
                }
                Err(e) => {
                    tracing::error!("❌ [USE_RESOURCE] Task join error: {}", e);
                    MessageFetchResult {
                        error: Some(format!("Task error: {}", e)),
                        ..MessageFetchResult::default()
                    }
                }
            }
        }
    })
}

/// バッチ処理によるメッセージ取得（内部実装）
async fn fetch_messages_batch() -> Result<MessageFetchResult, String> {
    let service_arc = get_global_service();
    let mut service = service_arc.lock().await;
    
    // 🚀 バッチ処理最適化: 一度に複数メッセージを取得
    match service.get_recent_messages_batch().await {
        Ok(messages) => {
            let fetch_count = messages.len();
            
            // 🚀 最適化: MessagesAddedイベントでバッチ送信
            if !messages.is_empty() {
                let send_result = send_app_event(AppEvent::MessagesAdded(messages.clone()));
                
                match send_result {
                    Ok(()) => {
                        tracing::info!(
                            "📤 [USE_RESOURCE] Sent {} messages via MessagesAdded event",
                            fetch_count
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ [USE_RESOURCE] Failed to send MessagesAdded event: {}",
                            e
                        );
                    }
                }
            }
            
            Ok(MessageFetchResult {
                messages,
                fetch_count,
                error: None,
                last_fetch_time: std::time::Instant::now(),
            })
        }
        Err(e) => {
            Err(format!("Service fetch error: {}", e))
        }
    }
}

/// Phase 2.2: リアルタイムメッセージストリームフック
/// 
/// Dioxus use_resource + インターバル処理による最適化
pub fn use_realtime_message_stream() -> Signal<Vec<GuiChatMessage>> {
    let app_context = use_app_context();
    let live_chat_state = app_context.live_chat;
    let message_stream_state = app_context.message_stream;
    
    // リアルタイムメッセージストリーム
    let realtime_messages = use_signal(Vec::<GuiChatMessage>::new);
    
    // 🚀 use_resource統合: 定期的なメッセージ取得
    use_effect(move || {
        let mut realtime_messages_clone = realtime_messages;
        let live_chat_clone = live_chat_state;
        let message_stream_clone = message_stream_state;
        
        spawn(async move {
            // 🚀 Dioxus推奨: インターバル処理の最適化
            let mut interval = interval(Duration::from_millis(500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            tracing::info!(
                "🚀 [REALTIME_STREAM] Starting optimized message stream (500ms interval)"
            );
            
            let mut last_message_count = 0;
            let mut cycle_count = 0;
            
            loop {
                interval.tick().await;
                cycle_count += 1;
                
                // 接続状態チェック
                let is_connected = live_chat_clone.read().is_connected;
                if !is_connected {
                    if cycle_count % 100 == 0 {
                        tracing::debug!(
                            "⏸️ [REALTIME_STREAM] Not connected, cycle #{}", 
                            cycle_count
                        );
                    }
                    continue;
                }
                
                // メッセージストリーム状態から差分を取得
                let current_messages = message_stream_clone.read().messages();
                let current_count = current_messages.len();
                
                // 🚀 差分更新最適化: 変更があった場合のみ更新
                if current_count != last_message_count {
                    tracing::info!(
                        "📨 [REALTIME_STREAM] Message count change: {} → {} (cycle #{})",
                        last_message_count,
                        current_count,
                        cycle_count
                    );
                    
                    // use_resourceパターン: 新着メッセージのみ抽出
                    let new_messages = if current_count > last_message_count {
                        let new_count = current_count - last_message_count;
                        current_messages
                            .iter()
                            .rev()
                            .take(new_count)
                            .cloned()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect()
                    } else {
                        // 全体更新が必要な場合（メッセージクリア等）
                        current_messages
                    };
                    
                    realtime_messages_clone.set(new_messages);
                    last_message_count = current_count;
                }
                
                // 定期的な生存確認ログ
                if cycle_count % 120 == 0 {
                    // 120 * 500ms = 60秒ごと
                    tracing::info!(
                        "🔄 [REALTIME_STREAM] Heartbeat: Cycle #{}, {} messages, connected: {}",
                        cycle_count,
                        current_count,
                        is_connected
                    );
                }
            }
        });
    });
    
    realtime_messages
}

/// Phase 2.2: Suspense対応メッセージローダーコンポーネント
#[component]
pub fn MessageLoader() -> Element {
    let message_resource = use_message_resource();
    
    match &*message_resource.read_unchecked() {
        Some(result) => {
            if let Some(error) = &result.error {
                rsx! {
                    div { class: "message-loader error",
                        "❌ メッセージ取得エラー: {error}"
                    }
                }
            } else {
                rsx! {
                    div { class: "message-loader success",
                        "✅ {result.fetch_count} 件のメッセージを取得"
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "message-loader loading",
                    "🔄 メッセージを読み込み中..."
                }
            }
        }
    }
}

/// Phase 2.2: 高度なuse_resourceパターン - 条件付きフェッチ
pub fn use_conditional_message_fetch(
    should_fetch: Signal<bool>,
    fetch_interval_ms: u64,
) -> Resource<Option<Vec<GuiChatMessage>>> {
    let app_context = use_app_context();
    let live_chat_state = app_context.live_chat;
    
    use_resource(move || {
        let should_fetch_value = *should_fetch.read();
        let is_connected = live_chat_state.read().is_connected;
        
        async move {
            if !should_fetch_value || !is_connected {
                tracing::debug!(
                    "⏸️ [CONDITIONAL_FETCH] Skipping fetch: should_fetch={}, connected={}",
                    should_fetch_value,
                    is_connected
                );
                return None;
            }
            
            tracing::info!(
                "🚀 [CONDITIONAL_FETCH] Starting conditional fetch with {}ms interval",
                fetch_interval_ms
            );
            
            // 🚀 use_resource + spawn_blocking最適化
            let fetch_result = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async {
                    let service_arc = get_global_service();
                    let mut service = service_arc.lock().await;
                    service.get_recent_messages_batch().await
                })
            }).await;
            
            match fetch_result {
                Ok(Ok(messages)) => {
                    tracing::info!(
                        "✅ [CONDITIONAL_FETCH] Fetched {} messages",
                        messages.len()
                    );
                    Some(messages)
                }
                Ok(Err(e)) => {
                    tracing::error!("❌ [CONDITIONAL_FETCH] Service error: {}", e);
                    None
                }
                Err(e) => {
                    tracing::error!("❌ [CONDITIONAL_FETCH] Task error: {}", e);
                    None
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_fetch_result_default() {
        let result = MessageFetchResult::default();
        assert_eq!(result.messages.len(), 0);
        assert_eq!(result.fetch_count, 0);
        assert!(result.error.is_none());
    }
}