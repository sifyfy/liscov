//! TTS（テキスト読み上げ）プラグイン
//!
//! 棒読みちゃん/VOICEVOX連携でチャットメッセージを音声読み上げする

pub mod backends;
pub mod config;
pub mod error;
pub mod launcher;
pub mod queue;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::gui::models::{GuiChatMessage, MessageType};
use crate::gui::plugin_system::{Plugin, PluginContext, PluginEvent, PluginInfo, PluginResult};
use crate::LiscovResult;

use backends::{BouyomichanBackend, TtsBackend, VoicevoxBackend};
use config::{TtsBackendType, TtsConfig};
use queue::{TtsMessage, TtsPriority, TtsQueue};

pub use config::TtsConfig as TtsPluginConfig;

/// TTSプラグイン
pub struct TtsPlugin {
    config: TtsConfig,
    context: Option<PluginContext>,
    backend: Option<Arc<dyn TtsBackend>>,
    queue: Option<TtsQueue>,
    queue_handle: Option<JoinHandle<()>>,
}

impl TtsPlugin {
    /// 新しいインスタンスを作成
    pub fn new() -> Self {
        Self {
            config: TtsConfig::default(),
            context: None,
            backend: None,
            queue: None,
            queue_handle: None,
        }
    }

    /// 設定を取得
    pub fn config(&self) -> &TtsConfig {
        &self.config
    }

    /// 設定を更新
    pub async fn update_config(&mut self, config: TtsConfig) -> LiscovResult<()> {
        let backend_changed = self.config.backend != config.backend;
        self.config = config.clone();

        // バックエンドが変更された場合は再初期化
        if backend_changed {
            self.initialize_backend().await?;
        }

        // 設定を保存
        if let Some(ref context) = self.context {
            let config_json = serde_json::to_value(&self.config)?;
            context
                .config_access
                .set_config(&context.plugin_id, "tts_config", config_json)
                .await?;
        }

        Ok(())
    }

    /// バックエンドを初期化
    async fn initialize_backend(&mut self) -> LiscovResult<()> {
        // 既存のキュー処理を停止
        if let Some(handle) = self.queue_handle.take() {
            handle.abort();
        }

        // バックエンドを作成
        let backend: Option<Arc<dyn TtsBackend>> = match self.config.backend {
            TtsBackendType::None => None,
            TtsBackendType::Bouyomichan => {
                Some(Arc::new(BouyomichanBackend::new(self.config.bouyomichan.clone())))
            }
            TtsBackendType::Voicevox => {
                Some(Arc::new(VoicevoxBackend::new(self.config.voicevox.clone())))
            }
        };

        // キューを作成
        if let Some(ref backend) = backend {
            let (queue, handle) = TtsQueue::new(backend.clone(), self.config.queue_size_limit);
            self.queue = Some(queue);
            self.queue_handle = Some(handle);
            tracing::info!("🔊 TTSバックエンド初期化: {}", backend.name());
        } else {
            self.queue = None;
            tracing::info!("🔊 TTS無効化");
        }

        self.backend = backend;
        Ok(())
    }

    /// 接続テスト
    pub async fn test_connection(&self) -> Result<bool, error::TtsError> {
        if let Some(ref backend) = self.backend {
            backend.test_connection().await
        } else {
            Ok(false)
        }
    }

    /// メッセージから読み上げテキストを生成
    fn format_message(&self, message: &GuiChatMessage) -> String {
        let mut parts = Vec::new();

        // 投稿者名
        if self.config.read_author_name {
            parts.push(message.author.clone());
        }

        // スーパーチャット金額
        if self.config.read_superchat_amount {
            match &message.message_type {
                MessageType::SuperChat { amount } => {
                    parts.push(format!("{}のスーパーチャット", amount));
                }
                MessageType::SuperSticker { amount } => {
                    parts.push(format!("{}のスーパーステッカー", amount));
                }
                MessageType::Membership { milestone_months } => {
                    if let Some(months) = milestone_months {
                        parts.push(format!("{}ヶ月のメンバーシップ", months));
                    } else {
                        parts.push("メンバー加入".to_string());
                    }
                }
                MessageType::MembershipGift { gift_count } => {
                    parts.push(format!("{}人へのメンバーシップギフト", gift_count));
                }
                _ => {}
            }
        }

        // メッセージ本文（サニタイズ）
        let content = self.sanitize_text(&message.content);
        if !content.is_empty() {
            parts.push(content);
        }

        // 結合して長さ制限
        let text = parts.join("、");
        text.chars().take(self.config.max_text_length).collect()
    }

