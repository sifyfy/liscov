use crate::chat_management::MessageFilter;
use crate::gui::components::{FilterPanel, ViewerInfoPanel};
use crate::gui::dom_controller::utils::create_chat_controller; // Phase 3.2
use crate::gui::hooks::use_live_chat::LiveChatHandle;
use crate::gui::models::SelectedViewer;
use crate::gui::performance_monitor::{record_performance_event, PerformanceEventType}; // Phase 5.2
use crate::gui::signal_optimizer::{process_batch_updates, queue_batch_update, BatchUpdateType}; // Phase 4.2
use crate::gui::signal_optimizer::{record_signal_update, register_signal, SignalType}; // Phase 4.1
use crate::gui::styles::theme::CssClasses;

// Message streaming integration
use crate::gui::message_stream::{DisplayLimit, MessageStream, MessageStreamConfig};
use crate::gui::models::{get_currency_name_ja, GuiChatMessage, MessageRun, MessageType};

// Phase 4.3: クロージャ最適化
use crate::gui::closure_optimizer::{
    get_optimized_signal_handler, perform_periodic_cleanup, record_closure_creation,
};

use dioxus::prelude::*;

/// アーカイブ検索の種類
#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveSearchType {
    /// 内容で検索
    Content,
    /// 投稿者で検索
    Author,
}

