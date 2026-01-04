//! 視聴者情報パネルコンポーネント
//!
//! コメントクリック時にスライドインで表示され、
//! 視聴者の読み仮名などのカスタム情報を編集できる。

use dioxus::prelude::*;

use crate::database::ViewerCustomInfo;
use crate::gui::hooks::use_live_chat::LiveChatHandle;
use crate::gui::models::{GuiChatMessage, SelectedViewer};

/// 視聴者情報パネルのプロパティ
#[derive(Props, Clone, PartialEq)]
pub struct ViewerInfoPanelProps {
    /// 選択された視聴者情報
    pub selected_viewer: SelectedViewer,
    /// パネルを閉じるハンドラ
    pub on_close: EventHandler<()>,
    /// LiveChatハンドル（視聴者情報の更新に使用）
    pub live_chat_handle: LiveChatHandle,
    /// コメント選択時のハンドラ（メッセージIDを返す）
    #[props(default)]
    pub on_message_select: Option<EventHandler<GuiChatMessage>>,
}

/// 視聴者情報パネルコンポーネント
#[component]
pub fn ViewerInfoPanel(props: ViewerInfoPanelProps) -> Element {
    // 選択された視聴者の読み仮名を取得
    let current_reading = props
        .selected_viewer
        .custom_info
        .as_ref()
        .and_then(|info| info.reading.clone())
        .unwrap_or_default();

    // 現在の視聴者チャンネルID
    let current_viewer_id = props.selected_viewer.viewer_channel_id.clone();

    // 前回の視聴者チャンネルIDを追跡（視聴者が変わったか検知）
    let mut prev_viewer_id = use_signal(|| current_viewer_id.clone());

    // 読み仮名の入力状態
    let mut reading_input = use_signal(|| current_reading.clone());

    // 視聴者が変わった場合に入力欄を更新
    if *prev_viewer_id.read() != current_viewer_id {
        reading_input.set(current_reading.clone());
        prev_viewer_id.set(current_viewer_id.clone());
    }

    // 保存中フラグ
    let mut is_saving = use_signal(|| false);

    // 保存成功メッセージ
    let mut save_message = use_signal(|| None::<String>);

    // 保存処理
    let handle_save = {
        let viewer = props.selected_viewer.clone();
        let live_chat_handle = props.live_chat_handle.clone();
        move |_| {
            let reading = reading_input.read().trim().to_string();
            let reading_opt = if reading.is_empty() {
                None
            } else {
                Some(reading)
            };

            // ViewerCustomInfoを作成
            let info = ViewerCustomInfo {
                id: None,
                broadcaster_channel_id: viewer.broadcaster_channel_id.clone(),
                viewer_channel_id: viewer.viewer_channel_id.clone(),
                reading: reading_opt,
                notes: None,
                custom_data: None,
                created_at: None,
                updated_at: None,
            };

            // 保存中フラグを設定
            is_saving.set(true);
            save_message.set(None);

            // LiveChatHandle経由で保存
            live_chat_handle.update_viewer_info(info);

            // 保存完了（非同期だが即座にUI更新）
            is_saving.set(false);
            save_message.set(Some("保存しました".to_string()));

            // 3秒後にメッセージをクリア
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                save_message.set(None);
            });
        }
    };

    rsx! {
        // スライドインパネル（モードレス - オーバーレイなし）
        div {
            style: "
                position: fixed;
                right: 0;
                top: 0;
                height: 100%;
                width: 320px;
                background-color: #2d2d3d;
                box-shadow: -4px 0 12px rgba(0, 0, 0, 0.3);
                z-index: 1000;
                overflow-y: auto;
                animation: slideIn 0.25s ease-out;
            ",

            // ヘッダー
            div {
                style: "
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                    padding: 16px 20px;
                    border-bottom: 1px solid #555;
                    background-color: #363648;
                ",
                h2 {
                    style: "font-size: 20px; font-weight: 600; color: #fff; margin: 0;",
                    "視聴者情報"
                }
                button {
                    style: "
                        padding: 8px 14px;
                        background: #555;
                        border: none;
                        border-radius: 4px;
                        color: #fff;
                        cursor: pointer;
                        font-size: 16px;
                    ",
                    onclick: move |_| props.on_close.call(()),
                    title: "閉じる",
                    "✕"
                }
            }

            // 視聴者情報セクション
            div {
                style: "padding: 20px;",

                // アイコンと名前
                div {
                    style: "display: flex; align-items: center; gap: 14px; margin-bottom: 16px;",
                    // アイコン
                    if let Some(icon_url) = &props.selected_viewer.message.author_icon_url {
                        img {
                            style: "width: 56px; height: 56px; border-radius: 50%;",
                            src: "{icon_url}",
                            alt: "視聴者アイコン",
                        }
                    } else {
                        div {
                            style: "
                                width: 56px;
                                height: 56px;
                                border-radius: 50%;
                                background-color: #555;
                                display: flex;
                                align-items: center;
                                justify-content: center;
                                font-size: 28px;
                            ",
                            "👤"
                        }
                    }
                    div {
                        p {
                            style: "font-size: 18px; font-weight: 600; color: #fff; margin: 0 0 4px 0;",
                            "{props.selected_viewer.display_name}"
                        }
                        if let Some(reading) = props.selected_viewer.reading() {
                            p {
                                style: "font-size: 16px; color: #a0e0ff; margin: 0;",
                                "({reading})"
                            }
                        }
                    }
                }

                // チャンネルID
                div {
                    style: "font-size: 13px; color: #aaa; word-break: break-all; margin-bottom: 20px;",
                    "Channel ID: {props.selected_viewer.viewer_channel_id}"
                }

                // 区切り線
                hr { style: "border: none; border-top: 1px solid #555; margin: 20px 0;" }

                // 読み仮名入力
                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; font-size: 16px; font-weight: 600; color: #fff; margin-bottom: 10px;",
                        "読み仮名（ふりがな）"
                    }
                    input {
                        style: "
                            width: 100%;
                            padding: 12px 14px;
                            border: 1px solid #666;
                            border-radius: 6px;
                            background-color: #454558;
                            color: #fff;
                            font-size: 16px;
                            box-sizing: border-box;
                        ",
                        r#type: "text",
                        placeholder: "例: やまだ たろう",
                        value: "{reading_input}",
                        oninput: move |e| reading_input.set(e.value()),
                    }
                    p {
                        style: "font-size: 14px; color: #bbb; margin-top: 8px;",
                        "視聴者名の横に括弧書きで表示されます"
                    }
                }

                // 保存ボタン
                div {
                    style: "display: flex; align-items: center; gap: 12px; margin-bottom: 20px;",
                    button {
                        style: "
                            flex: 1;
                            padding: 12px 20px;
                            background-color: #5865f2;
                            border: none;
                            border-radius: 6px;
                            color: #fff;
                            font-size: 16px;
                            font-weight: 600;
                            cursor: pointer;
                        ",
                        disabled: *is_saving.read(),
                        onclick: handle_save,
                        if *is_saving.read() {
                            "保存中..."
                        } else {
                            "保存"
                        }
                    }
                    if let Some(msg) = save_message.read().as_ref() {
                        span {
                            style: "font-size: 15px; color: #4ade80; font-weight: 500;",
                            "{msg}"
                        }
                    }
                }

                // 区切り線
                hr { style: "border: none; border-top: 1px solid #555; margin: 20px 0;" }

                // 投稿されたコメント一覧
                div {
                    // この視聴者のコメントをフィルタリング
                    {
                        let viewer_channel_id = props.selected_viewer.viewer_channel_id.clone();
                        let clicked_message_id = props.selected_viewer.message.id.clone();
                        let all_messages = props.live_chat_handle.messages.read();
                        let viewer_messages: Vec<_> = all_messages
                            .iter()
                            .filter(|m| m.channel_id == viewer_channel_id)
                            .collect();
                        let message_count = viewer_messages.len();

                        rsx! {
                            h3 {
                                style: "font-size: 16px; font-weight: 600; color: #fff; margin: 0 0 12px 0;",
                                "投稿されたコメント ({message_count}件/新着順)"
                            }
                            div {
                                style: "
                                    max-height: 300px;
                                    overflow-y: auto;
                                    display: flex;
                                    flex-direction: column;
                                    gap: 8px;
                                ",
                                for message in viewer_messages.iter().rev() {
                                    {
                                        let is_clicked = message.id == clicked_message_id;
                                        let border_style = if is_clicked {
                                            "border: 2px solid #5865f2; box-shadow: 0 0 6px rgba(88, 101, 242, 0.4);"
                                        } else {
                                            "border: 1px solid #555;"
                                        };

                                        // クリックハンドラ用にメッセージをクローン
                                        let message_for_click = (*message).clone();
                                        let on_select = props.on_message_select.clone();

                                        rsx! {
                                            div {
                                                key: "{message.id}",
                                                style: "
                                                    padding: 12px;
                                                    background-color: #454558;
                                                    border-radius: 8px;
                                                    cursor: pointer;
                                                    transition: background-color 0.15s;
                                                    {border_style}
                                                ",
                                                onclick: move |_| {
                                                    if let Some(handler) = &on_select {
                                                        handler.call(message_for_click.clone());
                                                    }
                                                },
                                                // タイムスタンプ
                                                p {
                                                    style: "font-size: 13px; color: #aaa; margin: 0 0 6px 0;",
                                                    "{message.timestamp}"
                                                }
                                                // メッセージ内容
                                                p {
                                                    style: "font-size: 15px; color: #fff; margin: 0; word-break: break-word; line-height: 1.4;",
                                                    "{message.content}"
                                                }
                                                // メッセージタイプバッジ
                                                {render_message_type_badge(message)}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // CSS アニメーション
        style {
            r#"
            @keyframes slideIn {{
                from {{
                    transform: translateX(100%);
                }}
                to {{
                    transform: translateX(0);
                }}
            }}
            "#
        }
    }
}

/// メッセージタイプに応じたバッジを描画
fn render_message_type_badge(message: &GuiChatMessage) -> Element {
    let (badge_text, badge_style) = match &message.message_type {
        crate::gui::models::MessageType::Text => return rsx! {},
        crate::gui::models::MessageType::SuperChat { amount } => {
            (format!("💰 {}", amount), "background-color: #fef3c7; color: #92400e;")
        }
        crate::gui::models::MessageType::SuperSticker { amount } => {
            (format!("🎨 {}", amount), "background-color: #ede9fe; color: #6b21a8;")
        }
        crate::gui::models::MessageType::Membership { milestone_months } => {
            match milestone_months {
                Some(months) => (format!("🎉 {}ヶ月継続", months), "background-color: #dbeafe; color: #1e40af;"),
                None => ("⭐ 新規メンバー".to_string(), "background-color: #dcfce7; color: #166534;"),
            }
        }
        crate::gui::models::MessageType::MembershipGift { gift_count } => {
            (format!("🎁 {}件ギフト", gift_count), "background-color: #fce7f3; color: #9d174d;")
        }
        crate::gui::models::MessageType::System => {
            ("ℹ️ システム".to_string(), "background-color: #4d4d5d; color: #ccc;")
        }
    };

    rsx! {
        span {
            style: "
                display: inline-block;
                margin-top: 8px;
                padding: 4px 8px;
                font-size: 12px;
                border-radius: 4px;
                {badge_style}
            ",
            "{badge_text}"
        }
    }
}
