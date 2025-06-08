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

    // スマートスクロール制御のための状態
    let mut user_has_scrolled = use_signal(|| false);
    let mut last_message_count = use_signal(|| 0usize);

    // 新着メッセージハイライト機能（修正版：安定したID管理）
    let mut recent_messages = use_signal(|| std::collections::HashSet::<String>::new());
    let highlight_enabled = use_signal(|| true);

    // ハイライト処理専用のカウンター（自動スクロールと独立）
    let mut last_highlight_count = use_signal(|| 0usize);

    // 新着メッセージ検出とハイライト機能（修正版：独立したカウンター）
    use_effect(move || {
        let current_count = filtered_messages.read().len();
        let last_highlight = *last_highlight_count.read();

        // メッセージ数が増加した場合のみ処理
        if current_count > last_highlight && current_count > 0 {
            let new_messages = current_count - last_highlight;

            tracing::info!(
                "📬 Highlight check: current={}, last_highlight={}, new_messages={}, enabled={}",
                current_count,
                last_highlight,
                new_messages,
                *highlight_enabled.read()
            );

            // ハイライトカウンターを即座に更新（重複処理を防ぐ）
            last_highlight_count.set(current_count);

            // 大量のメッセージが一度に追加された場合は処理をスキップ（初期読み込み時など）
            if new_messages <= 5 && *highlight_enabled.read() {
                tracing::info!(
                    "✨ New messages detected: {} new, adding to highlight",
                    new_messages
                );

                // 新着メッセージのユニークIDをハイライト対象に追加
                let filtered_msgs = filtered_messages.read();
                let mut current_recent = recent_messages.read().clone();
                let mut new_message_ids = Vec::new();

                // 新着メッセージのユニークIDを生成（タイムスタンプ+作者名+内容の一部）
                for i in last_highlight..current_count.min(filtered_msgs.len()) {
                    if let Some(message) = filtered_msgs.get(i) {
                        let unique_id = format!(
                            "{}:{}:{}",
                            message.timestamp,
                            message.author,
                            message.content.chars().take(20).collect::<String>()
                        );
                        current_recent.insert(unique_id.clone());
                        new_message_ids.push(unique_id);
                    }
                }
                recent_messages.set(current_recent);

                // カウンター処理は自動（ハッシュセットのサイズで管理）

                tracing::debug!(
                    "✨ Added {} messages to highlight: {:?}",
                    new_message_ids.len(),
                    new_message_ids
                );

                // 5秒後にハイライトを自動的に削除
                spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                    // ハイライト解除
                    let mut current_recent = recent_messages.read().clone();
                    for id in &new_message_ids {
                        current_recent.remove(id);
                    }
                    recent_messages.set(current_recent);

                    // カウンターは自動的に減少（ハッシュセットから削除されるため）

                    tracing::debug!(
                        "✨ Message highlight expired for {} messages: {:?}",
                        new_message_ids.len(),
                        new_message_ids
                    );
                });
            } else {
                tracing::info!(
                    "📦 Skipping highlight for bulk message load: {} messages",
                    new_messages
                );
            }
        }
    });

    // 修正された自動スクロール処理
    use_effect(move || {
        let current_message_count = filtered_messages.read().len();
        let last_count = *last_message_count.read();

        tracing::info!(
            "📊 Auto-scroll check: current={}, last={}, auto_scroll={}, user_scrolled={}",
            current_message_count,
            last_count,
            *auto_scroll.read(),
            *user_has_scrolled.read()
        );

        // 新着メッセージがある場合のみ自動スクロール実行
        if current_message_count > last_count && *auto_scroll.read() && !*user_has_scrolled.read() {
            tracing::info!("✅ Auto-scroll conditions met, executing scroll...");

            // メッセージカウントを先に更新
            last_message_count.set(current_message_count);

            spawn(async move {
                // 少し待ってからスクロール（DOM更新完了を待つ）
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // 確実な自動スクロール実装
                let _ = dioxus::document::eval(
                    r#"
                    (function() {
                        const container = document.getElementById('liscov-message-list');
                        if (container) {
                            console.log('🔍 Auto-scroll: Before - scrollTop:', container.scrollTop, 'scrollHeight:', container.scrollHeight);
                            
                            // フラグをリセットして確実にスクロール
                            window.liscovUserScrolled = false;
                            
                            // 即座にスクロール位置を設定
                            container.scrollTop = container.scrollHeight;
                            
                            // さらに少し待ってからスムーズスクロールで微調整
                            setTimeout(() => {
                                container.scrollTo({
                                    top: container.scrollHeight,
                                    behavior: 'smooth'
                                });
                                console.log('🚀 Auto-scroll executed. Height:', container.scrollHeight, 'ScrollTop:', container.scrollTop);
                            }, 50);
                        } else {
                            console.warn('⚠️ Auto-scroll failed: container not found');
                        }
                    })();
                    "#,
                );
            });
        } else if current_message_count != last_count {
            // メッセージ数が変わったが自動スクロール条件を満たさない場合も更新
            last_message_count.set(current_message_count);
            tracing::debug!("📝 Message count updated without auto-scroll");
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
                        console.log('📜 Enhanced scroll system initialized with container found');
                        
                        // 初期位置を最下部に設定
                        setTimeout(() => {
                            container.scrollTop = container.scrollHeight;
                            console.log('📍 Initial scroll to bottom completed');
                        }, 100);
                    } else {
                        console.warn('⚠️ Scroll container not found during initialization');
                    }
                } else {
                    console.log('📜 Scroll system already initialized');
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

            // チャットヘッダー
            div {
                class: CssClasses::CHAT_HEADER,
                style: "
                    flex-shrink: 0;
                ",

                // 接続状態表示
                div {
                    class: get_connection_status_class(*live_chat_handle.is_connected.read(), is_connecting),
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

                    button {
                        class: if *show_filter_panel.read() {
                            "px-3 py-1 bg-blue-600 text-white rounded text-sm"
                        } else {
                            "px-3 py-1 bg-blue-500 hover:bg-blue-600 text-white rounded text-sm"
                        },
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
                            class: "px-3 py-1 bg-green-500 hover:bg-green-600 text-white rounded text-sm ml-2",
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
                                        
                                        console.log('👇 Manual scroll to bottom executed. Height:', chatContainer.scrollHeight);
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
                        input {
                            r#type: "checkbox",
                            checked: *auto_scroll.read(),
                            onchange: move |event| auto_scroll.set(event.checked()),
                        }
                        "自動スクロール"
                    }

                    label {
                        class: CssClasses::CHECKBOX_LABEL,
                        input {
                            r#type: "checkbox",
                            checked: *show_timestamps.read(),
                            onchange: move |event| show_timestamps.set(event.checked()),
                        }
                        "タイムスタンプ"
                    }
                }
            }

            // フィルターパネル
            if *show_filter_panel.read() {
                div {
                    style: "flex-shrink: 0;",
                    FilterPanel {
                        filter: global_filter,
                        on_filter_change: handle_filter_change,
                    }
                }
            }

            // メッセージリスト（スクロール可能エリア）
            div {
                id: "liscov-message-list",
                class: CssClasses::MESSAGE_LIST,
                style: "
                    flex: 1;
                    overflow-y: auto;
                    overflow-x: hidden;
                    padding: 16px;
                    display: flex;
                    flex-direction: column;
                    gap: 12px;
                    scroll-behavior: smooth;
                ",
                // 安定したスクロールイベント処理
                onscroll: move |_| {
                    // デバウンス付きスクロール検出
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
                                
                                const wasUserScrolled = window.liscovUserScrolled;
                                window.liscovUserScrolled = !isAtBottom;
                                
                                // デバッグ情報
                                if (wasUserScrolled !== !isAtBottom) {
                                    console.log('📍 Scroll state changed:', isAtBottom ? 'At bottom' : 'User scrolled up');
                                }
                            }
                            "#,
                        );

                        // Rust側の状態も更新
                        if let Ok(result) = dioxus::document::eval("window.liscovUserScrolled || false").await {
                            if let Some(scrolled) = result.as_bool() {
                                if scrolled != *user_has_scrolled.read() {
                                    user_has_scrolled.set(scrolled);
                                }
                            }
                        }
                    });
                },

                if filtered_messages.read().is_empty() {
                    div {
                        class: CssClasses::NO_MESSAGES,
                        style: "
                            text-align: center;
                            padding: 40px 20px;
                            color: #888;
                            font-size: 16px;
                        ",
                        if live_chat_handle.messages.read().is_empty() {
                            "💬 メッセージがまだありません"
                            br {}
                            "接続を開始してライブチャットを監視しましょう！"
                        } else {
                            "🔍 フィルター条件に一致するメッセージがありません"
                            br {}
                            "フィルター設定を調整してください"
                        }
                    }
                                } else {
                    for (index, message) in filtered_messages.read().iter().enumerate() {
                        div {
                            key: "{index}",
                            class: "message-item",
                            style: {
                                // メッセージのユニークIDを生成してハイライト判定
                                let message_id = format!("{}:{}:{}",
                                                        message.timestamp,
                                                        message.author,
                                                        message.content.chars().take(20).collect::<String>());
                                let is_highlighted = recent_messages.read().contains(&message_id);

                                format!(
                                "
                                padding: 16px 20px;
                                margin-bottom: 12px;
                                border-radius: 12px;
                                background: {};
                                    border: 1px solid {};
                                    box-shadow: {};
                                    transition: all 0.3s ease;
                                position: relative;
                                border-left: 4px solid {};
                                cursor: default;
                                ",
                                    // 新着メッセージハイライト（修正版）
                                    if is_highlighted {
                                        "linear-gradient(135deg, #dbeafe, #bfdbfe)"
                                    } else if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() {
                                        "linear-gradient(135deg, #fbbf24, #f59e0b)"
                                    } else if message.is_member {
                                        "#f0f9ff"
                                    } else if index % 2 == 0 { "#ffffff" } else { "#f8fafc" },

                                    // ボーダー色
                                    if is_highlighted { "#3b82f6" }
                                    else if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() { "#f59e0b" }
                                    else if message.is_member { "#0ea5e9" }
                                    else { "#e2e8f0" },

                                    // シャドウ効果
                                    if is_highlighted { "0 4px 12px rgba(59, 130, 246, 0.15)" }
                                    else if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() { "0 4px 15px rgba(245, 158, 11, 0.15)" }
                                    else { "0 2px 4px rgba(0, 0, 0, 0.05)" },

                                    // 左ボーダー色
                                    if message.metadata.is_some() && message.metadata.as_ref().unwrap().amount.is_some() { "#f59e0b" }
                                    else if message.is_member { "#0ea5e9" }
                                    else { "#10b981" }
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

                            // メッセージのメタ情報
                            div {
                                style: "
                                    display: flex;
                                    justify-content: space-between;
                                    align-items: center;
                                    margin-bottom: 8px;
                                    padding-bottom: 8px;
                                    border-bottom: 1px solid #f1f5f9;
                                ",

                                // 作者名
                                span {
                                    style: "
                                        font-weight: 600;
                                        color: #374151;
                                        font-size: 14px;
                                    ",
                                    "{message.author}"
                                }

                                // タイムスタンプ
                                if *show_timestamps.read() {
                                    span {
                                        style: "
                                            font-size: 12px;
                                            color: #6b7280;
                                            background: #f8fafc;
                                            padding: 2px 8px;
                                            border-radius: 8px;
                                        ",
                                        "{message.timestamp}"
                                    }
                                }
                            }

                            // メッセージ本文
                            div {
                                style: "
                                    color: #1f2937;
                                    line-height: 1.5;
                                    word-wrap: break-word;
                                    margin-bottom: 8px;
                                ",
                                "{message.content}"
                            }

                            // SuperChat表示（強調）
                            if let Some(metadata) = &message.metadata {
                                if let Some(amount_str) = &metadata.amount {
                                    div {
                                        style: "
                                            margin-top: 8px;
                                            padding: 8px 12px;
                                            background: linear-gradient(135deg, #fbbf24, #f59e0b);
                                            color: white;
                                            font-weight: 700;
                                            font-size: 13px;
                                            border-radius: 8px;
                                            text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
                                        ",
                                        "💰 SuperChat: {amount_str}"
                                    }
                                }
                            }
                        }
                    }

                    // 最後のメッセージの後の余白（自動スクロール時の見切れ防止）
                    div {
                        style: "height: 20px; flex-shrink: 0;",
                    }
                }
            }

            // チャットフッター（デバッグ機能強化版）
            div {
                class: CssClasses::CHAT_FOOTER,
                style: "
                    flex-shrink: 0;
                    border-top: 1px solid #e2e8f0;
                    padding: 12px 16px;
                    background: #f8fafc;
                ",

                // 上段：統計情報
                div {
                    class: CssClasses::FOOTER_STATS,
                    style: "margin-bottom: 8px;",
                    span {
                        if global_filter.read().is_active() {
                            "{filtered_messages.read().len()} / {live_chat_handle.messages.read().len()} メッセージ"
                        } else {
                            "{live_chat_handle.messages.read().len()} メッセージ"
                        }
                    }
                    span {
                        style: "margin-left: 16px;",
                        if *live_chat_handle.is_connected.read() {
                            "🔄 受信中"
                        } else {
                            "⏸️ 停止中"
                        }
                    }

                    // 新着メッセージカウンター（簡略化版）
                    if recent_messages.read().len() > 0 {
                        span {
                            style: "
                                margin-left: 16px; 
                                font-size: 12px; 
                                color: white;
                                background: linear-gradient(135deg, #3b82f6, #1d4ed8);
                                padding: 4px 8px;
                                border-radius: 12px;
                                font-weight: 600;
                                box-shadow: 0 2px 8px rgba(59, 130, 246, 0.3);
                            ",
                            "✨ ハイライト中: {recent_messages.read().len()}"
                        }
                    }

                    // デバッグ情報
                    span {
                        style: "margin-left: 16px; font-size: 11px; color: #666;",
                        {format!("自動スクロール: {} | ユーザー操作: {} | ハイライト: {}",
                                if *auto_scroll.read() { "ON" } else { "OFF" },
                                if *user_has_scrolled.read() { "有" } else { "無" },
                                if *highlight_enabled.read() { "ON" } else { "OFF" })}
                    }
                }

                // 下段：操作ボタン
                div {
                    style: "display: flex; gap: 8px; align-items: center;",

                    // テストメッセージ追加
                    button {
                        style: "
                            padding: 6px 12px;
                            background: #10b981;
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 12px;
                            cursor: pointer;
                            transition: background 0.2s;
                        ",
                                                                        onclick: {
                                                        let handle = live_chat_handle.clone();
                            let auto_scroll_signal = auto_scroll;
                            let user_has_scrolled_signal = user_has_scrolled;
                            let last_count_signal = last_message_count;
                            move |_| {
                                // 既存のadd_test_messageメソッドを使用
                                let msg_count_before = handle.messages.read().len();
                                let last_count_before = *last_count_signal.read();

                                handle.add_test_message(
                                    "テストユーザー",
                                    &format!("テストメッセージ #{}", msg_count_before + 1),
                                    crate::gui::models::MessageType::Text
                                );

                                let msg_count_after = handle.messages.read().len();
                                tracing::info!("🧪 Test message added. Before: {}, After: {}, Last count: {}, Auto-scroll: {}, User scrolled: {}",
                                             msg_count_before, msg_count_after, last_count_before,
                                             *auto_scroll_signal.read(), *user_has_scrolled_signal.read());
                            }
                        },
                        "🧪 テスト"
                    }

                    // 強制スクロール
                    button {
                        style: "
                            padding: 6px 12px;
                            background: #3b82f6;
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 12px;
                            cursor: pointer;
                            transition: background 0.2s;
                        ",
                        onclick: move |_| {
                            spawn(async move {
                                let _ = dioxus::document::eval(
                                    r#"
                                    const container = document.getElementById('liscov-message-list');
                                    if (container) {
                                        window.liscovUserScrolled = false;
                                        container.scrollTop = container.scrollHeight;
                                        console.log('🔧 Force scroll executed. Height:', container.scrollHeight);
                                    }
                                    "#,
                                );
                            });
                        },
                        "🔧 強制スクロール"
                    }

                    // スクロール状態リセット
                    button {
                        style: "
                            padding: 6px 12px;
                            background: #f59e0b;
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 12px;
                            cursor: pointer;
                            transition: background 0.2s;
                        ",
                        onclick: {
                            let mut user_scrolled_signal = user_has_scrolled;
                            move |_| {
                                user_scrolled_signal.set(false);
                                spawn(async move {
                                    let _ = dioxus::document::eval(
                                        r#"
                                        window.liscovUserScrolled = false;
                                        console.log('🔄 Scroll state reset');
                                        "#,
                                    );
                                });
                            }
                        },
                        "🔄 状態リセット"
                    }

                    // ハイライト設定トグル
                    button {
                        style: format!(
                            "
                            padding: 6px 12px;
                            background: {};
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 12px;
                            cursor: pointer;
                            transition: background 0.2s;
                            ",
                            if *highlight_enabled.read() { "#10b981" } else { "#6b7280" }
                        ),
                        onclick: {
                            let mut highlight_signal = highlight_enabled;
                            move |_| {
                                let new_state = !*highlight_signal.read();
                                highlight_signal.set(new_state);
                                tracing::info!("✨ Highlight mode changed: {}", new_state);
                            }
                        },
                        if *highlight_enabled.read() { "✨ ハイライト ON" } else { "⚫ ハイライト OFF" }
                    }
                }

                // バージョン情報
                div {
                    style: "font-size: 10px; opacity: 0.7; margin-top: 4px;",
                    "Powered by Dioxus 0.6.3 • Auto-scroll v2.0"
                }
            }
        }
    }
}
