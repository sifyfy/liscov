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
