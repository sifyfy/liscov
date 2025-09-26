//! 配信終了検出とエラー分類機能
//!
//! 連続403エラーによる配信終了の検出とシステムメッセージ送信

use crate::gui::state_management::{get_state_manager, AppEvent};
use crate::gui::system_messages::{StreamStats, SystemMessageGenerator};
use std::time::{Duration, Instant};

/// エラー分類
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    /// 403 Forbidden - 配信終了の可能性
    Forbidden,
    /// 404 Not Found - リソースが見つからない
    NotFound,
    /// タイムアウト
    Timeout,
    /// ネットワーク接続エラー
    Network,
    /// レート制限
    RateLimit,
    /// その他のエラー
    Other(String),
}

impl ErrorType {
    /// エラー文字列から分類を判定
    pub fn from_error_string(error_str: &str) -> Self {
        let error_lower = error_str.to_lowercase();

        if error_lower.contains("403") || error_lower.contains("forbidden") {
            ErrorType::Forbidden
        } else if error_lower.contains("404") || error_lower.contains("not found") {
            ErrorType::NotFound
        } else if error_lower.contains("timeout") {
            ErrorType::Timeout
        } else if error_lower.contains("connection") || error_lower.contains("network") {
            ErrorType::Network
        } else if error_lower.contains("rate limit") || error_lower.contains("429") {
            ErrorType::RateLimit
        } else {
            ErrorType::Other(error_str.to_string())
        }
    }

    /// エラータイプの表示名
    pub fn display_name(&self) -> &str {
        match self {
            ErrorType::Forbidden => "403 Forbidden",
            ErrorType::NotFound => "404 Not Found",
            ErrorType::Timeout => "Timeout",
            ErrorType::Network => "Network Error",
            ErrorType::RateLimit => "Rate Limit",
            ErrorType::Other(_) => "Unknown Error",
        }
    }
}

/// 配信終了検出器
#[derive(Debug)]
pub struct StreamEndDetector {
    /// 連続エラー数
    consecutive_errors: u32,
    /// 最後のエラータイプ
    last_error_type: Option<ErrorType>,
    /// 配信開始時刻
    stream_start_time: Option<Instant>,
    /// 配信終了済みフラグ
    stream_ended: bool,
    /// 最後のシステムメッセージ送信時刻（スパム防止）
    last_system_message_time: Option<Instant>,
    /// 最初のエラー発生時刻（2分制限用）
    first_error_time: Option<Instant>,
}

/// 検出結果
#[derive(Debug, Clone, PartialEq)]
pub enum DetectionResult {
    /// 継続 - エラーだが配信は継続と判定
    Continue,
    /// 警告レベル - システムメッセージを送信
    Warning,
    /// 配信終了検出 - 停止処理を実行
    StreamEnded,
    /// すでに終了済み
    AlreadyEnded,
}

impl StreamEndDetector {
    /// 新しいDetectorを作成
    pub fn new() -> Self {
        let mut detector = Self {
            consecutive_errors: 0,
            last_error_type: None,
            stream_start_time: Some(Instant::now()),
            stream_ended: false,
            last_system_message_time: None,
            first_error_time: None,
        };
        detector.reset();
        detector
    }

    /// 成功時の処理（エラーカウンターをリセット）
    pub fn on_success(&mut self) {
        if self.consecutive_errors > 0 {
            tracing::info!(
                "✅ [STREAM_DETECTOR] API success after {} consecutive errors - resetting counter",
                self.consecutive_errors
            );
        }
        self.consecutive_errors = 0;
        self.last_error_type = None;
        self.first_error_time = None;
        if self.stream_ended {
            tracing::info!("?? [STREAM_DETECTOR] Resetting stream_end flag after success");
        }
        self.stream_ended = false;
    }

    /// エラー発生時の処理と配信終了判定
    pub fn on_error(&mut self, error_str: &str) -> DetectionResult {
        if self.stream_ended {
            return DetectionResult::AlreadyEnded;
        }

        let error_type = ErrorType::from_error_string(error_str);
        self.consecutive_errors += 1;
        self.last_error_type = Some(error_type.clone());

        // 最初のエラー時刻を記録
        if self.first_error_time.is_none() {
            self.first_error_time = Some(Instant::now());
        }

        // 2分制限チェック（連続エラーが続いている場合）
        if let Some(first_error) = self.first_error_time {
            if first_error.elapsed() > Duration::from_secs(120) {
                tracing::warn!(
                    "⏰ [STREAM_DETECTOR] 2-minute error limit exceeded - forcing stream end"
                );
                self.stream_ended = true;
                self.send_stream_ended_message();
                return DetectionResult::StreamEnded;
            }
        }

        tracing::info!(
            "🔍 [STREAM_DETECTOR] Error classified: {} (consecutive: {})",
            error_type.display_name(),
            self.consecutive_errors
        );

        // 403エラーの場合のみ配信終了判定を行う
        if matches!(error_type, ErrorType::Forbidden) {
            self.check_stream_end_condition()
        } else {
            // 403以外のエラーは警告のみ
            if self.consecutive_errors >= 3 && self.should_send_system_message() {
                self.last_system_message_time = Some(Instant::now());
                self.send_error_warning_message(&error_type);
            }
            DetectionResult::Continue
        }
    }

