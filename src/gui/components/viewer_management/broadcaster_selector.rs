//! 配信者チャンネル選択ドロップダウン

use dioxus::prelude::*;

use crate::database::{get_distinct_broadcaster_channels, BroadcasterChannel};

/// バックアップ結果
#[derive(Clone, PartialEq)]
pub enum BackupResult {
    Success(String),
    Error(String),
}

/// 配信者選択コンポーネントのProps
#[derive(Props, Clone, PartialEq)]
pub struct BroadcasterSelectorProps {
    /// 選択中の配信者チャンネルID
    pub selected: Signal<Option<String>>,
    /// 選択変更時のコールバック
    pub on_change: EventHandler<Option<String>>,
    /// 削除ボタンクリック時のコールバック（配信者チャンネルID, 配信者名, 視聴者数）
    #[props(optional)]
    pub on_delete_click: Option<EventHandler<(String, String, usize)>>,
    /// 外部からのリロードトリガー
    #[props(optional)]
    pub external_reload_trigger: Option<Signal<u32>>,
}

/// 配信者チャンネル選択ドロップダウン
#[component]
pub fn BroadcasterSelector(props: BroadcasterSelectorProps) -> Element {
    // リロードトリガー
    let mut reload_trigger = use_signal(|| 0u32);
    let mut is_refreshing = use_signal(|| false);
    // 初回マウント済みフラグ（無限ループ防止）
    let mut has_mounted = use_signal(|| false);
    // ハンバーガーメニューの開閉状態
    let mut menu_open = use_signal(|| false);
    // バックアップ結果表示
    let mut backup_result = use_signal(|| None::<BackupResult>);
    let mut is_backing_up = use_signal(|| false);

    // コンポーネントマウント時（タブ表示時）に配信者一覧を更新
    use_effect(move || {
        // 初回マウント時のみリロードをトリガー
        if !has_mounted() {
            has_mounted.set(true);
            reload_trigger.set(reload_trigger() + 1);
            tracing::info!("📋 BroadcasterSelector mounted - refreshing broadcaster list");
        }
    });

    // 配信者一覧を非同期で取得
    let external_trigger = props.external_reload_trigger.clone();
    let broadcasters = use_resource(move || async move {
        // reload_trigger を参照して再取得をトリガー
        let _ = reload_trigger();
        // external_reload_trigger も参照
        if let Some(ref ext) = external_trigger {
            let _ = ext();
        }
        match crate::database::get_connection().await {
            Ok(conn) => get_distinct_broadcaster_channels(&conn).unwrap_or_default(),
            Err(e) => {
                tracing::error!("Failed to get database connection: {}", e);
                Vec::new()
            }
        }
    });

    let selected_value = props.selected.read().clone().unwrap_or_default();

    // 更新ボタンのクリックハンドラ
    let on_refresh_click = move |_| {
        is_refreshing.set(true);
        let selected_id = props.selected.read().clone();

        spawn(async move {
            // 選択中の配信者がいる場合、YouTubeから最新情報を取得
            if let Some(channel_id) = selected_id {
                match fetch_broadcaster_info_from_youtube(&channel_id).await {
                    Ok(Some((name, handle))) => {
                        // DBに保存
                        if let Ok(conn) = crate::database::get_connection().await {
                            let profile = crate::database::BroadcasterProfile {
                                channel_id: channel_id.clone(),
                                channel_name: name.clone(),
                                handle: handle.clone(),
                                thumbnail_url: None,
                                created_at: None,
                                updated_at: None,
                            };
                            match crate::database::upsert_broadcaster_profile(&conn, &profile) {
                                Ok(_) => {
                                    tracing::info!(
                                        "🔄 Updated broadcaster profile: {} ({:?})",
                                        channel_id,
                                        name
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!("⚠️ Failed to update broadcaster profile: {}", e);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("⚠️ Could not fetch broadcaster info for {}", channel_id);
                    }
                    Err(e) => {
                        tracing::error!("❌ Error fetching broadcaster info: {}", e);
                    }
                }
            }

            // リロードをトリガー
            reload_trigger.set(reload_trigger() + 1);
            is_refreshing.set(false);
        });
    };

    // 選択中の配信者情報を取得
    let selected_broadcaster_info = {
        let selected_id = props.selected.read().clone();
        let broadcasters_data = broadcasters.read();
        if let (Some(id), Some(channels)) = (selected_id.as_ref(), broadcasters_data.as_ref()) {
            channels.iter().find(|c| &c.channel_id == id).cloned()
        } else {
            None
        }
    };

    // 削除ボタンのクリックハンドラ
    let on_delete_click_handler = {
        let on_delete = props.on_delete_click.clone();
        let selected_info = selected_broadcaster_info.clone();
        move |_| {
            if let (Some(handler), Some(info)) = (on_delete.as_ref(), selected_info.as_ref()) {
                let name = format_broadcaster_display(info);
                handler.call((info.channel_id.clone(), name, info.viewer_count));
            }
        }
    };

    rsx! {
        div {
            class: "broadcaster-selector",
            style: "margin-bottom: 16px;",

            div {
                style: "
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 8px;
                ",

                label {
                    style: "
                        font-weight: 600;
                        color: #374151;
                        font-size: 14px;
                    ",
                    "配信者チャンネル"
                }

                div {
                    style: "display: flex; gap: 8px; align-items: center;",

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
                        disabled: is_refreshing(),
                        onclick: on_refresh_click,
                        if is_refreshing() {
                            "🔄 更新中..."
                        } else {
                            "🔄 情報を更新"
                        }
                    }

                    // ハンバーガーメニュー（常に表示）
                    div {
                        style: "position: relative;",

                        // メニューボタン
                        button {
                            style: "
                                padding: 4px 8px;
                                border: 1px solid #e5e7eb;
                                border-radius: 6px;
                                background: white;
                                color: #64748b;
                                cursor: pointer;
                                font-size: 16px;
                                display: flex;
                                align-items: center;
                                justify-content: center;
                                transition: all 0.2s;
                                min-width: 32px;
                            ",
                            onclick: move |_| menu_open.set(!menu_open()),
                            "⋮"
                        }

                        // ドロップダウンメニュー
                        if menu_open() {
                            // オーバーレイ（メニュー外クリックで閉じる）
                            div {
                                style: "
                                    position: fixed;
                                    top: 0;
                                    left: 0;
                                    right: 0;
                                    bottom: 0;
                                    z-index: 999;
                                ",
                                onclick: move |_| menu_open.set(false),
                            }

                            // メニュー本体
                            div {
                                style: "
                                    position: absolute;
                                    top: 100%;
                                    right: 0;
                                    margin-top: 4px;
                                    background: white;
                                    border: 1px solid #e5e7eb;
                                    border-radius: 8px;
                                    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
                                    z-index: 1000;
                                    min-width: 180px;
                                    overflow: hidden;
                                ",

                                // バックアップ作成ボタン
                                button {
                                    class: "menu-item-backup",
                                    style: "
                                        width: 100%;
                                        padding: 10px 16px;
                                        border: none;
                                        background: white;
                                        color: #374151;
                                        cursor: pointer;
                                        font-size: 13px;
                                        display: flex;
                                        align-items: center;
                                        gap: 8px;
                                        text-align: left;
                                    ",
                                    disabled: is_backing_up(),
                                    onclick: move |_| {
                                        menu_open.set(false);
                                        is_backing_up.set(true);
                                        spawn(async move {
                                            match crate::database::create_backup() {
                                                Ok(path) => {
                                                    let path_str = path.display().to_string();
                                                    tracing::info!("📦 Backup created: {}", path_str);
                                                    backup_result.set(Some(BackupResult::Success(path_str)));
                                                }
                                                Err(e) => {
                                                    tracing::error!("❌ Backup failed: {}", e);
                                                    backup_result.set(Some(BackupResult::Error(e.to_string())));
                                                }
                                            }
                                            is_backing_up.set(false);
                                        });
                                    },
                                    span { style: "font-size: 14px;", "📦" }
                                    span {
                                        if is_backing_up() {
                                            "バックアップ作成中..."
                                        } else {
                                            "バックアップを作成"
                                        }
                                    }
                                }

                                // 削除ボタン（配信者が選択されている場合のみ表示）
                                if selected_broadcaster_info.is_some() && props.on_delete_click.is_some() {
                                    // セパレータ
                                    div {
                                        style: "
                                            height: 1px;
                                            background: #e5e7eb;
                                            margin: 4px 0;
                                        ",
                                    }

                                    button {
                                        class: "menu-item-delete",
                                        style: "
                                            width: 100%;
                                            padding: 10px 16px;
                                            border: none;
                                            background: white;
                                            color: #dc2626;
                                            cursor: pointer;
                                            font-size: 13px;
                                            display: flex;
                                            align-items: center;
                                            gap: 8px;
                                            text-align: left;
                                        ",
                                        onclick: {
                                            let handler = on_delete_click_handler.clone();
                                            move |e| {
                                                menu_open.set(false);
                                                handler(e);
                                            }
                                        },
                                        span { style: "font-size: 14px;", "🗑️" }
                                        span { "配信者を削除" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            select {
                style: "
                    width: 100%;
                    padding: 10px 12px;
                    border: 2px solid #e5e7eb;
                    border-radius: 8px;
                    font-size: 14px;
                    background: white;
                    cursor: pointer;
                    transition: border-color 0.2s;
                ",
                value: "{selected_value}",
                onchange: move |e| {
                    let value = e.value();
                    if value.is_empty() {
                        props.on_change.call(None);
                    } else {
                        props.on_change.call(Some(value));
                    }
                },

                option {
                    value: "",
                    "-- 配信者を選択してください --"
                }

                match &*broadcasters.read() {
                    Some(channels) => rsx! {
                        for channel in channels.iter() {
                            option {
                                key: "{channel.channel_id}",
                                value: "{channel.channel_id}",
                                "{format_broadcaster_display(channel)}"
                            }
                        }
                    },
                    None => rsx! {
                        option {
                            disabled: true,
                            "読み込み中..."
                        }
                    }
                }
            }

            // バックアップ結果通知
            if let Some(result) = backup_result.read().clone() {
                div {
                    style: "
                        position: fixed;
                        top: 0;
                        left: 0;
                        right: 0;
                        bottom: 0;
                        background: rgba(0, 0, 0, 0.5);
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        z-index: 2000;
                    ",
                    onclick: move |_| backup_result.set(None),

                    div {
                        style: "
                            background: white;
                            border-radius: 12px;
                            padding: 24px;
                            max-width: 500px;
                            width: 90%;
                            box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
                        ",
                        onclick: |e| e.stop_propagation(),

                        match result {
                            BackupResult::Success(path) => rsx! {
                                div {
                                    style: "text-align: center;",

                                    div {
                                        style: "font-size: 48px; margin-bottom: 16px;",
                                        "✅"
                                    }

                                    h3 {
                                        style: "
                                            color: #16a34a;
                                            margin: 0 0 12px 0;
                                            font-size: 18px;
                                        ",
                                        "バックアップが完了しました"
                                    }

                                    div {
                                        style: "
                                            background: #f1f5f9;
                                            border-radius: 8px;
                                            padding: 12px;
                                            margin-bottom: 16px;
                                            word-break: break-all;
                                            font-family: monospace;
                                            font-size: 12px;
                                            color: #475569;
                                            text-align: left;
                                        ",
                                        "{path}"
                                    }

                                    div {
                                        style: "display: flex; gap: 12px; justify-content: center;",

                                        // フォルダを開くボタン
                                        button {
                                            style: "
                                                padding: 10px 24px;
                                                background: #3b82f6;
                                                color: white;
                                                border: none;
                                                border-radius: 8px;
                                                cursor: pointer;
                                                font-size: 14px;
                                                font-weight: 500;
                                                display: flex;
                                                align-items: center;
                                                gap: 6px;
                                            ",
                                            onclick: {
                                                let path_clone = path.clone();
                                                move |_| {
                                                    open_backup_directory(&path_clone);
                                                }
                                            },
                                            span { "📂" }
                                            span { "フォルダを開く" }
                                        }

                                        // 閉じるボタン
                                        button {
                                            style: "
                                                padding: 10px 24px;
                                                background: #6b7280;
                                                color: white;
                                                border: none;
                                                border-radius: 8px;
                                                cursor: pointer;
                                                font-size: 14px;
                                                font-weight: 500;
                                            ",
                                            onclick: move |_| backup_result.set(None),
                                            "閉じる"
                                        }
                                    }
                                }
                            },
                            BackupResult::Error(error) => rsx! {
                                div {
                                    style: "text-align: center;",

                                    div {
                                        style: "font-size: 48px; margin-bottom: 16px;",
                                        "❌"
                                    }

                                    h3 {
                                        style: "
                                            color: #dc2626;
                                            margin: 0 0 12px 0;
                                            font-size: 18px;
                                        ",
                                        "バックアップに失敗しました"
                                    }

                                    div {
                                        style: "
                                            background: #fef2f2;
                                            border-radius: 8px;
                                            padding: 12px;
                                            margin-bottom: 16px;
                                            color: #991b1b;
                                            font-size: 13px;
                                            text-align: left;
                                        ",
                                        "{error}"
                                    }

                                    button {
                                        style: "
                                            padding: 10px 24px;
                                            background: #6b7280;
                                            color: white;
                                            border: none;
                                            border-radius: 8px;
                                            cursor: pointer;
                                            font-size: 14px;
                                            font-weight: 500;
                                        ",
                                        onclick: move |_| backup_result.set(None),
                                        "閉じる"
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

/// 配信者表示名をフォーマット
fn format_broadcaster_display(channel: &BroadcasterChannel) -> String {
    if let Some(ref name) = channel.channel_name {
        if let Some(ref handle) = channel.handle {
            // ハンドルに@がなければ付ける
            let handle_display = if handle.starts_with('@') {
                handle.clone()
            } else {
                format!("@{}", handle)
            };
            format!("{} ({})", name, handle_display)
        } else {
            name.clone()
        }
    } else if let Some(ref handle) = channel.handle {
        if handle.starts_with('@') {
            handle.clone()
        } else {
            format!("@{}", handle)
        }
    } else {
        truncate_channel_id(&channel.channel_id)
    }
}

/// チャンネルIDを表示用に短縮
fn truncate_channel_id(channel_id: &str) -> String {
    if channel_id.len() > 24 {
        format!("{}...", &channel_id[..21])
    } else {
        channel_id.to_string()
    }
}

/// バックアップディレクトリをファイラーで開く
fn open_backup_directory(backup_path: &str) {
    use std::path::Path;
    use std::process::Command;

    let path = Path::new(backup_path);

    // 親ディレクトリを取得
    let dir_to_open = path.parent().unwrap_or(path);

    #[cfg(target_os = "windows")]
    {
        // Windows: explorer.exe /select,<path> でファイルを選択した状態で開く
        let _ = Command::new("explorer.exe")
            .arg("/select,")
            .arg(backup_path)
            .spawn()
            .map_err(|e| tracing::error!("Failed to open explorer: {}", e));
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: open -R <path> でFinderで表示
        let _ = Command::new("open")
            .arg("-R")
            .arg(backup_path)
            .spawn()
            .map_err(|e| tracing::error!("Failed to open Finder: {}", e));
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: xdg-open <dir> でディレクトリを開く
        let _ = Command::new("xdg-open")
            .arg(dir_to_open)
            .spawn()
            .map_err(|e| tracing::error!("Failed to open file manager: {}", e));
    }

    tracing::info!("📂 Opening backup directory: {:?}", dir_to_open);
}

/// YouTubeからチャンネル情報を取得
async fn fetch_broadcaster_info_from_youtube(
    channel_id: &str,
) -> anyhow::Result<Option<(Option<String>, Option<String>)>> {
    // チャンネルページからチャンネル名とハンドルを取得
    let channel_url = format!("https://www.youtube.com/channel/{}", channel_id);

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;

    let response = client.get(&channel_url).send().await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let html = response.text().await?;

    let channel_name = crate::api::youtube::extract_broadcaster_channel_name(&html);
    let handle = crate::api::youtube::extract_broadcaster_handle(&html);

    if channel_name.is_some() || handle.is_some() {
        Ok(Some((channel_name, handle)))
    } else {
        Ok(None)
    }
}
