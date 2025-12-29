//! タブナビゲーション：複数機能タブの実装

use crate::gui::models::ActiveTab;
use dioxus::prelude::*;

/// タブナビゲーションバー
#[component]
pub fn TabNavigation(active_tab: ActiveTab, on_tab_change: EventHandler<ActiveTab>) -> Element {
    let tabs = vec![
        ActiveTab::ChatMonitor,
        ActiveTab::DataExport,
        ActiveTab::RevenueAnalytics,
        ActiveTab::Raw,
        ActiveTab::SignalAnalysis,
        ActiveTab::Settings,
    ];

    rsx! {
        div {
            class: "tab-navigation",
            style: "
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                border-radius: 12px;
                margin-bottom: 20px;
                box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
                overflow: hidden;
                flex-shrink: 0;
            ",

            // タブボタンコンテナ
            div {
                class: "tab-buttons",
                style: "
                    display: flex;
                    width: 100%;
                    position: relative;
                ",

                // 各タブボタン
                for tab in tabs {
                    TabButton {
                        key: "{tab:?}",
                        tab: tab.clone(),
                        is_active: tab == active_tab,
                        on_click: {
                            let tab_for_closure = tab.clone();
                            move |_| on_tab_change.call(tab_for_closure.clone())
                        },
                    }
                }
            }
        }
    }
}

/// 個別のタブボタンコンポーネント
#[component]
fn TabButton(tab: ActiveTab, is_active: bool, on_click: EventHandler<MouseEvent>) -> Element {
    let button_style = if is_active {
        "
            flex: 1;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 8px;
            padding: 12px 16px;
            border: none;
            border-radius: 8px;
            background: rgba(255, 255, 255, 0.95);
            color: #333;
            font-weight: 600;
            font-size: 14px;
            cursor: pointer;
            transition: all 0.3s ease;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.15);
            transform: translateY(-1px);
        "
    } else {
        "
            flex: 1;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 8px;
            padding: 12px 16px;
            border: none;
            border-radius: 8px;
            background: transparent;
            color: rgba(255, 255, 255, 0.8);
            font-weight: 500;
            font-size: 14px;
            cursor: pointer;
            transition: all 0.3s ease;
        "
    };

    rsx! {
        button {
            style: "{button_style}",
            onclick: on_click,

            // タブアイコン
            span {
                style: "font-size: 16px;",
                "{tab.icon()}"
            }

            // タブテキスト
            span {
                style: "white-space: nowrap;",
                "{tab.to_string()}"
            }
        }
    }
}

