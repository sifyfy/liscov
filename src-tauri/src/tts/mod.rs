//! TTS (Text-to-Speech) module
//!
//! Provides text-to-speech functionality with support for multiple backends
//! (Bouyomichan, VOICEVOX) and priority-based queue processing.

pub mod backends;
pub mod config;

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

pub use backends::{BouyomichanBackend, TtsBackendEnum, TtsError, VoicevoxBackend};
pub use config::{BouyomichanConfig, TtsBackendType, TtsConfig, VoicevoxConfig};

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
        let mut parts = Vec::new();

        // Add author name with optional honorific
        if config.read_author_name {
            if let Some(ref author) = item.author_name {
                let mut name = author.clone();

                // Strip @ prefix
                if config.strip_at_prefix && name.starts_with('@') {
                    name = name[1..].to_string();
                }

                // Strip handle suffix (e.g., @handle part after name)
                if config.strip_handle_suffix {
                    if let Some(pos) = name.find(" @") {
                        name = name[..pos].to_string();
                    }
                }

                // Add honorific
                if config.add_honorific {
                    name.push_str("さん");
                }

                parts.push(name);
            }
        }

        // Add Super Chat amount
        if config.read_superchat_amount {
            if let Some(ref amount) = item.amount {
                parts.push(format!("{}の", amount));
            }
        }

        // Add main text
        let mut text = item.text.clone();

        // Truncate if too long
        if text.chars().count() > config.max_text_length {
            text = text.chars().take(config.max_text_length).collect::<String>();
            text.push_str("、以下省略");
        }

        parts.push(text);

        parts.join("、")
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
                            // Format text
                            let text = {
                                let cfg = config.read().await;
                                let mut parts = Vec::new();

                                if cfg.read_author_name {
                                    if let Some(ref author) = item.author_name {
                                        let mut name = author.clone();
                                        if cfg.strip_at_prefix && name.starts_with('@') {
                                            name = name[1..].to_string();
                                        }
                                        if cfg.strip_handle_suffix {
                                            if let Some(pos) = name.find(" @") {
                                                name = name[..pos].to_string();
                                            }
                                        }
                                        if cfg.add_honorific {
                                            name.push_str("さん");
                                        }
                                        parts.push(name);
                                    }
                                }

                                if cfg.read_superchat_amount {
                                    if let Some(ref amount) = item.amount {
                                        parts.push(format!("{}の", amount));
                                    }
                                }

                                let mut text = item.text.clone();
                                if text.chars().count() > cfg.max_text_length {
                                    text = text.chars().take(cfg.max_text_length).collect::<String>();
                                    text.push_str("、以下省略");
                                }
                                parts.push(text);

                                parts.join("、")
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
