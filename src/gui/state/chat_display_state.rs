//! チャット表示機能の統合状態管理
//!
//! すべてのチャット表示関連の状態を一元管理し、Signal間の依存関係を明確化

use crate::{
    chat_management::MessageFilter,
    gui::{
        models::{ActiveTab, GuiChatMessage},
        unified_config::HighlightConfig,
    },
};
use dioxus::prelude::*;
use std::collections::HashSet;

/// チャット表示の統合状態
///
/// 全ての状態をここで一元管理し、相互依存を制御
#[derive(Clone)]
pub struct ChatDisplayState {
    // === メッセージ関連 ===
    /// 生メッセージリスト
    pub messages: Signal<Vec<GuiChatMessage>>,
    /// フィルタ適用済みメッセージリスト
    pub filtered_messages: Signal<Vec<GuiChatMessage>>,
    /// メッセージフィルタ設定
    pub message_filter: Signal<MessageFilter>,

    // === UI制御状態 ===
    /// 自動スクロール有効フラグ
    pub auto_scroll_enabled: Signal<bool>,
    /// ユーザーが手動スクロールしたかフラグ
    pub user_has_scrolled: Signal<bool>,
    /// 現在のアクティブタブ
    pub current_tab: Signal<ActiveTab>,
    /// フィルタパネル表示フラグ
    pub show_filter_panel: Signal<bool>,
    /// タイムスタンプ表示フラグ
    pub show_timestamps: Signal<bool>,

    // === ハイライト機能状態 ===
    /// ハイライト設定
    pub highlight_config: Signal<HighlightConfig>,
    /// ハイライト対象メッセージIDセット
    pub highlighted_message_ids: Signal<HashSet<String>>,
    /// ハイライト有効フラグ
    pub highlight_enabled: Signal<bool>,
    /// ハイライト継続時間（秒）
    pub highlight_duration: Signal<u64>,

    // === 内部制御状態 ===
    /// 最後に処理したメッセージ数（ハイライト計算用）
    pub last_message_count: Signal<usize>,
    /// 現在のスクロール位置
    pub scroll_position: Signal<f64>,
    /// テストボタン表示フラグ
    pub show_test_button: Signal<bool>,
}

impl ChatDisplayState {
    /// 新しい統合状態を初期化
    pub fn new() -> Self {
        Self {
            // メッセージ関連
            messages: use_signal(Vec::new),
            filtered_messages: use_signal(Vec::new),
            message_filter: use_signal(MessageFilter::default),

            // UI制御状態
            auto_scroll_enabled: use_signal(|| true),
            user_has_scrolled: use_signal(|| false),
            current_tab: use_signal(|| ActiveTab::ChatMonitor),
            show_filter_panel: use_signal(|| false),
            show_timestamps: use_signal(|| true),

            // ハイライト機能状態
            highlight_config: use_signal(|| HighlightConfig {
                enabled: true,
                duration_seconds: 5,
                max_messages: 10,
                long_term_mode: false,
                update_interval_ms: 300,
            }),
            highlighted_message_ids: use_signal(HashSet::new),
            highlight_enabled: use_signal(|| true),
            highlight_duration: use_signal(|| 5u64),

            // 内部制御状態
            last_message_count: use_signal(|| 0usize),
            scroll_position: use_signal(|| 0.0),
            show_test_button: use_signal(|| false),
        }
    }

    /// 外部から提供されたSignalで初期化（既存コンポーネントとの互換性用）
    pub fn from_external_signals(
        messages: Signal<Vec<GuiChatMessage>>,
        message_filter: Signal<MessageFilter>,
    ) -> Self {
        let mut state = Self::new();
        state.messages = messages;
        state.message_filter = message_filter;
        state
    }
}

/// 状態更新のための安全なインターフェース
impl ChatDisplayState {
    /// メッセージ数の変化を検出
    pub fn has_new_messages(&self) -> bool {
        let current_count = self.filtered_messages.read().len();
        let last_count = *self.last_message_count.read();
        current_count > last_count
    }

    /// ハイライト対象の新着メッセージ数を取得
    pub fn get_new_message_count(&self) -> usize {
        let current_count = self.filtered_messages.read().len();
        let last_count = *self.last_message_count.read();
        current_count.saturating_sub(last_count)
    }

    /// 最後に処理したメッセージ数を更新
    pub fn update_last_message_count(&mut self) {
        let current_count = self.filtered_messages.read().len();
        self.last_message_count
            .with_mut(|count| *count = current_count);
    }

    /// 自動スクロールの実行条件をチェック
    pub fn should_auto_scroll(&self) -> bool {
        *self.auto_scroll_enabled.read() && !*self.user_has_scrolled.read()
    }

    /// ハイライト機能の実行条件をチェック
    pub fn should_highlight(&self) -> bool {
        let config = self.highlight_config.read();
        config.enabled && *self.highlight_enabled.read()
    }

