use crate::chat_management::{MessageFilter, MessageType};
use dioxus::prelude::*;

/// フィルターパネルコンポーネント
#[component]
pub fn FilterPanel(
    filter: Signal<MessageFilter>,
    on_filter_change: EventHandler<MessageFilter>,
) -> Element {
    let mut show_advanced = use_signal(|| false);
    let mut keyword_input = use_signal(|| String::new());
    let mut author_input = use_signal(|| String::new());
    let mut min_amount = use_signal(|| String::new());
    let mut max_amount = use_signal(|| String::new());
    let mut start_date = use_signal(|| String::new());
    let mut end_date = use_signal(|| String::new());
    let mut min_length = use_signal(|| String::new());
    let mut max_length = use_signal(|| String::new());

    // フィルター状態をローカル状態に同期
    let current_filter = filter.read().clone();

    // パフォーマンス最適化：メッセージタイプのチェック状態を事前計算
    let message_type_checked = use_memo(move || {
        let filter = filter.read();
        [
            (
                MessageType::Regular,
                "💬 通常",
                "regular",
                filter.message_types.contains(&MessageType::Regular),
            ),
            (
                MessageType::SuperChat,
                "💰 Super Chat",
                "superchat",
                filter.message_types.contains(&MessageType::SuperChat),
            ),
            (
                MessageType::Membership,
                "⭐ メンバー",
                "membership",
                filter.message_types.contains(&MessageType::Membership),
            ),
            (
                MessageType::Question,
                "❓ 質問",
                "question",
                filter.message_types.contains(&MessageType::Question),
            ),
            (
                MessageType::Emoji,
                "😊 絵文字",
                "emoji",
                filter.message_types.contains(&MessageType::Emoji),
            ),
            (
                MessageType::Link,
                "🔗 リンク",
                "link",
                filter.message_types.contains(&MessageType::Link),
            ),
        ]
    });

    rsx! {
        div {
            class: "filter-panel bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 mb-4",

            // フィルターヘッダー
            div {
                class: "flex items-center justify-between mb-4",
                h3 {
                    class: "text-lg font-semibold text-gray-900 dark:text-white",
                    "🔍 メッセージフィルター"
                }
                div {
                    class: "flex items-center gap-2",
                    span {
                        class: "text-sm text-gray-500 dark:text-gray-400",
                        "アクティブ: {current_filter.active_filter_count()}"
                    }
                    button {
                        class: "px-3 py-1 bg-gray-500 hover:bg-gray-600 text-white rounded text-sm",
                        onclick: move |_| {
                            let new_filter = MessageFilter::new();
                            on_filter_change.call(new_filter);
                            keyword_input.set(String::new());
                            author_input.set(String::new());
                            min_amount.set(String::new());
                            max_amount.set(String::new());
                            start_date.set(String::new());
                            end_date.set(String::new());
                            min_length.set(String::new());
                            max_length.set(String::new());
                        },
                        "リセット"
                    }
                }
            }

            // 基本フィルター
            div {
                class: "space-y-4",

                // キーワード検索
                div {
                    class: "space-y-2",
                    label {
                        class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                        "🔎 キーワード検索"
                    }
                    div {
                        class: "flex gap-2",
                        input {
                            class: "flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white",
                            r#type: "text",
                            placeholder: "メッセージ内容を検索...",
                            value: "{keyword_input.read()}",
                            oninput: move |event| keyword_input.set(event.value())
                        }
                        button {
                            class: "px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded",
                            onclick: move |_| {
                                let keyword = keyword_input.read().trim().to_string();
                                if !keyword.is_empty() {
                                    let mut new_filter = filter.read().clone();
                                    new_filter.add_keyword(keyword.clone());
                                    on_filter_change.call(new_filter);
                                    keyword_input.set(String::new());
                                }
                            },
                            "追加"
                        }
                    }
                    // アクティブなキーワード一覧
                    if !current_filter.content_keywords.is_empty() {
                        div {
                            class: "flex flex-wrap gap-2",
                            for keyword in current_filter.content_keywords.iter() {
                                span {
                                    class: "inline-flex items-center gap-1 px-2 py-1 bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 rounded",
                                    "{keyword}"
                                    button {
                                        class: "text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-200",
                                        onclick: {
                                            let keyword = keyword.clone();
                                            move |_| {
                                                let mut new_filter = filter.read().clone();
                                                new_filter.remove_keyword(&keyword);
                                                on_filter_change.call(new_filter);
                                            }
                                        },
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }

                // 作者フィルター
                div {
                    class: "space-y-2",
                    label {
                        class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                        "👤 作者フィルター"
                    }
                    div {
                        class: "flex gap-2",
                        input {
                            class: "flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white",
                            r#type: "text",
                            placeholder: "作者名を入力...",
                            value: "{author_input.read()}",
                            oninput: move |event| author_input.set(event.value())
                        }
                        button {
                            class: "px-4 py-2 bg-green-500 hover:bg-green-600 text-white rounded",
                            onclick: move |_| {
                                let author = author_input.read().trim().to_string();
                                if !author.is_empty() {
                                    let mut new_filter = filter.read().clone();
                                    new_filter.add_author(author.clone());
                                    on_filter_change.call(new_filter);
                                    author_input.set(String::new());
                                }
                            },
                            "追加"
                        }
                    }
                    // アクティブな作者一覧
                    if !current_filter.author_filter.is_empty() {
                        div {
                            class: "flex flex-wrap gap-2",
                            for author in current_filter.author_filter.iter() {
                                span {
                                    class: "inline-flex items-center gap-1 px-2 py-1 bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200 rounded",
                                    "{author}"
                                    button {
                                        class: "text-green-600 dark:text-green-400 hover:text-green-800 dark:hover:text-green-200",
                                        onclick: {
                                            let author = author.clone();
                                            move |_| {
                                                let mut new_filter = filter.read().clone();
                                                new_filter.remove_author(&author);
                                                on_filter_change.call(new_filter);
                                            }
                                        },
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }

                // メッセージタイプフィルター
                div {
                    class: "space-y-2",
                    label {
                        class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                        "📝 メッセージタイプ"
                    }
                    div {
                        class: "grid grid-cols-2 md:grid-cols-3 gap-2",
                        for (message_type, label, _key, is_checked) in message_type_checked.read().iter() {
                            label {
                                class: "flex items-center space-x-2 cursor-pointer",
                                input {
                                    r#type: "checkbox",
                                    class: "rounded",
                                    checked: *is_checked,
                                    onchange: {
                                        let msg_type = message_type.clone();
                                        move |event: Event<FormData>| {
                                            let mut new_filter = filter.read().clone();
                                            if event.checked() {
                                                new_filter.message_types.insert(msg_type.clone());
                                            } else {
                                                new_filter.message_types.remove(&msg_type);
                                            }
                                            on_filter_change.call(new_filter);
                                        }
                                    }
                                }
                                span {
                                    class: "text-sm text-gray-700 dark:text-gray-300",
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }

            // 高度なフィルター
            div {
                class: "mt-4 border-t border-gray-200 dark:border-gray-600 pt-4",
                button {
                    class: "flex items-center gap-2 text-sm text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-200",
                    onclick: move |_| {
                        let current_value = *show_advanced.read();
                        show_advanced.set(!current_value);
                    },
                    if *show_advanced.read() {
                        "🔽 高度なフィルターを非表示"
                    } else {
                        "🔼 高度なフィルターを表示"
                    }
                }

                if *show_advanced.read() {
                    div {
                        class: "mt-4 space-y-4",

                        // 金額範囲フィルター
                        div {
                            class: "space-y-2",
                            label {
                                class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                                "💰 Super Chat金額範囲"
                            }
                            div {
                                class: "grid grid-cols-2 gap-2",
                                input {
                                    class: "px-3 py-2 border border-gray-300 dark:border-gray-600 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white",
                                    r#type: "number",
                                    placeholder: "最小金額",
                                    value: "{min_amount.read()}",
                                    oninput: move |event| min_amount.set(event.value())
                                }
                                input {
                                    class: "px-3 py-2 border border-gray-300 dark:border-gray-600 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white",
                                    r#type: "number",
                                    placeholder: "最大金額",
                                    value: "{max_amount.read()}",
                                    oninput: move |event| max_amount.set(event.value())
                                }
                            }
                            button {
                                class: "w-full px-4 py-2 bg-yellow-500 hover:bg-yellow-600 text-white rounded",
                                onclick: move |_| {
                                    let min = min_amount.read().parse::<f64>().ok();
                                    let max = max_amount.read().parse::<f64>().ok();
                                    let mut new_filter = filter.read().clone();
                                    new_filter.set_amount_range_detailed(min, max);
                                    on_filter_change.call(new_filter);
                                },
                                "金額範囲を適用"
                            }
                        }

                        // 時間範囲フィルター
                        div {
                            class: "space-y-2",
                            label {
                                class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                                "⏰ 時間範囲"
                            }
                            div {
                                class: "grid grid-cols-2 gap-2",
                                input {
                                    class: "px-3 py-2 border border-gray-300 dark:border-gray-600 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white",
                                    r#type: "datetime-local",
                                    value: "{start_date.read()}",
                                    oninput: move |event| start_date.set(event.value())
                                }
                                input {
                                    class: "px-3 py-2 border border-gray-300 dark:border-gray-600 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white",
                                    r#type: "datetime-local",
                                    value: "{end_date.read()}",
                                    oninput: move |event| end_date.set(event.value())
                                }
                            }
                            button {
                                class: "w-full px-4 py-2 bg-purple-500 hover:bg-purple-600 text-white rounded",
                                onclick: move |_| {
                                    // 時間範囲パース処理は実装を簡略化
                                    // 本格的な実装では適切な日時パースが必要
                                    tracing::info!("時間範囲フィルター適用: {} - {}", start_date.read(), end_date.read());
                                },
                                "時間範囲を適用"
                            }
                        }

                        // メッセージ長フィルター
                        div {
                            class: "space-y-2",
                            label {
                                class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                                "📏 メッセージ長"
                            }
                            div {
                                class: "grid grid-cols-2 gap-2",
                                input {
                                    class: "px-3 py-2 border border-gray-300 dark:border-gray-600 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white",
                                    r#type: "number",
                                    placeholder: "最小文字数",
                                    value: "{min_length.read()}",
                                    oninput: move |event| min_length.set(event.value())
                                }
                                input {
                                    class: "px-3 py-2 border border-gray-300 dark:border-gray-600 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white",
                                    r#type: "number",
                                    placeholder: "最大文字数",
                                    value: "{max_length.read()}",
                                    oninput: move |event| max_length.set(event.value())
                                }
                            }
                            button {
                                class: "w-full px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white rounded",
                                onclick: move |_| {
                                    let min = min_length.read().parse::<usize>().ok();
                                    let max = max_length.read().parse::<usize>().ok();
                                    let mut new_filter = filter.read().clone();
                                    new_filter.set_message_length_range(min, max);
                                    on_filter_change.call(new_filter);
                                },
                                "文字数範囲を適用"
                            }
                        }

                        // メンバーシップフィルター
                        div {
                            class: "space-y-2",
                            label {
                                class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                                "⭐ メンバーシップ"
                            }
                            div {
                                class: "flex gap-4",
                                label {
                                    class: "flex items-center space-x-2 cursor-pointer",
                                    input {
                                        r#type: "radio",
                                        name: "membership",
                                        value: "all",
                                        checked: current_filter.membership_filter.is_none(),
                                        onchange: move |_| {
                                            let mut new_filter = filter.read().clone();
                                            new_filter.membership_filter = None;
                                            on_filter_change.call(new_filter);
                                        }
                                    }
                                    span { class: "text-sm", "すべて" }
                                }
                                label {
                                    class: "flex items-center space-x-2 cursor-pointer",
                                    input {
                                        r#type: "radio",
                                        name: "membership",
                                        value: "members",
                                        checked: current_filter.membership_filter == Some(true),
                                        onchange: move |_| {
                                            let mut new_filter = filter.read().clone();
                                            new_filter.membership_filter = Some(true);
                                            on_filter_change.call(new_filter);
                                        }
                                    }
                                    span { class: "text-sm", "メンバーのみ" }
                                }
                                label {
                                    class: "flex items-center space-x-2 cursor-pointer",
                                    input {
                                        r#type: "radio",
                                        name: "membership",
                                        value: "non_members",
                                        checked: current_filter.membership_filter == Some(false),
                                        onchange: move |_| {
                                            let mut new_filter = filter.read().clone();
                                            new_filter.membership_filter = Some(false);
                                            on_filter_change.call(new_filter);
                                        }
                                    }
                                    span { class: "text-sm", "非メンバーのみ" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