    /// 配信終了条件をチェック
    fn check_stream_end_condition(&mut self) -> DetectionResult {
        match self.consecutive_errors {
            1..=2 => {
                // 軽微なエラー
                DetectionResult::Continue
            }
            3..=4 => {
                // 警告レベル
                if self.should_send_system_message() {
                    self.last_system_message_time = Some(Instant::now());
                    self.send_error_warning_message(&ErrorType::Forbidden);
                }
                DetectionResult::Warning
            }
            5..=7 => {
                // 注意レベル
                if self.should_send_system_message() {
                    self.last_system_message_time = Some(Instant::now());
                    self.send_error_warning_message(&ErrorType::Forbidden);
                }
                DetectionResult::Warning
            }
            _ => {
                // 8回以上 - 配信終了と判定
                tracing::info!(
                    "🔴 [STREAM_DETECTOR] Stream end detected: {} consecutive 403 errors",
                    self.consecutive_errors
                );
                self.stream_ended = true;
                self.send_stream_ended_message();
                DetectionResult::StreamEnded
            }
        }
    }

    /// システムメッセージを送信すべきかチェック（スパム防止）
    fn should_send_system_message(&self) -> bool {
        if let Some(last_time) = self.last_system_message_time {
            // 最後のメッセージから30秒以上経過している場合のみ送信
            last_time.elapsed() > Duration::from_secs(30)
        } else {
            true
        }
    }

    /// エラー警告メッセージを送信
    fn send_error_warning_message(&self, error_type: &ErrorType) {
        let message = SystemMessageGenerator::create_error_warning_message(
            self.consecutive_errors,
            error_type.display_name(),
        );

        match get_state_manager().send_event(AppEvent::MessageAdded(message)) {
            Ok(()) => {
                tracing::info!(
                    "📨 [STREAM_DETECTOR] Error warning message sent: {} consecutive errors",
                    self.consecutive_errors
                );
            }
            Err(e) => {
                tracing::error!("❌ [STREAM_DETECTOR] Failed to send error warning: {:?}", e);
            }
        }
    }

    /// 配信終了メッセージを送信
    fn send_stream_ended_message(&self) {
        // 統計情報を収集
        let state = match get_state_manager().get_state() {
            Ok(state) => state,
            Err(e) => {
                tracing::error!(
                    "❌ [STREAM_DETECTOR] Failed to get state for stats: {:?}",
                    e
                );
                return;
            }
        };

        let messages = state.messages();
        let start_time = self.stream_start_time.map(|instant| {
            // InstantからDateTimeへの変換
            let elapsed = instant.elapsed();
            chrono::Utc::now() - chrono::Duration::from_std(elapsed).unwrap_or_default()
        });

        let stats = SystemMessageGenerator::collect_stream_stats(
            &messages,
            start_time,
            self.consecutive_errors,
        );

        let message = SystemMessageGenerator::create_stream_ended_message(stats);

        match get_state_manager().send_event(AppEvent::MessageAdded(message)) {
            Ok(()) => {
                tracing::info!("📨 [STREAM_DETECTOR] Stream ended message sent successfully");
            }
            Err(e) => {
                tracing::error!(
                    "❌ [STREAM_DETECTOR] Failed to send stream ended message: {:?}",
                    e
                );
            }
        }
    }

    /// 配信終了状態を取得
    pub fn is_stream_ended(&self) -> bool {
        self.stream_ended
    }

    /// 連続エラー数を取得
    pub fn consecutive_errors(&self) -> u32 {
        self.consecutive_errors
    }

    /// 配信時間を取得（分）
    pub fn stream_duration_minutes(&self) -> u64 {
        if let Some(start_time) = self.stream_start_time {
            (start_time.elapsed().as_secs() / 60).max(0)
        } else {
            0
        }
    }

