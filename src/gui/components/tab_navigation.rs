use crate::gui::models::ActiveTab;
use dioxus::prelude::*;

/// タブナビゲーションコンポーネント
#[component]
pub fn TabNavigation(active_tab: ActiveTab, on_tab_change: EventHandler<ActiveTab>) -> Element {
    let tabs = vec![
        ActiveTab::ChatMonitor,
        ActiveTab::RevenueAnalytics,
        ActiveTab::EngagementAnalytics,
        ActiveTab::DataExport,
    ];

    rsx! {
        nav {
            class: "tab-navigation",
            style: "
                display: flex;
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                border-radius: 12px;
                padding: 8px;
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
                        key: format!("{:?}", tab),
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
            onmouseenter: move |_| {
                // ホバー効果はCSSで実装
            },

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
    match active_tab {
        ActiveTab::ChatMonitor => rsx! {
            div {
                class: "tab-content chat-monitor",
                style: "
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    height: 100%;
                    display: flex;
                    flex-direction: column;
                ",

                // 従来のチャットモニター機能を統合（フィルタ永続化対応）
                ChatMonitorContent {
                    live_chat_handle: live_chat_handle,
                    global_filter: global_filter,
                }
            }
        },
        ActiveTab::RevenueAnalytics => rsx! {
            div {
                class: "tab-content revenue-analytics",
                style: "
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    height: 100%;
                    overflow-y: auto;
                ",

                RevenueAnalyticsContent {
                    live_chat_handle: live_chat_handle
                }
            }
        },
        ActiveTab::EngagementAnalytics => rsx! {
            div {
                class: "tab-content engagement-analytics",
                style: "
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    height: 100%;
                    overflow-y: auto;
                ",

                EngagementAnalyticsContent {
                    live_chat_handle: live_chat_handle
                }
            }
        },
        ActiveTab::DataExport => rsx! {
            div {
                class: "tab-content data-export",
                style: "
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    height: 100%;
                    overflow-y: auto;
                ",

                DataExportContent {
                    live_chat_handle: live_chat_handle
                }
            }
        },

        ActiveTab::Settings => rsx! {
            div {
                class: "tab-content settings",
                style: "
                    padding: 20px;
                    background: #fff;
                    border-radius: 12px;
                    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                    height: 100%;
                    overflow-y: auto;
                ",

                SettingsContent {}
            }
        },
    }
}

/// チャットモニターコンテンツ
#[component]
fn ChatMonitorContent(
    live_chat_handle: crate::gui::hooks::LiveChatHandle,
    global_filter: Signal<crate::chat_management::MessageFilter>,
) -> Element {
    // コンポーネント初期化時のみログ出力
    use_effect(move || {
        tracing::info!("🖥️ ChatMonitorContent component initialized");
    });

    // メッセージ数のログは削除（頻繁すぎるため）
    // デバッグが必要な場合のみ、下記をコメントアウト
    /*
    use_effect(move || {
        let message_count = live_chat_handle.messages.read().len();
        tracing::debug!(
            "🖥️ ChatMonitorContent: {} messages in handle",
            message_count
        );
    });
    */

    rsx! {
        div {
            class: "chat-monitor-content",
            style: "display: flex; flex-direction: column; height: 100%;",

            // ヘッダー
            div {
                class: "content-header",
                style: "margin-bottom: 20px; flex-shrink: 0;",

                div {
                    style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;",

                    h2 {
                        style: "
                            font-size: 24px;
                            color: #333;
                            margin: 0;
                            display: flex;
                            align-items: center;
                            gap: 12px;
                        ",
                        "💬 Live Chat Monitor"
                    }

                    // リアルタイム統計ボタン（ヘッダーから移動）
                    button {
                        style: "
                            padding: 6px 12px;
                            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                            color: white;
                            border: none;
                            border-radius: 8px;
                            font-size: 12px;
                            cursor: pointer;
                            transition: all 0.3s ease;
                            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
                        ",
                        onclick: move |_| {
                            tracing::info!("🔄 Real-time Analytics button clicked");
                        },
                        "📊 Real-time Analytics"
                    }
                }

                p {
                    style: "
                        color: #666;
                        margin: 0;
                        font-size: 14px;
                    ",
                    "Monitor real-time YouTube live chat messages"
                }
            }

            // コンテンツエリア
            div {
                class: "content-body",
                style: "flex: 1; display: flex; gap: 20px; min-height: 0;",

                // 左パネル（入力・ステータス）
                div {
                    class: "left-panel",
                    style: "flex: 0 0 350px; display: flex; flex-direction: column; gap: 20px;",

                    crate::gui::components::InputSection {
                        live_chat_handle: live_chat_handle.clone()
                    }

                    crate::gui::components::StatusPanel {
                        live_chat_handle: live_chat_handle.clone()
                    }
                }

                // 右パネル（元のChatDisplay）
                div {
                    class: "right-panel",
                    style: "flex: 1; min-height: 0;",

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

// 軽量なエンゲージメント指標構造体
#[derive(Debug, Clone, PartialEq, Default)]
struct LightEngagementStats {
    unique_users: usize,
    total_messages: usize,
    emoji_percentage: f64,
    questions_count: usize,
    avg_message_length: f64,
}

/// エンゲージメント分析コンテンツ
#[component]
fn EngagementAnalyticsContent(live_chat_handle: crate::gui::hooks::LiveChatHandle) -> Element {
    // 軽量な分析データを直接計算（重い処理を避ける）
    let engagement_stats = use_memo(use_reactive!(|live_chat_handle| {
        let messages = live_chat_handle.messages.read();

        if messages.is_empty() {
            return LightEngagementStats::default();
        }

        let unique_users = messages
            .iter()
            .map(|m| &m.channel_id)
            .collect::<std::collections::HashSet<_>>()
            .len();

        let total_messages = messages.len();

        // 基本的な絵文字検出のみ
        let emoji_messages = messages
            .iter()
            .filter(|m| {
                m.content.contains("😀")
                    || m.content.contains("😂")
                    || m.content.contains("❤")
                    || m.content.contains("👍")
                    || m.content.contains("🎉")
                    || m.content.contains("✨")
            })
            .count();

        let questions = messages
            .iter()
            .filter(|m| m.content.contains("？") || m.content.contains("?"))
            .count();

        let avg_length = messages
            .iter()
            .map(|m| m.content.chars().count())
            .sum::<usize>() as f64
            / total_messages as f64;

        let emoji_percentage = (emoji_messages as f64 / total_messages as f64) * 100.0;

        LightEngagementStats {
            unique_users,
            total_messages,
            emoji_percentage,
            questions_count: questions,
            avg_message_length: avg_length,
        }
    }));

    rsx! {
        div {
            class: "engagement-analytics-content",

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
                    "📊 Engagement Analytics"
                }

                p {
                    style: "
                        color: #666;
                        margin: 0;
                        font-size: 14px;
                    ",
                    "Track viewer engagement and activity patterns"
                }
            }

            // 軽量エンゲージメント統計表示
            div {
                style: "
                    display: grid;
                    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
                    gap: 20px;
                    margin-bottom: 30px;
                ",

                // ユニーク視聴者数
                div {
                    style: "
                        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                        border-radius: 12px;
                        padding: 20px;
                        color: white;
                        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
                    ",

                    h3 {
                        style: "margin: 0 0 8px 0; font-size: 14px; opacity: 0.9;",
                        "ユニーク視聴者"
                    }

                    div {
                        style: "font-size: 28px; font-weight: bold; margin: 0;",
                        "{engagement_stats.read().unique_users}"
                    }
                }

                // 総メッセージ数
                div {
                    style: "
                        background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
                        border-radius: 12px;
                        padding: 20px;
                        color: white;
                        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
                    ",

                    h3 {
                        style: "margin: 0 0 8px 0; font-size: 14px; opacity: 0.9;",
                        "総メッセージ数"
                    }

                    div {
                        style: "font-size: 28px; font-weight: bold; margin: 0;",
                        "{engagement_stats.read().total_messages}"
                    }
                }

                // 絵文字使用率
                div {
                    style: "
                        background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
                        border-radius: 12px;
                        padding: 20px;
                        color: white;
                        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
                    ",

                    h3 {
                        style: "margin: 0 0 8px 0; font-size: 14px; opacity: 0.9;",
                        "絵文字使用率"
                    }

                    div {
                        style: "font-size: 28px; font-weight: bold; margin: 0;",
                        {format!("{:.1}%", engagement_stats.read().emoji_percentage)}
                    }
                }

                // 質問数
                div {
                    style: "
                        background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
                        border-radius: 12px;
                        padding: 20px;
                        color: white;
                        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
                    ",

                    h3 {
                        style: "margin: 0 0 8px 0; font-size: 14px; opacity: 0.9;",
                        "質問数"
                    }

                    div {
                        style: "font-size: 28px; font-weight: bold; margin: 0;",
                        "{engagement_stats.read().questions_count}"
                    }
                }
            }

            // 追加統計情報
            div {
                style: "
                    background: white;
                    border-radius: 8px;
                    padding: 20px;
                    border: 1px solid #e0e0e0;
                ",

                h3 {
                    style: "margin: 0 0 15px 0; color: #333; font-size: 18px;",
                    "詳細統計"
                }

                div {
                    style: "
                        display: grid;
                        grid-template-columns: 1fr 1fr;
                        gap: 15px;
                        color: #666;
                    ",

                    div {
                        "平均メッセージ長: "
                        span {
                            style: "font-weight: bold; color: #333;",
                            {format!("{:.1} 文字", engagement_stats.read().avg_message_length)}
                        }
                    }

                    div {
                        "エンゲージメント率: "
                        span {
                            style: "font-weight: bold; color: #333;",
                            {format!("{:.1}%", if engagement_stats.read().total_messages > 0 {
                                engagement_stats.read().unique_users as f64 / engagement_stats.read().total_messages as f64 * 100.0
                            } else { 0.0 })}
                        }
                    }
                }
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

/// 設定画面コンテンツ
#[component]
fn SettingsContent() -> Element {
    let mut app_state = use_context::<Signal<crate::gui::models::AppState>>();

    rsx! {
        div {
            class: "settings-content",

            // ヘッダー
            div {
                class: "content-header",
                style: "margin-bottom: 30px;",

                h2 {
                    style: "
                        font-size: 28px;
                        color: #333;
                        margin: 0 0 8px 0;
                        display: flex;
                        align-items: center;
                        gap: 12px;
                    ",
                    "⚙️ Settings"
                }

                p {
                    style: "
                        color: #666;
                        margin: 0;
                        font-size: 16px;
                    ",
                    "Configure application settings and preferences."
                }
            }

            // 設定ファイル情報
            div {
                style: "
                    background: #f8f9fa;
                    border: 1px solid #e9ecef;
                    border-radius: 8px;
                    padding: 16px;
                    margin-bottom: 20px;
                ",

                div {
                    style: "
                        display: flex;
                        align-items: center;
                        justify-content: space-between;
                        margin-bottom: 12px;
                    ",

                    h3 {
                        style: "margin: 0; color: #495057;",
                        "📁 設定ファイル"
                    }

                    div {
                        style: "display: flex; gap: 8px;",

                        button {
                            style: "
                                padding: 6px 12px;
                                background: #28a745;
                                color: white;
                                border: none;
                                border-radius: 4px;
                                cursor: pointer;
                                font-size: 13px;
                                transition: background-color 0.2s;
                            ",
                            onclick: move |_| {
                                let state = app_state.read().clone();
                                use crate::gui::config_manager::save_app_state_async;
                                save_app_state_async(state);
                                tracing::info!("💾 Manual config save requested");
                            },
                            "💾 保存"
                        }

                        button {
                            style: "
                                padding: 6px 12px;
                                background: #ffc107;
                                color: #333;
                                border: none;
                                border-radius: 4px;
                                cursor: pointer;
                                font-size: 13px;
                                transition: background-color 0.2s;
                            ",
                            onclick: move |_| {
                                use crate::gui::config_manager::get_config_manager;
                                let config_manager = get_config_manager();
                                if let Ok(manager_guard) = config_manager.lock() {
                                    if let Err(e) = manager_guard.reset_config() {
                                        tracing::error!("❌ Failed to reset config: {}", e);
                                    } else {
                                        tracing::info!("🔄 Configuration reset to defaults");
                                        // AppStateをデフォルトにリセット
                                        let mut state = app_state.write();
                                        *state = crate::gui::models::AppState::default();
                                    }
                                }
                            },
                            "🔄 リセット"
                        }
                    }
                }

                div {
                    style: "
                        font-size: 13px;
                        color: #6c757d;
                        font-family: 'Courier New', monospace;
                        background: white;
                        padding: 8px;
                        border-radius: 4px;
                        border: 1px solid #dee2e6;
                        word-break: break-all;
                    ",
                    {
                        // 設定ファイルパスを表示
                        use crate::gui::config_manager::get_config_manager;
                        let path = if let Ok(manager_guard) = get_config_manager().lock() {
                            manager_guard.get_config_file_path().display().to_string()
                        } else {
                            "設定ファイルパスを取得できませんでした".to_string()
                        };
                        format!("📍 {}", path)
                    }
                }
            }

            // 自動保存設定
            AutoSaveSettings {}

            // 生レスポンス保存設定
            crate::gui::components::raw_response_settings::RawResponseSettings {}

            // 自動保存に関する説明
            div {
                style: "
                    background: #e3f2fd;
                    border: 1px solid #bbdefb;
                    border-radius: 8px;
                    padding: 16px;
                    margin-top: 20px;
                ",

                h4 {
                    style: "
                        margin: 0 0 8px 0;
                        color: #1976d2;
                        display: flex;
                        align-items: center;
                        gap: 8px;
                    ",
                    "💡 自動保存について"
                }

                ul {
                    style: "
                        margin: 0;
                        padding-left: 20px;
                        color: #1565c0;
                        line-height: 1.5;
                    ",
                    li { "自動保存は上記の設定で有効・無効を切り替えできます" }
                    li { "有効にすると、チャットメッセージがリアルタイムで指定ファイルに保存されます" }
                    li { "無効の場合、メッセージはメモリ内のみで管理され、エクスポート機能で保存できます" }
                    li { "設定はアプリケーション終了時に自動的に保存されます" }
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

    // 現在の設定値を状態として管理
    let mut auto_save_enabled = use_signal(|| current_state.auto_save_enabled);
    let mut output_file = use_signal(|| current_state.output_file.clone());

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
                        checked: auto_save_enabled(),
                        onchange: move |event| {
                            let enabled = event.value().parse::<bool>().unwrap_or(false);
                            auto_save_enabled.set(enabled);

                            // AppStateを更新
                            let mut state = app_state.write();
                            state.auto_save_enabled = enabled;

                            // 設定を永続化
                            use crate::gui::config_manager::save_app_state_async;
                            save_app_state_async(state.clone());

                            tracing::info!("💾 Auto save setting changed: {}", enabled);
                        }
                    }
                    "自動保存を有効化"
                }

                div {
                    style: "
                        color: #6c757d;
                        font-size: 12px;
                        margin-left: 24px;
                        margin-top: 4px;
                    ",
                    "有効にすると、チャットメッセージがリアルタイムで指定ファイルに保存されます"
                }
            }

                        // 出力ファイル設定（自動保存が有効な場合のみ表示）
            if auto_save_enabled() {
                div {
                    label {
                        style: "
                            display: block;
                            margin-bottom: 4px;
                            font-weight: 500;
                            color: #495057;
                            font-size: 14px;
                        ",
                        "出力ファイルパス:"
                    }

                    div {
                        style: "
                            display: flex;
                            gap: 8px;
                            align-items: center;
                        ",

                        input {
                            style: "
                                flex: 1;
                                padding: 8px 12px;
                                border: 1px solid #ced4da;
                                border-radius: 4px;
                                font-size: 14px;
                                background: white;
                                box-sizing: border-box;
                            ",
                            r#type: "text",
                            value: "{output_file}",
                            placeholder: "live_chat.ndjson",
                            oninput: move |event| {
                                let new_path = event.value();
                                output_file.set(new_path.clone());

                                // AppStateも更新
                                let mut state = app_state.write();
                                state.output_file = new_path;

                                // 設定を永続化
                                use crate::gui::config_manager::save_app_state_async;
                                save_app_state_async(state.clone());
                            }
                        }

                        button {
                            style: "
                                padding: 8px 16px;
                                background: #007bff;
                                color: white;
                                border: none;
                                border-radius: 4px;
                                cursor: pointer;
                                font-size: 14px;
                                white-space: nowrap;
                                transition: background-color 0.2s;
                            ",
                                                        onclick: move |_| {
                                // ファイル保存ダイアログを開く
                                let mut output_file_clone = output_file.clone();
                                let mut app_state_clone = app_state.clone();

                                // 現在のファイル名を取得
                                let current_filename = output_file_clone.read().to_string();

                                wasm_bindgen_futures::spawn_local(async move {
                                    use rfd::AsyncFileDialog;

                                    if let Some(file_handle) = AsyncFileDialog::new()
                                        .set_title("保存ファイルを選択")
                                        .add_filter("NDJSON", &["ndjson", "jsonl"])
                                        .add_filter("JSON", &["json"])
                                        .add_filter("すべてのファイル", &["*"])
                                        .set_file_name(&current_filename)
                                        .save_file()
                                        .await
                                    {
                                        let path = file_handle.path().to_string_lossy().to_string();
                                        output_file_clone.set(path.clone());

                                        // AppStateも更新
                                        let mut state = app_state_clone.write();
                                        state.output_file = path;

                                        // 設定を永続化
                                        use crate::gui::config_manager::save_app_state_async;
                                        save_app_state_async(state.clone());

                                        tracing::info!("📁 Output file path selected: {}", state.output_file);
                                    }
                                });
                            },
                            "📁 参照"
                        }
                    }

                    div {
                        style: "
                            color: #6c757d;
                            font-size: 12px;
                            margin-top: 4px;
                        ",
                        "💡 チャットメッセージがndjson形式で保存されます"
                    }
                }
            } else {
                div {
                    style: "
                        background: #fff3cd;
                        border: 1px solid #ffeaa7;
                        border-radius: 4px;
                        padding: 12px;
                        color: #856404;
                        font-size: 13px;
                    ",
                    "自動保存が無効です。メッセージはメモリ内のみで管理され、エクスポート機能で保存できます。"
                }
            }
        }
    }
}