    /// テキストのサニタイズ
    fn sanitize_text(&self, text: &str) -> String {
        // URLを除去
        let url_pattern = regex::Regex::new(r"https?://\S+").unwrap();
        let text = url_pattern.replace_all(text, "").to_string();

        // 連続する空白を1つに
        let whitespace_pattern = regex::Regex::new(r"\s+").unwrap();
        let text = whitespace_pattern.replace_all(&text, " ").to_string();

        text.trim().to_string()
    }

    /// メッセージの優先度を決定
    fn get_priority(&self, message: &GuiChatMessage) -> TtsPriority {
        match &message.message_type {
            MessageType::SuperChat { .. } | MessageType::SuperSticker { .. } => {
                TtsPriority::SuperChat
            }
            MessageType::Membership { .. } | MessageType::MembershipGift { .. } => {
                TtsPriority::Membership
            }
            _ => TtsPriority::Normal,
        }
    }

    /// メッセージをキューに追加
    async fn queue_message(&self, message: &GuiChatMessage) -> LiscovResult<()> {
        if let Some(ref queue) = self.queue {
            let text = self.format_message(message);
            if !text.is_empty() {
                let priority = self.get_priority(message);
                let tts_message = TtsMessage { text, priority };
                queue.enqueue(tts_message).await?;
            }
        }
        Ok(())
    }
}

impl Default for TtsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for TtsPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "tts".to_string(),
            name: "TTS読み上げプラグイン".to_string(),
            version: "1.0.0".to_string(),
            description: "棒読みちゃん/VOICEVOX連携でチャットメッセージを音声読み上げ".to_string(),
            author: "Liscov Team".to_string(),
            enabled: self.config.enabled,
            dependencies: vec![],
        }
    }

    async fn initialize(&mut self, context: PluginContext) -> LiscovResult<()> {
        tracing::info!("🔊 TTSプラグインを初期化中...");

        // 保存済み設定を読み込み
        if let Ok(Some(config_value)) = context
            .config_access
            .get_config(&context.plugin_id, "tts_config")
            .await
        {
            if let Ok(config) = serde_json::from_value::<TtsConfig>(config_value) {
                tracing::info!("🔊 保存済みTTS設定を読み込み");
                self.config = config;
            }
        }

        self.context = Some(context);

        // バックエンドを初期化
        if self.config.enabled {
            self.initialize_backend().await?;
        }

        tracing::info!("✅ TTSプラグイン初期化完了");
        Ok(())
    }

    async fn shutdown(&mut self) -> LiscovResult<()> {
        tracing::info!("🔊 TTSプラグインを終了中...");

        // キュー処理を停止
        if let Some(handle) = self.queue_handle.take() {
            handle.abort();
        }

        self.queue = None;
        self.backend = None;

        tracing::info!("✅ TTSプラグイン終了完了");
        Ok(())
    }

    async fn handle_event(&mut self, event: PluginEvent) -> LiscovResult<PluginResult> {
        if !self.config.enabled {
            return Ok(PluginResult::Skipped);
        }

        match event {
            PluginEvent::MessageReceived(message) => {
                self.queue_message(&message).await?;
                Ok(PluginResult::Success)
            }
            PluginEvent::MessagesReceived(messages) => {
                for message in messages {
                    self.queue_message(&message).await?;
                }
                Ok(PluginResult::Success)
            }
            PluginEvent::ApplicationStopping => {
                self.shutdown().await?;
                Ok(PluginResult::Success)
            }
            _ => Ok(PluginResult::Skipped),
        }
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info() {
        let plugin = TtsPlugin::new();
        let info = plugin.info();
        assert_eq!(info.id, "tts");
        assert!(!info.enabled); // デフォルトは無効
    }

    #[test]
    fn test_sanitize_text() {
        let plugin = TtsPlugin::new();

        // URLを除去
        let text = plugin.sanitize_text("こんにちは https://example.com テスト");
        assert_eq!(text, "こんにちは テスト");

        // 連続空白を1つに
        let text = plugin.sanitize_text("こんにちは    テスト");
        assert_eq!(text, "こんにちは テスト");
    }

    #[test]
    fn test_format_message() {
        let mut plugin = TtsPlugin::new();
        plugin.config.read_author_name = true;
        plugin.config.read_superchat_amount = false;

        let message = GuiChatMessage {
            id: "1".to_string(),
            timestamp: "00:00:00".to_string(),
            timestamp_usec: "0".to_string(),
            message_type: MessageType::Text,
            author: "テストユーザー".to_string(),
            author_icon_url: None,
            channel_id: "UC123".to_string(),
            content: "こんにちは".to_string(),
            runs: vec![],
            metadata: None,
            is_member: false,
            comment_count: None,
        };

        let text = plugin.format_message(&message);
        assert!(text.contains("テストユーザー"));
        assert!(text.contains("こんにちは"));
    }
}
