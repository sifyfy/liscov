//! グローバルTTSマネージャー
//!
//! メッセージフローからTTS読み上げを呼び出すためのグローバルインスタンス

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

use crate::database::ViewerCustomInfo;
use crate::gui::models::GuiChatMessage;
use crate::gui::plugins::tts_plugin::backends::{BouyomichanBackend, TtsBackend, VoicevoxBackend};
use crate::gui::plugins::tts_plugin::config::{TtsBackendType, TtsConfig};
use crate::gui::plugins::tts_plugin::launcher;
use crate::gui::plugins::tts_plugin::queue::{TtsMessage, TtsPriority, TtsQueue};
use crate::gui::models::MessageType;

/// グローバルTTSマネージャー
static TTS_MANAGER: OnceLock<Arc<RwLock<TtsManager>>> = OnceLock::new();

/// TTSマネージャーのグローバルインスタンスを取得
pub fn get_tts_manager() -> Arc<RwLock<TtsManager>> {
    TTS_MANAGER
        .get_or_init(|| Arc::new(RwLock::new(TtsManager::new())))
        .clone()
}

/// TTSマネージャー
pub struct TtsManager {
    config: TtsConfig,
    queue: Option<TtsQueue>,
    _backend: Option<Arc<dyn TtsBackend>>,
    /// 視聴者情報キャッシュ（読み仮名用）
    viewer_info_cache: HashMap<String, ViewerCustomInfo>,
    /// 現在の配信者チャンネルID
    broadcaster_channel_id: Option<String>,
}

impl TtsManager {
    /// 新しいインスタンスを作成
    pub fn new() -> Self {
        Self {
            config: TtsConfig::default(),
            queue: None,
            _backend: None,
            viewer_info_cache: HashMap::new(),
            broadcaster_channel_id: None,
        }
    }

