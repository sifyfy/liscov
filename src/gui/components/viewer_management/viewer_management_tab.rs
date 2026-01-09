//! 視聴者管理タブのメインコンポーネント

use dioxus::prelude::*;

use crate::database::ViewerWithCustomInfo;

use super::broadcaster_selector::BroadcasterSelector;
use super::viewer_edit_modal::{BroadcasterDeleteConfirmDialog, DeleteConfirmDialog, ViewerEditModal};
use super::viewer_list::ViewerList;

/// 配信者削除対象の情報
#[derive(Clone, PartialEq)]
struct BroadcasterDeleteTarget {
    channel_id: String,
    name: String,
    viewer_count: usize,
}

/// 視聴者管理タブ
#[component]
pub fn ViewerManagementTab() -> Element {
    // 選択状態
    let mut selected_broadcaster = use_signal(|| None::<String>);
    let mut search_query = use_signal(|| String::new());

    // モーダル状態
    let mut editing_viewer = use_signal(|| None::<ViewerWithCustomInfo>);
    let mut delete_target = use_signal(|| None::<ViewerWithCustomInfo>);
    let mut broadcaster_delete_target = use_signal(|| None::<BroadcasterDeleteTarget>);

    // リロードトリガー
    let mut reload_trigger = use_signal(|| 0u32);

    // 編集ハンドラ
    let on_edit = move |viewer: ViewerWithCustomInfo| {
        editing_viewer.set(Some(viewer));
    };

    // 削除ハンドラ
    let on_delete = move |viewer: ViewerWithCustomInfo| {
        delete_target.set(Some(viewer));
    };

    // 保存完了ハンドラ
    let on_save_complete = move |_| {
        editing_viewer.set(None);
        reload_trigger.set(reload_trigger() + 1);
    };

    // 削除完了ハンドラ
    let on_delete_complete = move |_| {
        delete_target.set(None);
        reload_trigger.set(reload_trigger() + 1);
    };

    // 配信者削除ボタンクリックハンドラ
    let on_broadcaster_delete_click = move |(channel_id, name, viewer_count): (String, String, usize)| {
        broadcaster_delete_target.set(Some(BroadcasterDeleteTarget {
            channel_id,
            name,
            viewer_count,
        }));
    };

    // 配信者削除完了ハンドラ
    let on_broadcaster_delete_complete = move |_| {
        broadcaster_delete_target.set(None);
        selected_broadcaster.set(None);
        reload_trigger.set(reload_trigger() + 1);
    };

    rsx! {
        div {
            class: "viewer-management-tab",
            style: "
                display: flex;
                flex-direction: column;
                height: 100%;
                width: 100%;
                padding: 20px;
                box-sizing: border-box;
            ",

            // ヘッダー
            div {
                class: "header",
                style: "margin-bottom: 20px;",

                h2 {
                    style: "
                        font-size: 24px;
                        color: #1e293b;
                        margin: 0 0 8px 0;
                        display: flex;
                        align-items: center;
                        gap: 12px;
                    ",
                    "👥 視聴者管理"
                }

                p {
                    style: "color: #64748b; margin: 0; font-size: 14px;",
                    "配信者チャンネルごとに視聴者データを一覧・編集できます"
                }
            }

            // 配信者選択エリア
            div {
                class: "controls",
                style: "margin-bottom: 20px;",

                BroadcasterSelector {
                    selected: selected_broadcaster.clone(),
                    on_change: move |id| {
                        selected_broadcaster.set(id);
                        search_query.set(String::new());
                    },
                    on_delete_click: on_broadcaster_delete_click,
                    external_reload_trigger: Some(reload_trigger.clone()),
                }
            }

            // メインコンテンツ
            div {
                class: "content",
                style: "flex: 1; min-height: 0; width: 100%;",

                if let Some(broadcaster_id) = selected_broadcaster.read().clone() {
                    // 視聴者一覧を表示（reload_trigger で再取得）
                    div {
                        key: "{reload_trigger}",
                        style: "height: 100%;",
                        ViewerList {
                            broadcaster_id: broadcaster_id.clone(),
                            search_query: search_query.clone(),
                            on_edit: on_edit,
                            on_delete: on_delete,
                        }
                    }
                } else {
                    // 配信者未選択時のプレースホルダー
                    div {
                        style: "
                            display: flex;
                            flex-direction: column;
                            justify-content: center;
                            align-items: center;
                            height: 100%;
                            color: #94a3b8;
                        ",

                        div { style: "font-size: 64px; margin-bottom: 16px;", "📋" }

                        div {
                            style: "font-size: 18px; margin-bottom: 8px;",
                            "配信者チャンネルを選択してください"
                        }

                        div {
                            style: "font-size: 14px;",
                            "上のドロップダウンから配信者を選択すると、視聴者一覧が表示されます"
                        }
                    }
                }
            }

            // 編集モーダル
            if let Some(viewer) = editing_viewer.read().clone() {
                if let Some(broadcaster_id) = selected_broadcaster.read().clone() {
                    ViewerEditModal {
                        viewer: viewer,
                        broadcaster_id: broadcaster_id,
                        on_save: on_save_complete,
                        on_close: move |_| editing_viewer.set(None),
                    }
                }
            }

            // 削除確認ダイアログ
            if let Some(viewer) = delete_target.read().clone() {
                if let Some(broadcaster_id) = selected_broadcaster.read().clone() {
                    DeleteConfirmDialog {
                        viewer: viewer,
                        broadcaster_id: broadcaster_id,
                        on_confirm: on_delete_complete,
                        on_cancel: move |_| delete_target.set(None),
                    }
                }
            }

            // 配信者削除確認ダイアログ
            if let Some(target) = broadcaster_delete_target.read().clone() {
                BroadcasterDeleteConfirmDialog {
                    broadcaster_id: target.channel_id,
                    broadcaster_name: target.name,
                    viewer_count: target.viewer_count,
                    on_confirm: on_broadcaster_delete_complete,
                    on_cancel: move |_| broadcaster_delete_target.set(None),
                }
            }
        }
    }
}
