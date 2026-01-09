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
        ActiveTab::DataExport,
        ActiveTab::ViewerManagement, // 視聴者管理タブ
        ActiveTab::Settings,
        ActiveTab::SignalAnalysis,
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
                padding: 6px 12px;
                margin-bottom: 6px;
                box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
                backdrop-filter: blur(10px);
                border: 1px solid rgba(255, 255, 255, 0.2);
                min-height: 52px;
            ",

            // 左側: アクティブタブ情報
            div {
                class: "active-tab-info",
                style: "
                    display: flex;
                    align-items: center;
                    gap: 12px;
                    flex: 1;
                    min-width: 0;
                ",

                // タブアイコン（大きめ）
                div {
                    style: "
                        font-size: 32px;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        width: 48px;
                        height: 48px;
                        background: rgba(255, 255, 255, 0.15);
                        border-radius: 12px;
                        backdrop-filter: blur(10px);
                        border: 1px solid rgba(255, 255, 255, 0.2);
                        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
                    ",
                    "{active_tab.icon()}"
                }

                // タブ情報テキスト
                div {
                    style: "
                        display: flex;
                        flex-direction: column;
                        gap: 2px;
                        flex: 1;
                        min-width: 0;
                    ",

                    // タブ名
                    h1 {
                        style: "
                            font-size: clamp(1.2rem, 2.8vw, 1.6rem);
                            color: white;
                            margin: 0;
                            font-weight: 700;
                            text-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
                            letter-spacing: -0.02em;
                            line-height: 1.1;
                        ",
                        "{active_tab.to_string()}"
                    }

                    // タブ説明
                    p {
                        style: "
                            color: rgba(255, 255, 255, 0.8);
                            font-size: clamp(0.75rem, 1.6vw, 0.9rem);
                            font-weight: 400;
                            margin: 0;
                            line-height: 1.3;
                            text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
                        ",
                        "{active_tab.description()}"
                    }
                }
            }

            // 右側: タブナビゲーション
            nav {
                class: "tab-navigation-integrated",
                style: "
                    display: flex;
                    gap: 3px;
                    flex-shrink: 0;
                    background: rgba(255, 255, 255, 0.1);
                    border-radius: 10px;
                    padding: 4px;
                    backdrop-filter: blur(10px);
                    border: 1px solid rgba(255, 255, 255, 0.15);
                ",

                // 各タブボタン
                for tab in tabs {
                    IntegratedTabButton {
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
            gap: 4px;
            padding: 6px 10px;
            border: none;
            border-radius: 7px;
            background: rgba(255, 255, 255, 0.95);
            color: #333;
            font-weight: 700;
            font-size: 11px;
            cursor: pointer;
            transition: all 0.3s ease;
            box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
            min-width: 70px;
            transform: translateY(-1px);
        "
    } else {
        "
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 4px;
            padding: 6px 10px;
            border: none;
            border-radius: 7px;
            background: transparent;
            color: rgba(255, 255, 255, 0.7);
            font-weight: 500;
            font-size: 11px;
            cursor: pointer;
            transition: all 0.3s ease;
            min-width: 70px;
        "
    };

    rsx! {
        button {
            style: "{button_style}",
            onclick: on_click,

            // タブアイコン
            span {
                style: "font-size: 12px;",
                "{tab.icon()}"
            }

            // タブテキスト（レスポンシブ対応 - 小さい画面では非表示）
            span {
                style: "
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    max-width: 50px;
                    font-size: 10px;
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

    // AppStateコンテキストを提供（設定画面で使用）
    let app_state = use_signal(|| {
        // 設定ファイルから初期状態を読み込み
        let config_manager = crate::gui::config_manager::get_config_manager();
        if let Ok(manager_guard) = config_manager.lock() {
            if let Ok(config) = manager_guard.load_config() {
                let mut state = crate::gui::models::AppState::default();
                manager_guard.apply_to_app_state(&config, &mut state);

                // 起動時にURLは常にクリアする（前回のURLを残さない）
                state.url = String::new();

                tracing::info!("✅ Configuration loaded and applied to AppState");
                return state;
            }
        }
        tracing::warn!("⚠️ Failed to load configuration, using defaults");
        crate::gui::models::AppState::default()
    });

    // パフォーマンスモニターを完全無効化（CPU負荷軽減のため）
    // 起動時のパフォーマンス問題解決のため、すべてのモニター機能を無効化

    tracing::debug!(
        "🖥️ MainWindow: Rendering with active_tab={:?}",
        active_tab()
    );

    // AppStateコンテキストを提供
    use_context_provider(|| app_state.clone());

    // アプリケーション終了時の処理
    use_drop(move || {
        let state = app_state.read().clone();
        tokio::spawn(async move {
            // TTS終了処理
            crate::gui::tts_manager::shutdown_tts().await;

            // 設定を自動保存
            use crate::gui::config_manager::save_app_state_async;
            save_app_state_async(state);
            tracing::info!("💾 Configuration auto-saved on application exit");
        });
    });

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
                padding: 4px;
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
