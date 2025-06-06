use dioxus::prelude::*;

use crate::gui::{
    components::{TabContent, TabNavigation},
    hooks::use_live_chat,
    models::ActiveTab,
    styles::theme::get_embedded_css,
};

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

            // ヘッダー
            div {
                class: "app-header",
                style: "
                    text-align: center;
                    margin-bottom: 30px;
                    background: rgba(255, 255, 255, 0.1);
                    border-radius: 16px;
                    padding: 20px;
                    backdrop-filter: blur(10px);
                    border: 1px solid rgba(255, 255, 255, 0.2);
                ",

                h1 {
                    style: "
                        font-size: clamp(1.8rem, 5vw, 3rem);
                        color: white;
                        margin: 0 0 8px 0;
                        font-weight: 700;
                        text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
                        letter-spacing: -0.02em;
                    ",
                    "📺 liscov"
                }

                p {
                    style: "
                        color: rgba(255, 255, 255, 0.9);
                        margin: 0;
                        font-size: clamp(0.9rem, 2.5vw, 1.1rem);
                        font-weight: 400;
                    ",
                    "YouTube Live Chat Monitor - Advanced Analytics Edition"
                }

                // リアルタイム統計ボタン（オプション）
                button {
                    style: "
                        margin-top: 15px;
                        padding: 8px 20px;
                        background: rgba(255, 255, 255, 0.2);
                        color: white;
                        border: none;
                        border-radius: 20px;
                        font-size: 14px;
                        cursor: pointer;
                        transition: all 0.3s ease;
                        backdrop-filter: blur(5px);
                        border: 1px solid rgba(255, 255, 255, 0.3);
                    ",
                    onclick: move |_| {
                        tracing::info!("🔄 Real-time Analytics button clicked");
                    },
                    "📊 Real-time Analytics"
                }
            }

            // コンテンツエリア（タブナビゲーション + タブコンテンツ）
            div {
                style: "
                    flex: 1;
                    display: flex;
                    flex-direction: column;
                    overflow: hidden;
                ",

                // タブナビゲーション
                TabNavigation {
                    active_tab: active_tab(),
                    on_tab_change: move |new_tab| {
                        tracing::info!("🔄 Tab switched: {:?} → {:?}", active_tab(), new_tab);
                        active_tab.set(new_tab);
                    }
                }

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
