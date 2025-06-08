//! YouTubeレスポンス保存設定のGUIコンポーネント

use crate::gui::models::AppState;
use crate::gui::state_management::{get_state_manager, AppEvent};
use crate::io::SaveConfig;
use dioxus::prelude::*;
use std::path::PathBuf;

/// レスポンス保存設定コンポーネント
#[component]
pub fn RawResponseSettings() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let current_state = state.read();

    // 現在の設定値を状態として管理
    let mut save_enabled = use_signal(|| current_state.save_raw_responses);
    let mut file_path = use_signal(|| current_state.raw_response_file.clone());
    let mut max_file_size = use_signal(|| current_state.max_raw_file_size_mb);
    let mut rotation_enabled = use_signal(|| current_state.enable_file_rotation);

    rsx! {
        div {
            class: "raw-response-settings",
            h3 { "📝 生レスポンス保存設定" }

            div {
                class: "setting-group",
                label {
                    input {
                        r#type: "checkbox",
                        checked: save_enabled(),
                        onchange: move |evt| {
                            let enabled = evt.value().parse::<bool>().unwrap_or(false);
                            save_enabled.set(enabled);

                            // 状態を更新
                            let mut state_ref = state.write();
                            state_ref.save_raw_responses = enabled;

                            // 設定をサービスに適用
                            apply_save_config(
                                enabled,
                                file_path(),
                                max_file_size(),
                                rotation_enabled(),
                            );
                        }
                    }
                    " 生レスポンス保存を有効化"
                }
            }

            if save_enabled() {
                div {
                    class: "setting-group",
                    label {
                        "保存ファイルパス:"
                        div {
                            class: "file-path-input-group",
                            input {
                                r#type: "text",
                                value: file_path(),
                                placeholder: "例: C:\\Users\\Username\\Documents\\raw_responses.ndjson",
                                oninput: move |evt| {
                                    let path = evt.value();
                                    file_path.set(path.clone());

                                    // 状態を更新
                                    let mut state_ref = state.write();
                                    state_ref.raw_response_file = path.clone();

                                    // 設定をサービスに適用
                                    apply_save_config(
                                        save_enabled(),
                                        path,
                                        max_file_size(),
                                        rotation_enabled(),
                                    );
                                }
                            }
                            button {
                                class: "file-browse-button",
                                r#type: "button",
                                                                onclick: move |_| {
                                    // ファイル選択ダイアログを非同期で開く
                                    let mut file_path_signal = file_path.clone();
                                    let mut state_signal = state.clone();
                                    spawn(async move {
                                        if let Some(selected_path) = open_save_file_dialog().await {
                                            let path_str = selected_path.to_string_lossy().to_string();
                                            file_path_signal.set(path_str.clone());

                                            // 状態を更新
                                            let mut state_ref = state_signal.write();
                                            state_ref.raw_response_file = path_str.clone();

                                            // 設定をサービスに適用
                                            apply_save_config(
                                                true, // save_enabled
                                                path_str,
                                                state_ref.max_raw_file_size_mb,
                                                state_ref.enable_file_rotation,
                                            );
                                        }
                                    });
                                },
                                "📁 参照"
                            }
                        }
                    }
                }

                div {
                    class: "setting-group",
                    label {
                        "最大ファイルサイズ (MB):"
                        input {
                            r#type: "number",
                            value: max_file_size().to_string(),
                            min: "1",
                            max: "1000",
                            oninput: move |evt| {
                                if let Ok(size) = evt.value().parse::<u64>() {
                                    max_file_size.set(size);

                                    // 状態を更新
                                    let mut state_ref = state.write();
                                    state_ref.max_raw_file_size_mb = size;

                                    // 設定をサービスに適用
                                    apply_save_config(
                                        save_enabled(),
                                        file_path(),
                                        size,
                                        rotation_enabled(),
                                    );
                                }
                            }
                        }
                    }
                }

                div {
                    class: "setting-group",
                    label {
                        input {
                            r#type: "checkbox",
                            checked: rotation_enabled(),
                            onchange: move |evt| {
                                let enabled = evt.value().parse::<bool>().unwrap_or(false);
                                rotation_enabled.set(enabled);

                                // 状態を更新
                                let mut state_ref = state.write();
                                state_ref.enable_file_rotation = enabled;

                                // 設定をサービスに適用
                                apply_save_config(
                                    save_enabled(),
                                    file_path(),
                                    max_file_size(),
                                    enabled,
                                );
                            }
                        }
                        " ファイルローテーションを有効化"
                    }
                }

                div {
                    class: "current-path-info",
                    p {
                        strong { "💾 現在の保存先: " }
                        span {
                            class: "current-path",
                            "{file_path()}"
                        }
                    }
                }

                div {
                    class: "info-box",
                    p { "💡 ヒント:" }
                    ul {
                        li { "生レスポンスは将来のAPI変更に対応するために保存されます" }
                        li { "ファイルはndjson形式で保存されます" }
                        li { "ローテーション有効時、サイズ上限に達すると自動で新ファイルが作成されます" }
                        li { "📁 「参照」ボタンでファイル保存場所を簡単に選択できます" }
                    }
                }
            }
        }

        style { r"
            .raw-response-settings {{
                padding: 20px;
                background: #f5f5f5;
                border-radius: 8px;
                margin: 10px 0;
            }}
            
            .setting-group {{
                margin: 15px 0;
            }}
            
            .setting-group label {{
                display: flex;
                align-items: center;
                gap: 10px;
                font-weight: 500;
            }}
            
            .setting-group input[type=text],
            .setting-group input[type=number] {{
                padding: 8px;
                border: 1px solid #ddd;
                border-radius: 4px;
                font-size: 14px;
                flex: 1;
            }}
            
            .setting-group input[type=checkbox] {{
                margin-right: 8px;
            }}
            
            .file-path-input-group {{
                display: flex;
                gap: 8px;
                align-items: center;
                width: 100%;
            }}
            
            .file-path-input-group input {{
                flex: 1;
            }}
            
            .file-browse-button {{
                padding: 8px 12px;
                background: #007bff;
                color: white;
                border: none;
                border-radius: 4px;
                cursor: pointer;
                font-size: 14px;
                white-space: nowrap;
                transition: background-color 0.2s;
            }}
            
            .file-browse-button:hover {{
                background: #0056b3;
            }}
            
            .file-browse-button:active {{
                background: #004494;
            }}
            
            .current-path-info {{
                background: #f8f9fa;
                border: 1px solid #e9ecef;
                border-radius: 4px;
                padding: 10px;
                margin: 15px 0;
            }}
            
            .current-path-info p {{
                margin: 0;
                font-size: 14px;
            }}
            
            .current-path {{
                font-family: 'Courier New', monospace;
                background: #ffffff;
                padding: 2px 6px;
                border-radius: 3px;
                border: 1px solid #dee2e6;
                font-size: 13px;
                word-break: break-all;
            }}
            
            .info-box {{
                background: #e8f4fd;
                border: 1px solid #b8daff;
                border-radius: 4px;
                padding: 15px;
                margin-top: 20px;
            }}
            
            .info-box p {{
                margin: 0 0 10px 0;
                font-weight: bold;
                color: #0056b3;
            }}
            
            .info-box ul {{
                margin: 0;
                padding-left: 20px;
            }}
            
            .info-box li {{
                margin: 5px 0;
                color: #495057;
            }}
        " }
    }
}