    /// 状態の整合性をリセット（接続リセット時などに使用）
    pub fn reset_state(&mut self) {
        self.last_message_count.with_mut(|count| *count = 0);
        self.highlighted_message_ids.with_mut(|ids| ids.clear());
        self.user_has_scrolled
            .with_mut(|scrolled| *scrolled = false);
        self.scroll_position.with_mut(|pos| *pos = 0.0);

        tracing::info!("🔄 [UNIFIED_STATE] All chat display states reset");
    }

    /// デバッグ用：現在の状態をログ出力
    pub fn log_current_state(&self) {
        let message_count = self.filtered_messages.read().len();
        let last_count = *self.last_message_count.read();
        let auto_scroll = *self.auto_scroll_enabled.read();
        let user_scrolled = *self.user_has_scrolled.read();
        let highlight_enabled = *self.highlight_enabled.read();
        let highlight_count = self.highlighted_message_ids.read().len();

        tracing::debug!(
            "🔍 [STATE_DEBUG] Messages: {}/{}, AutoScroll: {}, UserScrolled: {}, Highlight: {} ({})",
            message_count, last_count, auto_scroll, user_scrolled, highlight_enabled, highlight_count
        );
    }
}

/// フィルタリング処理のための専用メソッド
impl ChatDisplayState {
    /// メッセージフィルタリングを実行し、結果を更新
    pub fn apply_message_filter(&mut self) {
        let messages = self.messages.read();
        let filter = self.message_filter.read();
        let filtered = filter.filter_messages(&messages);

        let old_count = self.filtered_messages.read().len();
        let new_count = filtered.len();

        self.filtered_messages.with_mut(|msgs| *msgs = filtered);

        if old_count != new_count {
            tracing::debug!(
                "🔍 [FILTER] Messages filtered: {} → {} (filter: {})",
                old_count,
                new_count,
                if filter.is_active() {
                    "active"
                } else {
                    "inactive"
                }
            );
        }
    }

    /// フィルタ設定を更新
    pub fn update_filter(&mut self, new_filter: MessageFilter) {
        self.message_filter.with_mut(|filter| *filter = new_filter);
        self.apply_message_filter(); // 即座にフィルタリングを適用
    }
}

/// ハイライト機能のための専用メソッド
impl ChatDisplayState {
    /// 新着メッセージのハイライトIDを生成
    pub fn generate_highlight_ids(&self) -> Vec<String> {
        let new_count = self.get_new_message_count();
        if new_count == 0 {
            return Vec::new();
        }

        let messages = self.filtered_messages.read();
        let config = self.highlight_config.read();
        let max_highlight = config.get_effective_max_messages().min(new_count);
        let start_index = messages.len() - max_highlight;

        messages
            .iter()
            .skip(start_index)
            .take(max_highlight)
            .map(|message| {
                format!(
                    "{}:{}:{}",
                    message.timestamp,
                    message.author,
                    message.content.chars().take(20).collect::<String>()
                )
            })
            .collect()
    }

    /// ハイライトIDセットを更新
    pub fn update_highlighted_ids(&mut self, new_ids: Vec<String>) {
        let new_ids_set: HashSet<String> = new_ids.iter().cloned().collect();
        self.highlighted_message_ids
            .with_mut(|ids| *ids = new_ids_set);

        tracing::info!(
            "🎯 [HIGHLIGHT] Updated highlight IDs: {} messages",
            new_ids.len()
        );
    }

    /// 指定されたIDのハイライトをクリア
    pub fn clear_highlight_ids(&mut self, ids_to_clear: &HashSet<String>) {
        self.highlighted_message_ids.with_mut(|ids| {
            for id in ids_to_clear {
                ids.remove(id);
            }
        });

        tracing::debug!(
            "🎯 [HIGHLIGHT] Cleared {} highlight IDs",
            ids_to_clear.len()
        );
    }

    /// ハイライト設定を動的更新
    pub fn update_highlight_config(&mut self, enabled: bool, duration: u64) {
        self.highlight_enabled.with_mut(|en| *en = enabled);
        self.highlight_duration.with_mut(|dur| *dur = duration);

        // 設定変更をHighlightConfigにも反映
        self.highlight_config.with_mut(|config| {
            config.enabled = enabled;
            config.duration_seconds = duration;
        });

        tracing::info!(
            "🎯 [HIGHLIGHT] Config updated: enabled={}, duration={}s",
            enabled,
            duration
        );
    }
}

impl Default for ChatDisplayState {
    fn default() -> Self {
        Self::new()
    }
}

/// デバッグ用の状態ダンプ
impl std::fmt::Debug for ChatDisplayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatDisplayState")
            .field("message_count", &self.messages.read().len())
            .field("filtered_count", &self.filtered_messages.read().len())
            .field("last_message_count", &*self.last_message_count.read())
            .field("auto_scroll_enabled", &*self.auto_scroll_enabled.read())
            .field("user_has_scrolled", &*self.user_has_scrolled.read())
            .field("highlight_enabled", &*self.highlight_enabled.read())
            .field(
                "highlighted_ids_count",
                &self.highlighted_message_ids.read().len(),
            )
            .finish()
    }
}