    /// 配信者チャンネルIDを設定し、視聴者情報をロード
    pub async fn set_broadcaster_channel_id(&mut self, broadcaster_id: String) {
        if self.broadcaster_channel_id.as_ref() == Some(&broadcaster_id) {
            return; // 同じIDなら何もしない
        }

        self.broadcaster_channel_id = Some(broadcaster_id.clone());
        self.viewer_info_cache.clear();

        // DBから視聴者情報をロード
        match crate::database::get_connection().await {
            Ok(conn) => {
                match crate::database::get_all_viewer_custom_info_for_broadcaster(
                    &conn,
                    &broadcaster_id,
                ) {
                    Ok(cache) => {
                        tracing::info!(
                            "🔊 TTS: Loaded {} viewer info entries for broadcaster",
                            cache.len()
                        );
                        self.viewer_info_cache = cache;
                    }
                    Err(e) => {
                        tracing::error!("🔊 TTS: Failed to load viewer info cache: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("🔊 TTS: Failed to get DB connection: {}", e);
            }
        }
    }

    /// 視聴者情報キャッシュを更新（外部からの同期用）
    pub fn update_viewer_info(&mut self, info: ViewerCustomInfo) {
        self.viewer_info_cache
            .insert(info.viewer_channel_id.clone(), info);
    }

    /// 視聴者の読み仮名を取得
    fn get_viewer_reading(&self, viewer_channel_id: &str) -> Option<&str> {
        self.viewer_info_cache
            .get(viewer_channel_id)
            .and_then(|info| info.reading.as_deref())
    }

    /// 投稿者名を処理（@除去、suffix除去）
    fn process_author_name(&self, name: &str) -> String {
        let mut result = name.to_string();

        // 先頭の@を除去
        if self.config.strip_at_prefix && result.starts_with('@') {
            result = result[1..].to_string();
        }

        // 末尾の -xxx (ハンドルsuffix) を除去
        if self.config.strip_handle_suffix {
            // 正規表現: 末尾の -[0-9a-z]{3} にマッチ
            let suffix_pattern = regex::Regex::new(r"-[0-9a-z]{3}$").unwrap();
            result = suffix_pattern.replace(&result, "").to_string();
        }

        result
    }

    /// 設定を更新してバックエンドを再初期化
    pub async fn update_config(&mut self, config: TtsConfig) {
        let was_enabled = self.config.enabled;
        self.config = config.clone();

        // 設定変更時は常にバックエンドを再初期化（話者IDや音量の変更を即座に反映）
        self.initialize_backend().await;

        // TTS有効化時に自動起動
        if self.config.enabled && !was_enabled {
            self.try_auto_launch();
        }

        tracing::info!(
            "🔊 TTS設定更新: enabled={}, backend={:?}",
            self.config.enabled,
            self.config.backend
        );
    }

    /// 自動起動を試みる（設定で有効な場合のみ）
    fn try_auto_launch(&self) {
        match self.config.backend {
            TtsBackendType::Bouyomichan if self.config.bouyomichan.auto_launch => {
                if let Err(e) = launcher::launch_backend(
                    TtsBackendType::Bouyomichan,
                    self.config.bouyomichan.executable_path.as_deref(),
                ) {
                    tracing::warn!("🔊 棒読みちゃんの自動起動に失敗: {}", e);
                }
            }
            TtsBackendType::Voicevox if self.config.voicevox.auto_launch => {
                if let Err(e) = launcher::launch_backend(
                    TtsBackendType::Voicevox,
                    self.config.voicevox.executable_path.as_deref(),
                ) {
                    tracing::warn!("🔊 VOICEVOXの自動起動に失敗: {}", e);
                }
            }
            _ => {}
        }
    }

    /// 現在の設定を取得
    pub fn config(&self) -> &TtsConfig {
        &self.config
    }

    /// バックエンドを初期化
    async fn initialize_backend(&mut self) {
        // 既存のキューを破棄（JoinHandleはDropで自動的にabortされる）
        self.queue = None;
        self._backend = None;

        if !self.config.enabled {
            tracing::info!("🔊 TTS無効化");
            return;
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
            let (queue, _handle) = TtsQueue::new(backend.clone(), self.config.queue_size_limit);
            self.queue = Some(queue);
            self._backend = Some(backend.clone());
            tracing::info!("🔊 TTSバックエンド初期化: {}", backend.name());
        }
    }

    /// メッセージを読み上げキューに追加
    pub async fn speak_message(&self, message: &GuiChatMessage) {
        if !self.config.enabled {
            return;
        }

        if let Some(ref queue) = self.queue {
            let text = self.format_message(message);
            if !text.is_empty() {
                let priority = self.get_priority(message);
                let tts_message = TtsMessage { text, priority };

                if let Err(e) = queue.enqueue(tts_message).await {
                    tracing::warn!("🔊 TTS キュー追加失敗: {}", e);
                }
            }
        }
    }

    /// メッセージから読み上げテキストを生成
    fn format_message(&self, message: &GuiChatMessage) -> String {
        let mut parts = Vec::new();

        // 投稿者名（読み仮名があればそちらを使用）
        if self.config.read_author_name {
            let author_name = self
                .get_viewer_reading(&message.channel_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| self.process_author_name(&message.author));

            // 敬称を付ける
            let author_with_honorific = if self.config.add_honorific {
                format!("{}さん", author_name)
            } else {
                author_name
            };
            parts.push(author_with_honorific);
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
}

impl Default for TtsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 設定読み込みと初期化を行う
pub async fn initialize_tts_from_config() {
    let manager = get_tts_manager();

    // UnifiedConfigManagerから設定を読み込み
    if let Ok(config_manager) = crate::gui::unified_config::UnifiedConfigManager::new().await {
        if let Ok(Some(config)) = config_manager
            .get_typed_config::<TtsConfig>("tts_config")
            .await
        {
            // アプリ起動時の自動起動
            if config.enabled {
                try_auto_launch_from_config(&config);
            }

            let mut mgr = manager.write().await;
            // update_configでは was_enabled=false のため自動起動が再度トリガーされるが、
            // is_process_running チェックにより二重起動は防がれる
            mgr.update_config(config).await;
            tracing::info!("🔊 TTS設定を読み込んで初期化しました");
            return;
        }
    }

    tracing::debug!("🔊 TTS設定なし、デフォルト状態で待機");
}

/// 設定から自動起動を試みる（アプリ起動時用）
fn try_auto_launch_from_config(config: &TtsConfig) {
    match config.backend {
        TtsBackendType::Bouyomichan if config.bouyomichan.auto_launch => {
            tracing::info!("🚀 アプリ起動時: 棒読みちゃんを自動起動中...");
            if let Err(e) = launcher::launch_backend(
                TtsBackendType::Bouyomichan,
                config.bouyomichan.executable_path.as_deref(),
            ) {
                tracing::warn!("🔊 棒読みちゃんの自動起動に失敗: {}", e);
            }
        }
        TtsBackendType::Voicevox if config.voicevox.auto_launch => {
            tracing::info!("🚀 アプリ起動時: VOICEVOXを自動起動中...");
            if let Err(e) = launcher::launch_backend(
                TtsBackendType::Voicevox,
                config.voicevox.executable_path.as_deref(),
            ) {
                tracing::warn!("🔊 VOICEVOXの自動起動に失敗: {}", e);
            }
        }
        _ => {}
    }
}

/// TTSシャットダウン処理（アプリ終了時に呼び出す）
pub async fn shutdown_tts() {
    let manager = get_tts_manager();
    let config = manager.read().await.config().clone();

    // 棒読みちゃんの終了処理
    if config.bouyomichan.auto_close_on_exit
        && launcher::was_launched_by_liscov(TtsBackendType::Bouyomichan)
    {
        tracing::info!("🔊 アプリ終了: 棒読みちゃんを終了します");
        launcher::terminate_launched_backend(TtsBackendType::Bouyomichan);
    }

    // VOICEVOXの終了処理
    if config.voicevox.auto_close_on_exit
        && launcher::was_launched_by_liscov(TtsBackendType::Voicevox)
    {
        tracing::info!("🔊 アプリ終了: VOICEVOXを終了します");
        launcher::terminate_launched_backend(TtsBackendType::Voicevox);
    }
}
