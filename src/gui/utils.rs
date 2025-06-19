// GUI用ユーティリティ関数

use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
// use serde::{Deserialize, Serialize}; // 現在未使用のため一時的にコメントアウト
use chrono::Local;
use directories::ProjectDirs;
use glob::glob;
use rand::Rng;
use std::fs;
use std::path::{Path, PathBuf};

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

/// XDGディレクトリからログディレクトリを取得
pub fn get_default_log_dir() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "sifyfy", "liscov")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    let log_dir = project_dirs.data_dir().join("logs");
    Ok(log_dir)
}

/// ログファイル名を生成（衝突回避付き）
pub fn generate_log_filename() -> String {
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let random_id: u32 = rand::thread_rng().gen();
    format!("liscov_{}_{:08x}.log", timestamp, random_id)
}

/// 古いログファイルをクリーンアップ
pub fn cleanup_old_log_files(log_dir: &Path, max_files: u32, pattern: &str) -> anyhow::Result<()> {
    if !log_dir.exists() {
        return Ok(());
    }

    let pattern_path = log_dir.join(pattern);
    let pattern_str = pattern_path.to_string_lossy();

    let mut log_files: Vec<_> = glob(&pattern_str)?
        .filter_map(|entry| entry.ok())
        .filter(|path| {
            // liscov_YYYY-MM-DD_HH-MM-SS_[a-f0-9]{8}.logパターンをチェック
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                let re = regex::Regex::new(
                    r"^liscov_\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2}_[a-f0-9]{8}\.log$",
                )
                .unwrap();
                re.is_match(filename)
            } else {
                false
            }
        })
        .filter_map(|path| {
            // ファイルのメタデータを取得
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(created) = metadata.created() {
                    Some((path, created))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // 作成日時でソート（新しいものが最初）
    log_files.sort_by(|a, b| b.1.cmp(&a.1));

    // max_files を超える古いファイルを削除
    if log_files.len() > max_files as usize {
        let files_to_delete = &log_files[max_files as usize..];

        for (file_path, _) in files_to_delete {
            if let Err(e) = fs::remove_file(file_path) {
                warn!(
                    "古いログファイルの削除に失敗: {} - {}",
                    file_path.display(),
                    e
                );
            } else {
                debug!("古いログファイルを削除: {}", file_path.display());
            }
        }

        info!(
            "{}個の古いログファイルをクリーンアップしました",
            files_to_delete.len()
        );
    }

    Ok(())
}

/// 強化されたログ初期化（設定とディレクトリ指定対応）
pub fn init_logging_with_config(
    log_config: &crate::gui::config_manager::LogConfig,
    custom_log_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    if !log_config.enable_file_logging {
        // ファイル出力無効の場合は従来通り
        return init_logging();
    }

    // ログディレクトリを決定（優先度順）
    let log_dir = if let Some(custom_dir) = custom_log_dir {
        custom_dir
    } else if let Some(config_dir) = &log_config.log_dir {
        config_dir.clone()
    } else {
        get_default_log_dir()?
    };

    // ログディレクトリを作成
    fs::create_dir_all(&log_dir)?;

    // 古いログファイルをクリーンアップ（同期実行）
    if log_config.auto_cleanup_enabled {
        if let Err(e) = cleanup_old_log_files(
            &log_dir,
            log_config.max_log_files,
            &log_config.log_filename_pattern,
        ) {
            warn!("ログファイルクリーンアップエラー: {}", e);
        }
    }

    // ログファイル名を生成
    let log_filename = generate_log_filename();
    let log_file_path = log_dir.join(log_filename);

    // ログレベルフィルターを設定
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_config.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // ファイル出力用のappenderを作成
    let file_appender =
        tracing_appender::rolling::never(&log_dir, log_file_path.file_name().unwrap());
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // コンソール出力とファイル出力の両方を設定
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .compact(),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json(),
        );

    subscriber.try_init()?;

    info!("ログファイル出力開始: {}", log_file_path.display());

    Ok(())
}

/// 強化されたログ初期化（後方互換性用）
pub fn init_logging() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let subscriber = tracing_subscriber::registry().with(env_filter).with(
        tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .compact(),
    );

    subscriber.try_init()?;

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

/// アプリケーション状態のダンプ（デバッグ用）
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

/// デスクトップサイズを取得（Tao/DioxusのEventLoopを使用）
pub fn get_primary_monitor_size() -> Option<(u32, u32)> {
    // Tao EventLoopを作成してモニター情報を取得
    let event_loop = dioxus::desktop::tao::event_loop::EventLoop::new();
    if let Some(monitor) = event_loop.primary_monitor() {
        let size = monitor.size();
        Some((size.width, size.height))
    } else {
        None
    }
}

/// 利用可能な全モニターのサイズを取得
pub fn get_available_monitors_bounds() -> Vec<(i32, i32, u32, u32)> {
    let event_loop = dioxus::desktop::tao::event_loop::EventLoop::new();
    event_loop
        .available_monitors()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            (position.x, position.y, size.width, size.height)
        })
        .collect()
}

/// ウィンドウ位置がデスクトップ範囲内にあるかチェック
pub fn validate_window_bounds(config: &mut crate::gui::config_manager::WindowConfig) {
    // Taoを使用してモニター情報を取得（より統一的なアプローチ）
    if let Some((primary_width, primary_height)) = get_primary_monitor_size() {
        // プライマリモニターサイズを使用して検証
        let screen_width = primary_width;
        let screen_height = primary_height;

        // ウィンドウがスクリーン範囲外にある場合は調整
        if config.x < 0 || config.x > (screen_width as i32) - (config.width as i32) {
            config.x = 100;
        }
        if config.y < 0 || config.y > (screen_height as i32) - (config.height as i32) {
            config.y = 100;
        }

        // ウィンドウサイズがスクリーンより大きい場合は調整
        if config.width > screen_width {
            config.width = screen_width.min(900);
        }
        if config.height > screen_height {
            config.height = screen_height.min(1080);
        }

        debug!(
            "🖥️ プライマリモニターサイズ: {}x{}, ウィンドウ位置調整済み",
            screen_width, screen_height
        );

        // 複数モニター環境での詳細情報をログ出力
        let monitors = get_available_monitors_bounds();
        if monitors.len() > 1 {
            debug!("🖥️ 複数モニター検出: {} 個のモニター", monitors.len());
            for (i, (x, y, w, h)) in monitors.iter().enumerate() {
                debug!("   モニター {}: {}x{} at ({}, {})", i + 1, w, h, x, y);
            }
        }
    } else {
        // フォールバック: 基本的な検証のみ
        if config.x < 0 {
            config.x = 100;
        }
        if config.y < 0 {
            config.y = 100;
        }
        if config.width < 400 {
            config.width = 400;
        }
        if config.height < 300 {
            config.height = 300;
        }
        warn!("⚠️ モニター情報を取得できませんでした。基本的な検証のみ実行");
    }
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
