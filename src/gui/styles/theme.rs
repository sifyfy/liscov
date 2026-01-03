//! テーマとスタイルヘルパー
//! Phase 4: 高度なスタイリング実装

use crate::gui::models::MessageType;

/// CSS クラス名の定数
pub struct CssClasses;

impl CssClasses {
    // アプリケーション
    pub const APP: &'static str = "app";
    pub const MAIN_WINDOW: &'static str = "main-window";
    pub const MAIN_CONTENT: &'static str = "main-content";
    pub const LEFT_PANEL: &'static str = "left-panel";
    pub const RIGHT_PANEL: &'static str = "right-panel";

    // ヘッダー
    pub const APP_HEADER: &'static str = "app-header";
    pub const APP_TITLE: &'static str = "app-title";
    pub const APP_SUBTITLE: &'static str = "app-subtitle";

    // 入力セクション
    pub const INPUT_SECTION: &'static str = "input-section";
    pub const FORM_GROUP: &'static str = "form-group";
    pub const FORM_LABEL: &'static str = "form-label";
    pub const FORM_INPUT: &'static str = "form-input";

    // ボタン
    pub const BTN: &'static str = "btn";
    pub const BTN_PRIMARY: &'static str = "btn-primary";
    pub const BTN_DANGER: &'static str = "btn-danger";
    pub const BTN_WARNING: &'static str = "btn-warning";
    pub const BTN_SECONDARY: &'static str = "btn-secondary";
    pub const BTN_GROUP: &'static str = "btn-group";

    // チャット
    pub const CHAT_DISPLAY: &'static str = "chat-display";
    pub const CHAT_HEADER: &'static str = "chat-header";
    pub const CHAT_FOOTER: &'static str = "chat-footer";
    pub const CONNECTION_STATUS: &'static str = "connection-status";
    pub const CHAT_CONTROLS: &'static str = "chat-controls";
    pub const CHECKBOX_LABEL: &'static str = "checkbox-label";

    // メッセージ
    pub const MESSAGE_LIST: &'static str = "message-list";
    pub const CHAT_MESSAGE: &'static str = "chat-message";
    pub const MESSAGE_HEADER: &'static str = "message-header";
    pub const MESSAGE_AUTHOR: &'static str = "message-author";
    pub const MESSAGE_TIMESTAMP: &'static str = "message-timestamp";
    pub const MESSAGE_CONTENT: &'static str = "message-content";
    pub const NO_MESSAGES: &'static str = "no-messages";

    // ステータス
    pub const STATUS_PANEL: &'static str = "status-panel";
    pub const STATUS_HEADER: &'static str = "status-header";
    pub const STATS_GRID: &'static str = "stats-grid";
    pub const STAT_ITEM: &'static str = "stat-item";
    pub const STAT_VALUE: &'static str = "stat-value";
    pub const STAT_LABEL: &'static str = "stat-label";

    // エラー
    pub const ERROR_MESSAGE: &'static str = "error-message";

    // フッター
    pub const FOOTER_STATS: &'static str = "footer-stats";
}

/// メッセージタイプに応じたCSSクラスを取得
pub fn get_message_class(message_type: &MessageType) -> String {
    let base_class = CssClasses::CHAT_MESSAGE;
    let type_class = match message_type {
        MessageType::Text => "text",
        MessageType::System => "system",
        MessageType::SuperChat { .. } => "superchat",
        MessageType::SuperSticker { .. } => "superchat",
        MessageType::Membership { .. } => "system",
        MessageType::MembershipGift { .. } => "membership-gift",
    };
    format!("{} {}", base_class, type_class)
}

/// 接続状態に応じたCSSクラスを取得
pub fn get_connection_status_class(is_connected: bool, is_connecting: bool) -> String {
    let base_class = CssClasses::CONNECTION_STATUS;
    let status_class = if is_connecting {
        "connecting"
    } else if is_connected {
        "connected"
    } else {
        "disconnected"
    };
    format!("{} {}", base_class, status_class)
}

/// ボタンの状態に応じたCSSクラスを取得
pub fn get_button_class(variant: &str, disabled: bool) -> String {
    let base_class = CssClasses::BTN;
    let variant_class = match variant {
        "primary" => CssClasses::BTN_PRIMARY,
        "danger" => CssClasses::BTN_DANGER,
        "warning" => CssClasses::BTN_WARNING,
        "secondary" => CssClasses::BTN_SECONDARY,
        _ => CssClasses::BTN_PRIMARY,
    };

    let mut classes = format!("{} {}", base_class, variant_class);
    if disabled {
        classes.push_str(" disabled");
    }
    classes
}

/// CSSの埋め込み用ヘルパー
pub fn get_embedded_css() -> &'static str {
    include_str!("theme.css")
}