/// タブコンテンツエリアコンポーネント
#[component]
pub fn TabContent(
    active_tab: ActiveTab,
    live_chat_handle: crate::gui::hooks::LiveChatHandle,
    global_filter: Signal<crate::chat_management::MessageFilter>,
) -> Element {
    rsx! {
        div {
            class: "tab-content-container",
            style: "height: 100%; position: relative;",

            // Chat Monitor タブ
            div {
                class: "tab-content chat-monitor",
                style: format!("
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    padding: 4px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    display: {};
                    flex-direction: column;
                ", if active_tab == ActiveTab::Chat || active_tab == ActiveTab::ChatMonitor { "flex" } else { "none" }),

                ChatMonitorContent {
                    live_chat_handle: live_chat_handle.clone(),
                    global_filter: global_filter,
                    active_tab: active_tab,
                }
            }

            // Export タブ
            div {
                class: "tab-content export",
                style: format!("
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    display: {};
                    overflow-y: auto;
                ", if active_tab == ActiveTab::Export || active_tab == ActiveTab::DataExport { "block" } else { "none" }),

                DataExportContent {
                    live_chat_handle: live_chat_handle.clone()
                }
            }

            // Revenue タブ
            div {
                class: "tab-content revenue",
                style: format!("
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    display: {};
                    overflow-y: auto;
                ", if active_tab == ActiveTab::Revenue || active_tab == ActiveTab::RevenueAnalytics { "block" } else { "none" }),

                RevenueAnalyticsContent {
                    live_chat_handle: live_chat_handle.clone()
                }
            }

            // Raw タブ
            div {
                class: "tab-content raw",
                style: format!("
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    display: {};
                    overflow-y: auto;
                ", if active_tab == ActiveTab::Raw { "block" } else { "none" }),

                crate::gui::components::raw_response_settings::RawResponseSettings {}
            }

            // Signal Analysis タブ
            div {
                class: "tab-content signal-analysis",
                style: format!("
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    display: {};
                    overflow-y: auto;
                ", if active_tab == ActiveTab::SignalAnalysis { "block" } else { "none" }),

                crate::gui::components::signal_analyzer::SignalAnalyzer {}
            }

            // Settings タブ
            div {
                class: "tab-content settings",
                style: format!("
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    display: {};
                    overflow-y: auto;
                ", if active_tab == ActiveTab::Settings { "block" } else { "none" }),

                SettingsContent {}
            }
        }
    }
}

/// 設定画面コンテンツ
#[component]
fn SettingsContent() -> Element {
    rsx! {
        div {
            class: "settings-content",

            h2 {
                style: "
                    font-size: 28px;
                    color: #333;
                    margin: 0 0 8px 0;
                ",
                "⚙️ Settings"
            }

            p {
                style: "color: #666; margin: 0 0 30px 0;",
                "Configure application settings and preferences."
            }

            // メンバー限定配信認証
            crate::gui::components::auth_panel::AuthPanel {}

            // チャット表示設定
            ChatDisplaySettings {}

            // ハイライト設定
            HighlightSettings {}

            // 自動保存設定
            AutoSaveSettings {}

            // UI設定
            UiSettings {}

            // Signal最適化設定
            SignalOptimizationSettings {}

            // レスポンス保存設定
            crate::gui::components::raw_response_settings::RawResponseSettings {}
        }
    }
}

/// チャット表示設定コンポーネント
#[component]
fn ChatDisplaySettings() -> Element {
    let app_state = use_context::<Signal<crate::gui::models::AppState>>();
    let font_size = use_signal(|| app_state.read().chat_display_config.message_font_size);

    // AppStateから設定を同期
    use_effect({
        let mut font_size = font_size.clone();
        let app_state = app_state.clone();

        move || {
            let config = app_state.read().chat_display_config.clone();
            font_size.set(config.message_font_size);
        }
    });

    // 設定を保存する関数
    let save_font_size = move |new_size: u8| {
        let mut app_state = app_state.clone();
        let mut font_size = font_size.clone();

        spawn(async move {
            // AppStateとSignalを更新
            app_state.with_mut(|state| {
                state.chat_display_config.message_font_size = new_size;
            });
            font_size.set(new_size);

            // 永続化
            if let Ok(config_manager) =
                crate::gui::unified_config::UnifiedConfigManager::new().await
            {
                let config = app_state.read().chat_display_config.clone();
                let _ = config_manager
                    .set_typed_config("chat_display", &config)
                    .await;
                let _ = config_manager.flush_dirty_configs().await;
            }
        });
    };

    rsx! {
        div {
            style: "
                background: #f8f9fa;
                border: 1px solid #e9ecef;
                border-radius: 8px;
                padding: 16px;
                margin-bottom: 20px;
            ",

            h3 {
                style: "
                    margin: 0 0 16px 0;
                    color: #495057;
                ",
                "💬 チャット表示設定"
            }

            // 文字サイズ設定
            div {
                style: "margin-bottom: 20px;",

                label {
                    style: "
                        display: block;
                        font-weight: 500;
                        color: #2d3748;
                        margin-bottom: 8px;
                        font-size: 14px;
                    ",
                    "📏 メッセージ文字サイズ: {font_size.read()}px"
                }

                div {
                    style: "display: flex; align-items: center; gap: 12px;",

                    span {
                        style: "font-size: 12px; color: #666;",
                        "8px"
                    }

                    input {
                        r#type: "range",
                        min: "8",
                        max: "24",
                        value: "{font_size.read()}",
                        style: "
                            flex: 1;
                            -webkit-appearance: none;
                            appearance: none;
                            height: 6px;
                            background: #ddd;
                            border-radius: 3px;
                            outline: none;
                        ",
                        oninput: move |event| {
                            if let Ok(size) = event.value().parse::<u8>() {
                                let clamped_size = size.max(8).min(24);
                                save_font_size(clamped_size);
                            }
                        }
                    }

                    span {
                        style: "font-size: 12px; color: #666;",
                        "24px"
                    }
                }

                // プレビュー
                div {
                    style: "
                        margin-top: 12px;
                        padding: 8px 12px;
                        background: white;
                        border: 1px solid #e2e8f0;
                        border-radius: 4px;
                    ",

                    div {
                        style: "
                            font-size: {font_size.read()}px;
                            line-height: 1.4;
                            color: #1a202c;
                        ",
                        "プレビュー: これがチャットメッセージの表示サイズです"
                    }
                }
            }

            // 説明文
            div {
                style: "
                    background: #e8f4fd;
                    border: 1px solid #b8daff;
                    border-radius: 4px;
                    padding: 12px;
                    margin-top: 16px;
                ",
                p {
                    style: "margin: 0 0 8px 0; font-weight: bold; color: #0056b3;",
                    "💡 チャット表示について"
                }
                ul {
                    style: "margin: 0; padding-left: 20px;",
                    li { "文字サイズは即座に反映されます" }
                    li { "設定は自動的に保存され、次回起動時にも適用されます" }
                    li { "プレビューで実際の表示サイズを確認できます" }
                }
            }
        }
    }
}

/// チャットモニターコンテンツ
#[component]
fn ChatMonitorContent(
    live_chat_handle: crate::gui::hooks::LiveChatHandle,
    global_filter: Signal<crate::chat_management::MessageFilter>,
    active_tab: ActiveTab,
) -> Element {
    rsx! {
        div {
            class: "chat-monitor-content",
            style: "display: flex; flex-direction: column; height: 100%;",

            // コンテンツエリア - 配信最適化：上下分割レイアウト
            div {
                class: "content-body",
                style: "flex: 1; display: flex; flex-direction: column; gap: 3px; min-height: 0;",

                // 上部パネル（入力・ステータス）- 水平コンパクト配置
                div {
                    class: "top-panel",
                    style: "
                        flex: 0 0 auto;
                        display: flex;
                        gap: 2px;
                        max-height: 180px;
                        padding: 2px 0;
                        align-items: stretch;
                    ",

                    // 接続設定（左側）- 50%幅
                    div {
                        style: "flex: 1;",
                        crate::gui::components::input_section::CompactInputSection {
                            live_chat_handle: live_chat_handle.clone()
                        }
                    }

                    // ステータス（右側）- 50%幅
                    div {
                        style: "flex: 1;",
                        crate::gui::components::status_panel::CompactStatusPanel {
                            live_chat_handle: live_chat_handle.clone()
                        }
                    }
                }

                // メインメッセージエリア（全幅）- 配信最適化
                div {
                    class: "main-panel",
                    style: "
                        flex: 1;
                        min-height: 0;
                        background: linear-gradient(135deg, #f8fafc 0%, #e2e8f0 100%);
                        border-radius: 12px;
                        padding: 2px;
                        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
                        border: 2px solid rgba(102, 126, 234, 0.2);
                    ",

                    crate::gui::components::ChatDisplay {
                        live_chat_handle: live_chat_handle.clone(),
                        global_filter: global_filter,
                    }
                }
            }
        }
    }
}

/// 収益分析コンテンツ
#[component]
fn RevenueAnalyticsContent(live_chat_handle: crate::gui::hooks::LiveChatHandle) -> Element {
    // リアルタイムで更新される収益分析データ
    let mut analytics = use_signal(|| crate::analytics::RevenueAnalytics::default());

    // メッセージ変更時にリアルタイム更新
    use_effect(move || {
        let messages = live_chat_handle.messages.read();
        let mut revenue_analytics = crate::analytics::RevenueAnalytics::new();

        // 全メッセージを処理してリアルタイム統計を更新
        for message in messages.iter() {
            revenue_analytics.update_from_message(message);
        }

        let total_revenue = revenue_analytics.total_revenue(); // 事前に値を取得
        analytics.set(revenue_analytics);

        tracing::debug!(
            "💰 Revenue Analytics: Updated with {} messages, total revenue: ¥{:.0}",
            messages.len(),
            total_revenue
        );
    });

    rsx! {
        div {
            class: "revenue-analytics-content",

            // ヘッダー
            div {
                class: "content-header",
                style: "margin-bottom: 20px;",

                h2 {
                    style: "
                        font-size: 24px;
                        color: #333;
                        margin: 0 0 8px 0;
                        display: flex;
                        align-items: center;
                        gap: 12px;
                    ",
                    "💰 収益分析ダッシュボード"
                }

                p {
                    style: "
                        color: #666;
                        margin: 0;
                        font-size: 14px;
                    ",
                    "Super Chat収益とメンバーシップ統計をリアルタイム分析"
                }
            }

            // 収益ダッシュボードコンポーネントを統合
            crate::gui::components::RevenueDashboard {
                analytics: analytics
            }
        }
    }
}

/// データエクスポートコンテンツ
#[component]
fn DataExportContent(live_chat_handle: crate::gui::hooks::LiveChatHandle) -> Element {
    rsx! {
        div {
            class: "data-export-content",

            // ヘッダー
            div {
                class: "content-header",
                style: "margin-bottom: 20px;",

                h2 {
                    style: "
                        font-size: 24px;
                        color: #333;
                        margin: 0 0 8px 0;
                        display: flex;
                        align-items: center;
                        gap: 12px;
                    ",
                    "📥 Data Export"
                }

                p {
                    style: "
                        color: #666;
                        margin: 0;
                        font-size: 14px;
                    ",
                    "Export chat data in various formats (CSV, JSON, Excel)"
                }
            }

            // エクスポートパネルコンポーネントを統合
            crate::gui::components::ExportPanel {}
        }
    }
}

/// ハイライト設定コンポーネント
#[component]
fn HighlightSettings() -> Element {
    // ハイライト設定の状態
    let mut highlight_enabled = use_signal(|| true);
    let mut highlight_duration = use_signal(|| 8u64);
    let mut max_messages = use_signal(|| 20usize);
    let mut long_term_mode = use_signal(|| false);

    // 初期設定の読み込み
    use_effect({
        let mut highlight_enabled = highlight_enabled.clone();
        let mut highlight_duration = highlight_duration.clone();
        let mut max_messages = max_messages.clone();
        let mut long_term_mode = long_term_mode.clone();

        move || {
            spawn(async move {
                if let Ok(config_manager) =
                    crate::gui::unified_config::UnifiedConfigManager::new().await
                {
                    let config: Option<crate::gui::unified_config::HighlightConfig> =
                        config_manager
                            .get_typed_config("highlight")
                            .await
                            .unwrap_or(None);

                    let config = config.unwrap_or_default();
                    highlight_enabled.set(config.enabled);
                    highlight_duration.set(config.duration_seconds);
                    max_messages.set(config.max_messages);
                    long_term_mode.set(config.long_term_mode);
                }
            });
        }
    });

    // 設定を保存する関数
    let save_config = move |enabled: bool, duration: u64, max_msgs: usize, long_term: bool| {
        spawn(async move {
            if let Ok(config_manager) =
                crate::gui::unified_config::UnifiedConfigManager::new().await
            {
                let config = crate::gui::unified_config::HighlightConfig {
                    enabled,
                    duration_seconds: duration,
                    max_messages: max_msgs,
                    long_term_mode: long_term,
                    ..Default::default()
                };

                if let Err(e) = config_manager.set_typed_config("highlight", &config).await {
                    tracing::error!("Failed to save highlight config: {}", e);
                } else {
                    let _ = config_manager.flush_dirty_configs().await;
                    tracing::info!(
                        "🎯 [SETTINGS] Config saved: enabled={}, duration={}s, max_messages={}, long_term={}",
                        config.enabled,
                        config.duration_seconds,
                        config.max_messages,
                        config.long_term_mode
                    );
                }
            }
        });
    };

    rsx! {
        div {
            style: "
                background: #f8f9fa;
                border: 1px solid #e9ecef;
                border-radius: 8px;
                padding: 16px;
                margin-bottom: 20px;
            ",

            h3 {
                style: "
                    margin: 0 0 16px 0;
                    color: #495057;
                    display: flex;
                    align-items: center;
                    gap: 8px;
                ",
                "🎯 ハイライト設定"
            }

            // ハイライト機能のオン・オフ
            div {
                style: "margin-bottom: 20px; padding-bottom: 16px; border-bottom: 1px solid #dee2e6;",
                label {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 8px;
                        font-weight: 600;
                        color: #495057;
                        cursor: pointer;
                        font-size: 16px;
                    ",
                    input {
                        r#type: "checkbox",
                        checked: highlight_enabled(),
                        style: "width: 18px; height: 18px; accent-color: #0d6efd;",
                        onchange: move |evt| {
                            let enabled = evt.checked();
                            highlight_enabled.set(enabled);
                            save_config(enabled, *highlight_duration.read(), *max_messages.read(), *long_term_mode.read());
                        }
                    }
                    "ハイライト機能を有効化"
                }
                div {
                    style: "margin-top: 8px; color: #6c757d; font-size: 14px;",
                    if highlight_enabled() {
                        "新着メッセージを青色でハイライト表示します"
                    } else {
                        "ハイライト機能が無効です（設定は保持されます）"
                    }
                }
            }

            // ハイライト詳細設定（有効時のみ表示）
            if highlight_enabled() {
                div {
                    style: "opacity: 1; transition: opacity 0.3s ease;",

                    // ハイライト時間設定
                    div {
                        style: "margin-bottom: 16px;",
                        label {
                            style: "
                                display: block;
                                margin-bottom: 8px;
                                font-weight: 500;
                                color: #495057;
                            ",
                            "ハイライト表示時間（秒）"
                        }
                        input {
                            r#type: "number",
                            min: "3",
                            max: "30",
                            value: highlight_duration().to_string(),
                            style: "
                                width: 100px;
                                padding: 8px 12px;
                                border: 1px solid #ced4da;
                                border-radius: 4px;
                                font-size: 14px;
                            ",
                            oninput: move |evt| {
                                if let Ok(duration) = evt.value().parse::<u64>() {
                                    highlight_duration.set(duration);
                                    save_config(*highlight_enabled.read(), duration, *max_messages.read(), *long_term_mode.read());
                                }
                            }
                        }
                        span {
                            style: "margin-left: 8px; color: #6c757d; font-size: 14px;",
                            "（推奨: 5-15秒）"
                        }
                    }

                    // メッセージ数設定
                    div {
                        style: "margin-bottom: 16px;",
                        label {
                            style: "
                                display: block;
                                margin-bottom: 8px;
                                font-weight: 500;
                                color: #495057;
                            ",
                            "同時ハイライト最大メッセージ数"
                        }
                        input {
                            r#type: "number",
                            min: "5",
                            max: "50",
                            value: max_messages().to_string(),
                            style: "
                                width: 100px;
                                padding: 8px 12px;
                                border: 1px solid #ced4da;
                                border-radius: 4px;
                                font-size: 14px;
                            ",
                            oninput: move |evt| {
                                if let Ok(count) = evt.value().parse::<usize>() {
                                    max_messages.set(count);
                                    save_config(*highlight_enabled.read(), *highlight_duration.read(), count, *long_term_mode.read());
                                }
                            }
                        }
                        span {
                            style: "margin-left: 8px; color: #6c757d; font-size: 14px;",
                            "（推奨: 10-30個）"
                        }
                    }

                    // 長時間稼働モード設定
                    div {
                        style: "margin-top: 20px; padding-top: 16px; border-top: 1px solid #dee2e6;",
                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                font-weight: 600;
                                color: #495057;
                                cursor: pointer;
                                font-size: 14px;
                                margin-bottom: 8px;
                            ",
                            input {
                                r#type: "checkbox",
                                checked: long_term_mode(),
                                style: "width: 16px; height: 16px; accent-color: #28a745;",
                                onchange: move |evt| {
                                    let long_term = evt.checked();
                                    long_term_mode.set(long_term);
                                    save_config(*highlight_enabled.read(), *highlight_duration.read(), *max_messages.read(), long_term);
                                }
                            }
                            "🕐 長時間稼働モード"
                        }
                        div {
                            style: "
                                font-size: 12px; 
                                color: #6c757d; 
                                line-height: 1.4;
                                margin-left: 24px;
                            ",
                            if long_term_mode() {
                                "リソース使用量を抑制し、長時間の安定稼働を優先します"
                            } else {
                                "通常モード：応答性を重視したハイライト処理"
                            }
                        }
                    }

                    // 自動計算される補完設定の説明
                    div {
                        style: "
                            background: #f8f9fa;
                            border: 1px solid #e9ecef;
                            border-radius: 6px;
                            padding: 12px;
                            margin-top: 16px;
                        ",
                        div {
                            style: "
                                font-size: 13px;
                                color: #495057;
                                font-weight: 500;
                                margin-bottom: 6px;
                            ",
                            "🔧 統一処理システム"
                        }
                        div {
                            style: "font-size: 12px; color: #6c757d; line-height: 1.4;",
                            {
                                format!("処理間隔: {}ms、最大ハイライト: {}個",
                                    if long_term_mode() { 500 } else { 300 },
                                    if long_term_mode() { max_messages().min(10) } else { max_messages() }
                                )
                            }
                        }
                        div {
                            style: "font-size: 11px; color: #999; margin-top: 4px;",
                            if long_term_mode() {
                                "※ 長時間稼働モードで負荷を軽減し、安定性を向上"
                            } else {
                                "※ 通常モードで最適なパフォーマンス"
                            }
                        }
                    }
                }
            } else {
                div {
                    style: "
                        opacity: 0.6;
                        padding: 16px;
                        text-align: center;
                        color: #6c757d;
                        font-style: italic;
                    ",
                    "ハイライト機能を有効化すると詳細設定が表示されます"
                }
            }

            // 説明文
            div {
                style: "
                    background: #e8f4fd;
                    border: 1px solid #b8daff;
                    border-radius: 4px;
                    padding: 12px;
                    margin-top: 16px;
                ",
                p {
                    style: "margin: 0 0 8px 0; font-weight: bold; color: #0056b3;",
                    "💡 ハイライト機能について"
                }
                ul {
                    style: "margin: 0; padding-left: 20px;",
                    li { "新着メッセージを青色で一定時間ハイライト表示" }
                    li { "高速配信でも最新メッセージを確実に認識可能" }
                    li { "設定変更は即座に反映されます" }
                    li { "見逃し防止システムが自動で動作します" }
                }
            }
        }
    }
}

/// 自動保存設定コンポーネント
#[component]
fn AutoSaveSettings() -> Element {
    let mut app_state = use_context::<Signal<crate::gui::models::AppState>>();
    let current_state = app_state.read();

    rsx! {
        div {
            style: "
                background: #f8f9fa;
                border: 1px solid #e9ecef;
                border-radius: 8px;
                padding: 16px;
                margin-bottom: 20px;
            ",

            h3 {
                style: "
                    margin: 0 0 16px 0;
                    color: #495057;
                ",
                "📁 自動保存設定"
            }

            // 自動保存のオン・オフ
            div {
                style: "margin-bottom: 16px;",
                label {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 8px;
                        font-weight: 500;
                        color: #2d3748;
                        cursor: pointer;
                        font-size: 14px;
                    ",
                    input {
                        r#type: "checkbox",
                        checked: current_state.auto_save_enabled,
                        onchange: move |event| {
                            let enabled = event.checked();
                            app_state.with_mut(|state| {
                                state.auto_save_enabled = enabled;
                            });
                        }
                    }
                    "自動保存を有効化"
                }
            }
        }
    }
}

/// UI設定コンポーネント
#[component]
fn UiSettings() -> Element {
    let mut show_test_button = use_signal(|| false);

    rsx! {
        div {
            style: "
                background: #f8f9fa;
                border: 1px solid #e9ecef;
                border-radius: 8px;
                padding: 16px;
                margin-bottom: 20px;
            ",

            h3 {
                style: "
                    margin: 0 0 16px 0;
                    color: #495057;
                ",
                "🎛️ UI設定"
            }

            // テストボタン表示設定
            div {
                style: "margin-bottom: 16px;",
                label {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 8px;
                        font-weight: 500;
                        color: #2d3748;
                        cursor: pointer;
                        font-size: 14px;
                    ",
                    input {
                        r#type: "checkbox",
                        checked: show_test_button(),
                        style: "width: 16px; height: 16px; accent-color: #0d6efd;",
                        onchange: move |evt| {
                            show_test_button.set(evt.checked());
                        }
                    }
                    "テストボタンを表示"
                }
            }
        }
    }
}

/// Signal最適化設定コンポーネント
#[component]
fn SignalOptimizationSettings() -> Element {
    let mut analysis_report = use_signal(|| String::new());
    let mut show_report = use_signal(|| false);

    rsx! {
        div {
            style: "
                background: #f0f8ff;
                border: 1px solid #b0d4f1;
                border-radius: 8px;
                padding: 16px;
                margin-bottom: 20px;
            ",

            h3 {
                style: "
                    margin: 0 0 16px 0;
                    color: #1e40af;
                ",
                "📊 Signal最適化分析"
            }

            // 説明文
            div {
                style: "
                    background: #e0f2fe;
                    border: 1px solid #b3e5fc;
                    border-radius: 4px;
                    padding: 12px;
                    margin-bottom: 16px;
                ",
                p {
                    style: "margin: 0; font-size: 13px; color: #01579b; line-height: 1.4;",
                    "アプリケーション内のSignal使用状況を分析し、重複Signal検出や最適化推奨事項を提供します。"
                }
            }

            // 操作ボタン
            div {
                style: "display: flex; gap: 12px; align-items: center;",

                button {
                    style: "
                        padding: 8px 16px;
                        background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
                        color: white;
                        border: none;
                        border-radius: 6px;
                        cursor: pointer;
                        font-size: 14px;
                        font-weight: 500;
                    ",
                    onclick: move |_| {
                        spawn(async move {
                            let report = crate::gui::signal_optimizer::generate_signal_analysis_report();
                            analysis_report.set(report);
                            show_report.set(true);
                        });
                    },
                    "📊 分析レポート生成"
                }
            }
        }
    }
}
