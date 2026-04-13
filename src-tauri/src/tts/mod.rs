//! TTS (Text-to-Speech) module
//!
//! Provides text-to-speech functionality with support for multiple backends
//! (Bouyomichan, VOICEVOX) and priority-based queue processing.

pub mod backends;
pub mod config;
pub mod process;

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

pub use backends::{BouyomichanBackend, TtsBackend, TtsError, VoicevoxBackend};
pub use config::{BouyomichanConfig, TtsBackendType, TtsConfig, VoicevoxConfig};
pub use process::TtsProcessManager;

/// TTS message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TtsPriority {
    /// Normal chat message (lowest priority)
    Normal = 0,
    /// Membership message
    Membership = 1,
    /// Super Chat/Sticker (highest priority)
    SuperChat = 2,
}

/// TTS queue item
#[derive(Debug, Clone)]
pub struct TtsQueueItem {
    pub text: String,
    pub priority: TtsPriority,
    pub author_name: Option<String>,
    pub amount: Option<String>,
    /// 配信内コメント回数（初回コメント判定に使用）
    pub in_stream_comment_count: Option<u32>,
}

/// TTS Manager handles TTS operations
pub struct TtsManager {
    config: Arc<RwLock<TtsConfig>>,
    backend: Arc<RwLock<Option<Box<dyn TtsBackend>>>>,
    queue: Arc<Mutex<VecDeque<TtsQueueItem>>>,
    is_processing: Arc<RwLock<bool>>,
    shutdown_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
}

impl TtsManager {
    /// Create a new TTS manager
    pub fn new(config: TtsConfig) -> Self {
        let backend = backends::create_backend(
            &config.backend,
            &config.bouyomichan,
            &config.voicevox,
        );
        Self::with_backend(config, backend)
    }

