use crate::gui::{
    hooks::LiveChatHandle,
    styles::theme::{get_button_class, CssClasses},
};
use dioxus::prelude::*;

/// URL検証用のヘルパー関数
fn is_valid_youtube_url(url: &str) -> bool {
    if url.trim().is_empty() {
        return false;
    }

    // YouTube URL パターンを検証
    let youtube_patterns = [
        "youtube.com/watch?v=",
        "youtu.be/",
        "m.youtube.com/watch?v=",
        "youtube.com/live/",
    ];

    youtube_patterns.iter().any(|pattern| url.contains(pattern))
}

/// URL検証結果のメッセージを取得
fn get_url_validation_message(url: &str) -> Option<String> {
    if url.trim().is_empty() {
        return None;
    }

    if !is_valid_youtube_url(url) {
        return Some(
            "有効なYouTube URLを入力してください (例: https://www.youtube.com/watch?v=...)"
                .to_string(),
        );
    }

    // より厳密な検証
    if !url.starts_with("http") {
        return Some("URLはhttp://またはhttps://で始まる必要があります".to_string());
    }

    None
}

/// 入力セクションコンポーネント
/// YouTube URL入力と設定を管理
/// Phase 3: LiveChatServiceとの統合完了
/// Phase 4: CSSクラスベースのスタイリング
/// Phase 5: エラーハンドリング・バリデーション強化
#[component]
pub fn InputSection(live_chat_handle: LiveChatHandle) -> Element {
    // AppStateにアクセスして設定を同期
    let mut app_state = use_context::<Signal<crate::gui::models::AppState>>();

    let mut url_input = use_signal(|| app_state.read().url.clone());
    let mut output_file = use_signal(|| app_state.read().output_file.clone());
    let mut auto_save_enabled = use_signal(|| app_state.read().auto_save_enabled);

    // URL入力の初期化をAppStateから行う
    use_effect(move || {
        let state = app_state.read();
        if url_input.read().is_empty() && !state.url.is_empty() {
            url_input.set(state.url.clone());
        }
        if *output_file.read() != state.output_file {
            output_file.set(state.output_file.clone());
        }
        if *auto_save_enabled.read() != state.auto_save_enabled {
            auto_save_enabled.set(state.auto_save_enabled);
        }
    });

    // LiveChatハンドルから状態を取得
    // ボタンの状態: より詳細な状態管理
    let state = live_chat_handle.state;
    let is_stopping = live_chat_handle.is_stopping;

    // URL入力欄の制御：接続中またはConnecting中はreadonlyに
    let should_disable_url_input = use_signal(move || match *state.read() {
        crate::gui::services::ServiceState::Connected
        | crate::gui::services::ServiceState::Connecting => true,
        _ => false,
    });

    // URL検証状態
    let url_validation_message = get_url_validation_message(&url_input.read());
    let is_url_valid = url_validation_message.is_none() && !url_input.read().trim().is_empty();

    rsx! {
        div {
            class: CssClasses::INPUT_SECTION,

            h3 {
                "🔗 接続設定"
            }

            // YouTube URL入力
            div {
                class: CssClasses::FORM_GROUP,
                label {
                    class: CssClasses::FORM_LABEL,
                    "YouTube Live URL:"
                }
                input {
                    class: format!("{} {}",
                        CssClasses::FORM_INPUT,
                        if url_validation_message.is_some() { "input-error" } else if is_url_valid { "input-valid" } else { "" }
                    ),
                    r#type: "text",
                    placeholder: "https://www.youtube.com/watch?v=...",
                    value: "{url_input}",
                    readonly: *should_disable_url_input.read(),
                    oninput: move |event| {
                        // Paused状態でURL変更時は継続トークンを破棄して開始ボタンに戻る
                        let new_url = event.value();
                        let current_state = state.read().clone();

                        if matches!(current_state, crate::gui::services::ServiceState::Paused) {
                            // StateManagerに新しいURLを通知（継続トークンも自動的にクリア）
                            use crate::gui::state_management::{get_state_manager, AppEvent};
                            let state_manager = get_state_manager();
                            if !new_url.trim().is_empty() {
                                let _ = state_manager.send_event(AppEvent::CurrentUrlUpdated(Some(new_url.clone())));
                            }

                            // 状態をIdleに戻す
                            let _ = state_manager.send_event(AppEvent::ServiceStateChanged(crate::gui::services::ServiceState::Idle));

                            tracing::info!("🔄 URL changed during pause - returning to start button");
                        }

                        url_input.set(new_url.clone());

                        // AppStateも更新
                        let mut state = app_state.write();
                        state.url = new_url;

                        // 設定を永続化
                        use crate::gui::config_manager::save_app_state_async;
                        save_app_state_async(state.clone());
                    },
                }

                // URL検証メッセージ
                if let Some(ref validation_msg) = url_validation_message {
                    div {
                        class: "validation-message error",
                        style: "
                            color: #e53e3e;
                            font-size: 12px;
                            margin-top: 4px;
                            display: flex;
                            align-items: center;
                            gap: 4px;
                        ",
                        span { "⚠️" }
                        span { "{validation_msg}" }
                    }
                } else if is_url_valid {
                    div {
                        class: "validation-message success",
                        style: "
                            color: #38a169;
                            font-size: 12px;
                            margin-top: 4px;
                            display: flex;
                            align-items: center;
                            gap: 4px;
                        ",
                        span { "✅" }
                        span { "有効なYouTube URLです" }
                    }
                }
            }

                                                // 自動保存状態の簡潔な表示
            div {
                style: if auto_save_enabled() {
                    "
                        padding: 8px 12px;
                        border-radius: 6px;
                        font-size: 13px;
                        margin-bottom: 8px;
                        background: #d4edda;
                        border: 1px solid #c3e6cb;
                        color: #155724;
                    "
                } else {
                    "
                        padding: 8px 12px;
                        border-radius: 6px;
                        font-size: 13px;
                        margin-bottom: 8px;
                        background: #fff3cd;
                        border: 1px solid #ffeaa7;
                        color: #856404;
                    "
                },

                if auto_save_enabled() {
                    "✅ 自動保存: 有効"
                } else {
                    "⚠️ 自動保存: 無効 (設定画面で有効化可能)"
                }
            }

            // エラー表示
            if let crate::gui::services::ServiceState::Error(ref error) = *state.read() {
                div {
                    class: CssClasses::ERROR_MESSAGE,
                    "❌ {error}"
                }
            }

            // 接続のヒント
            if !*should_disable_url_input.read() && url_input.read().trim().is_empty() {
                div {
                    style: "
                        background: linear-gradient(135deg, #ebf8ff 0%, #bee3f8 100%);
                        border: 1px solid #90cdf4;
                        color: #2b6cb0;
                        padding: 12px 16px;
                        border-radius: 8px;
                        margin: 16px 0;
                        font-size: 14px;
                        line-height: 1.5;
                    ",
                    div {
                        style: "font-weight: 600; margin-bottom: 4px;",
                        "💡 使用方法"
                    }
                    ol {
                        style: "margin-left: 16px;",
                        li { "YouTubeでライブ配信を開き、URLをコピー" }
                        li { "上記の入力欄にURLを貼り付け" }
                        li { "「▶️ 開始」をクリックして監視開始" }
                    }
                }
            }

            // 制御ボタン
            div {
                class: CssClasses::BTN_GROUP,
                style: "
                    display: flex;
                    gap: 8px;
                    flex-wrap: wrap;
                    align-items: center;
                ",

                                // メインボタン（開始/停止/再開）
                button {
                    class: {
                        let (button_type, is_disabled) = match *state.read() {
                            crate::gui::services::ServiceState::Connecting => ("warning", true),
                            crate::gui::services::ServiceState::Connected => ("danger", false),
                            crate::gui::services::ServiceState::Paused => ("success", false),
                            crate::gui::services::ServiceState::Error(_) => ("primary", !is_url_valid),
                            crate::gui::services::ServiceState::Idle => ("primary", !is_url_valid),
                        };

                        get_button_class(button_type, is_disabled || *is_stopping.read())
                    },
                    style: "
                        min-width: 120px;
                    ",
                    disabled: {
                        match *state.read() {
                            crate::gui::services::ServiceState::Connecting => true,
                            crate::gui::services::ServiceState::Connected => *is_stopping.read(),
                            crate::gui::services::ServiceState::Paused => false,
                            crate::gui::services::ServiceState::Error(_) => !is_url_valid || *is_stopping.read(),
                            crate::gui::services::ServiceState::Idle => !is_url_valid || *is_stopping.read(),
                        }
                    },
                    onclick: {
                        let handle = live_chat_handle.clone();
                        let url = url_input.read().clone();
                                                            let output = if auto_save_enabled() && !output_file.read().trim().is_empty() {
                                        Some(output_file.read().clone())
                                    } else {
                                        None
                                    };

                        move |_| {
                            // 停止処理中は操作を無効化
                            if *handle.is_stopping.read() {
                                tracing::debug!("🚫 Button click ignored - stopping in progress");
                                return;
                            }

                            let current_state = handle.state.read().clone();

                            match current_state {
                                crate::gui::services::ServiceState::Connected => {
                                    tracing::info!("⏸️ Pausing live chat monitoring");
                                    handle.pause_monitoring();
                                }
                                crate::gui::services::ServiceState::Paused => {
                                    tracing::info!("▶️ Resuming live chat monitoring");
                                    handle.resume_monitoring(output.clone());
                                }
                                crate::gui::services::ServiceState::Idle |
                                crate::gui::services::ServiceState::Error(_) => {
                                    tracing::info!("▶️ Starting live chat monitoring for URL: {}", url);

                                    // StateManagerにURLを通知
                                    use crate::gui::state_management::{get_state_manager, AppEvent};
                                    let state_manager = get_state_manager();
                                    let _ = state_manager.send_event(AppEvent::CurrentUrlUpdated(Some(url.clone())));

                                    handle.start_monitoring(url.clone(), output.clone());
                                }
                                crate::gui::services::ServiceState::Connecting => {
                                    // 接続中は何もしない
                                }
                            }
                        }
                    },

                    // ボタンテキスト
                    match *state.read() {
                        crate::gui::services::ServiceState::Connecting => "🔄 接続中...",
                        crate::gui::services::ServiceState::Connected => {
                            if *is_stopping.read() {
                                "🔄 停止中..."
                            } else {
                                "⏸️ 停止"
                            }
                        },
                        crate::gui::services::ServiceState::Paused => "▶️ 再開",
                        crate::gui::services::ServiceState::Error(_) => "▶️ 開始",
                        crate::gui::services::ServiceState::Idle => "▶️ 開始",
                    }
                }

                                // Paused状態での初期化ボタン
                if matches!(*state.read(), crate::gui::services::ServiceState::Paused) {
                    // 初期化ボタン（完全停止 + クリア）
                    button {
                        class: get_button_class("warning", *is_stopping.read()),
                        disabled: *is_stopping.read(),
                        style: "
                            min-width: 120px;
                        ",
                        onclick: {
                            let handle = live_chat_handle.clone();
                            move |_| {
                                tracing::info!("🔄 Initializing - stopping monitoring and clearing messages");

                                // 完全停止を実行
                                handle.stop_monitoring();

                                // メッセージもクリア
                                handle.clear_messages();
                            }
                        },
                        "🔄 初期化"
                    }
                }

                // クリアボタン（Paused以外の状態で表示）
                if !matches!(*state.read(), crate::gui::services::ServiceState::Paused) {
                    button {
                        class: get_button_class("secondary", live_chat_handle.messages.read().is_empty()),
                        disabled: live_chat_handle.messages.read().is_empty(),
                        onclick: {
                            let handle = live_chat_handle.clone();
                            move |_| {
                                handle.clear_messages();
                            }
                        },
                        "🗑️ クリア"
                    }
                }
            }
        }
    }
}
