//! 視聴者一覧テーブル

use dioxus::prelude::*;

use crate::database::{
    get_viewer_count_for_broadcaster, get_viewers_for_broadcaster, ViewerWithCustomInfo,
};

/// 視聴者一覧コンポーネントのProps
#[derive(Props, Clone, PartialEq)]
pub struct ViewerListProps {
    /// 配信者チャンネルID
    pub broadcaster_id: String,
    /// 検索クエリ
    pub search_query: Signal<String>,
    /// 編集ボタンクリック時のコールバック
    pub on_edit: EventHandler<ViewerWithCustomInfo>,
    /// 削除ボタンクリック時のコールバック
    pub on_delete: EventHandler<ViewerWithCustomInfo>,
}

const PAGE_SIZE: usize = 50;

/// 視聴者一覧テーブル
#[component]
pub fn ViewerList(props: ViewerListProps) -> Element {
    let mut page = use_signal(|| 0usize);
    let mut is_loading = use_signal(|| false);
    let mut viewers = use_signal(Vec::<ViewerWithCustomInfo>::new);
    let mut total_count = use_signal(|| 0usize);
    let mut error_message = use_signal(|| None::<String>);
    let mut reload_trigger = use_signal(|| 0u32);

    let broadcaster_id = props.broadcaster_id.clone();
    let mut search_query = props.search_query.clone();

    // データ取得エフェクト
    use_effect(move || {
        let broadcaster_id = broadcaster_id.clone();
        let search = search_query.read().clone();
        let _trigger = reload_trigger(); // reload_trigger を依存関係に含める

        spawn(async move {
            is_loading.set(true);
            error_message.set(None);

            match crate::database::get_connection().await {
                Ok(conn) => {
                    // 総件数取得
                    match get_viewer_count_for_broadcaster(&conn, &broadcaster_id) {
                        Ok(count) => total_count.set(count),
                        Err(e) => {
                            error_message.set(Some(format!("件数取得エラー: {}", e)));
                        }
                    }

                    // 視聴者一覧取得
                    let search_opt = if search.is_empty() {
                        None
                    } else {
                        Some(search.as_str())
                    };

                    match get_viewers_for_broadcaster(
                        &conn,
                        &broadcaster_id,
                        search_opt,
                        PAGE_SIZE,
                        page() * PAGE_SIZE,
                    ) {
                        Ok(data) => {
                            viewers.set(data);
                        }
                        Err(e) => {
                            error_message.set(Some(format!("データ取得エラー: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    error_message.set(Some(format!("DB接続エラー: {}", e)));
                }
            }

            is_loading.set(false);
        });
    });

    let total_pages = (total_count() + PAGE_SIZE - 1) / PAGE_SIZE.max(1);

    // 更新ボタンのクリックハンドラ
    let on_refresh_click = move |_| {
        reload_trigger.set(reload_trigger() + 1);
        tracing::info!("🔄 Viewer list refresh triggered");
    };

    rsx! {
        div {
            class: "viewer-list",
            style: "display: flex; flex-direction: column; height: 100%; width: 100%;",

            // ヘッダー情報（件数 + 検索ボックス）
            div {
                style: "
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 12px;
                    padding: 8px 12px;
                    background: #f8fafc;
                    border-radius: 8px;
                    gap: 16px;
                ",

                // 左側: 件数とページ情報 + 更新ボタン
                div {
                    style: "display: flex; align-items: center; gap: 16px;",

                    span {
                        style: "font-size: 14px; color: #64748b; font-weight: 500;",
                        "全 {total_count()} 件"
                    }

                    if total_pages > 1 {
                        span {
                            style: "font-size: 14px; color: #94a3b8;",
                            "ページ {page() + 1} / {total_pages}"
                        }
                    }

                    // 更新ボタン
                    button {
                        style: "
                            padding: 4px 10px;
                            border: 1px solid #e5e7eb;
                            border-radius: 6px;
                            background: white;
                            color: #64748b;
                            cursor: pointer;
                            font-size: 12px;
                            display: flex;
                            align-items: center;
                            gap: 4px;
                            transition: all 0.2s;
                        ",
                        disabled: is_loading(),
                        onclick: on_refresh_click,
                        if is_loading() {
                            "🔄 読込中..."
                        } else {
                            "🔄 リスト更新"
                        }
                    }
                }

                // 右側: 検索ボックス
                div {
                    style: "position: relative; min-width: 200px; max-width: 300px;",

                    input {
                        style: "
                            width: 100%;
                            padding: 6px 10px 6px 32px;
                            border: 1px solid #e2e8f0;
                            border-radius: 6px;
                            font-size: 13px;
                            box-sizing: border-box;
                            background: white;
                        ",
                        r#type: "text",
                        placeholder: "名前・読み仮名・メモで検索...",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                    }

                    span {
                        style: "
                            position: absolute;
                            left: 10px;
                            top: 50%;
                            transform: translateY(-50%);
                            color: #94a3b8;
                            font-size: 12px;
                        ",
                        "🔍"
                    }

                    // クリアボタン
                    if !search_query.read().is_empty() {
                        button {
                            style: "
                                position: absolute;
                                right: 6px;
                                top: 50%;
                                transform: translateY(-50%);
                                background: none;
                                border: none;
                                color: #94a3b8;
                                cursor: pointer;
                                padding: 2px;
                                font-size: 12px;
                            ",
                            onclick: move |_| search_query.set(String::new()),
                            "✕"
                        }
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
                        margin-bottom: 12px;
                    ",
                    "{err}"
                }
            }

            // ローディング
            if is_loading() {
                div {
                    style: "
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        padding: 40px;
                        color: #64748b;
                    ",
                    "読み込み中..."
                }
            } else if viewers.read().is_empty() {
                div {
                    style: "
                        display: flex;
                        flex-direction: column;
                        justify-content: center;
                        align-items: center;
                        padding: 40px;
                        color: #94a3b8;
                    ",
                    div { style: "font-size: 48px; margin-bottom: 16px;", "📭" }
                    div { "視聴者データがありません" }
                }
            } else {
                // テーブル
                div {
                    style: "
                        flex: 1;
                        overflow-y: auto;
                        overflow-x: hidden;
                        border: 1px solid #e2e8f0;
                        border-radius: 8px;
                        width: 100%;
                    ",

                    table {
                        style: "
                            width: 100%;
                            border-collapse: collapse;
                            font-size: 13px;
                            table-layout: fixed;
                        ",

                        thead {
                            style: "
                                position: sticky;
                                top: 0;
                                background: #f8fafc;
                                z-index: 1;
                            ",
                            tr {
                                // 表示名: 広め
                                th { style: "padding: 12px 8px; text-align: center; border-bottom: 2px solid #e2e8f0; font-weight: 600; width: 30%;", "表示名" }
                                // 読み仮名: 中程度
                                th { style: "padding: 12px 8px; text-align: center; border-bottom: 2px solid #e2e8f0; font-weight: 600; width: 18%;", "読み仮名" }
                                // メッセージ数: ヘッダーが改行しない幅
                                th { style: "padding: 12px 8px; text-align: center; border-bottom: 2px solid #e2e8f0; font-weight: 600; width: 100px; white-space: nowrap;", "メッセージ数" }
                                // タグ: 残りスペース
                                th { style: "padding: 12px 8px; text-align: center; border-bottom: 2px solid #e2e8f0; font-weight: 600;", "タグ" }
                                // 操作: 固定幅（ボタン2つが横並びになる幅）
                                th { style: "padding: 12px 8px; text-align: center; border-bottom: 2px solid #e2e8f0; font-weight: 600; width: 150px;", "操作" }
                            }
                        }

                        tbody {
                            for viewer in viewers.read().iter() {
                                ViewerRow {
                                    key: "{viewer.channel_id}",
                                    viewer: viewer.clone(),
                                    on_edit: props.on_edit.clone(),
                                    on_delete: props.on_delete.clone(),
                                }
                            }
                        }
                    }
                }

                // ページネーション
                if total_pages > 1 {
                    div {
                        style: "
                            display: flex;
                            justify-content: center;
                            gap: 8px;
                            margin-top: 16px;
                        ",

                        button {
                            style: "
                                padding: 8px 16px;
                                border: 1px solid #e2e8f0;
                                border-radius: 6px;
                                background: white;
                                cursor: pointer;
                                transition: all 0.2s;
                            ",
                            disabled: page() == 0,
                            onclick: move |_| page.set(page().saturating_sub(1)),
                            "← 前へ"
                        }

                        span {
                            style: "
                                display: flex;
                                align-items: center;
                                padding: 8px 16px;
                                color: #64748b;
                            ",
                            "{page() + 1} / {total_pages}"
                        }

                        button {
                            style: "
                                padding: 8px 16px;
                                border: 1px solid #e2e8f0;
                                border-radius: 6px;
                                background: white;
                                cursor: pointer;
                                transition: all 0.2s;
                            ",
                            disabled: page() + 1 >= total_pages,
                            onclick: move |_| page.set(page() + 1),
                            "次へ →"
                        }
                    }
                }
            }
        }
    }
}

/// 視聴者行コンポーネントのProps
#[derive(Props, Clone, PartialEq)]
struct ViewerRowProps {
    viewer: ViewerWithCustomInfo,
    on_edit: EventHandler<ViewerWithCustomInfo>,
    on_delete: EventHandler<ViewerWithCustomInfo>,
}

/// 視聴者行コンポーネント
#[component]
fn ViewerRow(props: ViewerRowProps) -> Element {
    let viewer = props.viewer.clone();

    rsx! {
        tr {
            style: "
                border-bottom: 1px solid #f1f5f9;
                transition: background 0.2s;
            ",

            // 表示名
            td {
                style: "padding: 10px 8px; overflow: hidden;",
                div {
                    style: "font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                    title: "{viewer.display_name}",
                    "{viewer.display_name}"
                }
                div {
                    style: "font-size: 11px; color: #94a3b8; margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                    "{truncate_id(&viewer.channel_id)}"
                }
            }

            // 読み仮名
            td {
                style: "padding: 10px 8px; color: #64748b; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                title: "{viewer.reading.as_deref().unwrap_or(\"\")}",
                {viewer.reading.as_deref().unwrap_or("-")}
            }

            // メッセージ数
            td {
                style: "padding: 10px 8px; text-align: right; font-family: monospace;",
                "{viewer.message_count}"
            }

            // タグ
            td {
                style: "padding: 10px 8px;",
                div {
                    style: "display: flex; flex-wrap: wrap; gap: 4px;",
                    for tag in viewer.tags.iter() {
                        span {
                            key: "{tag}",
                            style: "
                                padding: 2px 8px;
                                background: #e0f2fe;
                                color: #0369a1;
                                border-radius: 12px;
                                font-size: 11px;
                            ",
                            "{tag}"
                        }
                    }
                }
            }

            // 操作
            td {
                style: "padding: 10px 8px; text-align: center;",
                div {
                    style: "display: flex; justify-content: center; gap: 8px;",

                    button {
                        style: "
                            padding: 6px 12px;
                            border: 1px solid #3b82f6;
                            border-radius: 6px;
                            background: white;
                            color: #3b82f6;
                            cursor: pointer;
                            font-size: 12px;
                            transition: all 0.2s;
                        ",
                        onclick: {
                            let viewer = props.viewer.clone();
                            move |_| props.on_edit.call(viewer.clone())
                        },
                        "編集"
                    }

                    button {
                        style: "
                            padding: 6px 12px;
                            border: 1px solid #ef4444;
                            border-radius: 6px;
                            background: white;
                            color: #ef4444;
                            cursor: pointer;
                            font-size: 12px;
                            transition: all 0.2s;
                        ",
                        onclick: {
                            let viewer = props.viewer.clone();
                            move |_| props.on_delete.call(viewer.clone())
                        },
                        "削除"
                    }
                }
            }
        }
    }
}

/// IDを短縮表示
fn truncate_id(id: &str) -> String {
    if id.len() > 16 {
        format!("{}...", &id[..13])
    } else {
        id.to_string()
    }
}