/// ファイル保存ダイアログを開く
async fn open_save_file_dialog() -> Option<PathBuf> {
    match rfd::AsyncFileDialog::new()
        .set_title("生レスポンス保存先を選択")
        .add_filter("NDJSON ファイル", &["ndjson"])
        .add_filter("JSON ファイル", &["json"])
        .add_filter("すべてのファイル", &["*"])
        .set_file_name("raw_responses.ndjson")
        .save_file()
        .await
    {
        Some(file_handle) => {
            let path = file_handle.path().to_path_buf();
            tracing::info!("📁 Selected save path: {}", path.display());
            Some(path)
        }
        None => {
            tracing::debug!("📁 File save dialog cancelled");
            None
        }
    }
}

/// 保存設定をサービスに適用
fn apply_save_config(enabled: bool, file_path: String, max_size_mb: u64, rotation: bool) {
    let config = SaveConfig {
        enabled,
        file_path: file_path.clone(),
        max_file_size_mb: max_size_mb,
        enable_rotation: rotation,
        max_backup_files: 5, // デフォルト値
    };

    tracing::info!(
        "🔧 apply_save_config called: enabled={}, file_path={}, max_size_mb={}, rotation={}",
        enabled,
        file_path,
        max_size_mb,
        rotation
    );

    // 状態管理経由でサービスに設定を送信
    let state_manager = get_state_manager();
    match state_manager.send_event(AppEvent::UpdateSaveConfig(config)) {
        Ok(_) => {
            tracing::info!("✅ UpdateSaveConfig event sent successfully");
        }
        Err(e) => {
            tracing::error!("❌ Failed to send UpdateSaveConfig event: {}", e);
        }
    }
}
