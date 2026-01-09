//! 視聴者編集モーダル

use dioxus::prelude::*;

use crate::database::{
    delete_broadcaster_data, delete_viewer_data, update_viewer_profile_metadata,
    upsert_viewer_custom_info, ViewerCustomInfo, ViewerWithCustomInfo,
};

/// 視聴者編集モーダルのProps
#[derive(Props, Clone, PartialEq)]
pub struct ViewerEditModalProps {
    /// 編集対象の視聴者情報
    pub viewer: ViewerWithCustomInfo,
    /// 配信者チャンネルID
    pub broadcaster_id: String,
    /// 保存完了時のコールバック
    pub on_save: EventHandler<()>,
    /// キャンセル時のコールバック
    pub on_close: EventHandler<()>,
}

/// 視聴者編集モーダル
#[component]
pub fn ViewerEditModal(props: ViewerEditModalProps) -> Element {
    let viewer = props.viewer.clone();

    // フォーム状態
    let mut reading = use_signal(|| viewer.reading.clone().unwrap_or_default());
    let mut notes = use_signal(|| viewer.notes.clone().unwrap_or_default());
    let mut tags_input = use_signal(|| viewer.tags.join(", "));
    let mut membership_level =
        use_signal(|| viewer.membership_level.clone().unwrap_or_default());

    // UI状態
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);

    let broadcaster_id = props.broadcaster_id.clone();
    let viewer_channel_id = viewer.channel_id.clone();
    let on_save = props.on_save.clone();

    // 保存処理
    let handle_save = move |_| {
        let broadcaster_id = broadcaster_id.clone();
        let viewer_channel_id = viewer_channel_id.clone();
        let reading_val = reading.read().clone();
        let notes_val = notes.read().clone();
        let tags_val = tags_input.read().clone();
        let membership_val = membership_level.read().clone();
        let on_save = on_save.clone();

        spawn(async move {
            is_saving.set(true);
            error_message.set(None);
            success_message.set(None);

            match crate::database::get_connection().await {
                Ok(conn) => {
                    // viewer_custom_info を更新
                    let custom_info = ViewerCustomInfo {
                        id: None,
                        broadcaster_channel_id: broadcaster_id.clone(),
                        viewer_channel_id: viewer_channel_id.clone(),
                        reading: if reading_val.is_empty() {
                            None
                        } else {
                            Some(reading_val)
                        },
                        notes: if notes_val.is_empty() {
                            None
                        } else {
                            Some(notes_val)
                        },
                        custom_data: None,
                        created_at: None,
                        updated_at: None,
                    };

                    if let Err(e) = upsert_viewer_custom_info(&conn, &custom_info) {
                        error_message.set(Some(format!("カスタム情報の保存に失敗: {}", e)));
                        is_saving.set(false);
                        return;
                    }

                    // タグをパース
                    let tags: Vec<String> = tags_val
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    // viewer_profiles を更新
                    let membership_opt = if membership_val.is_empty() {
                        None
                    } else {
                        Some(membership_val.as_str())
                    };

                    if let Err(e) = update_viewer_profile_metadata(
                        &conn,
                        &viewer_channel_id,
                        Some(&tags),
                        membership_opt,
                    ) {
                        error_message.set(Some(format!("プロフィールの保存に失敗: {}", e)));
                        is_saving.set(false);
                        return;
                    }

                    success_message.set(Some("保存しました".to_string()));

                    // 少し待ってからコールバック
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    on_save.call(());
                }
                Err(e) => {
                    error_message.set(Some(format!("DB接続エラー: {}", e)));
                }
            }

            is_saving.set(false);
        });
    };

    rsx! {
        // オーバーレイ
        div {
            class: "modal-overlay",
            style: "
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: rgba(0, 0, 0, 0.5);
                display: flex;
                justify-content: center;
                align-items: center;
                z-index: 1000;
            ",
            onclick: {
                let on_close = props.on_close.clone();
                move |_| on_close.call(())
            },

            // モーダル本体
            div {
                class: "modal-content",
                style: "
                    background: white;
                    border-radius: 12px;
                    padding: 24px;
                    min-width: 500px;
                    max-width: 600px;
                    max-height: 90vh;
                    overflow-y: auto;
                    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
                ",
                onclick: |e| e.stop_propagation(),

                // ヘッダー
                div {
                    style: "
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                        margin-bottom: 20px;
                        padding-bottom: 16px;
                        border-bottom: 1px solid #e2e8f0;
                    ",

                    h2 {
                        style: "margin: 0; font-size: 20px; color: #1e293b;",
                        "👤 視聴者情報の編集"
                    }

                    button {
                        style: "
                            background: none;
                            border: none;
                            font-size: 24px;
                            cursor: pointer;
                            color: #94a3b8;
                            padding: 4px;
                        ",
                        onclick: {
                            let on_close = props.on_close.clone();
                            move |_| on_close.call(())
                        },
                        "×"
                    }
                }

                // 視聴者基本情報（読み取り専用）
                div {
                    style: "
                        background: #f8fafc;
                        border-radius: 8px;
                        padding: 16px;
                        margin-bottom: 20px;
                    ",

                    div {
                        style: "font-weight: 600; font-size: 16px; margin-bottom: 8px;",
                        "{props.viewer.display_name}"
                    }

                    div {
                        style: "font-size: 12px; color: #64748b;",
                        "チャンネルID: {props.viewer.channel_id}"
                    }

                    div {
                        style: "
                            display: flex;
                            gap: 16px;
                            margin-top: 12px;
                            font-size: 13px;
                            color: #64748b;
                        ",
                        span { "メッセージ数: {props.viewer.message_count}" }
                        span { "貢献額: ¥{props.viewer.total_contribution:.0}" }
                    }
                }

                // エラーメッセージ
                if let Some(err) = error_message.read().as_ref() {
                    div {
                        style: "
                            padding: 12px;
                            background: #fef2f2;
                            border: 1px solid #fecaca;
                            border-radius: 8px;
                            color: #dc2626;
                            margin-bottom: 16px;
                        ",
                        "{err}"
                    }
                }

                // 成功メッセージ
                if let Some(msg) = success_message.read().as_ref() {
                    div {
                        style: "
                            padding: 12px;
                            background: #f0fdf4;
                            border: 1px solid #bbf7d0;
                            border-radius: 8px;
                            color: #16a34a;
                            margin-bottom: 16px;
                        ",
                        "{msg}"
                    }
                }

                // フォーム
                div {
                    style: "display: flex; flex-direction: column; gap: 16px;",

                    // 読み仮名
                    div {
                        label {
                            style: "
                                display: block;
                                font-weight: 500;
                                color: #374151;
                                margin-bottom: 6px;
                                font-size: 14px;
                            ",
                            "読み仮名"
                        }
                        input {
                            style: "
                                width: 100%;
                                padding: 10px 12px;
                                border: 2px solid #e5e7eb;
                                border-radius: 8px;
                                font-size: 14px;
                                transition: border-color 0.2s;
                                box-sizing: border-box;
                            ",
                            r#type: "text",
                            placeholder: "例: やまだたろう",
                            value: "{reading}",
                            oninput: move |e| reading.set(e.value()),
                        }
                        div {
                            style: "font-size: 11px; color: #94a3b8; margin-top: 4px;",
                            "TTS読み上げ時に使用されます"
                        }
                    }

                    // メモ
                    div {
                        label {
                            style: "
                                display: block;
                                font-weight: 500;
                                color: #374151;
                                margin-bottom: 6px;
                                font-size: 14px;
                            ",
                            "メモ"
                        }
                        textarea {
                            style: "
                                width: 100%;
                                padding: 10px 12px;
                                border: 2px solid #e5e7eb;
                                border-radius: 8px;
                                font-size: 14px;
                                min-height: 80px;
                                resize: vertical;
                                box-sizing: border-box;
                            ",
                            placeholder: "この視聴者に関するメモ",
                            value: "{notes}",
                            oninput: move |e| notes.set(e.value()),
                        }
                    }

                    // タグ
                    div {
                        label {
                            style: "
                                display: block;
                                font-weight: 500;
                                color: #374151;
                                margin-bottom: 6px;
                                font-size: 14px;
                            ",
                            "タグ（カンマ区切り）"
                        }
                        input {
                            style: "
                                width: 100%;
                                padding: 10px 12px;
                                border: 2px solid #e5e7eb;
                                border-radius: 8px;
                                font-size: 14px;
                                box-sizing: border-box;
                            ",
                            r#type: "text",
                            placeholder: "例: 常連, VIP, 要注意",
                            value: "{tags_input}",
                            oninput: move |e| tags_input.set(e.value()),
                        }
                    }

                    // メンバーシップレベル
                    div {
                        label {
                            style: "
                                display: block;
                                font-weight: 500;
                                color: #374151;
                                margin-bottom: 6px;
                                font-size: 14px;
                            ",
                            "メンバーシップレベル"
                        }
                        input {
                            style: "
                                width: 100%;
                                padding: 10px 12px;
                                border: 2px solid #e5e7eb;
                                border-radius: 8px;
                                font-size: 14px;
                                box-sizing: border-box;
                            ",
                            r#type: "text",
                            placeholder: "例: Gold, Silver",
                            value: "{membership_level}",
                            oninput: move |e| membership_level.set(e.value()),
                        }
                    }
                }

                // アクションボタン
                div {
                    style: "
                        display: flex;
                        justify-content: flex-end;
                        gap: 12px;
                        margin-top: 24px;
                        padding-top: 16px;
                        border-top: 1px solid #e2e8f0;
                    ",

                    button {
                        style: "
                            padding: 10px 20px;
                            border: 1px solid #e2e8f0;
                            border-radius: 8px;
                            background: white;
                            color: #64748b;
                            cursor: pointer;
                            font-size: 14px;
                        ",
                        onclick: {
                            let on_close = props.on_close.clone();
                            move |_| on_close.call(())
                        },
                        "キャンセル"
                    }

                    button {
                        style: "
                            padding: 10px 20px;
                            border: none;
                            border-radius: 8px;
                            background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
                            color: white;
                            cursor: pointer;
                            font-size: 14px;
                            font-weight: 500;
                        ",
                        disabled: is_saving(),
                        onclick: handle_save,
                        if is_saving() { "保存中..." } else { "保存" }
                    }
                }
            }
        }
    }
}

