use dioxus::prelude::*;

use crate::gui::{
    components::TabContent, hooks::use_live_chat, models::ActiveTab,
    styles::theme::get_embedded_css,
};

/// 統合ヘッダー・タブナビゲーションコンポーネント
/// Phase 2: ヘッダーとタブを水平統合したレイアウト
#[component]
fn IntegratedHeaderTabs(active_tab: ActiveTab, on_tab_change: EventHandler<ActiveTab>) -> Element {
    let tabs = vec![
        ActiveTab::ChatMonitor,
        ActiveTab::RevenueAnalytics,
        ActiveTab::EngagementAnalytics,
        ActiveTab::DataExport,
    ];

    rsx! {
        div {
            class: "integrated-header-tabs",
            style: "
                display: flex;
                align-items: center;
                justify-content: space-between;
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                border-radius: 12px;
                padding: 8px 16px;
                margin-bottom: 16px;
                box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
                backdrop-filter: blur(10px);
                border: 1px solid rgba(255, 255, 255, 0.2);
                min-height: 56px;
            ",

            // 左側: アプリタイトル
            div {
                class: "app-title",
                style: "
                    display: flex;
                    align-items: center;
                    gap: 8px;
                    flex-shrink: 0;
                ",

                h1 {
                    style: "
                        font-size: clamp(1.1rem, 2.5vw, 1.4rem);
                        color: white;
                        margin: 0;
                        font-weight: 600;
                        text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
                        letter-spacing: -0.01em;
                    ",
                    "📺 liscov"
                }

                span {
                    style: "
                        color: rgba(255, 255, 255, 0.7);
                        font-size: clamp(0.7rem, 1.8vw, 0.85rem);
                        font-weight: 400;
                        margin-left: 4px;
                    ",
                    "Live Chat Monitor"
                }
            }

            // 右側: タブナビゲーション
            nav {
                class: "tab-navigation-integrated",
                style: "
                    display: flex;
                    gap: 4px;
                    flex-shrink: 0;
                ",

                // 各タブボタン
                for tab in tabs {
                    IntegratedTabButton {
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

/// 統合レイアウト用のタブボタンコンポーネント
#[component]
fn IntegratedTabButton(
    tab: ActiveTab,
    is_active: bool,
    on_click: EventHandler<MouseEvent>,
) -> Element {
    let button_style = if is_active {
        "
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 6px;
            padding: 8px 12px;
            border: none;
            border-radius: 6px;
            background: rgba(255, 255, 255, 0.95);
            color: #333;
            font-weight: 600;
            font-size: 12px;
            cursor: pointer;
            transition: all 0.3s ease;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
            min-width: 80px;
        "
    } else {
        "
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 6px;
            padding: 8px 12px;
            border: none;
            border-radius: 6px;
            background: rgba(255, 255, 255, 0.1);
            color: rgba(255, 255, 255, 0.8);
            font-weight: 500;
            font-size: 12px;
            cursor: pointer;
            transition: all 0.3s ease;
            min-width: 80px;
        "
    };

    rsx! {
        button {
            style: "{button_style}",
            onclick: on_click,

            // タブアイコン
            span {
                style: "font-size: 14px;",
                "{tab.icon()}"
            }

            // タブテキスト（レスポンシブ対応 - 小さい画面では非表示）
            span {
                style: "
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    max-width: 60px;
                ",
                "{tab.to_string()}"
            }
        }
    }
}

/// メインウィンドウコンポーネント（フィルタ永続化版）
/// Phase 1-2: タブシステム統合版
#[component]
pub fn MainWindow() -> Element {
    let live_chat_handle = use_live_chat();
    let mut active_tab = use_signal(|| ActiveTab::ChatMonitor);

    // フィルタ状態をアプリレベルで永続化
    let global_filter = use_signal(|| crate::chat_management::MessageFilter::new());

    // パフォーマンスモニターを完全無効化（CPU負荷軽減のため）
    // 起動時のパフォーマンス問題解決のため、すべてのモニター機能を無効化

    tracing::debug!(
        "🖥️ MainWindow: Rendering with active_tab={:?}",
        active_tab()
    );

    rsx! {
        // CSSスタイルをdocument headに注入
        document::Style {
            {get_embedded_css()}
        }

        div {
            class: "main-window",
            style: "
                min-height: 100vh;
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                padding: 20px;
                box-sizing: border-box;
                display: flex;
                flex-direction: column;
            ",

            // 統合ヘッダー・タブナビゲーション（Phase 2: 統合版）
            IntegratedHeaderTabs {
                active_tab: active_tab(),
                on_tab_change: move |new_tab| {
                    tracing::info!("🔄 Tab switched: {:?} → {:?}", active_tab(), new_tab);
                    active_tab.set(new_tab);
                }
            }

            // コンテンツエリア（タブコンテンツのみ）
            div {
                style: "
                    flex: 1;
                    display: flex;
                    flex-direction: column;
                    overflow: hidden;
                ",

                // タブコンテンツ（フィルタ永続化対応）
                div {
                    style: "
                        flex: 1;
                        overflow-y: auto;
                        overflow-x: hidden;
                    ",

                    TabContent {
                        active_tab: active_tab(),
                        live_chat_handle: live_chat_handle,
                        global_filter: global_filter,  // グローバルフィルタを渡す
                    }
                }
            }
        }
    }
}
