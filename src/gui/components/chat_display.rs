use crate::chat_management::MessageFilter;
use crate::gui::components::filter_panel::FilterPanel;
use crate::gui::dom_controller::utils::create_chat_controller; // Phase 3.2
use crate::gui::hooks::use_live_chat::LiveChatHandle;
use crate::gui::performance_monitor::{record_performance_event, PerformanceEventType}; // Phase 5.2
use crate::gui::signal_optimizer::{process_batch_updates, queue_batch_update, BatchUpdateType}; // Phase 4.2
use crate::gui::signal_optimizer::{record_signal_update, register_signal, SignalType}; // Phase 4.1
use crate::gui::styles::theme::{get_connection_status_class, CssClasses};
use crate::gui::timer_service::cancel_highlight_clear_tasks; // Phase 3.3

// Message streaming integration
use crate::gui::message_stream::{DisplayLimit, MessageStream, MessageStreamConfig};
use crate::gui::models::GuiChatMessage;

// Phase 4.3: クロージャ最適化
use crate::gui::closure_optimizer::{
    create_weak_signal_connection, get_closure_optimizer, get_optimized_signal_handler,
    perform_periodic_cleanup, record_closure_creation,
};

use dioxus::prelude::*;

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
    let chat_config = app_state.read().chat_display_config.clone();

    // MessageStream初期化（新規追加）
    let mut message_stream = use_signal(|| {
        let config = MessageStreamConfig {
            display_limit: DisplayLimit::Fixed(100), // デフォルト100件制限
            max_display_count: 100,
            enable_virtual_scroll: true,
            target_fps: 60,
            enable_archive: true,
            archive_search_enabled: true,
        };
        MessageStream::new(config)
    });

    // MessageStream統計表示用
    let stream_stats = use_signal(|| message_stream.read().stats());

    // 基本状態の初期化
    let user_has_scrolled = use_signal(|| false);
    let mut show_filter_panel = use_signal(|| false);
    let highlighted_message_ids = use_signal(|| std::collections::HashSet::<String>::new());
    let last_message_count = use_signal(|| 0usize);
    let mut last_effect_time = use_signal(|| std::time::Instant::now()); // デバウンス用

    // 最適化版：統合設定Signalで4回のAppStateアクセスを1回に削減
    let chat_config = use_memo(move || app_state.read().chat_display_config.clone());

    // 個別設定値は統合設定から派生（再レンダリング最小化）
    let auto_scroll_enabled = use_memo(move || chat_config.read().auto_scroll_enabled);
    let show_timestamps = use_memo(move || chat_config.read().show_timestamps);
    let highlight_enabled = use_memo(move || chat_config.read().highlight_enabled);
    let message_font_size = use_memo(move || chat_config.read().message_font_size);

    // MessageStreamへのメッセージ同期（新規追加）
    use_effect({
        let live_chat_handle = live_chat_handle.clone();
        let mut message_stream = message_stream.clone();
        let mut stream_stats = stream_stats.clone();

        move || {
            let current_messages = live_chat_handle.messages.read();
            let current_count = current_messages.len();
            let stream_total = message_stream.read().total_count();

            // 新しいメッセージがある場合にMessageStreamに追加
            if current_count > stream_total {
                let new_messages: Vec<GuiChatMessage> = current_messages
                    .iter()
                    .skip(stream_total)
                    .cloned()
                    .collect();

                if !new_messages.is_empty() {
                    message_stream.with_mut(|stream| {
                        stream.push_messages(new_messages.clone());
                    });

                    // 統計更新
                    stream_stats.set(message_stream.read().stats());

                    tracing::debug!(
                        "📦 [MessageStream] Added {} messages, display: {}, archived: {}",
                        new_messages.len(),
                        message_stream.read().display_count(),
                        message_stream.read().archived_count()
                    );
                }
            }
        }
    });

    // **メッセージフィルタリング処理**（MessageStreamベース）
    let filtered_messages = use_memo({
        let message_stream = message_stream.clone();
        let global_filter = global_filter.clone();
        let stream_stats = stream_stats.clone(); // 統計変更を依存関係に追加
        move || {
            // stream_statsを読み取って依存関係を強制的に登録
            let _stats = stream_stats.read();
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
            "chat_user_has_scrolled",
            SignalType::UserHasScrolled,
            "ChatDisplay",
        );
        register_signal(
            "chat_show_filter_panel",
            SignalType::ShowFilterPanel,
            "ChatDisplay",
        );
        register_signal(
            "chat_highlight_enabled",
            SignalType::HighlightEnabled,
            "ChatDisplay",
        );
        register_signal(
            "chat_highlighted_message_ids",
            SignalType::HighlightedMessageIds,
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

    // Phase 3.3: コンポーネントアンマウント時のタイマークリーンアップ
    use_drop(move || {
        let cancelled = cancel_highlight_clear_tasks();
        if cancelled > 0 {
            tracing::info!(
                "⏱️ [TIMER] Cleanup: Cancelled {} highlight tasks",
                cancelled
            );
        }
    });

    // 修正版：正しい依存関係設定でuse_effectを実行
    use_effect(move || {
        // 重要：filtered_messagesを最初に読み取って依存関係を登録
        let current_count = filtered_messages.read().len();

        let current_time = std::time::Instant::now();
        let last_time = *last_effect_time.read();

        // デバウンス処理: 50ms以内の連続実行を制限
        if current_time.duration_since(last_time).as_millis() < 50 {
            tracing::debug!("⏭️ [DEBOUNCE] Skipping use_effect execution (too frequent)");
            return;
        }

        last_effect_time.set(current_time);

        let previous_count = *last_message_count.read();

        if current_count > previous_count {
            let new_count = current_count - previous_count;

            // Phase 4.3: 最適化されたSignal更新
            let optimized_handler =
                get_optimized_signal_handler("chat_last_message_count", "ChatDisplay");
            {
                let mut last_count = last_message_count.clone();
                last_count.set(current_count);
                optimized_handler(); // 統合処理を実行
            }

            tracing::info!(
                "📨 [ChatDisplay] New messages: {} (+{})",
                current_count,
                new_count
            );

            // ハイライト処理（軽量版） - DOM操作なし、Signalのみ
            if highlight_enabled() && new_count > 0 {
                let messages = filtered_messages.read();
                let max_highlight = new_count.min(5); // 最大5個
                let start_index = messages.len() - max_highlight;

                let new_ids: std::collections::HashSet<String> = messages
                    .iter()
                    .skip(start_index)
                    .take(max_highlight)
                    .map(|message| {
                        format!(
                            "{}:{}:{}",
                            message.timestamp,
                            message.author,
                            message.content.chars().take(20).collect::<String>()
                        )
                    })
                    .collect();

                {
                    let mut highlight_ids = highlighted_message_ids.clone();
                    highlight_ids.set(new_ids.clone());

                    // 軽量版: Signal更新のみ、DOM操作なし
                    record_signal_update("chat_highlighted_message_ids");

                    tracing::debug!(
                        "🎯 [HIGHLIGHT] Lightweight highlight applied to {} messages",
                        new_ids.len()
                    );
                }

                // 軽量版: シンプルなタイマーによる自動クリア
                {
                    let highlighted_message_ids_clear = highlighted_message_ids.clone();
                    spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                        // Signal操作のみ、タイマーサービス不使用
                        let mut highlight_clear = highlighted_message_ids_clear.clone();
                        highlight_clear.set(std::collections::HashSet::new());

                        tracing::debug!("🎯 [HIGHLIGHT] Lightweight clear after 5s");
                    });
                }
            }

            // Phase 4.2: 新着メッセージ時のBatch処理スクロール
            if auto_scroll_enabled() && !*user_has_scrolled.read() {
                // Phase 4.2: スクロールをBatch処理キューに追加
                queue_batch_update("chat_scroll", BatchUpdateType::DomUpdate);

                // バックグラウンドでBatch処理を実行
                spawn(async move {
                    // Phase 5.2: Batch処理パフォーマンス監視
                    record_performance_event(PerformanceEventType::BatchProcessing, "ChatDisplay");

                    let processed = process_batch_updates().await;
                    if processed > 0 {
                        tracing::debug!(
                            "📦 [BATCH] Processed {} updates including scroll",
                            processed
                        );
                    }

                    // Phase 5.2: DOM操作パフォーマンス監視
                    record_performance_event(PerformanceEventType::DomOperation, "ChatDisplay");

                    // フォールバック：Batch処理が失敗した場合の直接スクロール
                    let controller = create_chat_controller("liscov-message-list");
                    if let Err(e) = controller.scroll_to_bottom(false).await {
                        tracing::debug!("📜 [DOM] Fallback scroll skipped: {}", e);
                    }
                });
            }
        }
    });

    // Phase 3.2: DOM操作（DomController版）
    use_effect({
        let auto_scroll_enabled = auto_scroll_enabled.clone();
        let user_has_scrolled = user_has_scrolled.clone();

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

                tracing::info!("🎮 [DOM] Phase 3.2 Controller ready");

                // 定期的な自動スクロール（高精度）
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    if auto_scroll_enabled() && !*user_has_scrolled.read() {
                        // Phase 3.2: 高精度自動スクロール
                        if let Err(e) = controller.scroll_to_bottom(false).await {
                            tracing::debug!("📜 [DOM] Auto-scroll skipped: {}", e);
                        }
                    }
                }
            });
        }
    });

    let is_connecting = matches!(
        *live_chat_handle.state.read(),
        crate::gui::services::ServiceState::Connecting
    );

    rsx! {
        div {
            class: CssClasses::CHAT_DISPLAY,
            style: "
                height: 100%;
                display: flex;
                flex-direction: column;
                overflow: hidden;
            ",

            // チャットヘッダー
            div {
                class: CssClasses::CHAT_HEADER,
                style: "
                    flex-shrink: 0;
                    padding: 4px 8px !important;
                    background: #f7fafc;
                    border-bottom: 1px solid #e2e8f0;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                ",

                // 接続状態表示
                div {
                    class: get_connection_status_class(*live_chat_handle.is_connected.read(), is_connecting),
                    style: "
                        font-weight: 600;
                        padding: 4px 10px !important;
                        border-radius: 16px;
                        font-size: 12px !important;
                        display: flex;
                        align-items: center;
                        gap: 6px;
                    ",
                    match *live_chat_handle.state.read() {
                        crate::gui::services::ServiceState::Connected => "🟢 接続中",
                        crate::gui::services::ServiceState::Connecting => "🟡 接続中...",
                        crate::gui::services::ServiceState::Paused => "⏸️ 一時停止",
                        crate::gui::services::ServiceState::Idle => "⚪ 待機中",
                        crate::gui::services::ServiceState::Error(_) => "🔴 エラー",
                    }
                }

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

                    // 最新に戻るボタン
                    if *user_has_scrolled.read() {
                        button {
                            class: "px-2 py-1 bg-green-500 hover:bg-green-600 text-white rounded text-xs ml-1",
                            style: "font-size: 11px; min-height: 26px;",
                            onclick: {
                                let mut user_has_scrolled = user_has_scrolled.clone();
                            let optimized_handler = create_optimized_handler("chat_user_has_scrolled");
                                move |_| {
                                    user_has_scrolled.set(false);

                                    // Phase 4.1: Signal更新記録
                                    record_signal_update("chat_user_has_scrolled");

                                    // Phase 4.2: スクロール状態更新をBatch処理
                                    queue_batch_update("chat_user_has_scrolled", BatchUpdateType::HighPriority);

                                // Phase 4.3: 最適化されたハンドラー実行
                                optimized_handler();

                                    spawn(async move {
                                        // Phase 3.2: DomController使用
                                        let controller = create_chat_controller("liscov-message-list");
                                        if let Err(e) = controller.reset_user_scroll().await {
                                            tracing::warn!("🔄 [DOM] Reset scroll failed: {}", e);
                                        }
                                        if let Err(e) = controller.scroll_to_bottom(true).await {
                                            tracing::warn!("📜 [DOM] Force scroll failed: {}", e);
                                        }
                                    });
                                }
                            },
                            "📍 最新に戻る"
                        }
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

                                        // ハイライト切り替え
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
                            checked: highlight_enabled(),
                            onchange: {
                                let mut app_state = app_state.clone();

                                move |event: dioxus::events::FormEvent| {
                                    let enabled = event.checked();
                                    app_state.with_mut(|state| {
                                        state.chat_display_config.highlight_enabled = enabled;
                                    });

                                    // Phase 4.3: 統合記録処理
                                    record_signal_update("chat_highlight_enabled");
                                    queue_batch_update("chat_highlight_enabled", BatchUpdateType::Normal);
                                    record_performance_event(PerformanceEventType::SignalUpdate, "ChatDisplay");

                                    // 軽量版: ハイライト無効化時の処理
                                    if !enabled {
                                        tracing::debug!("🎯 [HIGHLIGHT] Highlight disabled by user");
                                    }
                                }
                            },
                            style: "width: 14px; height: 14px;",
                        }
                        "ハイライト"
                    }

                    // 表示件数設定（MessageStream）
                    div {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 4px !important;
                            font-size: 12px !important;
                            color: #4a5568;
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

                                        let new_config = MessageStreamConfig {
                                            display_limit: if count >= 999999 {
                                                DisplayLimit::Unlimited
                                            } else {
                                                DisplayLimit::Fixed(count)
                                            },
                                            max_display_count: count,
                                            enable_virtual_scroll: true,
                                            target_fps: 60,
                                            enable_archive: true,
                                            archive_search_enabled: true,
                                        };

                                        // MessageStreamの設定更新
                                        message_stream.with_mut(|stream| {
                                            stream.update_config(new_config);
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
                            {
                                let current_limit = match message_stream.read().config().display_limit {
                                    DisplayLimit::Fixed(count) => count,
                                    DisplayLimit::Unlimited => 999999,
                                    _ => 100,
                                };

                                rsx! {
                                    option {
                                        value: "50",
                                        selected: current_limit == 50,
                                        "50件"
                                    }
                                    option {
                                        value: "100",
                                        selected: current_limit == 100,
                                        "100件"
                                    }
                                    option {
                                        value: "200",
                                        selected: current_limit == 200,
                                        "200件"
                                    }
                                    option {
                                        value: "500",
                                        selected: current_limit == 500,
                                        "500件"
                                    }
                                    option {
                                        value: "999999",
                                        selected: current_limit >= 999999,
                                        "無制限"
                                    }
                                }
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
                    },
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

                if highlight_enabled() {
                    span {
                        "🎯 ハイライト: {highlighted_message_ids.read().len()}"
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

                                // メッセージ表示（修復版） - 一時的にコメントアウト
                /*
                for message in filtered_messages.read().iter() {
                    rsx! {
                        div {
                            key: "{message.timestamp}-{message.author}",
                            class: {
                                let mut classes = vec![CssClasses::CHAT_MESSAGE];
                                if message.is_member {
                                    classes.push("member");
                                }
                                let message_id = format!("{}:{}:{}",
                                    message.timestamp,
                                    message.author,
                                    message.content.chars().take(20).collect::<String>()
                                );
                                if highlighted_message_ids.read().contains(&message_id) {
                                    classes.push("liscov-highlight-animation");
                                }
                                classes.join(" ")
                            },
                            style: {
                                let message_id = format!("{}:{}:{}",
                                    message.timestamp,
                                    message.author,
                                    message.content.chars().take(20).collect::<String>()
                                );
                                let is_highlighted = highlighted_message_ids.read().contains(&message_id);
                                if is_highlighted {
                                    format!("
                                        margin-bottom: 4px;
                                        padding: 4px 8px;
                                        border-radius: 4px;
                                        background: #fef3c7;
                                        border-left: 3px solid #f59e0b;
                                        font-size: {}px;
                                        line-height: 1.4;
                                        animation: highlight-pulse 2s ease-in-out;
                                    ", message_font_size())
                                } else {
                                    format!("
                                        margin-bottom: 4px;
                                        padding: 4px 8px;
                                        border-radius: 4px;
                                        font-size: {}px;
                                        line-height: 1.4;
                                    ", message_font_size())
                                }
                            },

                            // 1行目：メタデータ行
                            div {
                                style: "
                                        display: flex;
                                        align-items: center;
                                        gap: 8px;
                                        margin-bottom: 2px;
                                        font-size: 11px;
                                    ",

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

                                // バッジ表示
                                if let Some(metadata) = &message.metadata {
                                    for badge in &metadata.badge_info {
                                        if let Some(image_url) = &badge.image_url {
                                            // 画像バッジ
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

                                // コメント回数表示
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

                            // 2行目：メッセージ本文
                            div {
                                style: "
                                        color: #1a202c;
                                        padding-left: 4px;
                                        line-height: 1.3;
                                        word-wrap: break-word;
                                    ",
                                "{message.content}"
                            }
                        }
                    }
                }
                */

                                // Step 4: ハイライト機能付きメッセージ表示
                for message in filtered_messages.read().iter() {
                    {
                        // メッセージIDの計算（ハイライト判定用）
                        let message_id = format!("{}:{}:{}",
                            message.timestamp,
                            message.author,
                            message.content.chars().take(20).collect::<String>()
                        );
                        let is_highlighted = highlighted_message_ids.read().contains(&message_id);

                        rsx! {
                            div {
                                key: "{message.timestamp}-{message.author}",
                                class: {
                                    let mut classes = vec![CssClasses::CHAT_MESSAGE];
                                    if message.is_member {
                                        classes.push("member");
                                    }
                                    if is_highlighted {
                                        classes.push("liscov-highlight-animation");
                                    }
                                    classes.join(" ")
                                },
                                style: if is_highlighted {
                                    format!("
                                        margin-bottom: 4px;
                                        padding: 4px 8px;
                                        border-radius: 4px;
                                        background: #fef3c7;
                                        border-left: 3px solid #f59e0b;
                                        font-size: {}px;
                                        line-height: 1.4;
                                        animation: highlight-pulse 2s ease-in-out;
                                    ", message_font_size())
                                } else {
                                    format!("
                                        margin-bottom: 4px;
                                        padding: 4px 8px;
                                        border-radius: 4px;
                                        font-size: {}px;
                                        line-height: 1.4;
                                    ", message_font_size())
                                },

                                                                // 1行目：メタデータ行（時刻、アイコン、ユーザー名、バッジ、コメント回数）
                                div {
                                    style: "
                                        display: flex;
                                        align-items: center;
                                        gap: 8px;
                                        margin-bottom: 2px;
                                        font-size: 11px;
                                    ",

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

                                // 2行目：メッセージ本文
                                div {
                                    style: "
                                        color: #1a202c;
                                        padding-left: 4px;
                                        line-height: 1.3;
                                        word-wrap: break-word;
                                    ",
                                    "{message.content}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