    /// デバッグ用：強制的に配信終了状態にする
    #[cfg(test)]
    pub fn force_stream_ended(&mut self) {
        self.stream_ended = true;
        self.consecutive_errors = 8;
        self.first_error_time = Some(Instant::now());
    }

    /// 監視開始時に全状態をリセット
    pub fn reset(&mut self) {
        self.consecutive_errors = 0;
        self.last_error_type = None;
        self.stream_start_time = Some(Instant::now());
        self.stream_ended = false;
        self.last_system_message_time = None;
        self.first_error_time = None;
    }
}

impl Default for StreamEndDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_classification() {
        assert_eq!(
            ErrorType::from_error_string("403 Forbidden"),
            ErrorType::Forbidden
        );
        assert_eq!(
            ErrorType::from_error_string("HTTP 403"),
            ErrorType::Forbidden
        );
        assert_eq!(
            ErrorType::from_error_string("404 Not Found"),
            ErrorType::NotFound
        );
        assert_eq!(
            ErrorType::from_error_string("timeout error"),
            ErrorType::Timeout
        );
        assert_eq!(
            ErrorType::from_error_string("network connection failed"),
            ErrorType::Network
        );

        match ErrorType::from_error_string("unknown error") {
            ErrorType::Other(s) => assert_eq!(s, "unknown error"),
            _ => panic!("Expected Other variant"),
        }
    }

    #[test]
    fn test_stream_end_detection() {
        let mut detector = StreamEndDetector::new();

        // 1-2回のエラーは継続
        assert_eq!(
            detector.on_error("403 Forbidden"),
            DetectionResult::Continue
        );
        assert_eq!(
            detector.on_error("403 Forbidden"),
            DetectionResult::Continue
        );

        // 3-7回のエラーは警告
        assert_eq!(detector.on_error("403 Forbidden"), DetectionResult::Warning);
        assert_eq!(detector.on_error("403 Forbidden"), DetectionResult::Warning);
        assert_eq!(detector.on_error("403 Forbidden"), DetectionResult::Warning);
        assert_eq!(detector.on_error("403 Forbidden"), DetectionResult::Warning);
        assert_eq!(detector.on_error("403 Forbidden"), DetectionResult::Warning);

        // 8回目で配信終了
        assert_eq!(
            detector.on_error("403 Forbidden"),
            DetectionResult::StreamEnded
        );
        assert!(detector.is_stream_ended());

        // 終了後はAlreadyEnded
        assert_eq!(
            detector.on_error("403 Forbidden"),
            DetectionResult::AlreadyEnded
        );
    }

    #[test]
    fn test_success_resets_counter() {
        let mut detector = StreamEndDetector::new();

        // 3回エラー
        detector.on_error("403 Forbidden");
        detector.on_error("403 Forbidden");
        detector.on_error("403 Forbidden");
        assert_eq!(detector.consecutive_errors(), 3);

        // 成功でリセット
        detector.on_success();
        assert_eq!(detector.consecutive_errors(), 0);

        // 再度エラーは1回目扱い
        assert_eq!(
            detector.on_error("403 Forbidden"),
            DetectionResult::Continue
        );
        assert_eq!(detector.consecutive_errors(), 1);
    }

    #[test]
    #[test]
    fn test_reset_clears_stream_end_flag() {
        let mut detector = StreamEndDetector::new();

        detector.force_stream_ended();
        assert!(detector.is_stream_ended());
        assert!(detector.consecutive_errors() >= 8);

        detector.reset();
        assert!(!detector.is_stream_ended());
        assert_eq!(detector.consecutive_errors(), 0);
    }
    fn test_non_403_errors() {
        let mut detector = StreamEndDetector::new();

        // 404エラーは配信終了判定にならない
        for _ in 0..10 {
            let result = detector.on_error("404 Not Found");
            assert_ne!(result, DetectionResult::StreamEnded);
        }
        assert!(!detector.is_stream_ended());
    }

    #[test]
    fn test_error_display_names() {
        assert_eq!(ErrorType::Forbidden.display_name(), "403 Forbidden");
        assert_eq!(ErrorType::NotFound.display_name(), "404 Not Found");
        assert_eq!(ErrorType::Timeout.display_name(), "Timeout");
        assert_eq!(ErrorType::Network.display_name(), "Network Error");
        assert_eq!(ErrorType::RateLimit.display_name(), "Rate Limit");
        assert_eq!(
            ErrorType::Other("test".to_string()).display_name(),
            "Unknown Error"
        );
    }
}
