use crate::chat_management::MessageFilter;
use crate::gui::{
    components::FilterPanel,
    hooks::LiveChatHandle,
    styles::theme::{get_connection_status_class, CssClasses},
};
use dioxus::prelude::*;

/// チャット表示コンポーネント（改良版）
/// Phase 4.1: フィルター機能とパフォーマンス最適化
#[component]
pub fn ChatDisplay(
    live_chat_handle: LiveChatHandle,
    global_filter: Signal<MessageFilter>, // グローバルフィルタ追加
) -> Element {
    // ローカル状態を削除してグローバルフィルタを使用
    let mut show_filter_panel = use_signal(|| false);

    // フィルター変更ハンドラー（グローバルフィルタ更新）
    let handle_filter_change = move |new_filter: MessageFilter| {
        global_filter.set(new_filter);
    };

    // フィルターされたメッセージを計算（グローバルフィルタ使用）
    let filtered_messages = use_memo(move || {
        let all_messages = live_chat_handle.messages.read();
        let filter = global_filter.read();

        if !filter.is_active() {
            return all_messages.clone();
        }

        // バッチフィルタリングで高速化
        filter.filter_messages(&all_messages)
    });

    // メッセージ数の変化をログに記録（軽量化）
    use_effect(move || {
        let total_count = live_chat_handle.messages.read().len();
        let filtered_count = filtered_messages.read().len();
        // 大きな変化のみログ出力して負荷軽減
        if total_count != filtered_count && (total_count % 10 == 0 || filtered_count % 10 == 0) {
            tracing::debug!(
                "📺 ChatDisplay: Showing {} filtered messages (total: {})",
                filtered_count,
                total_count
            );
        }
    });

    // オプション設定
    let mut auto_scroll = use_signal(|| true);
    let mut show_timestamps = use_signal(|| true);
    let mut show_test_button = use_signal(|| false); // デフォルトは非表示

    // スマートスクロール制御のための状態
    let mut user_has_scrolled = use_signal(|| false);
    let last_message_count = use_signal(|| 0usize);

    // ハイライト機能専用の状態管理
    let highlighted_messages = use_signal(|| std::collections::HashSet::<String>::new());
    let highlight_last_count = use_signal(|| 0usize);

    // ハイライト設定の読み込み
    let highlight_config = use_signal(|| crate::gui::unified_config::HighlightConfig::default());

    // メッセージID生成の共通関数（DRY原則適用）
    let generate_message_ids = |messages: &[crate::gui::models::GuiChatMessage],
                                start_index: usize,
                                count: usize|
     -> Vec<String> {
        messages
            .iter()
            .skip(start_index)
            .take(count)
            .map(|message| {
                format!(
                    "{}:{}:{}",
                    message.timestamp,
                    message.author,
                    message.content.chars().take(20).collect::<String>()
                )
            })
            .collect()
    };

    // 単一メッセージのID生成（効率的）
    let generate_single_message_id = |message: &crate::gui::models::GuiChatMessage| -> String {
        format!(
            "{}:{}:{}",
            message.timestamp,
            message.author,
            message.content.chars().take(20).collect::<String>()
        )
    };

    // 設定の初期化と変更監視
    use_effect({
        let mut highlight_config = highlight_config.clone();
        let mut show_test_button = show_test_button.clone();

        move || {
            spawn(async move {
                // 設定マネージャーから設定を読み込み
                if let Ok(config_manager) =
                    crate::gui::unified_config::UnifiedConfigManager::new().await
                {
                    // ハイライト設定の読み込み
                    let config: Option<crate::gui::unified_config::HighlightConfig> =
                        config_manager
                            .get_typed_config("highlight")
                            .await
                            .unwrap_or(None);

                    let final_config = config.unwrap_or_default();
                    highlight_config.set(final_config);

                    tracing::info!(
                        "🎯 [HIGHLIGHT] Config loaded: duration={}s, max_messages={}",
                        highlight_config.read().duration_seconds,
                        highlight_config.read().max_messages
                    );

                    // テストボタン表示設定の読み込み
                    let test_button_visible: Option<bool> = config_manager
                        .get_typed_config("ui.show_test_button")
                        .await
                        .unwrap_or(None);

                    show_test_button.set(test_button_visible.unwrap_or(false)); // デフォルトは非表示

                    tracing::info!(
                        "🎛️ [UI] Test button visibility: {}",
                        show_test_button.read()
                    );
                }
            });
        }
    });

    // 停止・クリア後の復旧機能
    use_effect({
        let live_chat_handle = live_chat_handle.clone();
        let mut highlight_last_count = highlight_last_count.clone();

        move || {
            let connection_state = *live_chat_handle.is_connected.read();
            let message_count = live_chat_handle.messages.read().len();

            // 接続復旧時にカウンターをリセット
            if connection_state && message_count == 0 {
                highlight_last_count.set(0);
                tracing::info!("🎯 [HIGHLIGHT] Reset after clear/reconnect");
            }
        }
    });

    // ハイライト用の中間Signal（完全分離設計）
    let highlight_trigger = use_signal(|| 0usize);

    // ハイライト用メッセージ数検出（読み取り専用）
    use_effect({
        let filtered_messages = filtered_messages.clone();
        let mut highlight_trigger = highlight_trigger.clone();

        move || {
            let current_count = filtered_messages.read().len();
            // ハイライトトリガーを更新（読み書き分離）
            highlight_trigger.set(current_count);
        }
    });

    // ハイライト用カウント更新Signal（完全分離設計）
    let highlight_count_updater = use_signal(|| 0usize);

    // ハイライトカウント更新専用（書き込み専用）
    use_effect({
        let highlight_count_updater = highlight_count_updater.clone();
        let mut highlight_last_count = highlight_last_count.clone();

        move || {
            let new_count = *highlight_count_updater.read();
            highlight_last_count.set(new_count);
        }
    });

    // 反応的ハイライト検出（主要システム）- 最終完全分離版
    use_effect({
        let highlight_trigger = highlight_trigger.clone();
        let highlighted_messages = highlighted_messages.clone();
        let highlight_last_count = highlight_last_count.clone();
        let mut highlight_count_updater = highlight_count_updater.clone();
        let mut highlight_config = highlight_config.clone();

        move || {
            let current_count = *highlight_trigger.read(); // トリガー監視
            let last_count = *highlight_last_count.read();
            let config = highlight_config.read().clone();

            // ハイライト機能が無効化されている場合はスキップ
            if !config.enabled {
                // カウントだけ更新してハイライト処理は行わない
                if current_count != last_count {
                    highlight_count_updater.set(current_count);
                }
                return;
            }

            if current_count > last_count {
                let new_message_count = current_count - last_count;

                tracing::info!(
                    "🎯 [HIGHLIGHT-REACTIVE] Count: {} → {} (+{})",
                    last_count,
                    current_count,
                    new_message_count
                );

                // ハイライト処理を別のSpawnで実行（読み書き分離）
                let filtered_messages_for_highlight = filtered_messages.clone();
                let highlighted_messages_for_add = highlighted_messages.clone();
                spawn(async move {
                    // 現在の設定を使用（初期化時と設定変更時に反映済み）
                    let config_for_highlight = config.clone();
                    let current_messages = filtered_messages_for_highlight.read();

                    // 設定に基づく大量メッセージ対応
                    let max_highlight = config_for_highlight.max_messages;
                    let start_index = if new_message_count > max_highlight {
                        current_count - max_highlight // 最新N個のみ
                    } else {
                        last_count // 全て
                    };

                    // iterator チェーン版：関数型プログラミング的アプローチ
                    let new_message_ids: Vec<String> = generate_message_ids(
                        &current_messages,
                        start_index,
                        current_count - start_index,
                    );

                    if !new_message_ids.is_empty() {
                        tracing::info!(
                            "🎯 [HIGHLIGHT-REACTIVE] Adding: {} of {} messages (max: {})",
                            new_message_ids.len(),
                            new_message_count,
                            max_highlight
                        );

                        // ハイライト追加処理（完全分離版）
                        let mut highlighted_messages_clone = highlighted_messages_for_add.clone();
                        let new_message_ids_clone = new_message_ids.clone();
                        spawn(async move {
                            let mut current_highlighted = highlighted_messages_clone.read().clone();
                            for id in &new_message_ids_clone {
                                current_highlighted.insert(id.clone());
                            }
                            highlighted_messages_clone.set(current_highlighted);
                        });

                        // 設定時間後にハイライト削除（完全分離版）
                        let highlighted_messages_for_removal = highlighted_messages_for_add.clone();
                        let new_message_ids_removal = new_message_ids.clone();
                        let duration_secs = config_for_highlight.duration_seconds;
                        spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(duration_secs))
                                .await;

                            // 削除処理を別のSpawnで実行（読み書き分離）
                            let mut highlighted_messages_writer =
                                highlighted_messages_for_removal.clone();
                            let ids_to_remove = new_message_ids_removal.clone();
                            spawn(async move {
                                let mut current_highlighted =
                                    highlighted_messages_writer.read().clone();
                                for id in &ids_to_remove {
                                    current_highlighted.remove(id);
                                }
                                highlighted_messages_writer.set(current_highlighted);
                                tracing::info!(
                                    "🎯 [HIGHLIGHT-REACTIVE] Removed: {} messages after {}s",
                                    ids_to_remove.len(),
                                    duration_secs
                                );
                            });
                        });
                    }
                });

                // カウント更新（完全分離版）
                highlight_count_updater.set(current_count);
            }
        }
    });

    // 補完的な周期チェック（バックアップシステム）- 自動計算版
    use_effect({
        let filtered_messages = filtered_messages.clone();
        let highlighted_messages = highlighted_messages.clone();
        let highlight_last_count = highlight_last_count.clone();
        let mut highlight_config = highlight_config.clone();

        move || {
            spawn(async move {
                loop {
                    let config = highlight_config.read().clone();

                    // ハイライト機能が無効化されている場合はループを継続
                    if !config.enabled {
                        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await; // 5秒待ってから再チェック
                        continue;
                    }

                    // 固定間隔でチェック
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        config.get_backup_check_interval_ms(),
                    ))
                    .await;

                    let current_messages = filtered_messages.read();
                    let current_count = current_messages.len();
                    let last_count = *highlight_last_count.read();

                    if current_count > last_count {
                        let new_message_count = current_count - last_count;
                        tracing::info!(
                            "🎯 [HIGHLIGHT-BACKUP] Missed detection: {} → {} (+{})",
                            last_count,
                            current_count,
                            new_message_count
                        );

                        // 自動計算による補完処理
                        let max_highlight = config.get_backup_max_messages();
                        let start_index = if new_message_count > max_highlight {
                            current_count - max_highlight // 最新N個のみ
                        } else {
                            last_count // 全て
                        };

                        // iterator チェーン版：バックアップシステムも関数型アプローチ
                        let new_message_ids: Vec<String> = generate_message_ids(
                            &current_messages,
                            start_index,
                            current_count - start_index,
                        );

                        if !new_message_ids.is_empty() {
                            // 【修正】バックアップシステムもSpawnで分離
                            let mut highlighted_messages_backup = highlighted_messages.clone();
                            let new_message_ids_backup = new_message_ids.clone();
                            let max_highlight_info = max_highlight;
                            let config_max_messages = config.max_messages;
                            spawn(async move {
                                let mut current_highlighted =
                                    highlighted_messages_backup.read().clone();
                                for id in &new_message_ids_backup {
                                    current_highlighted.insert(id.clone());
                                }
                                highlighted_messages_backup.set(current_highlighted);
                                tracing::info!("🎯 [HIGHLIGHT-BACKUP] Added: {} of {} messages (max: {}, auto-calc from {})", 
                                              new_message_ids_backup.len(), new_message_count, max_highlight_info, config_max_messages);
                            });

                            // 設定時間より短めでハイライト削除（補完・完全分離版）
                            let highlighted_messages_for_backup_removal =
                                highlighted_messages.clone();
                            let new_message_ids_backup_removal = new_message_ids.clone();
                            let duration_secs = config.duration_seconds.saturating_sub(2).max(3); // 設定時間-2秒（最低3秒）
                            spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(duration_secs))
                                    .await;

                                // 削除処理を別のSpawnで実行（読み書き分離）
                                let mut highlighted_messages_backup_writer =
                                    highlighted_messages_for_backup_removal.clone();
                                let backup_ids_to_remove = new_message_ids_backup_removal.clone();
                                spawn(async move {
                                    let mut current_highlighted =
                                        highlighted_messages_backup_writer.read().clone();
                                    for id in &backup_ids_to_remove {
                                        current_highlighted.remove(id);
                                    }
                                    highlighted_messages_backup_writer.set(current_highlighted);
                                });
                            });
                        }
                        // バックアップシステムはカウンターを更新しない（メインシステムに任せる）
                    }
                }
            });
        }
    });

    // 自動スクロールトリガー（中間Signal）
    let scroll_trigger = use_signal(|| 0usize);

    // 新着メッセージ検出（読み取り専用）
    use_effect({
        let filtered_messages = filtered_messages.clone();
        let mut scroll_trigger = scroll_trigger.clone();

        move || {
            let current_count = filtered_messages.read().len();
            // メッセージ数が変化したらスクロールトリガーを更新
            scroll_trigger.set(current_count);
        }
    });

    // 自動スクロール実行（書き込み専用）
    use_effect({
        let scroll_trigger = scroll_trigger.clone();
        let auto_scroll = auto_scroll.clone();
        let user_has_scrolled = user_has_scrolled.clone();

        move || {
            let _current_count = *scroll_trigger.read(); // トリガー監視

            // 自動スクロール条件：自動スクロール有効、ユーザー操作なし
            if *auto_scroll.read() && !*user_has_scrolled.read() {
                // DOM操作を非同期で実行（Signal読み書きと分離）
                spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    let _ = dioxus::document::eval(
                        r#"
                        const container = document.getElementById('liscov-message-list');
                        if (container) {
                            window.liscovUserScrolled = false;
                            container.scrollTop = container.scrollHeight;
                            
                            setTimeout(() => {
                                container.scrollTo({
                                    top: container.scrollHeight,
                                    behavior: 'smooth'
                                });
                            }, 50);
                        }
                        "#,
                    );
                });
            }
        }
    });

    // メッセージカウント追跡（読み取り専用）
    let message_count_trigger = use_signal(|| 0usize);

    // メッセージカウント更新（書き込み専用・分離版）
    use_effect({
        let filtered_messages = filtered_messages.clone();
        let mut message_count_trigger = message_count_trigger.clone();

        move || {
            let current_count = filtered_messages.read().len();
            // 前回と異なる場合のみトリガー更新（無限ループ回避）
            message_count_trigger.set(current_count);
        }
    });

    // 最後のメッセージカウント追跡（完全分離版）
    use_effect({
        let message_count_trigger = message_count_trigger.clone();
        let mut last_message_count = last_message_count.clone();

        move || {
            let current_count = *message_count_trigger.read();
            // このSignalは読み取り専用として使用
            last_message_count.set(current_count);
        }
    });

    // 改良されたスクロール状態監視とコンテナ初期化
    use_effect(move || {
        spawn(async move {
            // DOM要素が確実に存在するまで少し待つ
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // 初期化スクリプト（コンテナ確認付き）
            let _ = dioxus::document::eval(
                r#"
                if (!window.liscovScrollInitialized) {
                    window.liscovScrollInitialized = true;
                    window.liscovUserScrolled = false;
                    
                    // コンテナの存在確認
                    const container = document.getElementById('liscov-message-list');
                    if (container) {
                        // 初期位置を最下部に設定
                        setTimeout(() => {
                            container.scrollTop = container.scrollHeight;
                        }, 100);
                    }
                }
                "#,
            );
        });
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

            // チャットヘッダー - 配信最適化
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

                // 接続状態表示 - 配信最適化
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

                // チャット制御 - 配信最適化
                div {
                    class: CssClasses::CHAT_CONTROLS,
                    style: "
                        display: flex;
                        gap: 8px !important;
                        align-items: center;
                    ",

                    button {
                        class: if *show_filter_panel.read() {
                            "px-2 py-1 bg-blue-600 text-white rounded text-xs"
                        } else {
                            "px-2 py-1 bg-blue-500 hover:bg-blue-600 text-white rounded text-xs"
                        },
                        style: "font-size: 11px; min-height: 26px;",
                        onclick: move |_| {
                            let current_value = *show_filter_panel.read();
                            show_filter_panel.set(!current_value);
                        },
                        if global_filter.read().is_active() {
                            "🔍 フィルター ({global_filter.read().active_filter_count()})"
                        } else {
                            "🔍 フィルター"
                        }
                    }

                    // 最新に戻るボタン（ユーザーがスクロールした時のみ表示）
                    if *user_has_scrolled.read() {
                        button {
                            class: "px-2 py-1 bg-green-500 hover:bg-green-600 text-white rounded text-xs ml-1",
                            style: "font-size: 11px; min-height: 26px;",
                            onclick: move |_| {
                                user_has_scrolled.set(false);
                                spawn(async move {
                                    let _ = dioxus::document::eval(
                                        r#"
                                        const chatContainer = document.getElementById('liscov-message-list');
                                        if (chatContainer) {
                                        // 確実に状態をリセット
                                        window.liscovUserScrolled = false;
                                        
                                        // 即座にスクロール位置を設定
                                            chatContainer.scrollTop = chatContainer.scrollHeight;
                                        
                                        // 追加でスムーズスクロール
                                        setTimeout(() => {
                                            chatContainer.scrollTo({
                                                top: chatContainer.scrollHeight,
                                                behavior: 'smooth'
                                            });
                                        }, 50);
                                        }
                                        "#,
                                    );
                                });
                            },
                            "📍 最新に戻る"
                        }
                    }

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
                            checked: *auto_scroll.read(),
                            onchange: move |event| auto_scroll.set(event.checked()),
                            style: "width: 14px; height: 14px;",
                        }
                        "自動スクロール"
                    }

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
                            checked: *show_timestamps.read(),
                            onchange: move |event| show_timestamps.set(event.checked()),
                            style: "width: 14px; height: 14px;",
                        }
                        "タイムスタンプ"
                    }
                }
            }

            // フィルターパネル - 配信最適化
            if *show_filter_panel.read() {
                div {
                    style: "
                        flex-shrink: 0;
                        padding: 4px 8px;
                        border-bottom: 1px solid #e2e8f0;
                        background: #f8fafc;
                    ",
                    FilterPanel {
                        filter: global_filter,
                        on_filter_change: handle_filter_change,
                    }
                }
            }

            // メッセージリスト（スクロール可能エリア）- ゲーム配信最適化
            div {
                id: "liscov-message-list",
                class: CssClasses::MESSAGE_LIST,
                style: "
                    flex: 1;
                    overflow-y: auto;
                    overflow-x: hidden;
                    padding: 4px 8px;
                    display: flex;
                    flex-direction: column;
                    gap: 3px;
                    scroll-behavior: smooth;
                    background: #fafbfc;
                ",
                // 安定したスクロールイベント処理
                onscroll: move |_| {
                    // デバウンス付きスクロール検出
                    let user_has_scrolled_clone = user_has_scrolled.clone();
                    spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        let _ = dioxus::document::eval(
                            r#"
                            const container = document.getElementById('liscov-message-list');
                            if (container) {
                                // より厳密なしきい値でユーザースクロールを検出
                                const threshold = 20; // 20pxの余裕
                                const isAtBottom = 
                                    container.scrollHeight - container.scrollTop <= 
                                    container.clientHeight + threshold;
                                
                                window.liscovUserScrolled = !isAtBottom;
                            }
                            "#,
                        );

                        // Rust側の状態も更新（別のSpawnで実行）
                        let mut user_has_scrolled_update = user_has_scrolled_clone.clone();
                        spawn(async move {
                            if let Ok(result) = dioxus::document::eval("window.liscovUserScrolled || false").await {
                                if let Some(scrolled) = result.as_bool() {
                                    let current_scrolled = *user_has_scrolled_update.read();
                                    if scrolled != current_scrolled {
                                        user_has_scrolled_update.set(scrolled);
                                    }
                                }
                            }
                        });
                    });
                },

                if filtered_messages.read().is_empty() {
                    div {
                        class: CssClasses::NO_MESSAGES,
                        style: "
                            text-align: center;
                            padding: 40px 16px;
                            color: #4b5563;
                            font-size: 20px;
                            font-weight: 600;
                            background: #f8fafc;
                            border-radius: 12px;
                            margin: 12px;
                            border: 2px dashed #cbd5e1;
                        ",
                        if live_chat_handle.messages.read().is_empty() {
                            div {
                                style: "font-size: 48px; margin-bottom: 20px;",
                                "💬"
                            }
                            div {
                                style: "margin-bottom: 10px;",
                                "メッセージがまだありません"
                            }
                            div {
                                style: "font-size: 16px; color: #6b7280;",
                                "接続を開始してライブチャットを監視しましょう！"
                            }
                        } else {
                            div {
                                style: "font-size: 48px; margin-bottom: 20px;",
                                "🔍"
                            }
                            div {
                                style: "margin-bottom: 10px;",
                                "フィルター条件に一致するメッセージがありません"
                            }
                            div {
                                style: "font-size: 16px; color: #6b7280;",
                                "フィルター設定を調整してください"
                            }
                        }
                    }
                                } else {
                    for (index, message) in filtered_messages.read().iter().enumerate() {
                        div {
                            key: "{index}",
                            class: "message-item",
                            style: {
                                // メッセージのユニークIDを生成してハイライト判定
                                let message_id = generate_single_message_id(message);
                                let is_highlighted = highlighted_messages.read().contains(&message_id);

                                format!(
                                "
                                padding: 8px 12px;
                                margin-bottom: 3px;
                                border-radius: 8px;
                                background: {};
                                border: 2px solid {};
                                box-shadow: {};
                                transition: all 0.2s ease;
                                position: relative;
                                border-left: 6px solid {};
                                cursor: default;
                                ",
                                    // 可読性重視の薄い背景色
                                    if is_highlighted {
                                        "linear-gradient(135deg, #f0f8ff, #e6f3ff)"  // 非常に薄い青
                                    } else if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() {
                                        "linear-gradient(135deg, #fffbeb, #fef3c7)"  // 非常に薄い黄色
                                    } else if message.is_member {
                                        "#f0fdf4"  // 非常に薄い緑
                                    } else if index % 2 == 0 { "#ffffff" } else { "#f8fafc" },

                                    // より薄いボーダー色
                                    if is_highlighted { "#93c5fd" }  // 薄い青
                                    else if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() { "#fbbf24" }  // 薄い黄色
                                    else if message.is_member { "#4ade80" }  // 薄い緑
                                    else { "#e2e8f0" },

                                    // 控えめなシャドウ効果
                                    if is_highlighted { "0 2px 8px rgba(59, 130, 246, 0.12)" }
                                    else if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() { "0 2px 10px rgba(251, 191, 36, 0.15)" }
                                    else { "0 1px 3px rgba(0, 0, 0, 0.05)" },

                                    // 左ボーダー色も薄く調整
                                    if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() { "#fbbf24" }
                                    else if message.is_member { "#4ade80" }
                                    else { "#6366f1" }
                                )
                            },
                            // ホバー効果のJavaScriptを一時的に無効化（CPU負荷軽減のため）
                            // onmouseenter: move |_| {
                            //     // 軽量なhover効果（必要最小限）
                            //     let script = format!(
                            //         "const el=document.querySelectorAll('.message-item')[{}];if(el){{el.style.transform='translateY(-2px)';el.style.boxShadow='0 8px 16px rgba(0,0,0,0.1)';}}",
                            //         index
                            //     );
                            //     spawn(async move {
                            //         let _ = dioxus::document::eval(&script);
                            //     });
                            // },
                            // onmouseleave: move |_| {
                            //     let script = format!(
                            //         "const el=document.querySelectorAll('.message-item')[{}];if(el){{el.style.transform='';el.style.boxShadow='';}}",
                            //         index
                            //     );
                            //     spawn(async move {
                            //         let _ = dioxus::document::eval(&script);
                            //     });
                            // },

                            // メッセージのメタ情報 - 配信最適化
                            div {
                                style: "
                                    display: flex;
                                    justify-content: space-between;
                                    align-items: center;
                                    margin-bottom: 3px;
                                    padding-bottom: 3px;
                                    border-bottom: 2px solid #e2e8f0;
                                ",

                                // 作者名 - 配信用大型フォント
                                span {
                                    style: format!("
                                        font-weight: 700;
                                        color: {};
                                        font-size: 19px;
                                        text-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
                                    ",
                                        // 背景色に合わせた読みやすい作者名色
                                        if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() {
                                            "#d97706"  // SuperChatは中程度の金色（変更なし）
                                        } else if message.is_member {
                                            "#16a34a"  // メンバーは少し薄めの緑色
                                        } else {
                                            "#374151"  // 通常は読みやすいダークグレー
                                        }
                                    ),
                                    "{message.author}"
                                }

                                // タイムスタンプ - 配信用強化
                                if *show_timestamps.read() {
                                    span {
                                        style: "
                                            font-size: 16px;
                                            color: #4b5563;
                                            background: #e5e7eb;
                                            padding: 4px 10px;
                                            border-radius: 6px;
                                            font-weight: 600;
                                            border: 1px solid #d1d5db;
                                        ",
                                        "{message.timestamp}"
                                    }
                                }
                            }

                            // メッセージ本文（安全なレンダリング） - 配信最適化
                            div {
                                style: "
                                    color: #111827;
                                    line-height: 1.3;
                                    word-wrap: break-word;
                                    margin-bottom: 3px;
                                    display: flex;
                                    flex-wrap: wrap;
                                    align-items: center;
                                    gap: 4px;
                                    font-size: 18px;
                                    font-weight: 500;
                                ",

                                // runsが空の場合は従来のcontentを表示（後方互換性）
                                if message.runs.is_empty() {
                                    "{message.content}"
                                }

                                // runsから安全にレンダリング
                                for run in &message.runs {
                                    match run {
                                        crate::gui::models::MessageRun::Text { content } => rsx! {
                                            span { "{content}" }
                                        },
                                        crate::gui::models::MessageRun::Emoji { emoji_id, image_url, alt_text } => rsx! {
                                            if !image_url.is_empty() {
                                                img {
                                                    src: "{image_url}",
                                                    alt: "{alt_text}",
                                                    title: "{emoji_id}",
                                                    style: "
                                                        width: 28px;
                                                        height: 28px;
                                                        vertical-align: middle;
                                                        object-fit: contain;
                                                        border-radius: 4px;
                                                        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
                                                    "
                                                }
                                            } else {
                                                // フォールバック：画像がない場合はalt_textを表示
                                                span {
                                                    style: "
                                                        font-style: italic;
                                                        color: #6b7280;
                                                        font-size: 12px;
                                                    ",
                                                    "[{alt_text}]"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // SuperChat表示（配信最適化）
                            if let Some(metadata) = &message.metadata {
                                if let Some(amount_str) = &metadata.amount {
                                    div {
                                        style: "
                                            margin-top: 6px;
                                            padding: 10px 16px;
                                            background: linear-gradient(135deg, #fcd34d, #f59e0b, #d97706);
                                            color: white;
                                            font-weight: 800;
                                            font-size: 16px;
                                            border-radius: 8px;
                                            text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
                                            border: 2px solid #d97706;
                                            box-shadow: 0 4px 12px rgba(217, 119, 6, 0.4);
                                            text-align: center;
                                            animation: pulse 2s infinite;
                                        ",
                                        "💰 SuperChat: {amount_str}"
                                    }
                                }
                            }
                        }
                    }

                    // 最後のメッセージの後の余白（自動スクロール時の見切れ防止）
                    div {
                        style: "height: 8px; flex-shrink: 0;",
                    }
                }
            }

            // チャットフッター（配信最適化版）
            div {
                class: CssClasses::CHAT_FOOTER,
                style: "
                    flex-shrink: 0;
                    border-top: 2px solid #cbd5e1;
                    padding: 6px 8px;
                    background: linear-gradient(135deg, #f1f5f9, #e2e8f0);
                    box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.05);
                ",

                // 上段：デバッグ情報のみ - 配信最適化
                div {
                    class: CssClasses::FOOTER_STATS,
                    style: "
                        margin-bottom: 3px; 
                        font-size: 13px; 
                        font-weight: 600;
                        display: flex;
                        align-items: center;
                        gap: 6px;
                        flex-wrap: wrap;
                        justify-content: space-between;
                    ",

                    // デバッグ情報 - 配信用最小化
                    span {
                        style: "font-size: 10px; color: #6b7280; background: #f3f4f6; padding: 3px 6px; border-radius: 4px;",
                        {
                            let highlight_status = if highlight_config.read().enabled { "ON" } else { "OFF" };
                            format!("📍{} ✨{}",
                                if *auto_scroll.read() { "AUTO" } else { "MANUAL" },
                                highlight_status
                            )
                        }
                    }
                }

                // 下段：操作ボタン - 配信用コンパクト化
                div {
                    style: "
                        display: flex; 
                        gap: 4px; 
                        align-items: center; 
                        flex-wrap: nowrap;
                        justify-content: flex-end;
                    ",

                    // テストメッセージ追加（設定により表示制御）
                    if *show_test_button.read() {
                        button {
                            style: "
                                padding: 4px 8px;
                                background: #10b981;
                                color: white;
                                border: none;
                                border-radius: 6px;
                                font-size: 11px;
                                cursor: pointer;
                                transition: background 0.2s;
                                white-space: nowrap;
                                min-width: 50px;
                            ",
                            onclick: {
                                let handle = live_chat_handle.clone();
                                move |_| {
                                    // 既存のadd_test_messageメソッドを使用
                                    let msg_count_before = handle.messages.read().len();

                                    handle.add_test_message(
                                        "テストユーザー",
                                        &format!("テストメッセージ #{}", msg_count_before + 1),
                                        crate::gui::models::MessageType::Text
                                    );
                                }
                            },
                            "🧪 テスト"
                        }
                    }

                    // 強制スクロール
                    button {
                        style: "
                            padding: 4px 8px;
                            background: #3b82f6;
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 11px;
                            cursor: pointer;
                            transition: background 0.2s;
                            white-space: nowrap;
                            min-width: 50px;
                        ",
                        onclick: move |_| {
                            spawn(async move {
                                let _ = dioxus::document::eval(
                                    r#"
                                    const container = document.getElementById('liscov-message-list');
                                    if (container) {
                                        window.liscovUserScrolled = false;
                                        container.scrollTop = container.scrollHeight;
                                    }
                                    "#,
                                );
                            });
                        },
                        "💨 スクロール"
                    }
                }
            }
        }
    }
}