    /// 指定されたバックエンドで TtsManager を作成する
    pub fn with_backend(config: TtsConfig, backend: Option<Box<dyn TtsBackend>>) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            backend: Arc::new(RwLock::new(backend)),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            is_processing: Arc::new(RwLock::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Update configuration and save to file
    pub async fn update_config(&self, config: TtsConfig) {
        // Save to file
        if let Err(e) = config.save() {
            log::error!("Failed to save TTS config: {}", e);
        }

        let backend = backends::create_backend(
            &config.backend,
            &config.bouyomichan,
            &config.voicevox,
        );
        *self.config.write().await = config;
        *self.backend.write().await = backend;
    }

    /// Get current configuration
    pub async fn get_config(&self) -> TtsConfig {
        self.config.read().await.clone()
    }

    /// Test connection to current backend
    pub async fn test_connection(&self) -> Result<bool, TtsError> {
        let backend = self.backend.read().await;
        match backend.as_ref() {
            Some(b) => b.test_connection().await,
            None => Ok(false),
        }
    }

    /// Test connection to a specific backend type
    pub async fn test_backend_connection(&self, backend_type: TtsBackendType) -> Result<bool, TtsError> {
        let config = self.config.read().await;
        let test_backend = backends::create_backend(
            &backend_type,
            &config.bouyomichan,
            &config.voicevox,
        );

        match test_backend {
            Some(b) => b.test_connection().await,
            None => Ok(false),
        }
    }

    /// Format text for TTS reading
    pub async fn format_text(&self, item: &TtsQueueItem) -> String {
        let config = self.config.read().await;
        build_tts_text(
            item.author_name.as_deref(),
            item.amount.as_deref(),
            &item.text,
            config.read_author_name,
            config.strip_at_prefix,
            config.strip_handle_suffix,
            config.add_honorific,
            config.read_superchat_amount,
            config.max_text_length,
        )
    }

    /// Add item to queue
    pub async fn enqueue(&self, item: TtsQueueItem) {
        let config = self.config.read().await;

        // Check if enabled
        if !config.enabled {
            return;
        }

        // 初回コメントのみ読み上げ: 2回目以降はスキップ
        if should_skip_tts(config.first_comment_only, item.in_stream_comment_count) {
            log::debug!(
                "TTS skipped: first_comment_only enabled, count={:?}",
                item.in_stream_comment_count
            );
            return;
        }

        let mut queue = self.queue.lock().await;

        // Check queue size limit
        if queue.len() >= config.queue_size_limit {
            log::warn!("TTS queue full, dropping oldest message");
            queue.pop_front();
        }

        // Insert based on priority (higher priority items go to front)
        let insert_pos = queue
            .iter()
            .position(|q| q.priority < item.priority)
            .unwrap_or(queue.len());

        queue.insert(insert_pos, item);
        log::debug!("TTS queue size: {}", queue.len());
    }

    /// Speak text directly (bypasses queue)
    pub async fn speak_direct(&self, text: &str) -> Result<(), TtsError> {
        let backend = self.backend.read().await;
        match backend.as_ref() {
            Some(b) => b.speak(text).await,
            None => Err(TtsError::Connection("No backend configured".to_string())),
        }
    }

    /// Start queue processing
    pub async fn start_processing(&self) {
        let mut is_processing = self.is_processing.write().await;
        if *is_processing {
            log::warn!("TTS processing already running");
            return;
        }
        *is_processing = true;
        drop(is_processing);

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let queue = Arc::clone(&self.queue);
        let backend = Arc::clone(&self.backend);
        let config = Arc::clone(&self.config);
        let is_processing = Arc::clone(&self.is_processing);

        tokio::spawn(async move {
            log::info!("TTS queue processing started");

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        log::info!("TTS queue processing shutdown requested");
                        break;
                    }
                    _ = async {
                        // Get next item from queue
                        let item = {
                            let mut q = queue.lock().await;
                            q.pop_front()
                        };

                        if let Some(item) = item {
                            // Format text using shared helper
                            let text = {
                                let cfg = config.read().await;
                                let base = build_tts_text(
                                    item.author_name.as_deref(),
                                    item.amount.as_deref(),
                                    &item.text,
                                    cfg.read_author_name,
                                    cfg.strip_at_prefix,
                                    cfg.strip_handle_suffix,
                                    cfg.add_honorific,
                                    cfg.read_superchat_amount,
                                    cfg.max_text_length,
                                );
                                // 初回コメントプレフィックス
                                match build_first_comment_prefix(
                                    cfg.first_comment_prefix_enabled,
                                    &cfg.first_comment_prefix,
                                    item.in_stream_comment_count,
                                ) {
                                    Some(prefix) => format!("{}{}", prefix, base),
                                    None => base,
                                }
                            };

                            // Speak
                            let b = backend.read().await;
                            if let Some(ref backend) = *b {
                                if let Err(e) = backend.speak(&text).await {
                                    log::error!("TTS speak error: {}", e);
                                }
                            }
                        } else {
                            // No items, wait a bit
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    } => {}
                }
            }

            *is_processing.write().await = false;
            log::info!("TTS queue processing stopped");
        });
    }

    /// Stop queue processing
    pub async fn stop_processing(&self) {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(()).await;
        }
    }

    /// Clear the queue
    pub async fn clear_queue(&self) {
        self.queue.lock().await.clear();
    }

    /// Get queue size
    pub async fn queue_size(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// Check if processing is running
    pub async fn is_processing(&self) -> bool {
        *self.is_processing.read().await
    }

    /// Get backend name
    pub async fn backend_name(&self) -> Option<&'static str> {
        let backend = self.backend.read().await;
        backend.as_ref().map(|b| b.name())
    }
}

impl Default for TtsManager {
    fn default() -> Self {
        // Load config from file
        let config = TtsConfig::load();
        Self::new(config)
    }
}

// ============================================================================
// Pure helper functions for TTS text generation (04_tts.md)
// ============================================================================

/// デフォルトの初回コメントプレフィックス
const DEFAULT_FIRST_COMMENT_PREFIX: &str = "1回目のコメント。";

/// プレフィックス文言を解決する。空または空白のみの場合はデフォルトにフォールバック。
pub(crate) fn resolve_first_comment_prefix(configured: &str) -> &str {
    if configured.trim().is_empty() {
        DEFAULT_FIRST_COMMENT_PREFIX
    } else {
        configured
    }
}

/// 初回コメントのみ読み上げ設定に基づき、このメッセージをスキップすべきか判定する
pub(crate) fn should_skip_tts(first_comment_only: bool, in_stream_comment_count: Option<u32>) -> bool {
    if !first_comment_only {
        return false;
    }
    match in_stream_comment_count {
        Some(count) => count > 1,
        // システムメッセージ等（カウントなし）はスキップしない
        None => false,
    }
}

/// 初回コメントプレフィックスを生成する。付加不要な場合は None を返す。
pub(crate) fn build_first_comment_prefix(
    enabled: bool,
    configured_prefix: &str,
    in_stream_comment_count: Option<u32>,
) -> Option<String> {
    if !enabled {
        return None;
    }
    match in_stream_comment_count {
        Some(1) => Some(resolve_first_comment_prefix(configured_prefix).to_string()),
        _ => None,
    }
}