/// 削除確認ダイアログのProps
#[derive(Props, Clone, PartialEq)]
pub struct DeleteConfirmDialogProps {
    /// 削除対象の視聴者情報
    pub viewer: ViewerWithCustomInfo,
    /// 配信者チャンネルID
    pub broadcaster_id: String,
    /// 削除完了時のコールバック
    pub on_confirm: EventHandler<()>,
    /// キャンセル時のコールバック
    pub on_cancel: EventHandler<()>,
}

/// 削除確認ダイアログ
#[component]
pub fn DeleteConfirmDialog(props: DeleteConfirmDialogProps) -> Element {
    let mut is_deleting = use_signal(|| false);
    let mut delete_profile = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    let broadcaster_id = props.broadcaster_id.clone();
    let viewer_channel_id = props.viewer.channel_id.clone();
    let on_confirm = props.on_confirm.clone();

    let handle_delete = move |_| {
        let broadcaster_id = broadcaster_id.clone();
        let viewer_channel_id = viewer_channel_id.clone();
        let should_delete_profile = delete_profile();
        let on_confirm = on_confirm.clone();

        spawn(async move {
            is_deleting.set(true);
            error_message.set(None);

            match crate::database::get_connection().await {
                Ok(conn) => {
                    match delete_viewer_data(
                        &conn,
                        &broadcaster_id,
                        &viewer_channel_id,
                        should_delete_profile,
                    ) {
                        Ok(_) => {
                            on_confirm.call(());
                        }
                        Err(e) => {
                            error_message.set(Some(format!("削除に失敗: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    error_message.set(Some(format!("DB接続エラー: {}", e)));
                }
            }

            is_deleting.set(false);
        });
    };

    rsx! {
        // オーバーレイ
        div {
            class: "modal-overlay",
            style: "
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: rgba(0, 0, 0, 0.5);
                display: flex;
                justify-content: center;
                align-items: center;
                z-index: 1000;
            ",
            onclick: {
                let on_cancel = props.on_cancel.clone();
                move |_| on_cancel.call(())
            },

            // ダイアログ本体
            div {
                class: "confirm-dialog",
                style: "
                    background: white;
                    border-radius: 12px;
                    padding: 24px;
                    min-width: 400px;
                    max-width: 500px;
                    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
                ",
                onclick: |e| e.stop_propagation(),

                // ヘッダー
                div {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 12px;
                        margin-bottom: 16px;
                    ",

                    span { style: "font-size: 32px;", "⚠️" }

                    h3 {
                        style: "margin: 0; font-size: 18px; color: #dc2626;",
                        "削除の確認"
                    }
                }

                // メッセージ
                p {
                    style: "color: #374151; margin-bottom: 16px;",
                    "「{props.viewer.display_name}」のデータを削除しますか？"
                }

                // エラーメッセージ
                if let Some(err) = error_message.read().as_ref() {
                    div {
                        style: "
                            padding: 12px;
                            background: #fef2f2;
                            border: 1px solid #fecaca;
                            border-radius: 8px;
                            color: #dc2626;
                            margin-bottom: 16px;
                        ",
                        "{err}"
                    }
                }

                // プロフィールも削除するかどうか
                div {
                    style: "
                        background: #fff7ed;
                        border: 1px solid #fed7aa;
                        border-radius: 8px;
                        padding: 12px;
                        margin-bottom: 20px;
                    ",

                    label {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 8px;
                            cursor: pointer;
                        ",

                        input {
                            r#type: "checkbox",
                            checked: delete_profile(),
                            onchange: move |e| delete_profile.set(e.checked()),
                        }

                        span {
                            style: "font-size: 14px; color: #9a3412;",
                            "全データを削除（プロフィール情報も含む）"
                        }
                    }

                    div {
                        style: "font-size: 12px; color: #c2410c; margin-top: 8px; margin-left: 24px;",
                        "※ チェックしない場合、この配信者のカスタム情報のみ削除されます"
                    }
                }

                // アクションボタン
                div {
                    style: "
                        display: flex;
                        justify-content: flex-end;
                        gap: 12px;
                    ",

                    button {
                        style: "
                            padding: 10px 20px;
                            border: 1px solid #e2e8f0;
                            border-radius: 8px;
                            background: white;
                            color: #64748b;
                            cursor: pointer;
                            font-size: 14px;
                        ",
                        onclick: {
                            let on_cancel = props.on_cancel.clone();
                            move |_| on_cancel.call(())
                        },
                        "キャンセル"
                    }

                    button {
                        style: "
                            padding: 10px 20px;
                            border: none;
                            border-radius: 8px;
                            background: #dc2626;
                            color: white;
                            cursor: pointer;
                            font-size: 14px;
                            font-weight: 500;
                        ",
                        disabled: is_deleting(),
                        onclick: handle_delete,
                        if is_deleting() { "削除中..." } else { "削除" }
                    }
                }
            }
        }
    }
}

/// 配信者削除確認ダイアログのProps
#[derive(Props, Clone, PartialEq)]
pub struct BroadcasterDeleteConfirmDialogProps {
    /// 配信者チャンネルID
    pub broadcaster_id: String,
    /// 配信者表示名
    pub broadcaster_name: String,
    /// 関連する視聴者数
    pub viewer_count: usize,
    /// 削除完了時のコールバック
    pub on_confirm: EventHandler<()>,
    /// キャンセル時のコールバック
    pub on_cancel: EventHandler<()>,
}

/// 配信者削除確認ダイアログ
#[component]
pub fn BroadcasterDeleteConfirmDialog(props: BroadcasterDeleteConfirmDialogProps) -> Element {
    let mut is_deleting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    let broadcaster_id = props.broadcaster_id.clone();
    let on_confirm = props.on_confirm.clone();

    let handle_delete = move |_| {
        let broadcaster_id = broadcaster_id.clone();
        let on_confirm = on_confirm.clone();

        spawn(async move {
            is_deleting.set(true);
            error_message.set(None);

            match crate::database::get_connection().await {
                Ok(conn) => match delete_broadcaster_data(&conn, &broadcaster_id) {
                    Ok((broadcaster_deleted, viewers_deleted)) => {
                        tracing::info!(
                            "🗑️ Broadcaster deleted: {}, viewers: {}",
                            broadcaster_deleted,
                            viewers_deleted
                        );
                        on_confirm.call(());
                    }
                    Err(e) => {
                        error_message.set(Some(format!("削除に失敗: {}", e)));
                    }
                },
                Err(e) => {
                    error_message.set(Some(format!("DB接続エラー: {}", e)));
                }
            }

            is_deleting.set(false);
        });
    };

    rsx! {
        // オーバーレイ
        div {
            class: "modal-overlay",
            style: "
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: rgba(0, 0, 0, 0.5);
                display: flex;
                justify-content: center;
                align-items: center;
                z-index: 1000;
            ",
            onclick: {
                let on_cancel = props.on_cancel.clone();
                move |_| on_cancel.call(())
            },

            // ダイアログ本体
            div {
                class: "confirm-dialog",
                style: "
                    background: white;
                    border-radius: 12px;
                    padding: 24px;
                    min-width: 400px;
                    max-width: 500px;
                    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
                ",
                onclick: |e| e.stop_propagation(),

                // ヘッダー
                div {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 12px;
                        margin-bottom: 16px;
                    ",

                    span { style: "font-size: 32px;", "⚠️" }

                    h3 {
                        style: "margin: 0; font-size: 18px; color: #dc2626;",
                        "配信者データの削除"
                    }
                }

                // メッセージ
                p {
                    style: "color: #374151; margin-bottom: 16px;",
                    "「{props.broadcaster_name}」のデータを削除しますか？"
                }

                // 警告メッセージ
                div {
                    style: "
                        background: #fef2f2;
                        border: 1px solid #fecaca;
                        border-radius: 8px;
                        padding: 16px;
                        margin-bottom: 20px;
                    ",

                    div {
                        style: "
                            font-weight: 600;
                            color: #dc2626;
                            margin-bottom: 8px;
                            display: flex;
                            align-items: center;
                            gap: 8px;
                        ",
                        "🚨 この操作は取り消せません"
                    }

                    div {
                        style: "font-size: 14px; color: #991b1b;",
                        "以下のデータが削除されます："
                    }

                    ul {
                        style: "
                            margin: 8px 0 0 0;
                            padding-left: 20px;
                            font-size: 14px;
                            color: #991b1b;
                        ",

                        li { "配信者プロフィール情報" }
                        li {
                            "この配信者に紐づく視聴者カスタム情報（{props.viewer_count}件）"
                        }
                    }
                }

                // エラーメッセージ
                if let Some(err) = error_message.read().as_ref() {
                    div {
                        style: "
                            padding: 12px;
                            background: #fef2f2;
                            border: 1px solid #fecaca;
                            border-radius: 8px;
                            color: #dc2626;
                            margin-bottom: 16px;
                        ",
                        "{err}"
                    }
                }

                // アクションボタン
                div {
                    style: "
                        display: flex;
                        justify-content: flex-end;
                        gap: 12px;
                    ",

                    button {
                        style: "
                            padding: 10px 20px;
                            border: 1px solid #e2e8f0;
                            border-radius: 8px;
                            background: white;
                            color: #64748b;
                            cursor: pointer;
                            font-size: 14px;
                        ",
                        onclick: {
                            let on_cancel = props.on_cancel.clone();
                            move |_| on_cancel.call(())
                        },
                        "キャンセル"
                    }

                    button {
                        style: "
                            padding: 10px 20px;
                            border: none;
                            border-radius: 8px;
                            background: #dc2626;
                            color: white;
                            cursor: pointer;
                            font-size: 14px;
                            font-weight: 500;
                        ",
                        disabled: is_deleting(),
                        onclick: handle_delete,
                        if is_deleting() { "削除中..." } else { "削除" }
                    }
                }
            }
        }
    }
}
