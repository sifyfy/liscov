use crate::analytics::export::{ExportFormat, SortOrder};
use dioxus::prelude::*;

/// エクスポートパネルコンポーネント（Week 23-24実装）
#[component]
pub fn ExportPanel() -> Element {
    // エクスポート設定の状態管理
    let mut export_format = use_signal(|| ExportFormat::Json);
    let mut include_metadata = use_signal(|| true);
    let mut include_system_messages = use_signal(|| false);
    let mut include_deleted_messages = use_signal(|| false);
    let mut max_records = use_signal(|| None::<usize>);
    let mut sort_order = use_signal(|| SortOrder::Chronological);
    let is_exporting = use_signal(|| false);
    let export_progress = use_signal(|| 0.0);
    let last_export_result = use_signal(|| None::<String>);

    // 日付範囲フィルタリング
    let mut date_filter_enabled = use_signal(|| false);
    let mut start_date = use_signal(|| "".to_string());
    let mut end_date = use_signal(|| "".to_string());

    rsx! {
        div {
            style: "
                background: white;
                padding: 30px;
                border-radius: 15px;
                margin: 20px 0;
                box-shadow: 0 8px 25px rgba(0,0,0,0.1);
                border-left: 5px solid #3498db;
            ",

            h2 {
                style: "
                    color: #2c3e50;
                    margin-bottom: 25px;
                    font-size: 1.8rem;
                    display: flex;
                    align-items: center;
                    gap: 10px;
                ",
                span { style: "font-size: 2rem;", "📤" }
                "データエクスポート"
            }

            div {
                style: "
                    display: grid;
                    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
                    gap: 25px;
                    margin-bottom: 25px;
                ",

                // エクスポート形式選択
                div {
                    style: "
                        background: #f8f9fa;
                        padding: 20px;
                        border-radius: 10px;
                        border: 1px solid #e9ecef;
                    ",

                    h3 {
                        style: "margin: 0 0 15px 0; color: #495057; font-size: 1.2rem;",
                        "📋 エクスポート形式"
                    }

                    div {
                        style: "display: flex; flex-direction: column; gap: 10px;",

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                                padding: 8px;
                                border-radius: 6px;
                                transition: background-color 0.2s;
                            ",
                            input {
                                r#type: "radio",
                                name: "export_format",
                                checked: matches!(export_format(), ExportFormat::Json),
                                onchange: move |_| export_format.set(ExportFormat::Json),
                            }
                            span { "📄 JSON形式" }
                            small { style: "color: #6c757d; margin-left: auto;", "構造化データ" }
                        }

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                                padding: 8px;
                                border-radius: 6px;
                                transition: background-color 0.2s;
                            ",
                            input {
                                r#type: "radio",
                                name: "export_format",
                                checked: matches!(export_format(), ExportFormat::Csv),
                                onchange: move |_| export_format.set(ExportFormat::Csv),
                            }
                            span { "📊 CSV形式" }
                            small { style: "color: #6c757d; margin-left: auto;", "表計算対応" }
                        }

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                                padding: 8px;
                                border-radius: 6px;
                                transition: background-color 0.2s;
                            ",
                            input {
                                r#type: "radio",
                                name: "export_format",
                                checked: matches!(export_format(), ExportFormat::Excel),
                                onchange: move |_| export_format.set(ExportFormat::Excel),
                            }
                            span { "📈 Excel形式" }
                            small { style: "color: #6c757d; margin-left: auto;", "高機能・複数シート" }
                        }
                    }
                }

                // フィルター設定
                div {
                    style: "
                        background: #f8f9fa;
                        padding: 20px;
                        border-radius: 10px;
                        border: 1px solid #e9ecef;
                    ",

                    h3 {
                        style: "margin: 0 0 15px 0; color: #495057; font-size: 1.2rem;",
                        "🔧 フィルター設定"
                    }

                    div {
                        style: "display: flex; flex-direction: column; gap: 12px;",

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                            ",
                            input {
                                r#type: "checkbox",
                                checked: include_metadata(),
                                onchange: move |evt| include_metadata.set(evt.checked()),
                            }
                            "メタデータを含める"
                        }

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                            ",
                            input {
                                r#type: "checkbox",
                                checked: include_system_messages(),
                                onchange: move |evt| include_system_messages.set(evt.checked()),
                            }
                            "システムメッセージを含める"
                        }

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                            ",
                            input {
                                r#type: "checkbox",
                                checked: include_deleted_messages(),
                                onchange: move |evt| include_deleted_messages.set(evt.checked()),
                            }
                            "削除されたメッセージを含める"
                        }

                        div {
                            style: "border-top: 1px solid #dee2e6; padding-top: 10px; margin-top: 5px;",

                            label {
                                style: "display: block; margin-bottom: 5px; color: #495057;",
                                "最大レコード数（空欄で全件）"
                            }
                            input {
                                r#type: "number",
                                placeholder: "例: 1000",
                                min: "1",
                                style: "
                                    width: 100%;
                                    padding: 8px;
                                    border: 1px solid #ced4da;
                                    border-radius: 4px;
                                    font-size: 0.9rem;
                                ",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    if value.is_empty() {
                                        max_records.set(None);
                                    } else if let Ok(num) = value.parse::<usize>() {
                                        max_records.set(Some(num));
                                    }
                                },
                            }
                        }
                    }
                }

                // ソート設定
                div {
                    style: "
                        background: #f8f9fa;
                        padding: 20px;
                        border-radius: 10px;
                        border: 1px solid #e9ecef;
                    ",

                    h3 {
                        style: "margin: 0 0 15px 0; color: #495057; font-size: 1.2rem;",
                        "📑 ソート設定"
                    }

                    select {
                        style: "
                            width: 100%;
                            padding: 10px;
                            border: 1px solid #ced4da;
                            border-radius: 6px;
                            background: white;
                            font-size: 0.95rem;
                        ",
                        onchange: move |evt| {
                            let value = evt.value();
                            sort_order.set(match value.as_str() {
                                "reverse_chronological" => SortOrder::ReverseChronological,
                                "by_author" => SortOrder::ByAuthor,
                                "by_message_type" => SortOrder::ByMessageType,
                                "by_amount" => SortOrder::ByAmount,
                                _ => SortOrder::Chronological,
                            });
                        },

                        option { value: "chronological", selected: true, "時系列順（古い→新しい）" }
                        option { value: "reverse_chronological", "時系列順（新しい→古い）" }
                        option { value: "by_author", "投稿者名順" }
                        option { value: "by_message_type", "メッセージタイプ順" }
                        option { value: "by_amount", "Super Chat金額順" }
                    }
                }

                // 日付範囲フィルター
                div {
                    style: "
                        background: #f8f9fa;
                        padding: 20px;
                        border-radius: 10px;
                        border: 1px solid #e9ecef;
                    ",

                    h3 {
                        style: "margin: 0 0 15px 0; color: #495057; font-size: 1.2rem;",
                        "📅 日付範囲フィルター"
                    }

                    label {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 8px;
                            cursor: pointer;
                            margin-bottom: 15px;
                        ",
                        input {
                            r#type: "checkbox",
                            checked: date_filter_enabled(),
                            onchange: move |evt| date_filter_enabled.set(evt.checked()),
                        }
                        "日付範囲でフィルタリング"
                    }

                    if date_filter_enabled() {
                        div {
                            style: "display: flex; flex-direction: column; gap: 10px;",

                            div {
                                label {
                                    style: "display: block; margin-bottom: 5px; color: #495057;",
                                    "開始日時"
                                }
                                input {
                                    r#type: "datetime-local",
                                    value: start_date(),
                                    style: "
                                        width: 100%;
                                        padding: 8px;
                                        border: 1px solid #ced4da;
                                        border-radius: 4px;
                                    ",
                                    onchange: move |evt| start_date.set(evt.value()),
                                }
                            }

                            div {
                                label {
                                    style: "display: block; margin-bottom: 5px; color: #495057;",
                                    "終了日時"
                                }
                                input {
                                    r#type: "datetime-local",
                                    value: end_date(),
                                    style: "
                                        width: 100%;
                                        padding: 8px;
                                        border: 1px solid #ced4da;
                                        border-radius: 4px;
                                    ",
                                    onchange: move |evt| end_date.set(evt.value()),
                                }
                            }
                        }
                    }
                }
            }

            // 進捗表示
            if is_exporting() {
                div {
                    style: "
                        background: #e3f2fd;
                        border: 1px solid #bbdefb;
                        border-radius: 8px;
                        padding: 20px;
                        margin-bottom: 20px;
                    ",

                    h4 {
                        style: "margin: 0 0 10px 0; color: #1976d2;",
                        "🔄 エクスポート中..."
                    }

                    div {
                        style: "
                            background: #f5f5f5;
                            height: 8px;
                            border-radius: 4px;
                            overflow: hidden;
                            margin-bottom: 10px;
                        ",
                        div {
                            style: format!(
                                "background: linear-gradient(90deg, #2196f3, #21cbf3);
                                height: 100%;
                                width: {}%;
                                transition: width 0.3s ease;",
                                export_progress() * 100.0
                            )
                        }
                    }

                    p {
                        style: "margin: 0; color: #1976d2; font-size: 0.9rem;",
                        "進捗: {export_progress() * 100.0:.1}%"
                    }
                }
            }

            // エクスポートボタン
            div {
                style: "text-align: center;",

                button {
                    style: format!(
                        "background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                        color: white;
                        border: none;
                        padding: 15px 40px;
                        border-radius: 8px;
                        font-size: 1.1rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: all 0.3s ease;
                        opacity: {};
                        transform: scale(1);
                        box-shadow: 0 4px 15px rgba(102, 126, 234, 0.3);",
                        if is_exporting() { "0.7" } else { "1.0" }
                    ),
                    disabled: is_exporting(),
                    onclick: move |_| {
                        // エクスポート処理を開始
                        start_export(
                            export_format(),
                            include_metadata(),
                            include_system_messages(),
                            include_deleted_messages(),
                            max_records(),
                            sort_order(),
                            date_filter_enabled(),
                            start_date(),
                            end_date(),
                            is_exporting.clone(),
                            export_progress.clone(),
                            last_export_result.clone(),
                        );
                    },

                    if is_exporting() {
                        "エクスポート中... ⏳"
                    } else {
                        "データをエクスポート 🚀"
                    }
                }
            }

            // 最後のエクスポート結果
            if let Some(result) = last_export_result() {
                div {
                    style: "
                        margin-top: 20px;
                        padding: 15px;
                        background: #d4edda;
                        border: 1px solid #c3e6cb;
                        border-radius: 8px;
                        color: #155724;
                    ",

                    h4 {
                        style: "margin: 0 0 8px 0;",
                        "✅ エクスポート完了"
                    }

                    p {
                        style: "margin: 0; font-size: 0.9rem;",
                        "{result}"
                    }
                }
            }
        }
    }
}

/// エクスポート処理を開始する関数
fn start_export(
    format: ExportFormat,
    _include_metadata: bool,
    _include_system_messages: bool,
    _include_deleted_messages: bool,
    _max_records: Option<usize>,
    _sort_order: SortOrder,
    _date_filter_enabled: bool,
    _start_date: String,
    _end_date: String,
    mut is_exporting: Signal<bool>,
    mut export_progress: Signal<f64>,
    mut last_export_result: Signal<Option<String>>,
) {
    is_exporting.set(true);
    export_progress.set(0.0);
    last_export_result.set(None);

    spawn(async move {
        // 模擬的なエクスポート処理（実際の実装では実データを使用）
        for i in 1..=10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            export_progress.set(i as f64 / 10.0);
        }

        // エクスポート完了
        let file_extension = format.file_extension();
        let result_message = format!(
            "{}形式でのエクスポートが完了しました。ファイル: export_data.{}",
            match format {
                ExportFormat::Json => "JSON",
                ExportFormat::Csv => "CSV",
                ExportFormat::Excel => "Excel",
            },
            file_extension
        );

        last_export_result.set(Some(result_message));
        is_exporting.set(false);
        export_progress.set(0.0);
    });
}