/// メッセージ表示エリア
///
/// Phase 4.1実装: Signal最適化統合
/// - Signal依存関係分析
/// - 重複Signal検出
/// - パフォーマンス最適化
#[component]
pub fn ChatDisplay(
    live_chat_handle: LiveChatHandle,
    global_filter: Signal<MessageFilter>, // グローバルフィルタ追加
) -> Element {
    // AppStateからチャット表示設定を取得
    let app_state = use_context::<Signal<crate::gui::models::AppState>>();
    let _chat_config = app_state.read().chat_display_config.clone();

    // MessageStream初期化（新規追加）
    let message_stream = use_signal(|| {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Unlimited, // デフォルト無制限
            max_display_count: usize::MAX,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };
        MessageStream::new(config)
    });

    // MessageStream統計表示用
    let stream_stats = use_signal(|| message_stream.read().stats());

    // MessageStreamに同期済みのメッセージ数を追跡
    let synced_message_count = use_signal(|| 0usize);

    // 基本状態の初期化
    let mut show_filter_panel = use_signal(|| false);
    let last_message_count = use_signal(|| 0usize);
    let _last_effect_time = use_signal(|| std::time::Instant::now()); // 未使用

    // MessageStream連携：アーカイブ検索機能の追加
    let show_archive_search = use_signal(|| false);
    let mut search_query = use_signal(|| String::new());
    let mut search_type = use_signal(|| ArchiveSearchType::Content);
    let search_results = use_signal(|| Vec::<GuiChatMessage>::new());
    let is_searching = use_signal(|| false);

    // 視聴者情報パネル用の状態
    let mut selected_viewer = use_signal(|| None::<SelectedViewer>);
    let mut show_viewer_panel = use_signal(|| false);

    // 最適化版：統合設定Signalで4回のAppStateアクセスを1回に削減
    let chat_config = use_memo(move || app_state.read().chat_display_config.clone());

    // 個別設定値は統合設定から派生（再レンダリング最小化）
    let auto_scroll_enabled = use_memo(move || chat_config.read().auto_scroll_enabled);
    let show_timestamps = use_memo(move || chat_config.read().show_timestamps);
    let message_font_size = use_memo(move || chat_config.read().message_font_size);

    // 🎯 Phase 2.4: イベント駆動型MessageStream同期（ポーリング廃止）
    // LiveChatHandleのシグナルを監視し、変更時にMessageStreamを更新
    // 修正: 複数メッセージが同時に到着した場合も全て同期する
    use_effect({
        let live_chat_handle = live_chat_handle.clone();
        let mut message_stream = message_stream.clone();
        let mut stream_stats = stream_stats.clone();
        let mut synced_message_count = synced_message_count.clone();

        move || {
            // message_added_eventシグナルの変更を監視
            let event_count = (live_chat_handle.message_added_event)();
            let messages = live_chat_handle.messages.read();
            let current_message_count = messages.len();
            let already_synced = *synced_message_count.read();

            tracing::debug!(
                "🔄 [EVENT_CHAT_SYNC] Event triggered: event_count={}, message_count={}, synced={}",
                event_count,
                current_message_count,
                already_synced
            );

            // 未同期のメッセージがある場合、全て同期
            if current_message_count > already_synced {
                let new_messages_count = current_message_count - already_synced;

                message_stream.with_mut(|stream| {
                    // 新しいメッセージのみをMessageStreamに追加
                    for msg in messages.iter().skip(already_synced) {
                        stream.push_message(msg.clone());
                    }
                });

                // 同期済みカウントを更新
                synced_message_count.set(current_message_count);

                // 統計情報を更新
                stream_stats.set(message_stream.read().stats());

                tracing::debug!(
                    "📦 [EVENT_CHAT_SYNC] MessageStream updated: {} new messages, display={}, archived={}",
                    new_messages_count,
                    message_stream.read().display_count(),
                    message_stream.read().archived_count()
                );
            }
        }
    });

    // メッセージクリア検出（シグナルベース）
    use_effect({
        let live_chat_handle = live_chat_handle.clone();
        let mut message_stream = message_stream.clone();
        let mut stream_stats = stream_stats.clone();
        let mut synced_message_count = synced_message_count.clone();

        move || {
            let messages = live_chat_handle.messages.read();

            // メッセージがクリアされた場合の処理
            if messages.is_empty() && message_stream.read().total_count() > 0 {
                tracing::info!("🗑️ [EVENT_CHAT_SYNC] Messages cleared, resetting MessageStream");
                message_stream.with_mut(|stream| stream.clear());
                stream_stats.set(message_stream.read().stats());
                synced_message_count.set(0);
            }
        }
    });

    // 🚀 **Dioxus memo_chain最適化**: 効率的なフィルタリング処理
    // Step 1: 差分更新システム連携 - 新着メッセージのフィルタリング
    let _new_filtered_message = use_memo({
        let _live_chat_handle = live_chat_handle.clone();
        let global_filter = global_filter.clone();
        move || {
            if let Some(new_msg) = live_chat_handle.new_message.read().as_ref() {
                let filter = global_filter.read();
                if filter.matches(new_msg) {
                    Some(new_msg.clone())
                } else {
                    None
                }
            } else {
                None
            }
        }
    });

    // Step 2: 全メッセージフィルタリング（必要時のみ）
    let filtered_messages = use_memo({
        let message_stream = message_stream.clone();
        let global_filter = global_filter.clone();
        let _trigger = live_chat_handle.message_added_event; // Signal依存関係
        move || {
            let display_messages = message_stream.read().display_messages();
            let filter = global_filter.read();
            filter.filter_messages(&display_messages)
        }
    });

    // 初期設定の読み込み
    use_effect({
        let mut app_state = app_state.clone();

        move || {
            spawn(async move {
                if let Ok(config_manager) =
                    crate::gui::unified_config::UnifiedConfigManager::new().await
                {
                    let config: Option<crate::gui::unified_config::ChatDisplayConfig> =
                        config_manager
                            .get_typed_config("chat_display")
                            .await
                            .unwrap_or(None);

                    let config = config.unwrap_or_default();

                    // AppStateを更新
                    app_state.with_mut(|state| {
                        state.chat_display_config = config.clone();
                    });

                    tracing::info!(
                        "💬 [CHAT DISPLAY] Settings loaded: font_size={}px",
                        config.message_font_size
                    );
                }
            });
        }
    });

    // アーカイブ検索機能
    use_effect({
        let message_stream = message_stream.clone();
        let search_query = search_query.clone();
        let search_type = search_type.clone();
        let mut search_results = search_results.clone();
        let mut is_searching = is_searching.clone();

        move || {
            let query = search_query.read().clone();
            let search_type_val = search_type.read().clone();

            if query.len() >= 2 && message_stream.read().config().archive_search_enabled {
                is_searching.set(true);

                spawn(async move {
                    // 検索実行（非同期）
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    let results: Vec<GuiChatMessage> = {
                        let stream = message_stream.read();
                        match search_type_val {
                            ArchiveSearchType::Content => stream
                                .search_by_content(&query)
                                .into_iter()
                                .cloned()
                                .collect(),
                            ArchiveSearchType::Author => stream
                                .search_by_author(&query)
                                .into_iter()
                                .cloned()
                                .collect(),
                        }
                    };

                    search_results.set(results.clone());
                    is_searching.set(false);

                    tracing::info!(
                        "🔍 [ARCHIVE SEARCH] Found {} results for '{query}' (type: {:?})",
                        results.len(),
                        search_type_val
                    );
                });
            } else if query.is_empty() {
                search_results.set(Vec::new());
                is_searching.set(false);
            }
        }
    });

    // 最適化版：Signal登録とクロージャ最適化を統合初期化
    use_effect(move || {
        // Signal登録（Phase 4.1）
        register_signal(
            "chat_auto_scroll_enabled",
            SignalType::AutoScrollEnabled,
            "ChatDisplay",
        );
        register_signal(
            "chat_show_timestamps",
            SignalType::ShowTimestamps,
            "ChatDisplay",
        );
        register_signal(
            "chat_show_filter_panel",
            SignalType::ShowFilterPanel,
            "ChatDisplay",
        );
        register_signal(
            "chat_last_message_count",
            SignalType::LastMessageCount,
            "ChatDisplay",
        );
        register_signal(
            "chat_message_font_size",
            SignalType::MessageFontSize,
            "ChatDisplay",
        );

        // クロージャ最適化初期化（Phase 4.3）
        record_closure_creation();

        // 定期的なクリーンアップを開始
        spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                perform_periodic_cleanup();
            }
        });

        tracing::info!("📊 [SIGNAL] ChatDisplay optimization systems initialized");
    });

    // Phase 4.3: 最適化されたハンドラー関数群（簡略版）
    let create_optimized_handler = |signal_name: &str| {
        record_closure_creation();
        get_optimized_signal_handler(signal_name, "ChatDisplay")
    };

    // Phase 3.2: DOM制御モジュール（各場所で直接作成に変更）

    // 🚀 無限ループ回避版：メッセージカウント監視
    use_effect({
        let _live_chat_handle = live_chat_handle.clone();
        let mut last_message_count = last_message_count.clone();

        move || {
            // 差分更新イベント監視（無限ループ回避）
            let _event_trigger = live_chat_handle.message_added_event;
            let current_count = live_chat_handle.messages.read().len();

            last_message_count.set(current_count);

            tracing::debug!(
                "📨 [ChatDisplay] Display messages: {} (+1 new)",
                current_count
            );

            // Phase 4.2: 新着メッセージ時のBatch処理スクロール
            // auto_scroll_enabled のみで制御（user_has_scrolled はボタン表示用）
            if auto_scroll_enabled() {
                queue_batch_update("chat_scroll", BatchUpdateType::DomUpdate);

                spawn(async move {
                    record_performance_event(PerformanceEventType::BatchProcessing, "ChatDisplay");

                    let processed = process_batch_updates().await;
                    if processed > 0 {
                        tracing::debug!(
                            "📦 [BATCH] Processed {} updates including scroll",
                            processed
                        );
                    }

                    record_performance_event(PerformanceEventType::DomOperation, "ChatDisplay");

                    let controller = create_chat_controller("liscov-message-list");
                    if let Err(e) = controller.scroll_to_bottom(false).await {
                        tracing::debug!("📜 [DOM] Fallback scroll skipped: {}", e);
                    }
                });
            }
        }
    });

    // Phase 3.2: DOM操作（DomController版）
    // スクロール位置監視はonscrollイベントで実施（eval recvが機能しないため）
    use_effect({
        let auto_scroll_enabled = auto_scroll_enabled.clone();

        move || {
            spawn(async move {
                // DOM初期化（100ms待機）
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Phase 3.2: 高精度DOM制御初期化
                let mut controller = create_chat_controller("liscov-message-list");
                if let Err(e) = controller.initialize().await {
                    tracing::error!("🎮 [DOM] Initialization failed: {}", e);
                    return;
                }

                // グローバル変数を初期化
                let init_js = r#"
                    window.liscovScrollDistance = 0;
                    window.liscovIsFarFromBottom = false;
                "#;
                let _ = document::eval(init_js);

                tracing::info!("🎮 [DOM] Phase 3.2 Controller ready");

                // 定期的な自動スクロール
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    // 自動スクロール（auto_scroll_enabled のみで制御）
                    if auto_scroll_enabled() {
                        // Phase 3.2: 高精度自動スクロール
                        if let Err(e) = controller.scroll_to_bottom(false).await {
                            tracing::debug!("📜 [DOM] Auto-scroll skipped: {}", e);
                        }
                    }
                }
            });
        }
    });

    rsx! {
        div {
            class: CssClasses::CHAT_DISPLAY,
            style: "
                height: 100%;
                display: flex;
                flex-direction: column;
                overflow: hidden;
            ",

            // チャット制御
            div {
                    class: CssClasses::CHAT_CONTROLS,
                    style: "
                        display: flex;
                        gap: 8px !important;
                        align-items: center;
                    ",

                    // フィルターボタン
                    button {
                        class: if *show_filter_panel.read() {
                            "px-2 py-1 bg-blue-600 text-white rounded text-xs"
                        } else {
                            "px-2 py-1 bg-blue-500 hover:bg-blue-600 text-white rounded text-xs"
                        },
                        style: "font-size: 11px; min-height: 26px;",
                                            onclick: {
                            let optimized_handler = create_optimized_handler("chat_show_filter_panel");
                            move |_| {
                            let current_value = *show_filter_panel.read();
                            show_filter_panel.set(!current_value);

                            // Phase 4.1: Signal更新記録
                            record_signal_update("chat_show_filter_panel");

                            // Phase 4.2: UI更新をBatch処理
                            queue_batch_update("chat_show_filter_panel", BatchUpdateType::Normal);

                            // Phase 5.2: UI再描画パフォーマンス監視
                            record_performance_event(PerformanceEventType::UiRedraw, "ChatDisplay");

                                // Phase 4.3: 最適化されたハンドラー実行
                                optimized_handler();
                            }
                        },
                        if global_filter.read().is_active() {
                            "🔍 フィルター ({global_filter.read().active_filter_count()})"
                        } else {
                            "🔍 フィルター"
                        }
                    }

                    // アーカイブ検索ボタン（MessageStream機能）
                    if message_stream.read().config().archive_search_enabled && message_stream.read().archived_count() > 0 {
                        button {
                            class: if *show_archive_search.read() {
                                "px-2 py-1 bg-purple-600 text-white rounded text-xs"
                            } else {
                                "px-2 py-1 bg-purple-500 hover:bg-purple-600 text-white rounded text-xs"
                            },
                            style: "font-size: 11px; min-height: 26px;",
                            onclick: {
                                let mut show_archive_search = show_archive_search.clone();
                                move |_| {
                                    let current_value = *show_archive_search.read();
                                    show_archive_search.set(!current_value);

                                    record_signal_update("chat_show_archive_search");
                                    queue_batch_update("chat_show_archive_search", BatchUpdateType::Normal);
                                }
                            },
                            if search_results.read().is_empty() {
                                "📚 アーカイブ検索"
                            } else {
                                "📚 検索 ({search_results.read().len()})"
                            }
                        }
                    }

                    // 最新に戻るボタン（常に表示）
                    button {
                        class: "px-2 py-1 bg-green-500 hover:bg-green-600 text-white rounded text-xs ml-1",
                        style: "font-size: 11px; min-height: 26px;",
                        onclick: {
                            let mut app_state = app_state.clone();
                            move |_| {
                                // 自動スクロールをONにして最新にスクロール
                                app_state.with_mut(|state| {
                                    state.chat_display_config.auto_scroll_enabled = true;
                                });

                                // Phase 4.1: Signal更新記録
                                record_signal_update("chat_auto_scroll_enabled");

                                // Phase 4.2: スクロール状態更新をBatch処理
                                queue_batch_update("chat_auto_scroll_enabled", BatchUpdateType::HighPriority);

                                spawn(async move {
                                    // Phase 3.2: DomController使用
                                    let controller = create_chat_controller("liscov-message-list");
                                    if let Err(e) = controller.scroll_to_bottom(true).await {
                                        tracing::warn!("📜 [DOM] Force scroll failed: {}", e);
                                    }
                                });
                            }
                        },
                        "📍 最新に戻る"
                    }

                    // 自動スクロール切り替え
                    label {
                        class: CssClasses::CHECKBOX_LABEL,
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 4px !important;
                            font-size: 12px !important;
                            color: #4a5568;
                            cursor: pointer;
                            user-select: none;
                        ",
                        input {
                            r#type: "checkbox",
                            checked: auto_scroll_enabled(),
                            onchange: {
                                let mut app_state = app_state.clone();
                                move |event: dioxus::events::FormEvent| {
                                    app_state.with_mut(|state| {
                                        state.chat_display_config.auto_scroll_enabled = event.checked();
                                    });
                                    record_signal_update("chat_auto_scroll_enabled");
                                    queue_batch_update("chat_auto_scroll_enabled", BatchUpdateType::Normal);
                                }
                            },
                            style: "width: 14px; height: 14px;",
                        }
                        "自動スクロール"
                    }

                    // タイムスタンプ表示切り替え
                    label {
                        class: CssClasses::CHECKBOX_LABEL,
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 4px !important;
                            font-size: 12px !important;
                            color: #4a5568;
                            cursor: pointer;
                            user-select: none;
                        ",
                        input {
                            r#type: "checkbox",
                            checked: show_timestamps(),
                            onchange: {
                                let mut app_state = app_state.clone();
                                move |event: dioxus::events::FormEvent| {
                                    app_state.with_mut(|state| {
                                        state.chat_display_config.show_timestamps = event.checked();
                                    });
                                    record_signal_update("chat_show_timestamps");
                                    queue_batch_update("chat_show_timestamps", BatchUpdateType::Normal);
                                }
                            },
                            style: "width: 14px; height: 14px;",
                        }
                        "タイムスタンプ"
                    }

                    // MessageStream設定グループ
                    div {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 8px !important;
                            font-size: 12px !important;
                            color: #4a5568;
                            background: #f0f9ff;
                            padding: 4px 8px;
                            border-radius: 4px;
                            border: 1px solid #bae6fd;
                        ",

                        // 表示件数設定
                        div {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 4px !important;
                            ",
                            span { "表示:" }
                            select {
                                style: "
                                    font-size: 11px;
                                    padding: 2px 4px;
                                    border: 1px solid #cbd5e0;
                                    border-radius: 3px;
                                    background: white;
                                ",
                                value: {
                                    match message_stream.read().config().display_limit {
                                        DisplayLimit::Fixed(count) => count.to_string(),
                                        DisplayLimit::Unlimited => "999999".to_string(),
                                        _ => "100".to_string(),
                                    }
                                },
                                onchange: {
                                    let mut message_stream = message_stream.clone();
                                    let mut stream_stats = stream_stats.clone();

                                    move |event: dioxus::events::FormEvent| {
                                        if let Ok(count) = event.value().parse::<usize>() {
                                            tracing::info!(
                                                "🔧 [MessageStream] Changing display limit from {} to {} messages",
                                                message_stream.read().display_count(),
                                                count
                                            );

                                            let mut current_config = message_stream.read().config().clone();
                                            current_config.display_limit = if count >= 999999 {
                                                DisplayLimit::Unlimited
                                            } else {
                                                DisplayLimit::Fixed(count)
                                            };
                                            current_config.max_display_count = count;

                                            // MessageStreamの設定更新
                                            message_stream.with_mut(|stream| {
                                                stream.update_config(current_config);
                                            });

                                            // 統計強制更新（Signal変更を確実に検出させる）
                                            let new_stats = message_stream.read().stats();
                                            stream_stats.set(new_stats);

                                            tracing::info!(
                                                "✅ [MessageStream] Display limit updated: display={}, archived={}, reduction={}%",
                                                message_stream.read().display_count(),
                                                message_stream.read().archived_count(),
                                                message_stream.read().stats().effective_reduction_percent
                                            );

                                            // Signal更新記録
                                            record_signal_update("message_stream_config");
                                            queue_batch_update("message_stream_display_limit", BatchUpdateType::HighPriority);
                                        } else {
                                            tracing::warn!("🚨 [MessageStream] Invalid display count: {}", event.value());
                                        }
                                    }
                                },

                            option {
                                value: "50",
                                selected: {
                                    let current_limit = match message_stream.read().config().display_limit {
                                        DisplayLimit::Fixed(count) => count,
                                        DisplayLimit::Unlimited => 999999,
                                        _ => 100,
                                    };
                                    current_limit == 50
                                },
                                "50件"
                            }
                            option {
                                value: "100",
                                selected: {
                                    let current_limit = match message_stream.read().config().display_limit {
                                        DisplayLimit::Fixed(count) => count,
                                        DisplayLimit::Unlimited => 999999,
                                        _ => 100,
                                    };
                                    current_limit == 100
                                },
                                "100件"
                            }
                            option {
                                value: "200",
                                selected: {
                                    let current_limit = match message_stream.read().config().display_limit {
                                        DisplayLimit::Fixed(count) => count,
                                        DisplayLimit::Unlimited => 999999,
                                        _ => 100,
                                    };
                                    current_limit == 200
                                },
                                "200件"
                            }
                            option {
                                value: "500",
                                selected: {
                                    let current_limit = match message_stream.read().config().display_limit {
                                        DisplayLimit::Fixed(count) => count,
                                        DisplayLimit::Unlimited => 999999,
                                        _ => 100,
                                    };
                                    current_limit == 500
                                },
                                "500件"
                            }
                            option {
                                value: "999999",
                                selected: {
                                    let current_limit = match message_stream.read().config().display_limit {
                                        DisplayLimit::Fixed(count) => count,
                                        DisplayLimit::Unlimited => 999999,
                                        _ => 100,
                                    };
                                    current_limit >= 999999
                                },
                                "無制限"
                            }
                            }
                        }
                    }
                }

            // フィルターパネル
            if *show_filter_panel.read() {
                FilterPanel {
                    filter: global_filter,
                    on_filter_change: move |new_filter: MessageFilter| {
                        global_filter.set(new_filter);
                    }
                }
            }

            // アーカイブ検索パネル（MessageStream連携）
            if *show_archive_search.read() {
                div {
                    style: "
                        background: #f8f4ff;
                        border: 1px solid #d8b4fe;
                        border-radius: 8px;
                        padding: 16px;
                        margin: 8px;
                        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                    ",

                    h3 {
                        style: "
                            color: #6b46c1;
                            margin: 0 0 12px 0;
                            font-size: 1.1rem;
                            display: flex;
                            align-items: center;
                            gap: 8px;
                        ",
                        "📚 アーカイブ検索"
                        span {
                            style: "
                                font-size: 0.8rem;
                                color: #9ca3af;
                                font-weight: normal;
                            ",
                            "({message_stream.read().archived_count()}件)"
                        }
                    }

                    div {
                        style: "
                            display: flex;
                            flex-direction: column;
                            gap: 12px;
                        ",

                        // 検索設定
                        div {
                            style: "
                                display: flex;
                                gap: 12px;
                                align-items: center;
                                flex-wrap: wrap;
                            ",

                            // 検索タイプ選択
                            div {
                                style: "display: flex; gap: 8px; align-items: center;",

                                label {
                                    style: "
                                        display: flex;
                                        align-items: center;
                                        gap: 4px;
                                        cursor: pointer;
                                        font-size: 0.9rem;
                                    ",
                                    input {
                                        r#type: "radio",
                                        name: "search_type",
                                        checked: matches!(search_type(), ArchiveSearchType::Content),
                                        onchange: move |_| search_type.set(ArchiveSearchType::Content),
                                    }
                                    "内容検索"
                                }

                                label {
                                    style: "
                                        display: flex;
                                        align-items: center;
                                        gap: 4px;
                                        cursor: pointer;
                                        font-size: 0.9rem;
                                    ",
                                    input {
                                        r#type: "radio",
                                        name: "search_type",
                                        checked: matches!(search_type(), ArchiveSearchType::Author),
                                        onchange: move |_| search_type.set(ArchiveSearchType::Author),
                                    }
                                    "投稿者検索"
                                }
                            }

                            // 検索入力
                            div {
                                style: "flex: 1; min-width: 200px;",

                                input {
                                    r#type: "text",
                                    placeholder: match search_type() {
                                        ArchiveSearchType::Content => "メッセージ内容を検索...",
                                        ArchiveSearchType::Author => "投稿者名を検索...",
                                    },
                                    value: search_query(),
                                    style: "
                                        width: 100%;
                                        padding: 8px 12px;
                                        border: 1px solid #d1d5db;
                                        border-radius: 6px;
                                        font-size: 0.9rem;
                                        background: white;
                                    ",
                                    oninput: move |event| {
                                        search_query.set(event.value());
                                    },
                                }
                            }

                            // 検索状態表示
                            if *is_searching.read() {
                                span {
                                    style: "
                                        color: #6b46c1;
                                        font-size: 0.8rem;
                                        display: flex;
                                        align-items: center;
                                        gap: 4px;
                                    ",
                                    "🔍 検索中..."
                                }
                            }
                        }

                        // 検索結果表示
                        if !search_results.read().is_empty() {
                            div {
                                style: "
                                    border-top: 1px solid #e5e7eb;
                                    padding-top: 12px;
                                ",

                                div {
                                    style: "
                                        font-size: 0.9rem;
                                        color: #6b46c1;
                                        margin-bottom: 8px;
                                        font-weight: 600;
                                    ",
                                    "検索結果: {search_results.read().len()}件"
                                }

                                div {
                                    style: "
                                        max-height: 200px;
                                        overflow-y: auto;
                                        border: 1px solid #e5e7eb;
                                        border-radius: 4px;
                                        background: white;
                                    ",

                                    for (index, result) in search_results.read().iter().enumerate() {
                                        div {
                                            key: "{result.timestamp}-{result.author}-{index}",
                                            style: "
                                                padding: 8px 12px;
                                                border-bottom: 1px solid #f3f4f6;
                                                cursor: pointer;
                                                transition: background-color 0.2s;
                                            ",
                                            onmouseenter: move |_| {
                                                // ホバー効果（簡易実装）
                                            },
                                            onclick: {
                                                let result = result.clone();
                                                move |_| {
                                                    tracing::info!(
                                                        "🔍 [ARCHIVE SEARCH] Selected result: {} - {}",
                                                        result.author,
                                                        result.content.chars().take(50).collect::<String>()
                                                    );
                                                    // 将来的に、選択したメッセージを表示エリアに復帰する機能を実装
                                                }
                                            },

                                            // 検索結果の表示
                                            div {
                                                style: "
                                                    display: flex;
                                                    align-items: center;
                                                    gap: 8px;
                                                    margin-bottom: 4px;
                                                    font-size: 0.8rem;
                                                ",

                                                span {
                                                    style: "color: #6b7280; font-size: 0.75rem;",
                                                    "{result.timestamp}"
                                                }

                                                span {
                                                    style: "color: #374151; font-weight: 600;",
                                                    "{result.author}"
                                                }
                                            }

                                            div {
                                                style: "
                                                    color: #1f2937;
                                                    font-size: 0.85rem;
                                                    line-height: 1.3;
                                                    word-wrap: break-word;
                                                ",
                                                "{result.content}"
                                            }
                                        }
                                    }
                                }
                            }
                        } else if !search_query.read().is_empty() && !*is_searching.read() {
                            div {
                                style: "
                                    text-align: center;
                                    color: #6b7280;
                                    font-size: 0.9rem;
                                    padding: 16px;
                                    border: 1px dashed #d1d5db;
                                    border-radius: 4px;
                                ",
                                "検索結果が見つかりませんでした"
                            }
                        }
                    }
                }
            }

            // メッセージ統計（MessageStream版）
            div {
                class: CssClasses::STATUS_PANEL,
                style: "
                    flex-shrink: 0;
                    padding: 4px 12px !important;
                    background: #f8fafc;
                    border-bottom: 1px solid #e2e8f0;
                    font-size: 11px !important;
                    color: #64748b;
                    display: flex;
                    justify-content: space-between;
                    flex-wrap: wrap;
                    gap: 8px;
                ",

                span {
                    "📊 フィルタ後: {filtered_messages.read().len()} / 表示枠: {stream_stats.read().display_count}"
                }

                span {
                    "📦 アーカイブ: {stream_stats.read().archived_count}"
                }

                span {
                    "💾 メモリ: {stream_stats.read().display_memory_mb():.1}MB"
                }

                if stream_stats.read().effective_reduction_percent > 0 {
                    span {
                        style: "color: #059669; font-weight: 600;",
                        "📉 削減: {stream_stats.read().effective_reduction_percent}%"
                    }
                }
            }

            // メッセージリスト
            div {
                id: "liscov-message-list",
                class: CssClasses::MESSAGE_LIST,
                style: "
                    flex: 1;
                    overflow-y: auto;
                    padding: 4px 8px !important;
                    background: white;
                    scroll-behavior: smooth;
                ",

                // メッセージ表示
                for message in filtered_messages.read().iter() {
                    {
                        // メッセージタイプに応じたスタイルを決定（YouTubeの実際の色を使用）
                        let (type_style, type_header, text_color) = get_message_type_style_with_colors(message);
                        let is_special_message = !matches!(message.message_type, MessageType::Text | MessageType::System);
                        // テキスト色（YouTubeの色があればそれを使用、なければデフォルト黒）
                        let content_text_color = text_color.unwrap_or_else(|| "#1a202c".to_string());

                        // メッセージクリック時のハンドラを作成
                        let msg_for_click = message.clone();
                        let live_chat_handle_click = live_chat_handle.clone();

                        // 選択中のメッセージかどうかを判定
                        let is_selected = selected_viewer
                            .read()
                            .as_ref()
                            .map(|v| v.message.id == message.id)
                            .unwrap_or(false);

                        // 選択中の場合は枠線を追加
                        let selected_style = if is_selected {
                            "border: 2px solid #5865f2; box-shadow: 0 0 8px rgba(88, 101, 242, 0.5);"
                        } else {
                            "border: 2px solid transparent; box-shadow: none;"
                        };

                        // スクロール用のメッセージID
                        let message_id_attr = message.id.clone();

                        rsx! {
                            div {
                                key: "{message.timestamp}-{message.author}",
                                "data-message-id": "{message_id_attr}",
                                class: {
                                    let type_class = message.message_type.as_string();
                                    let mut classes = vec![CssClasses::CHAT_MESSAGE];
                                    if message.is_member {
                                        classes.push("member");
                                    }
                                    classes.push(&type_class);
                                    classes.join(" ")
                                },
                                style: "
                                    margin-bottom: 4px;
                                    border-radius: 4px;
                                    overflow: hidden;
                                    cursor: pointer;
                                    {selected_style}
                                ",
                                onclick: move |_| {
                                    // 配信者チャンネルIDを取得
                                    let broadcaster_id = live_chat_handle_click.get_broadcaster_channel_id()
                                        .unwrap_or_else(|| "unknown".to_string());

                                    // 視聴者のカスタム情報をキャッシュから取得
                                    let custom_info = live_chat_handle_click.viewer_info_cache
                                        .read()
                                        .get(&msg_for_click.channel_id)
                                        .cloned();

                                    // 選択された視聴者情報を設定
                                    let viewer = SelectedViewer::new(
                                        broadcaster_id,
                                        msg_for_click.channel_id.clone(),
                                        msg_for_click.author.clone(),
                                        msg_for_click.clone(),
                                        custom_info,
                                    );

                                    selected_viewer.set(Some(viewer));
                                    show_viewer_panel.set(true);
                                },

                                // 特殊メッセージタイプのヘッダー行
                                if is_special_message {
                                    {render_type_header_with_colors(message, type_header, message_font_size().into())}
                                }

                                // 1行目：メタデータ行（時刻、アイコン、ユーザー名、バッジ、コメント回数）
                                // 白背景で常に読みやすく
                                div {
                                    style: format!("
                                        display: flex;
                                        align-items: center;
                                        gap: 8px;
                                        margin-bottom: 0;
                                        padding: 4px 8px;
                                        font-size: {}px;
                                        background: white;
                                    ", message_font_size()),

                                    // タイムスタンプ
                                    if show_timestamps() {
                                        span {
                                            style: "
                                                color: #64748b;
                                                font-size: 10px;
                                                white-space: nowrap;
                                            ",
                                            "{message.timestamp}"
                                        }
                                    }

                                    // 投稿者アイコン
                                    if let Some(icon_url) = &message.author_icon_url {
                                        img {
                                            src: "{icon_url}",
                                            alt: "{message.author}のアイコン",
                                            style: "
                                                width: 20px;
                                                height: 20px;
                                                border-radius: 50%;
                                                object-fit: cover;
                                                flex-shrink: 0;
                                            ",
                                        }
                                    }

                                    // ユーザー名
                                    span {
                                        class: "message-author",
                                        style: if message.is_member {
                                            "
                                                font-weight: 600;
                                                color: #059669;
                                                white-space: nowrap;
                                            "
                                        } else {
                                            "
                                                font-weight: 600;
                                                color: #2563eb;
                                                white-space: nowrap;
                                            "
                                        },
                                        "{message.author}"
                                    }

                                    // 読み仮名（登録されている場合）
                                    if let Some(reading) = live_chat_handle.get_viewer_reading(&message.channel_id) {
                                        span {
                                            style: "
                                                color: #6b7280;
                                                font-size: 11px;
                                                white-space: nowrap;
                                            ",
                                            "({reading})"
                                        }
                                    }

                                    // バッジ表示（メンバーバッジ、スタンプ等）
                                    if let Some(metadata) = &message.metadata {
                                        for badge in &metadata.badge_info {
                                            if let Some(image_url) = &badge.image_url {
                                                // 画像バッジ（スタンプ等）
                                                img {
                                                    src: "{image_url}",
                                                    alt: "{badge.tooltip}",
                                                    title: "{badge.tooltip}",
                                                    style: "
                                                        width: 16px;
                                                        height: 16px;
                                                        border-radius: 2px;
                                                        vertical-align: middle;
                                                    ",
                                                }
                                            } else if badge.tooltip.contains("メンバー") || badge.tooltip.contains("Member") {
                                                // フォールバック：テキストバッジ（メンバーのみ）
                                                span {
                                                    style: "
                                                        background: #10b981;
                                                        color: white;
                                                        font-size: 9px;
                                                        padding: 1px 4px;
                                                        border-radius: 3px;
                                                        white-space: nowrap;
                                                    ",
                                                    "メンバー"
                                                }
                                            }
                                        }
                                    }

                                    // コメント回数表示（新着表示）
                                    div {
                                        style: if let Some(count) = message.comment_count {
                                            if count == 1 {
                                                "
                                                    flex: 1;
                                                    color: #dc2626;
                                                    font-size: 10px;
                                                    font-weight: bold;
                                                    text-align: right;
                                                    white-space: nowrap;
                                                    background: #fef2f2;
                                                    padding: 1px 4px;
                                                    border-radius: 3px;
                                                    border: 1px solid #fecaca;
                                                "
                                            } else {
                                                "
                                                    flex: 1;
                                                    color: #9ca3af;
                                                    font-size: 10px;
                                                    text-align: right;
                                                    white-space: nowrap;
                                                "
                                            }
                                        } else {
                                            "
                                                flex: 1;
                                                color: #9ca3af;
                                                font-size: 10px;
                                                text-align: right;
                                                white-space: nowrap;
                                            "
                                        },
                                        {
                                            if let Some(count) = message.comment_count {
                                                if count == 1 {
                                                    "🎉#1".to_string()
                                                } else {
                                                    format!("#{}", count)
                                                }
                                            } else {
                                                "".to_string()
                                            }
                                        }
                                    }
                                }

                                // 2行目：メッセージ本文（SuperChat/SuperStickerはYouTubeの色を使用）
                                div {
                                    style: format!("
                                        color: {};
                                        padding: 8px;
                                        line-height: 1.4;
                                        word-wrap: break-word;
                                        font-size: {}px;
                                        {}
                                    ", content_text_color, message_font_size(), type_style),
                                    {render_message_content(&message.runs, &message.content, message_font_size())}
                                }
                            }
                        }
                    }
                }
            }
        }

        // 視聴者情報パネル（選択されている場合のみ表示）
        if *show_viewer_panel.read() {
            if let Some(viewer) = selected_viewer.read().as_ref() {
                {
                    // コメント選択時のハンドラ用にクローン
                    let live_chat_handle_for_panel = live_chat_handle.clone();
                    let viewer_clone = viewer.clone();

                    // 自動スクロール設定更新用
                    let mut app_state_for_panel = app_state.clone();

                    rsx! {
                        ViewerInfoPanel {
                            selected_viewer: viewer_clone,
                            on_close: move |_| {
                                // パネルを閉じるが、選択状態は維持（最後にクリックしたコメントが分かるように）
                                show_viewer_panel.set(false);
                            },
                            live_chat_handle: live_chat_handle.clone(),
                            on_message_select: Some(EventHandler::new(move |message: GuiChatMessage| {
                                // 自動スクロールを無効化
                                app_state_for_panel.with_mut(|state| {
                                    state.chat_display_config.auto_scroll_enabled = false;
                                });

                                // 配信者チャンネルIDを取得
                                let broadcaster_id = live_chat_handle_for_panel.get_broadcaster_channel_id()
                                    .unwrap_or_else(|| "unknown".to_string());

                                // 視聴者のカスタム情報をキャッシュから取得
                                let custom_info = live_chat_handle_for_panel.viewer_info_cache
                                    .read()
                                    .get(&message.channel_id)
                                    .cloned();

                                // 選択された視聴者情報を更新
                                let new_viewer = SelectedViewer::new(
                                    broadcaster_id,
                                    message.channel_id.clone(),
                                    message.author.clone(),
                                    message.clone(),
                                    custom_info,
                                );
                                selected_viewer.set(Some(new_viewer));

                                // JavaScriptでメッセージまでスクロール
                                let message_id = message.id.clone();
                                spawn(async move {
                                    let js = format!(
                                        r#"
                                        (function() {{
                                            const el = document.querySelector('[data-message-id="{}"]');
                                            if (el) {{
                                                el.scrollIntoView({{ behavior: 'smooth', block: 'center' }});
                                            }}
                                        }})();
                                        "#,
                                        message_id
                                    );
                                    let _ = document::eval(&js).await;
                                });
                            })),
                        }
                    }
                }
            }
        }
    }
}

/// メッセージからYouTubeの実際の色を使用してスタイルとヘッダー情報を返す
/// 戻り値: (背景スタイル, ヘッダー情報, テキスト色)
fn get_message_type_style_with_colors(message: &GuiChatMessage) -> (String, (&'static str, &'static str, Option<String>), Option<String>) {
    match &message.message_type {
        MessageType::SuperChat { amount } => {
            // YouTubeの実際の色を使用
            if let Some(colors) = message.metadata.as_ref().and_then(|m| m.superchat_colors.as_ref()) {
                (
                    format!(
                        "background: linear-gradient(135deg, {} 0%, {} 100%); border-left: 4px solid {};",
                        colors.header_background, colors.body_background, colors.header_background
                    ),
                    ("💰", "Super Chat", Some(amount.clone())),
                    Some(colors.body_text.clone()),
                )
            } else {
                // フォールバック（色情報がない場合）
                (
                    "background: linear-gradient(135deg, #fff7ed 0%, #fed7aa 100%); border-left: 4px solid #f6ad55;".to_string(),
                    ("💰", "Super Chat", Some(amount.clone())),
                    None,
                )
            }
        }
        MessageType::SuperSticker { amount } => {
            // YouTubeの実際の色を使用
            if let Some(colors) = message.metadata.as_ref().and_then(|m| m.superchat_colors.as_ref()) {
                (
                    format!(
                        "background: {}; border-left: 4px solid {};",
                        colors.body_background, colors.header_background
                    ),
                    ("🎨", "Super Sticker", Some(amount.clone())),
                    Some(colors.body_text.clone()),
                )
            } else {
                // フォールバック
                (
                    "background: #fef2f2; border-left: 4px solid #fc8181;".to_string(),
                    ("🎨", "Super Sticker", Some(amount.clone())),
                    None,
                )
            }
        }
        MessageType::Membership { milestone_months } => {
            if let Some(months) = milestone_months {
                (
                    // パープルグラデーション（マイルストーン）
                    "background: linear-gradient(135deg, #faf5ff 0%, #e9d5ff 100%); border-left: 4px solid #9f7aea;".to_string(),
                    ("🏆", "マイルストーン", Some(format!("{}ヶ月", months))),
                    None,
                )
            } else {
                (
                    // グリーングラデーション（新規メンバー）
                    "background: linear-gradient(135deg, #f0fff4 0%, #c6f6d5 100%); border-left: 4px solid #48bb78;".to_string(),
                    ("🎉", "メンバー加入", None),
                    None,
                )
            }
        }
        MessageType::MembershipGift { gift_count } => (
            // ブルーグラデーション
            "background: linear-gradient(135deg, #eff6ff 0%, #bfdbfe 100%); border-left: 4px solid #4299e1;".to_string(),
            ("🎁", "メンバーシップギフト", Some(format!("{}人", *gift_count))),
            None,
        ),
        MessageType::Text | MessageType::System => (
            // 通常スタイル
            String::new(),
            ("", "", None),
            None,
        ),
    }
}

/// メッセージタイプのヘッダー行をレンダリング
/// ヘッダー行は白背景なので、YouTubeのheader_background色をテキストに使用（特徴的な色で識別しやすい）
fn render_type_header_with_colors(
    message: &GuiChatMessage,
    header_info: (&'static str, &'static str, Option<String>),
    font_size: u32,
) -> Element {
    let (icon, label, badge_text) = header_info;

    // ヘッダーテキスト色、バッジ背景色、バッジテキスト色を決定
    // 白背景なので、YouTubeのheader_background色（特徴的な色）をテキストに使用
    let (header_color, badge_bg, badge_text_color): (String, String, &str) = match &message.message_type {
        MessageType::SuperChat { .. } | MessageType::SuperSticker { .. } => {
            if let Some(colors) = message.metadata.as_ref().and_then(|m| m.superchat_colors.as_ref()) {
                // header_background色をテキストに使用（赤、オレンジ、青など特徴的な色）
                // バッジ背景は同じ色、バッジテキストは白
                (colors.header_background.clone(), colors.header_background.clone(), "white")
            } else {
                // フォールバック
                ("#c2410c".to_string(), "#c2410c".to_string(), "white")
            }
        }
        MessageType::Membership { milestone_months } => {
            if milestone_months.is_some() {
                ("#6b21a8".to_string(), "#6b21a8".to_string(), "white") // パープル（マイルストーン）
            } else {
                ("#166534".to_string(), "#166534".to_string(), "white") // グリーン（新規メンバー）
            }
        }
        MessageType::MembershipGift { .. } => ("#1d4ed8".to_string(), "#1d4ed8".to_string(), "white"), // ブルー
        _ => ("#374151".to_string(), "#e5e7eb".to_string(), "#374151"), // デフォルト
    };

    // 通貨名を取得（日本円以外の場合）
    let currency_name: Option<&'static str> = match &message.message_type {
        MessageType::SuperChat { amount } | MessageType::SuperSticker { amount } => {
            get_currency_name_ja(amount)
        }
        _ => None,
    };

    // バッジテキストに通貨名を追加
    let badge_display = badge_text.map(|text| {
        if let Some(currency) = currency_name {
            format!("{} ({})", text, currency)
        } else {
            text
        }
    });

    rsx! {
        div {
            style: format!("
                display: flex;
                align-items: center;
                gap: 8px;
                padding: 6px 8px;
                font-size: {}px;
                font-weight: 600;
                color: {};
                background: white;
            ", font_size, header_color),

            // アイコン
            span {
                style: format!("font-size: {}px;", font_size + 2),
                "{icon}"
            }

            // ラベル
            span {
                "{label}"
            }

            // バッジ（金額・月数など）
            if let Some(text) = badge_display {
                span {
                    style: format!("
                        background: {};
                        color: {};
                        padding: 4px 12px;
                        border-radius: 16px;
                        font-size: {}px;
                        font-weight: 700;
                    ", badge_bg, badge_text_color, font_size),
                    "{text}"
                }
            }
        }
    }
}

/// メッセージコンテンツをレンダリング（テキストとスタンプ画像を混在表示）
fn render_message_content(
    runs: &[MessageRun],
    content_fallback: &str,
    font_size: u8,
) -> Element {
    let emoji_size = u32::from(font_size) + 4;
    // runsが空の場合はcontent_fallbackを使用
    if runs.is_empty() {
        return rsx! {
            span { "{content_fallback}" }
        };
    }

    rsx! {
        for (index, run) in runs.iter().enumerate() {
            {
                match run {
                    MessageRun::Text { content } => rsx! {
                        span { key: "text-{index}", "{content}" }
                    },
                    MessageRun::Emoji { image_url, alt_text, .. } => rsx! {
                        img {
                            key: "emoji-{index}",
                            src: "{image_url}",
                            alt: "{alt_text}",
                            title: "{alt_text}",
                            style: format!("
                                height: {}px;
                                width: auto;
                                vertical-align: middle;
                                margin: 0 2px;
                            ", emoji_size),
                        }
                    },
                }
            }
        }
    }
}