/// Process author name: strip @prefix, strip -xxx handle suffix, add honorific
///
/// Spec (04_tts.md):
/// - strip_at_prefix=true → 先頭の @ を除去
/// - strip_handle_suffix=true → 末尾の -xxx サフィックスを除去
/// - add_honorific=true → 「さん」を付与
pub(crate) fn process_author_name(
    name: &str,
    strip_at: bool,
    strip_handle: bool,
    honorific: bool,
) -> String {
    let s = if strip_at { name.strip_prefix('@').unwrap_or(name) } else { name };
    let s = if strip_handle { s.rfind('-').map_or(s, |pos| &s[..pos]) } else { s };
    if honorific { format!("{}さん", s) } else { s.to_string() }
}

/// Truncate text to max_length (by chars), appending "、以下省略" if truncated
pub(crate) fn truncate_text(text: &str, max_length: usize) -> String {
    if text.chars().count() > max_length {
        let mut truncated: String = text.chars().take(max_length).collect();
        truncated.push_str("、以下省略");
        truncated
    } else {
        text.to_string()
    }
}

/// Build complete TTS text from parts
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_tts_text(
    author_name: Option<&str>,
    amount: Option<&str>,
    message: &str,
    read_author_name: bool,
    strip_at_prefix: bool,
    strip_handle_suffix: bool,
    add_honorific: bool,
    read_superchat_amount: bool,
    max_text_length: usize,
) -> String {
    let mut parts = Vec::new();

    if read_author_name {
        if let Some(author) = author_name {
            parts.push(process_author_name(
                author,
                strip_at_prefix,
                strip_handle_suffix,
                add_honorific,
            ));
        }
    }

    if read_superchat_amount {
        if let Some(amt) = amount {
            parts.push(format!("{}の", amt));
        }
    }

    parts.push(truncate_text(message, max_text_length));

    parts.join("、")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // process_author_name (04_tts.md: 投稿者名処理)
    // ========================================================================

    // ---- Spec table examples (04_tts.md lines 169-174) ----

    #[test]
    fn spec_example_all_options_on() {
        // spec: @田中-abc, strip_at=true, strip_handle=true, honorific=true → 田中さん
        assert_eq!(
            process_author_name("@田中-abc", true, true, true),
            "田中さん"
        );
    }

    #[test]
    fn spec_example_strip_at_false() {
        // spec: @田中-abc, strip_at=false, strip_handle=true, honorific=true → @田中さん
        // Note: @は残るが、@田中-abc から -abc は除去される → @田中さん
        assert_eq!(
            process_author_name("@田中-abc", false, true, true),
            "@田中さん"
        );
    }

    #[test]
    fn spec_example_no_suffix() {
        // spec: 田中みな子 → 田中みな子さん (ハイフンなし → strip_handle_suffixは何もしない)
        assert_eq!(
            process_author_name("田中みな子", true, true, true),
            "田中みな子さん"
        );
    }

    // ---- Additional edge cases ----

    #[test]
    fn author_name_strip_at_only() {
        assert_eq!(
            process_author_name("@田中", true, false, false),
            "田中"
        );
    }

    #[test]
    fn author_name_strip_handle_removes_last_hyphen_suffix() {
        // strip_handle_suffix removes trailing -xxx suffix
        assert_eq!(
            process_author_name("名前-handle", false, true, false),
            "名前"
        );
    }

    #[test]
    fn author_name_strip_handle_no_hyphen() {
        // No hyphen → nothing to strip
        assert_eq!(
            process_author_name("田中太郎", false, true, false),
            "田中太郎"
        );
    }

    #[test]
    fn author_name_honorific_false() {
        assert_eq!(
            process_author_name("田中-abc", true, true, false),
            "田中"
        );
    }

    #[test]
    fn author_name_all_options_off() {
        assert_eq!(
            process_author_name("@田中-abc", false, false, false),
            "@田中-abc"
        );
    }

    #[test]
    fn author_name_multiple_hyphens() {
        // rfind('-') removes only the last -suffix
        assert_eq!(
            process_author_name("田中-太郎-xyz", false, true, false),
            "田中-太郎"
        );
    }

    // ========================================================================
    // truncate_text (04_tts.md: テキスト切り詰め)
    // ========================================================================

    #[test]
    fn truncate_within_limit() {
        assert_eq!(truncate_text("こんにちは", 200), "こんにちは");
    }

    #[test]
    fn truncate_at_exact_limit() {
        let text: String = "あ".repeat(200);
        assert_eq!(truncate_text(&text, 200), text);
    }

    #[test]
    fn truncate_exceeding_limit() {
        let text: String = "あ".repeat(201);
        let expected: String = "あ".repeat(200) + "、以下省略";
        assert_eq!(truncate_text(&text, 200), expected);
    }

    #[test]
    fn truncate_empty() {
        assert_eq!(truncate_text("", 200), "");
    }

    // ========================================================================
    // build_tts_text (04_tts.md: 完全なTTSテキスト生成)
    // ========================================================================

    #[test]
    fn build_text_full_superchat() {
        // spec: "田中さん、¥500の、こんにちは"
        let result = build_tts_text(
            Some("田中"),
            Some("¥500"),
            "こんにちは",
            true,  // read_author_name
            true,  // strip_at
            true,  // strip_handle
            true,  // add_honorific
            true,  // read_superchat_amount
            200,   // max_text_length
        );
        assert_eq!(result, "田中さん、¥500の、こんにちは");
    }

    #[test]
    fn build_text_no_author() {
        let result = build_tts_text(
            None,
            None,
            "こんにちは",
            true, true, true, true, true, 200,
        );
        assert_eq!(result, "こんにちは");
    }

    #[test]
    fn build_text_author_name_disabled() {
        let result = build_tts_text(
            Some("田中"),
            None,
            "こんにちは",
            false, // read_author_name disabled
            true, true, true, true, 200,
        );
        assert_eq!(result, "こんにちは");
    }

    #[test]
    fn build_text_amount_disabled() {
        let result = build_tts_text(
            Some("田中"),
            Some("¥500"),
            "テスト",
            true, true, true, true,
            false, // read_superchat_amount disabled
            200,
        );
        assert_eq!(result, "田中さん、テスト");
    }

    #[test]
    fn build_text_with_at_prefix_author() {
        let result = build_tts_text(
            Some("@user123"),
            None,
            "hello",
            true, true, true, true, true, 200,
        );
        assert_eq!(result, "user123さん、hello");
    }

    #[test]
    fn build_text_spec_example_superchat() {
        // spec (04_tts.md lines 194-203):
        // 投稿者: @山田太郎-xyz, SuperChat ¥500, 本文: こんにちは！
        // → 山田太郎さん、500円のスーパーチャット、こんにちは
        // Note: amount formatting (¥500→500円のスーパーチャット) is done
        // before enqueueing, so here we test with pre-formatted amount
        let result = build_tts_text(
            Some("@山田太郎-xyz"),
            Some("¥500"),
            "こんにちは！",
            true, true, true, true, true, 200,
        );
        assert_eq!(result, "山田太郎さん、¥500の、こんにちは！");
    }

    #[test]
    fn build_text_strip_handle_suffix_applied() {
        // Verifies that -suffix is stripped in build_tts_text pipeline
        let result = build_tts_text(
            Some("@田中-abc"),
            None,
            "テスト",
            true, true, true, true, false, 200,
        );
        assert_eq!(result, "田中さん、テスト");
    }

    // ========================================================================
    // TtsConfig defaults (04_tts.md: 設定デフォルト値)
    // ========================================================================

    #[test]
    fn tts_config_defaults() {
        let config = TtsConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.backend, TtsBackendType::None);
        assert!(config.read_author_name);
        assert!(config.add_honorific);
        assert!(config.strip_at_prefix);
        assert!(config.strip_handle_suffix);
        assert!(config.read_superchat_amount);
        assert_eq!(config.max_text_length, 200);
        assert_eq!(config.queue_size_limit, 50);
        assert!(!config.first_comment_prefix_enabled);
        assert_eq!(config.first_comment_prefix, "");
        assert!(!config.first_comment_only);
    }

    #[test]
    fn bouyomichan_config_defaults() {
        let config = BouyomichanConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 50080);
        assert_eq!(config.voice, 0);
        assert_eq!(config.volume, -1);
        assert_eq!(config.speed, -1);
        assert_eq!(config.tone, -1);
        assert!(!config.auto_launch);
        assert!(config.exe_path.is_none());
        assert!(config.auto_close);
    }

    #[test]
    fn voicevox_config_defaults() {
        let config = VoicevoxConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 50021);
        assert_eq!(config.speaker_id, 1);
        assert_eq!(config.volume_scale, 1.0);
        assert_eq!(config.speed_scale, 1.0);
        assert_eq!(config.pitch_scale, 0.0);
        assert_eq!(config.intonation_scale, 1.0);
        assert!(!config.auto_launch);
        assert!(config.exe_path.is_none());
        assert!(config.auto_close);
    }

    // ========================================================================
    // TtsPriority ordering (04_tts.md: 優先度)
    // ========================================================================

    #[test]
    fn priority_ordering() {
        assert!(TtsPriority::Normal < TtsPriority::Membership);
        assert!(TtsPriority::Membership < TtsPriority::SuperChat);
    }

    // ========================================================================
    // resolve_first_comment_prefix (04_tts.md: 初回コメントプレフィックス解決)
    // ========================================================================

    #[test]
    fn resolve_prefix_custom_text() {
        assert_eq!(resolve_first_comment_prefix("カスタム。"), "カスタム。");
    }

    #[test]
    fn resolve_prefix_empty_falls_back_to_default() {
        // AC-8: 空の場合はデフォルト「1回目のコメント。」
        assert_eq!(resolve_first_comment_prefix(""), "1回目のコメント。");
    }

    #[test]
    fn resolve_prefix_whitespace_only_falls_back_to_default() {
        // 空白のみの場合もデフォルトにフォールバック
        assert_eq!(resolve_first_comment_prefix("   "), "1回目のコメント。");
    }

    // ========================================================================
    // should_skip_tts / build_first_comment_prefix (04_tts.md: 初回コメント判定)
    // ========================================================================

    #[test]
    fn first_comment_only_skips_second_message() {
        // AC-5: first_comment_only=ON, 2回目 → スキップ
        assert!(should_skip_tts(true, Some(2)));
    }

    #[test]
    fn first_comment_only_allows_first_message() {
        // AC-4: first_comment_only=ON, 1回目 → 読み上げ
        assert!(!should_skip_tts(true, Some(1)));
    }

    #[test]
    fn first_comment_only_off_allows_all() {
        // AC-6: first_comment_only=OFF → 通常通り
        assert!(!should_skip_tts(false, Some(5)));
    }

    #[test]
    fn first_comment_only_none_count_allows() {
        // in_stream_comment_count=None（システムメッセージ等）→ スキップしない
        assert!(!should_skip_tts(true, None));
    }

    #[test]
    fn prefix_on_first_comment() {
        // AC-1: プレフィックスON + 初回 → プレフィックス付加
        let result = build_first_comment_prefix(true, "", Some(1));
        assert_eq!(result, Some("1回目のコメント。".to_string()));
    }

    #[test]
    fn prefix_on_second_comment() {
        // AC-2: プレフィックスON + 2回目 → なし
        let result = build_first_comment_prefix(true, "", Some(2));
        assert_eq!(result, None);
    }

    #[test]
    fn prefix_off() {
        // AC-3: プレフィックスOFF → なし
        let result = build_first_comment_prefix(false, "", Some(1));
        assert_eq!(result, None);
    }

    #[test]
    fn prefix_custom_text_on_first() {
        let result = build_first_comment_prefix(true, "初コメ！", Some(1));
        assert_eq!(result, Some("初コメ！".to_string()));
    }

    #[test]
    fn prefix_with_superchat_first_comment() {
        // Edge Case: スーパーチャットが初回コメント
        let prefix = build_first_comment_prefix(true, "", Some(1));
        let tts_text = build_tts_text(
            Some("@山田太郎-xyz"), Some("¥500"), "こんにちは",
            true, true, true, true, true, 200,
        );
        let result = match prefix {
            Some(p) => format!("{}{}", p, tts_text),
            None => tts_text,
        };
        assert_eq!(result, "1回目のコメント。山田太郎さん、¥500の、こんにちは");
    }

    // ========================================================================
    // TtsManager::enqueue 統合テスト（スキップ判定の配線確認）
    // ========================================================================

    /// テスト用: enabled=true, backend=None の TtsConfig を生成
    fn test_config_with_first_comment(first_comment_only: bool) -> TtsConfig {
        TtsConfig {
            enabled: true,
            first_comment_only,
            ..TtsConfig::default()
        }
    }

    #[tokio::test]
    async fn enqueue_skips_second_comment_when_first_comment_only() {
        // first_comment_only=true の場合、count=2 のメッセージはキューに入らない
        let manager = TtsManager::new(test_config_with_first_comment(true));
        let item = TtsQueueItem {
            text: "テスト".to_string(),
            priority: TtsPriority::Normal,
            author_name: Some("テスター".to_string()),
            amount: None,
            in_stream_comment_count: Some(2),
        };
        manager.enqueue(item).await;
        assert_eq!(manager.queue_size().await, 0);
    }

    #[tokio::test]
    async fn enqueue_allows_first_comment_when_first_comment_only() {
        // first_comment_only=true の場合、count=1 のメッセージはキューに入る
        let manager = TtsManager::new(test_config_with_first_comment(true));
        let item = TtsQueueItem {
            text: "テスト".to_string(),
            priority: TtsPriority::Normal,
            author_name: Some("テスター".to_string()),
            amount: None,
            in_stream_comment_count: Some(1),
        };
        manager.enqueue(item).await;
        assert_eq!(manager.queue_size().await, 1);
    }

    #[tokio::test]
    async fn enqueue_allows_all_when_first_comment_only_off() {
        // first_comment_only=false の場合、全てキューに入る
        let manager = TtsManager::new(test_config_with_first_comment(false));
        let item = TtsQueueItem {
            text: "テスト".to_string(),
            priority: TtsPriority::Normal,
            author_name: Some("テスター".to_string()),
            amount: None,
            in_stream_comment_count: Some(5),
        };
        manager.enqueue(item).await;
        assert_eq!(manager.queue_size().await, 1);
    }

    // ========================================================================
    // TtsManager::get_config（L85のmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn get_config_returns_initial_config() {
        // spec: TtsManager::new に渡した設定が get_config で取得できる
        let manager = TtsManager::new(TtsConfig {
            enabled: true,
            ..TtsConfig::default()
        });
        let config = manager.get_config().await;
        assert!(config.enabled);
    }

    // ========================================================================
    // TtsManager::update_config（L70のmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn update_config_reflects_changes_in_get_config() {
        // spec: update_config 後に get_config でフィールドが反映される
        let manager = TtsManager::new(TtsConfig::default());
        let new_config = TtsConfig {
            enabled: true,
            queue_size_limit: 10,
            ..TtsConfig::default()
        };
        manager.update_config(new_config).await;
        let config = manager.get_config().await;
        assert!(config.enabled);
        assert_eq!(config.queue_size_limit, 10);
    }

    // ========================================================================
    // TtsManager::format_text（L114のmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn format_text_uses_config_to_build_text() {
        // spec: add_honorific=true の設定で format_text は著者名に「さん」を付与する
        let manager = TtsManager::new(TtsConfig {
            add_honorific: true,
            read_author_name: true,
            ..TtsConfig::default()
        });
        let item = TtsQueueItem {
            text: "こんにちは".to_string(),
            priority: TtsPriority::Normal,
            author_name: Some("田中".to_string()),
            amount: None,
            in_stream_comment_count: None,
        };
        let result = manager.format_text(&item).await;
        // 空文字でもなく、元テキストそのままでもない（著者名が付加される）
        assert!(!result.is_empty());
        assert_ne!(result, "");
        // add_honorific=true なので「田中さん」が含まれる
        assert!(result.contains("田中さん"));
    }

    // ========================================================================
    // enqueue がキュー満杯時に最古を破棄する（L149のmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn enqueue_drops_oldest_when_queue_full() {
        // spec: queue_size_limit=2 で 3件enqueue すると最古が破棄されてサイズは2
        let manager = TtsManager::new(TtsConfig {
            enabled: true,
            queue_size_limit: 2,
            ..TtsConfig::default()
        });
        for i in 0..3 {
            manager.enqueue(TtsQueueItem {
                text: format!("メッセージ{}", i),
                priority: TtsPriority::Normal,
                author_name: None,
                amount: None,
                in_stream_comment_count: None,
            }).await;
        }
        assert_eq!(manager.queue_size().await, 2);
    }

    #[tokio::test]
    async fn enqueue_oldest_is_dropped_not_newest() {
        // spec: 満杯時に破棄されるのは最古（先頭）のアイテム
        let manager = TtsManager::new(TtsConfig {
            enabled: true,
            queue_size_limit: 2,
            ..TtsConfig::default()
        });
        manager.enqueue(TtsQueueItem {
            text: "最古".to_string(),
            priority: TtsPriority::Normal,
            author_name: None,
            amount: None,
            in_stream_comment_count: None,
        }).await;
        manager.enqueue(TtsQueueItem {
            text: "2番目".to_string(),
            priority: TtsPriority::Normal,
            author_name: None,
            amount: None,
            in_stream_comment_count: None,
        }).await;
        manager.enqueue(TtsQueueItem {
            text: "最新".to_string(),
            priority: TtsPriority::Normal,
            author_name: None,
            amount: None,
            in_stream_comment_count: None,
        }).await;
        // 最古「最古」が破棄され、「2番目」と「最新」が残る
        let queue = manager.queue.lock().await;
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0].text, "2番目");
        assert_eq!(queue[1].text, "最新");
    }

    // ========================================================================
    // enqueue が優先度を正しく挿入する（L157の3つのmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn enqueue_superchat_goes_before_normal() {
        // spec: Normal enqueue後に SuperChat を enqueue → 先頭が SuperChat
        let manager = TtsManager::new(TtsConfig {
            enabled: true,
            ..TtsConfig::default()
        });
        manager.enqueue(TtsQueueItem {
            text: "ノーマル".to_string(),
            priority: TtsPriority::Normal,
            author_name: None,
            amount: None,
            in_stream_comment_count: None,
        }).await;
        manager.enqueue(TtsQueueItem {
            text: "スーパーチャット".to_string(),
            priority: TtsPriority::SuperChat,
            author_name: None,
            amount: None,
            in_stream_comment_count: None,
        }).await;
        let queue = manager.queue.lock().await;
        assert_eq!(queue[0].priority, TtsPriority::SuperChat);
        assert_eq!(queue[0].text, "スーパーチャット");
    }

    #[tokio::test]
    async fn enqueue_normal_messages_are_fifo() {
        // spec: Normal 2件は挿入順（FIFO）で並ぶ
        let manager = TtsManager::new(TtsConfig {
            enabled: true,
            ..TtsConfig::default()
        });
        manager.enqueue(TtsQueueItem {
            text: "a".to_string(),
            priority: TtsPriority::Normal,
            author_name: None,
            amount: None,
            in_stream_comment_count: None,
        }).await;
        manager.enqueue(TtsQueueItem {
            text: "b".to_string(),
            priority: TtsPriority::Normal,
            author_name: None,
            amount: None,
            in_stream_comment_count: None,
        }).await;
        let queue = manager.queue.lock().await;
        assert_eq!(queue[0].text, "a");
        assert_eq!(queue[1].text, "b");
    }

    // ========================================================================
    // is_processing が初期状態で false を返す（L272のmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn is_processing_returns_false_initially() {
        // spec: 新規作成した TtsManager は is_processing=false
        let manager = TtsManager::new(TtsConfig::default());
        assert!(!manager.is_processing().await);
    }

    // ========================================================================
    // backend_name がバックエンド種別を正しく返す（L277のmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn backend_name_returns_bouyomichan_for_bouyomichan_config() {
        // spec: Bouyomichan 設定では backend_name が Some("Bouyomichan") を返す
        let manager = TtsManager::new(TtsConfig {
            backend: TtsBackendType::Bouyomichan,
            ..TtsConfig::default()
        });
        assert_eq!(manager.backend_name().await, Some("Bouyomichan"));
    }

    #[tokio::test]
    async fn backend_name_returns_voicevox_for_voicevox_config() {
        // spec: Voicevox 設定では backend_name が Some("VOICEVOX") を返す
        let manager = TtsManager::new(TtsConfig {
            backend: TtsBackendType::Voicevox,
            ..TtsConfig::default()
        });
        assert_eq!(manager.backend_name().await, Some("VOICEVOX"));
    }

    #[tokio::test]
    async fn backend_name_returns_none_for_none_config() {
        // spec: None 設定では backend_name が None を返す
        let manager = TtsManager::new(TtsConfig {
            backend: TtsBackendType::None,
            ..TtsConfig::default()
        });
        assert_eq!(manager.backend_name().await, None);
    }

    // ========================================================================
    // TtsManager::clear_queue（L262のmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn clear_queue_removes_all_items() {
        // spec: clear_queue 後は queue_size が 0 になる
        let manager = TtsManager::new(TtsConfig {
            enabled: true,
            ..TtsConfig::default()
        });
        for i in 0..3 {
            manager.enqueue(TtsQueueItem {
                text: format!("アイテム{}", i),
                priority: TtsPriority::Normal,
                author_name: None,
                amount: None,
                in_stream_comment_count: None,
            }).await;
        }
        assert_eq!(manager.queue_size().await, 3);
        manager.clear_queue().await;
        assert_eq!(manager.queue_size().await, 0);
    }

    // ========================================================================
    // default_true が auto_close のデフォルト値として true を返す（L35のmutantをkill）
    // ========================================================================

    #[test]
    fn bouyomichan_auto_close_defaults_to_true_via_serde() {
        // spec: TOML に auto_close が未指定の場合、serde default により true になる
        let config: BouyomichanConfig = toml::from_str(
            "host=\"localhost\"\nport=50080\nvoice=0\nvolume=-1\nspeed=-1\ntone=-1"
        ).expect("TOMLパース失敗");
        assert!(config.auto_close);
    }

    // ========================================================================
    // MockTtsBackend（TtsBackend トレイトのテスト用実装）
    // ========================================================================

    struct MockTtsBackend {
        speak_calls: Arc<Mutex<Vec<String>>>,
        connection_ok: bool,
    }

    impl MockTtsBackend {
        fn connected() -> Self {
            Self {
                speak_calls: Arc::new(Mutex::new(Vec::new())),
                connection_ok: true,
            }
        }

        fn disconnected() -> Self {
            Self {
                speak_calls: Arc::new(Mutex::new(Vec::new())),
                connection_ok: false,
            }
        }
    }

    #[async_trait::async_trait]
    impl TtsBackend for MockTtsBackend {
        async fn test_connection(&self) -> Result<bool, backends::TtsError> {
            Ok(self.connection_ok)
        }
        async fn speak(&self, text: &str) -> Result<(), backends::TtsError> {
            self.speak_calls.lock().await.push(text.to_string());
            Ok(())
        }
        fn name(&self) -> &'static str {
            "Mock"
        }
    }

    // ========================================================================
    // TtsManager::test_connection（L90のmutantをkill: Ok(true)/Ok(false)）
    // ========================================================================

    #[tokio::test]
    async fn test_connection_delegates_to_backend_connected() {
        // spec: バックエンドが接続成功を返す → test_connection は true を返す
        let manager = TtsManager::with_backend(
            TtsConfig::default(),
            Some(Box::new(MockTtsBackend::connected())),
        );
        let result = manager.test_connection().await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_connection_delegates_to_backend_disconnected() {
        // spec: バックエンドが接続失敗を返す → test_connection は false を返す
        let manager = TtsManager::with_backend(
            TtsConfig::default(),
            Some(Box::new(MockTtsBackend::disconnected())),
        );
        let result = manager.test_connection().await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_connection_returns_false_when_no_backend() {
        // spec: バックエンドが None → test_connection は false を返す
        let manager = TtsManager::with_backend(TtsConfig::default(), None);
        let result = manager.test_connection().await.unwrap();
        assert!(!result);
    }

    // ========================================================================
    // TtsManager::test_backend_connection（L99のmutantをkill: Ok(true)）
    // ========================================================================

    #[tokio::test]
    async fn test_backend_connection_returns_false_for_none_type() {
        // spec: TtsBackendType::None → test_backend_connection は false を返す
        let manager = TtsManager::new(TtsConfig::default());
        let result = manager.test_backend_connection(TtsBackendType::None).await.unwrap();
        assert!(!result);
    }

    // ========================================================================
    // TtsManager::speak_direct（L166のmutantをkill: Ok(())）
    // ========================================================================

    #[tokio::test]
    async fn speak_direct_delegates_to_backend() {
        // spec: speak_direct はバックエンドの speak を呼び出す
        let mock = MockTtsBackend::connected();
        let speak_calls = Arc::clone(&mock.speak_calls);
        let manager = TtsManager::with_backend(
            TtsConfig::default(),
            Some(Box::new(mock)),
        );
        manager.speak_direct("テスト発話").await.unwrap();
        let calls = speak_calls.lock().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "テスト発話");
    }

    #[tokio::test]
    async fn speak_direct_returns_error_when_no_backend() {
        // spec: バックエンドが None → speak_direct はエラーを返す
        let manager = TtsManager::with_backend(TtsConfig::default(), None);
        let result = manager.speak_direct("テスト").await;
        assert!(result.is_err());
    }

    // ========================================================================
    // TtsManager::start_processing / stop_processing（L175, L255のmutantをkill）
    // ========================================================================

    #[tokio::test]
    async fn start_processing_sets_is_processing_true() {
        // spec: start_processing 後は is_processing が true になる
        let manager = TtsManager::with_backend(
            TtsConfig::default(),
            Some(Box::new(MockTtsBackend::connected())),
        );
        assert!(!manager.is_processing().await);
        manager.start_processing().await;
        assert!(manager.is_processing().await);
        // cleanup
        manager.stop_processing().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    #[tokio::test]
    async fn stop_processing_sets_is_processing_false() {
        // spec: start_processing → stop_processing 後は is_processing が false になる
        let manager = TtsManager::with_backend(
            TtsConfig::default(),
            Some(Box::new(MockTtsBackend::connected())),
        );
        manager.start_processing().await;
        assert!(manager.is_processing().await);
        manager.stop_processing().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        assert!(!manager.is_processing().await);
    }
}
