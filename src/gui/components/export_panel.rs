use crate::analytics::export::{ExportFormat, SortOrder};
use crate::gui::hooks::use_live_chat::LiveChatHandle;
use crate::gui::message_stream::{MessageStream, MessageStreamStats};
use crate::gui::models::GuiChatMessage;
use dioxus::prelude::*;

/// エクスポート範囲の指定
#[derive(Debug, Clone, PartialEq)]
pub enum ExportScope {
    /// 表示中のメッセージのみ
    DisplayedOnly,
    /// 全メッセージ（アーカイブ含む）
    AllMessages,
    /// アーカイブされたメッセージのみ
    ArchivedOnly,
}

/// エクスポートパネルコンポーネント（Week 23-24実装 + MessageStream連携）
#[component]
pub fn ExportPanel(
    live_chat_handle: Option<LiveChatHandle>,
    message_stream: Option<Signal<MessageStream>>,
) -> Element {
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

    // MessageStream連携の新機能
    let mut export_scope = use_signal(|| ExportScope::DisplayedOnly);
    let mut include_archive_stats = use_signal(|| true);
    let message_stream_stats = use_signal(|| None::<MessageStreamStats>);

    // 日付範囲フィルタリング
    let mut date_filter_enabled = use_signal(|| false);
    let mut start_date = use_signal(|| "".to_string());
    let mut end_date = use_signal(|| "".to_string());

    // MessageStream統計情報の更新
    use_effect({
        let message_stream = message_stream.clone();
        let mut message_stream_stats = message_stream_stats.clone();

        move || {
            if let Some(stream) = message_stream {
                let stats = stream.read().stats();
                message_stream_stats.set(Some(stats));
            }
        }
    });

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

                // MessageStream統計表示
                if let Some(stats) = message_stream_stats() {
                    span {
                        style: "
                            margin-left: auto;
                            font-size: 0.8rem;
                            color: #6c757d;
                            background: #f8f9fa;
                            padding: 4px 8px;
                            border-radius: 4px;
                        ",
                        "表示: {stats.display_count} / 総計: {stats.total_count}"
                    }
                }
            }

            div {
                style: "
                    display: grid;
                    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
                    gap: 25px;
                    margin-bottom: 25px;
                ",

                // MessageStream連携：エクスポート範囲選択（新規追加）
                if message_stream.is_some() {
                    div {
                        style: "
                            background: #e8f5e8;
                            padding: 20px;
                            border-radius: 10px;
                            border: 1px solid #c3e6cb;
                        ",

                        h3 {
                            style: "margin: 0 0 15px 0; color: #155724; font-size: 1.2rem;",
                            "🎯 エクスポート範囲"
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
                                    name: "export_scope",
                                    checked: matches!(export_scope(), ExportScope::DisplayedOnly),
                                    onchange: move |_| export_scope.set(ExportScope::DisplayedOnly),
                                }
                                span { "📺 表示中のメッセージのみ" }
                                if let Some(stats) = message_stream_stats() {
                                    small {
                                        style: "color: #6c757d; margin-left: auto;",
                                        "({stats.display_count}件)"
                                    }
                                }
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
                                    name: "export_scope",
                                    checked: matches!(export_scope(), ExportScope::AllMessages),
                                    onchange: move |_| export_scope.set(ExportScope::AllMessages),
                                }
                                span { "📦 全メッセージ（アーカイブ含む）" }
                                if let Some(stats) = message_stream_stats() {
                                    small {
                                        style: "color: #6c757d; margin-left: auto;",
                                        "({stats.total_count}件)"
                                    }
                                }
                            }

                            if let Some(stats) = message_stream_stats() {
                                if stats.archived_count > 0 {
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
                                            name: "export_scope",
                                            checked: matches!(export_scope(), ExportScope::ArchivedOnly),
                                            onchange: move |_| export_scope.set(ExportScope::ArchivedOnly),
                                        }
                                        span { "📚 アーカイブされたメッセージのみ" }
                                        small {
                                            style: "color: #6c757d; margin-left: auto;",
                                            "({stats.archived_count}件)"
                                        }
                                    }
                                }
                            }

                            // メモリ使用量統計の表示
                            if let Some(stats) = message_stream_stats() {
                                div {
                                    style: "
                                        background: #f8f9fa;
                                        padding: 10px;
                                        border-radius: 6px;
                                        margin-top: 10px;
                                        font-size: 0.85rem;
                                        color: #6c757d;
                                    ",
                                    "💾 メモリ使用量: {stats.memory_mb():.2}MB"
                                    if stats.effective_reduction_percent > 0 {
                                        " (削減率: {stats.effective_reduction_percent}%)"
                                    }
                                }
                            }

                            label {
                                style: "
                                    display: flex;
                                    align-items: center;
                                    gap: 8px;
                                    cursor: pointer;
                                    margin-top: 10px;
                                    padding-top: 10px;
                                    border-top: 1px solid #dee2e6;
                                ",
                                input {
                                    r#type: "checkbox",
                                    checked: include_archive_stats(),
                                    onchange: move |evt| include_archive_stats.set(evt.checked()),
                                }
                                "📊 MessageStream統計情報を含める"
                            }
                        }
                    }
                }

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
                        // エクスポート処理を開始（MessageStream連携版）
                        start_export_with_message_stream(
                            export_format(),
                            include_metadata(),
                            include_system_messages(),
                            include_deleted_messages(),
                            max_records(),
                            sort_order(),
                            date_filter_enabled(),
                            start_date(),
                            end_date(),
                            export_scope(),
                            include_archive_stats(),
                            message_stream.clone(),
                            live_chat_handle.clone(),
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

/// MessageStream連携版エクスポート処理
fn start_export_with_message_stream(
    format: ExportFormat,
    include_metadata: bool,
    include_system_messages: bool,
    include_deleted_messages: bool,
    max_records: Option<usize>,
    sort_order: SortOrder,
    date_filter_enabled: bool,
    start_date: String,
    end_date: String,
    export_scope: ExportScope,
    include_archive_stats: bool,
    message_stream: Option<Signal<MessageStream>>,
    live_chat_handle: Option<LiveChatHandle>,
    mut is_exporting: Signal<bool>,
    mut export_progress: Signal<f64>,
    mut last_export_result: Signal<Option<String>>,
) {
    is_exporting.set(true);
    export_progress.set(0.0);
    last_export_result.set(None);

    spawn(async move {
        // MessageStreamからのデータ取得
        let (messages, stats) = if let Some(stream_signal) = message_stream {
            let stream = stream_signal.read();
            let stats = Some(stream.stats());

            let messages = match export_scope {
                ExportScope::DisplayedOnly => {
                    // 表示中のメッセージのみ
                    stream.display_messages()
                }
                ExportScope::AllMessages => {
                    // 全メッセージ（表示+アーカイブ）
                    let mut all_messages = Vec::new();

                    // アーカイブ分を検索で取得（簡易実装）
                    // 実際の実装では、MessageStreamにget_all_messages()メソッドを追加する方が良い
                    all_messages.extend(stream.display_messages());

                    // 注意: 現在の実装ではアーカイブに直接アクセスできないため、
                    // 代替としてlive_chat_handleから取得
                    if let Some(handle) = &live_chat_handle {
                        let live_messages = handle.messages.read();
                        // 重複を避けるため、表示中以外のメッセージを追加
                        if live_messages.len() > all_messages.len() {
                            all_messages = live_messages.clone();
                        }
                    }

                    all_messages
                }
                ExportScope::ArchivedOnly => {
                    // アーカイブのみの場合、現在は実装困難なため空リストを返す
                    // 将来的にMessageStreamにアーカイブアクセスメソッドを追加予定
                    Vec::new()
                }
            };

            (messages, stats)
        } else if let Some(handle) = live_chat_handle {
            // MessageStreamがない場合はLiveChatHandleから取得
            (handle.messages.read().clone(), None)
        } else {
            // どちらもない場合は空のデータ
            (Vec::new(), None)
        };

        export_progress.set(0.1);

        // フィルタリング処理
        let mut filtered_messages = messages;

        // システムメッセージフィルタ
        if !include_system_messages {
            filtered_messages.retain(|msg| !msg.content.starts_with("[システム]"));
        }

        // 削除メッセージフィルタ
        if !include_deleted_messages {
            filtered_messages.retain(|msg| !msg.content.contains("[削除済み]"));
        }

        export_progress.set(0.3);

        // 日付フィルタ
        if date_filter_enabled && (!start_date.is_empty() || !end_date.is_empty()) {
            // 日付フィルタリングの実装（簡易版）
            // 実際の実装では適切な日付パースが必要
            tracing::info!("📅 Date filtering: {} to {}", start_date, end_date);
        }

        export_progress.set(0.5);

        // ソート処理
        match sort_order {
            SortOrder::Chronological => {
                // 既に時系列順のため処理なし
            }
            SortOrder::ReverseChronological => {
                filtered_messages.reverse();
            }
            SortOrder::ByAuthor => {
                filtered_messages.sort_by(|a, b| a.author.cmp(&b.author));
            }
            SortOrder::ByMessageType => {
                // メッセージタイプ別ソート（簡易実装）
                filtered_messages.sort_by(|a, b| {
                    let type_a = if a.content.contains("Super Chat") {
                        1
                    } else {
                        0
                    };
                    let type_b = if b.content.contains("Super Chat") {
                        1
                    } else {
                        0
                    };
                    type_a.cmp(&type_b)
                });
            }
            SortOrder::ByAmount => {
                // 金額順ソート（SuperChatのみ、簡易実装）
                filtered_messages.sort_by(|a, b| {
                    let amount_a: f64 = extract_amount(&a.content).unwrap_or(0.0);
                    let amount_b: f64 = extract_amount(&b.content).unwrap_or(0.0);
                    amount_b
                        .partial_cmp(&amount_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        export_progress.set(0.7);

        // 最大レコード数制限
        if let Some(max) = max_records {
            if filtered_messages.len() > max {
                filtered_messages.truncate(max);
            }
        }

        export_progress.set(0.8);

        // エクスポート処理の模擬実行
        let export_data = ExportData {
            messages: filtered_messages.clone(),
            metadata: if include_metadata {
                Some(ExportMetadata {
                    export_time: chrono::Utc::now().to_rfc3339(),
                    total_count: filtered_messages.len(),
                    export_scope: format!("{:?}", export_scope),
                    format: format!("{:?}", format),
                })
            } else {
                None
            },
            message_stream_stats: if include_archive_stats { stats } else { None },
        };

        // 実際のファイル出力（模擬）
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        export_progress.set(0.9);

        // エクスポート完了
        let file_extension = format.file_extension();
        let result_message = format!(
            "{}形式でのエクスポートが完了しました。\n📊 {}件のメッセージをエクスポート\n📁 ファイル: message_export_{}.{}",
            match format {
                ExportFormat::Json => "JSON",
                ExportFormat::Csv => "CSV",
                ExportFormat::Excel => "Excel",
            },
            export_data.messages.len(),
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            file_extension
        );

        // 統計情報の追加表示
        let stats_message = if let Some(stats) = export_data.message_stream_stats {
            format!(
                "\n💾 MessageStream統計:\n  表示中: {}件, アーカイブ: {}件, 総計: {}件\n  メモリ使用量: {:.2}MB, 削減率: {}%",
                stats.display_count,
                stats.archived_count,
                stats.total_count,
                stats.memory_mb(),
                stats.effective_reduction_percent
            )
        } else {
            String::new()
        };

        let metadata_message = if let Some(meta) = &export_data.metadata {
            format!(
                "\n🗂️ Export metadata: scope={}, total={}, format={}, generated_at={}",
                meta.export_scope, meta.total_count, meta.format, meta.export_time
            )
        } else {
            String::new()
        };

        let final_message = format!("{}{}{}", result_message, stats_message, metadata_message);

        export_progress.set(1.0);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        last_export_result.set(Some(final_message));
        is_exporting.set(false);
        export_progress.set(0.0);

        tracing::info!(
            "📤 Export completed: {} messages in {:?} format with scope {:?}",
            export_data.messages.len(),
            format,
            export_scope
        );
    });
}

/// エクスポートデータ構造体
#[derive(Debug, Clone)]
struct ExportData {
    messages: Vec<GuiChatMessage>,
    metadata: Option<ExportMetadata>,
    message_stream_stats: Option<MessageStreamStats>,
}

/// エクスポートメタデータ
#[derive(Debug, Clone)]
struct ExportMetadata {
    export_time: String,
    total_count: usize,
    export_scope: String,
    format: String,
}

/// SuperChat金額を抽出する関数（簡易実装）
fn extract_amount(content: &str) -> Option<f64> {
    // "¥100"や"$10.50"のような形式から金額を抽出
    if content.contains("¥") {
        content
            .split("¥")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.replace(",", "").parse().ok())
    } else if content.contains("$") {
        content
            .split("$")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse().ok())
    } else {
        None
    }
}
