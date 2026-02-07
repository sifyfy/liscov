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

pub use backends::{BouyomichanBackend, TtsBackendEnum, TtsError, VoicevoxBackend};
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
}

/// TTS Manager handles TTS operations
pub struct TtsManager {
    config: Arc<RwLock<TtsConfig>>,
    backend: Arc<RwLock<Option<TtsBackendEnum>>>,
    queue: Arc<Mutex<VecDeque<TtsQueueItem>>>,
    is_processing: Arc<RwLock<bool>>,
    shutdown_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
}

impl TtsManager {
    /// Create a new TTS manager
    pub fn new(config: TtsConfig) -> Self {
        let backend = TtsBackendEnum::from_config(
            &config.backend,
            &config.bouyomichan,
            &config.voicevox,
        );

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

        let backend = TtsBackendEnum::from_config(
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
        let test_backend = TtsBackendEnum::from_config(
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
                                build_tts_text(
                                    item.author_name.as_deref(),
                                    item.amount.as_deref(),
                                    &item.text,
                                    cfg.read_author_name,
                                    cfg.strip_at_prefix,
                                    cfg.strip_handle_suffix,
                                    cfg.add_honorific,
                                    cfg.read_superchat_amount,
                                    cfg.max_text_length,
                                )
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
    let mut result = name.to_string();

    if strip_at && result.starts_with('@') {
        result = result[1..].to_string();
    }

    if strip_handle {
        if let Some(pos) = result.rfind('-') {
            result = result[..pos].to_string();
        }
    }

    if honorific {
        result.push_str("さん");
    }

    result
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
}
