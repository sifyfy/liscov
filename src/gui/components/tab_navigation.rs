use crate::gui::models::ActiveTab;
use dioxus::prelude::*;

/// タブナビゲーションコンポーネント
#[component]
pub fn TabNavigation(active_tab: ActiveTab, on_tab_change: EventHandler<ActiveTab>) -> Element {
    let tabs = vec![
        ActiveTab::ChatMonitor,
        ActiveTab::RevenueAnalytics,
        ActiveTab::DataExport,
        ActiveTab::Settings,
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

/// タブコンテンツエリアコンポーネント（永続化版）
#[component]
pub fn TabContent(
    active_tab: ActiveTab,
    live_chat_handle: crate::gui::hooks::LiveChatHandle,
    global_filter: Signal<crate::chat_management::MessageFilter>,
) -> Element {
    // すべてのタブコンテンツを常に描画し、表示/非表示で切り替え
    // これによりコンポーネントの再作成とuse_effectの再実行を防止
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
                ", if active_tab == ActiveTab::ChatMonitor { "flex" } else { "none" }),

                ChatMonitorContent {
                    live_chat_handle: live_chat_handle.clone(),
                    global_filter: global_filter,
                    active_tab: active_tab,
                }
            }

            // Revenue Analytics タブ
            div {
                class: "tab-content revenue-analytics",
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
                ", if active_tab == ActiveTab::RevenueAnalytics { "block" } else { "none" }),

                RevenueAnalyticsContent {
                    live_chat_handle: live_chat_handle.clone()
                }
            }

            // Data Export タブ
            div {
                class: "tab-content data-export",
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
                ", if active_tab == ActiveTab::DataExport { "block" } else { "none" }),

                DataExportContent {
                    live_chat_handle: live_chat_handle.clone()
                }
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

/// チャットモニターコンテンツ
#[component]
fn ChatMonitorContent(
    live_chat_handle: crate::gui::hooks::LiveChatHandle,
    global_filter: Signal<crate::chat_management::MessageFilter>,
    active_tab: ActiveTab,
) -> Element {
    // コンポーネント初期化時のみログ出力（一度だけ実行）
    use_effect(move || {
        tracing::debug!("🖥️ ChatMonitorContent component initialized (persistent)");
    });

    // Chat Monitorタブの可視性変更を監視する自動スクロール処理
    use_effect(move || {
        spawn(async move {
            // MutationObserverを使ってタブの表示状態変更を監視
            let _ = dioxus::document::eval(
                r#"
                // 既存のオブザーバーがあれば削除
                if (window.liscovTabObserver) {
                    window.liscovTabObserver.disconnect();
                }
                
                console.log('🔄 [TAB-OBSERVER] Setting up visibility observer');
                
                // Chat Monitorタブのコンテナを監視
                const observerCallback = function(mutations) {
                    mutations.forEach(function(mutation) {
                        if (mutation.type === 'attributes' && mutation.attributeName === 'style') {
                            const target = mutation.target;
                            
                            // Chat Monitorタブのコンテナかチェック
                            if (target.classList.contains('tab-content') && 
                                target.classList.contains('chat-monitor')) {
                                
                                const displayStyle = window.getComputedStyle(target).display;
                                console.log('🔄 [TAB-OBSERVER] Display style changed to:', displayStyle);
                                
                                // flexに変わった（表示された）場合のみ自動スクロール処理を実行
                                if (displayStyle === 'flex') {
                                    console.log('✅ [TAB-OBSERVER] Chat Monitor became visible, checking auto-scroll');
                                    
                                    setTimeout(() => {
                                        const container = document.getElementById('liscov-message-list');
                                        if (!container) {
                                            console.log('❌ [TAB-OBSERVER] Container not found');
                                            return;
                                        }
                                        
                                        // 自動スクロールのチェックボックス状態を確認
                                        const autoScrollElements = document.querySelectorAll('input[type="checkbox"]');
                                        let isAutoScrollEnabled = false;
                                        
                                        for (let checkbox of autoScrollElements) {
                                            const parentLabel = checkbox.closest('label');
                                            if (parentLabel && parentLabel.textContent.includes('自動スクロール')) {
                                                isAutoScrollEnabled = checkbox.checked;
                                                console.log('🎯 [TAB-OBSERVER] Auto-scroll checkbox checked:', checkbox.checked);
                                                break;
                                            }
                                        }
                                        
                                        // ユーザーが手動スクロールしていないかチェック
                                        const userScrolled = window.liscovUserScrolled || false;
                                        console.log('👤 [TAB-OBSERVER] User scrolled state:', userScrolled);
                                        
                                        if (isAutoScrollEnabled && !userScrolled) {
                                            const oldScrollTop = container.scrollTop;
                                            const scrollHeight = container.scrollHeight;
                                            
                                            container.scrollTop = scrollHeight;
                                            
                                            setTimeout(() => {
                                                container.scrollTo({
                                                    top: scrollHeight,
                                                    behavior: 'smooth'
                                                });
                                            }, 50);
                                            
                                            console.log('✅ [TAB-OBSERVER] Auto-scroll executed on tab activation:', oldScrollTop, '->', scrollHeight);
                                        } else {
                                            console.log('⏭️ [TAB-OBSERVER] Auto-scroll skipped - enabled:', isAutoScrollEnabled, 'userScrolled:', userScrolled);
                                        }
                                    }, 150); // タブ切り替えアニメーション完了を待つ
                                }
                            }
                        }
                    });
                };
                
                // MutationObserverを作成・開始
                window.liscovTabObserver = new MutationObserver(observerCallback);
                
                // タブコンテンツコンテナが存在するまで待つ
                const waitForContainer = function() {
                    const tabContainer = document.querySelector('.tab-content-container');
                    if (tabContainer) {
                        // 全ての子要素（タブコンテンツ）を監視
                        const tabContents = tabContainer.querySelectorAll('.tab-content');
                        tabContents.forEach(function(tabContent) {
                            window.liscovTabObserver.observe(tabContent, {
                                attributes: true,
                                attributeFilter: ['style']
                            });
                        });
                        console.log('✅ [TAB-OBSERVER] Started observing', tabContents.length, 'tab contents');
                    } else {
                        // コンテナがまだない場合は少し待って再試行
                        setTimeout(waitForContainer, 100);
                    }
                };
                
                waitForContainer();
                "#,
            ).await;
        });
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

/// 設定画面コンテンツ
#[component]
fn SettingsContent() -> Element {
    let _app_state = use_context::<Signal<crate::gui::models::AppState>>();

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

            // ハイライト設定
            HighlightSettings {}

            // 自動保存設定
            AutoSaveSettings {}

            // UI設定
            UiSettings {}

            // レスポンス保存設定
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

/// ハイライト設定コンポーネント
#[component]
fn HighlightSettings() -> Element {
    // ハイライト設定の状態
    let mut highlight_enabled = use_signal(|| true);
    let mut highlight_duration = use_signal(|| 8u64);
    let mut max_messages = use_signal(|| 20usize);

    // 初期設定の読み込み
    use_effect({
        let mut highlight_enabled = highlight_enabled.clone();
        let mut highlight_duration = highlight_duration.clone();
        let mut max_messages = max_messages.clone();

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
                }
            });
        }
    });

    // 設定を保存する関数
    let save_config = move |enabled: bool, duration: u64, max: usize| {
        spawn(async move {
            if let Ok(config_manager) =
                crate::gui::unified_config::UnifiedConfigManager::new().await
            {
                let config = crate::gui::unified_config::HighlightConfig {
                    enabled,
                    duration_seconds: duration,
                    max_messages: max,
                };

                let _ = config_manager.set_typed_config("highlight", &config).await;
                let _ = config_manager.flush_dirty_configs().await;

                tracing::info!(
                    "🎯 [SETTINGS] Config saved: enabled={}, duration={}s, max_messages={}",
                    enabled,
                    duration,
                    max
                );
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
                            save_config(enabled, *highlight_duration.read(), *max_messages.read());
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
                                    save_config(*highlight_enabled.read(), duration, *max_messages.read());
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
                                    save_config(*highlight_enabled.read(), *highlight_duration.read(), count);
                                }
                            }
                        }
                        span {
                            style: "margin-left: 8px; color: #6c757d; font-size: 14px;",
                            "（推奨: 10-30個）"
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
                            "🔧 自動最適化設定"
                        }
                        div {
                            style: "font-size: 12px; color: #6c757d; line-height: 1.4;",
                            {
                                let backup_count = ((max_messages() as f32) * 0.5).ceil() as usize;
                                format!("補完システム: 最大{}個、チェック間隔500ms", backup_count)
                            }
                        }
                        div {
                            style: "font-size: 11px; color: #999; margin-top: 4px;",
                            "※ 見逃し防止のバックアップシステムが自動で動作します"
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

/// UI設定コンポーネント
#[component]
fn UiSettings() -> Element {
    // テストボタン表示設定の状態
    let mut show_test_button = use_signal(|| false);

    // 初期設定の読み込み
    use_effect({
        let mut show_test_button = show_test_button.clone();

        move || {
            spawn(async move {
                if let Ok(config_manager) =
                    crate::gui::unified_config::UnifiedConfigManager::new().await
                {
                    let test_button_visible: Option<bool> = config_manager
                        .get_typed_config("ui.show_test_button")
                        .await
                        .unwrap_or(None);

                    show_test_button.set(test_button_visible.unwrap_or(false));
                }
            });
        }
    });

    // 設定を保存する関数
    let save_config = move |show_test: bool| {
        spawn(async move {
            if let Ok(config_manager) =
                crate::gui::unified_config::UnifiedConfigManager::new().await
            {
                let _ = config_manager
                    .set_typed_config("ui.show_test_button", &show_test)
                    .await;
                let _ = config_manager.flush_dirty_configs().await;

                tracing::info!(
                    "🎛️ [UI SETTINGS] Test button visibility saved: {}",
                    show_test
                );
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
                            let enabled = evt.checked();
                            show_test_button.set(enabled);
                            save_config(enabled);
                        }
                    }
                    "テストボタンを表示"
                }

                div {
                    style: "
                        color: #6c757d;
                        font-size: 12px;
                        margin-left: 24px;
                        margin-top: 4px;
                    ",
                    if show_test_button() {
                        "チャットフッターにテストメッセージ追加ボタンが表示されます"
                    } else {
                        "テストボタンが非表示になります（トラブル時の動作確認用）"
                    }
                }
            }

            // 説明文
            div {
                style: "
                    background: #fff3cd;
                    border: 1px solid #ffeaa7;
                    border-radius: 4px;
                    padding: 12px;
                    margin-top: 16px;
                ",
                p {
                    style: "margin: 0 0 8px 0; font-weight: bold; color: #856404;",
                    "💡 テストボタンについて"
                }
                ul {
                    style: "margin: 0; padding-left: 20px; color: #856404;",
                    li { "開発時やトラブル時の動作確認に使用" }
                    li { "一般的な利用には不要なため、デフォルトは非表示" }
                    li { "有効にするとチャットフッターに「🧪 テスト」ボタンが表示" }
                    li { "設定変更は即座に反映されます" }
                }
            }
        }
    }
}
