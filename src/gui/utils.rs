// GUI用ユーティリティ関数

use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// URLバリデーション
pub fn validate_youtube_url(url: &str) -> bool {
    url.starts_with("https://youtube.com/watch?v=")
        || url.starts_with("https://www.youtube.com/watch?v=")
        || url.starts_with("https://youtu.be/")
}

/// ビデオIDをURLから抽出
pub fn extract_video_id(url: &str) -> Option<String> {
    // 簡単な実装（Phase 1用）
    if let Some(start) = url.find("v=") {
        let id_part = &url[start + 2..];
        if let Some(end) = id_part.find('&') {
            Some(id_part[..end].to_string())
        } else {
            Some(id_part.to_string())
        }
    } else if url.contains("youtu.be/") {
        if let Some(start) = url.rfind('/') {
            let id_part = &url[start + 1..];
            if let Some(end) = id_part.find('?') {
                Some(id_part[..end].to_string())
            } else {
                Some(id_part.to_string())
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// 時刻フォーマット
pub fn format_timestamp() -> String {
    chrono::Utc::now().format("%H:%M:%S").to_string()
}

/// デバッグレベル設定
#[derive(Debug, Clone)]
pub enum DebugLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl DebugLevel {
    pub fn as_filter(&self) -> &'static str {
        match self {
            DebugLevel::Off => "off",
            DebugLevel::Error => "error",
            DebugLevel::Warn => "warn",
            DebugLevel::Info => "info",
            DebugLevel::Debug => "debug",
            DebugLevel::Trace => "trace",
        }
    }
}

/// 環境に応じたログ初期化（軽量版）
pub fn init_logging() -> anyhow::Result<()> {
    // RUST_LOG環境変数を最優先で使用
    let env_filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        // RUST_LOG環境変数が設定されている場合はそれを使用
        EnvFilter::try_new(rust_log)?
    } else {
        // RUST_LOG環境変数が設定されていない場合のみ独自の設定を使用
        let debug_level = std::env::var("LISCOV_DEBUG_LEVEL").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "info" // デバッグ版でもinfoレベルに軽量化
            } else {
                "warn" // リリース版はwarnレベルに軽量化
            }
            .to_string()
        });

        EnvFilter::try_new(format!(
            "liscov={},tokio=warn,hyper=warn,reqwest=warn", // すべてのライブラリのログを削減
            debug_level
        ))?
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false) // ターゲット情報を削除してI/O負荷軽減
                .with_thread_ids(false) // スレッドID出力を削除してI/O負荷軽減
                .with_file(false) // ファイル名出力を削除してI/O負荷軽減
                .with_line_number(false), // 行番号出力を削除してI/O負荷軽減
        )
        .with(env_filter)
        .init();

    // 起動時のログも削減
    Ok(())
}

/// デバッグ用のメッセージダンプ
pub fn dump_gui_message(message: &crate::gui::models::GuiChatMessage, context: &str) {
    debug!(
        context = context,
        timestamp = %message.timestamp,
        message_type = ?message.message_type,
        author = %message.author,
        channel_id = %message.channel_id,
        content_length = message.content.len(),
        content_preview = %message.content.chars().take(50).collect::<String>(),
        has_metadata = message.metadata.is_some(),
        "📨 GUI Message processed"
    );
}

/// サービス状態の変更をログ
pub fn log_service_state_change(
    old_state: &crate::gui::services::ServiceState,
    new_state: &crate::gui::services::ServiceState,
) {
    match (old_state, new_state) {
        (old, new) if std::mem::discriminant(old) != std::mem::discriminant(new) => {
            info!(
                old_state = ?old,
                new_state = ?new,
                "🔄 Service state changed"
            );
        }
        _ => {
            debug!(
                state = ?new_state,
                "📊 Service state checked"
            );
        }
    }
}

/// API リクエスト/レスポンスのログ
pub fn log_api_request(url: &str, request_count: usize) {
    debug!(
        url = %url,
        request_count = request_count,
        "📡 API request sent"
    );
}

pub fn log_api_response(
    response_size: usize,
    message_count: usize,
    has_continuation: bool,
    duration_ms: u64,
) {
    debug!(
        response_size_bytes = response_size,
        message_count = message_count,
        has_continuation = has_continuation,
        duration_ms = duration_ms,
        "📨 API response received"
    );
}

/// UI更新のパフォーマンス測定
pub struct UiUpdateTimer {
    start: std::time::Instant,
    context: String,
}

impl UiUpdateTimer {
    pub fn new(context: impl Into<String>) -> Self {
        Self {
            start: std::time::Instant::now(),
            context: context.into(),
        }
    }
}

impl Drop for UiUpdateTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        if duration.as_millis() > 16 {
            // 60fps以下の場合警告
            warn!(
                context = %self.context,
                duration_ms = duration.as_millis(),
                "⚠️ Slow UI update detected"
            );
        } else {
            debug!(
                context = %self.context,
                duration_ms = duration.as_millis(),
                "✅ UI update completed"
            );
        }
    }
}

/// エラー詳細のログ
pub fn log_error_with_context(error: &anyhow::Error, context: &str) {
    error!(
        context = context,
        error = %error,
        error_chain = ?error.chain().collect::<Vec<_>>(),
        "❌ Error occurred"
    );
}

/// メモリ使用量の監視（デバッグ用）
#[cfg(debug_assertions)]
pub fn log_memory_usage(context: &str) {
    // 簡易的なメモリ使用量ログ（実際の実装では専用ライブラリを使用）
    debug!(context = context, "💾 Memory usage check (placeholder)");
}

#[cfg(not(debug_assertions))]
pub fn log_memory_usage(_context: &str) {
    // リリースビルドでは何もしない
}

/// ファイル操作のログ
pub fn log_file_operation(
    operation: &str,
    file_path: &str,
    success: bool,
    size_bytes: Option<usize>,
) {
    if success {
        info!(
            operation = operation,
            file_path = file_path,
            size_bytes = size_bytes,
            "📁 File operation successful"
        );
    } else {
        error!(
            operation = operation,
            file_path = file_path,
            "❌ File operation failed"
        );
    }
}

/// 設定値のデバッグダンプ
pub fn dump_app_state(state: &crate::gui::models::AppState) {
    debug!(
        url = %state.url,
        output_file = %state.output_file,
        is_connected = state.is_connected,
        message_count = state.message_count,
        request_count = state.request_count,
        messages_in_memory = state.messages.len(),
        "📊 App state dump"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_youtube_url() {
        assert!(validate_youtube_url(
            "https://youtube.com/watch?v=dQw4w9WgXcQ"
        ));
        assert!(validate_youtube_url(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        ));
        assert!(validate_youtube_url("https://youtu.be/dQw4w9WgXcQ"));
        assert!(!validate_youtube_url("https://example.com"));
    }

    #[test]
    fn test_extract_video_id() {
        assert_eq!(
            extract_video_id("https://youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(extract_video_id("https://example.com"), None);
    }
}
