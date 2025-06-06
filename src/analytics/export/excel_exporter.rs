use super::{ExportConfig, ExportError, FormatHandler, SessionData, SortOrder};
use rust_xlsxwriter::{Color, Format, Workbook, Worksheet, XlsxError};

/// Excel形式エクスポーター（Week 21-22実装）
pub struct ExcelExporter {
    include_charts: bool,
    multi_sheet: bool,
    cell_formatting: bool,
}

impl ExcelExporter {
    pub fn new() -> Self {
        Self {
            include_charts: true,
            multi_sheet: true,
            cell_formatting: true,
        }
    }

    pub fn with_charts(mut self, include_charts: bool) -> Self {
        self.include_charts = include_charts;
        self
    }

    pub fn with_multi_sheet(mut self, multi_sheet: bool) -> Self {
        self.multi_sheet = multi_sheet;
        self
    }

    pub fn with_cell_formatting(mut self, cell_formatting: bool) -> Self {
        self.cell_formatting = cell_formatting;
        self
    }

    /// ワークブックを作成してデータを書き込み
    fn create_workbook(
        &self,
        data: &SessionData,
        config: &ExportConfig,
    ) -> Result<Vec<u8>, ExportError> {
        let mut workbook = Workbook::new();

        // メインデータシート
        self.create_messages_sheet(&mut workbook, data, config)?;

        // 複数シート対応
        if self.multi_sheet {
            self.create_statistics_sheet(&mut workbook, data)?;
            self.create_viewers_sheet(&mut workbook, data)?;
            self.create_sentiment_sheet(&mut workbook, data)?;
        }

        // メタデータシート
        if config.include_metadata {
            self.create_metadata_sheet(&mut workbook, data, config)?;
        }

        // ワークブックをバイト配列として取得
        let buffer = workbook
            .save_to_buffer()
            .map_err(|e| ExportError::Serialization(format!("Excel generation failed: {}", e)))?;

        Ok(buffer)
    }

    /// メッセージシートを作成
    fn create_messages_sheet(
        &self,
        workbook: &mut Workbook,
        data: &SessionData,
        config: &ExportConfig,
    ) -> Result<(), ExportError> {
        let mut worksheet = workbook.add_worksheet().set_name("Messages")?;

        // セル書式設定
        let header_format = if self.cell_formatting {
            Some(
                Format::new()
                    .set_bold()
                    .set_background_color(Color::RGB(0x4472C4))
                    .set_font_color(Color::White)
                    .set_border(rust_xlsxwriter::FormatBorder::Thin),
            )
        } else {
            None
        };

        let super_chat_format = if self.cell_formatting {
            Some(
                Format::new()
                    .set_background_color(Color::RGB(0xFFEB9C))
                    .set_border(rust_xlsxwriter::FormatBorder::Thin),
            )
        } else {
            None
        };

        let membership_format = if self.cell_formatting {
            Some(
                Format::new()
                    .set_background_color(Color::RGB(0xC6EFCE))
                    .set_border(rust_xlsxwriter::FormatBorder::Thin),
            )
        } else {
            None
        };

        // ヘッダー行
        let headers = vec![
            "ID",
            "タイムスタンプ",
            "投稿者",
            "投稿者ID",
            "内容",
            "タイプ",
            "金額",
            "通貨",
            "絵文字数",
            "文字数",
            "削除済み",
            "モデレーター",
            "メンバー",
            "認証済み",
            "バッジ",
        ];

        for (col, header) in headers.iter().enumerate() {
            if let Some(ref format) = header_format {
                worksheet.write_string_with_format(0, col as u16, *header, format)?;
            } else {
                worksheet.write_string(0, col as u16, *header)?;
            }
        }

        // データをソートしてフィルタリング
        let mut filtered_data = data.clone();
        self.apply_filters(&mut filtered_data, config);
        self.sort_data(&mut filtered_data, config.sort_order);

        // データ行
        for (row, message) in filtered_data.messages.iter().enumerate() {
            let row_idx = (row + 1) as u32;

            // 行の書式を決定（メッセージタイプによる）
            let row_format = if self.cell_formatting {
                match message.message_type.as_str() {
                    "super-chat" => super_chat_format.as_ref(),
                    "membership" => membership_format.as_ref(),
                    _ => None,
                }
            } else {
                None
            };

            // 各列に値を書き込み
            let values: Vec<Box<dyn WriteToExcel>> = vec![
                Box::new(message.id.clone()),
                Box::new(message.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()),
                Box::new(message.author.clone()),
                Box::new(message.author_id.clone()),
                Box::new(message.content.clone()),
                Box::new(message.message_type.clone()),
                Box::new(message.amount.map_or("".to_string(), |a| a.to_string())),
                Box::new(message.currency.clone().unwrap_or_default()),
                Box::new(message.emoji_count.to_string()),
                Box::new(message.word_count.to_string()),
                Box::new(
                    if message.is_deleted {
                        "はい"
                    } else {
                        "いいえ"
                    }
                    .to_string(),
                ),
                Box::new(
                    if message.is_moderator {
                        "はい"
                    } else {
                        "いいえ"
                    }
                    .to_string(),
                ),
                Box::new(
                    if message.is_member {
                        "はい"
                    } else {
                        "いいえ"
                    }
                    .to_string(),
                ),
                Box::new(
                    if message.is_verified {
                        "はい"
                    } else {
                        "いいえ"
                    }
                    .to_string(),
                ),
                Box::new(message.badges.join(", ")),
            ];

            for (col, value) in values.iter().enumerate() {
                if let Some(format) = row_format {
                    value.write_with_format(&mut worksheet, row_idx, col as u16, format)?;
                } else {
                    value.write(&mut worksheet, row_idx, col as u16)?;
                }
            }
        }

        // 列幅を自動調整
        for col in 0..headers.len() {
            worksheet.set_column_width(col as u16, self.calculate_column_width(col, &headers))?;
        }

        Ok(())
    }

    /// 統計シートを作成
    fn create_statistics_sheet(
        &self,
        workbook: &mut Workbook,
        data: &SessionData,
    ) -> Result<(), ExportError> {
        let worksheet = workbook.add_worksheet().set_name("Statistics")?;

        let header_format = if self.cell_formatting {
            Some(
                Format::new()
                    .set_bold()
                    .set_background_color(Color::RGB(0x70AD47))
                    .set_font_color(Color::White),
            )
        } else {
            None
        };

        // 統計情報を書き込み
        let stats = vec![
            ("総メッセージ数", data.statistics.total_messages.to_string()),
            (
                "ユニーク視聴者数",
                data.statistics.unique_viewers.to_string(),
            ),
            (
                "総Super Chat金額",
                format!("¥{:.2}", data.statistics.total_super_chat_amount),
            ),
            (
                "総メンバーシップ数",
                data.statistics.total_memberships.to_string(),
            ),
            (
                "分当たり平均メッセージ数",
                format!("{:.2}", data.statistics.average_messages_per_minute),
            ),
            (
                "最大同時視聴者数",
                data.statistics.peak_concurrent_viewers.to_string(),
            ),
            (
                "エンゲージメント率",
                format!("{:.2}%", data.statistics.engagement_rate),
            ),
            (
                "絵文字使用率",
                format!("{:.2}%", data.statistics.emoji_usage_rate),
            ),
        ];

        if let Some(ref format) = header_format {
            worksheet.write_string_with_format(0, 0, "項目", format)?;
            worksheet.write_string_with_format(0, 1, "値", format)?;
        } else {
            worksheet.write_string(0, 0, "項目")?;
            worksheet.write_string(0, 1, "値")?;
        }

        for (row, (label, value)) in stats.iter().enumerate() {
            let row_idx = (row + 1) as u32;
            worksheet.write_string(row_idx, 0, *label)?;
            worksheet.write_string(row_idx, 1, value)?;
        }

        // 時間別活動データ
        let start_row = stats.len() + 3;
        if let Some(ref format) = header_format {
            worksheet.write_string_with_format(start_row as u32, 0, "時間", format)?;
            worksheet.write_string_with_format(start_row as u32, 1, "メッセージ数", format)?;
        } else {
            worksheet.write_string(start_row as u32, 0, "時間")?;
            worksheet.write_string(start_row as u32, 1, "メッセージ数")?;
        }

        for (idx, (hour, count)) in data.statistics.hourly_activity.iter().enumerate() {
            let row_idx = (start_row + idx + 1) as u32;
            worksheet.write_string(row_idx, 0, &format!("{}時", hour))?;
            worksheet.write_number(row_idx, 1, *count as f64)?;
        }

        Ok(())
    }

    /// 視聴者シートを作成
    fn create_viewers_sheet(
        &self,
        workbook: &mut Workbook,
        data: &SessionData,
    ) -> Result<(), ExportError> {
        let worksheet = workbook.add_worksheet().set_name("Viewers")?;

        let header_format = if self.cell_formatting {
            Some(
                Format::new()
                    .set_bold()
                    .set_background_color(Color::RGB(0xE7E6E6))
                    .set_font_color(Color::Black),
            )
        } else {
            None
        };

        // ヘッダー
        let headers = vec![
            "チャンネルID",
            "表示名",
            "初回出現",
            "最終出現",
            "総メッセージ数",
            "総Super Chat",
            "絵文字使用率",
            "平均メッセージ長",
            "メンバー",
            "モデレーター",
            "タグ",
        ];

        for (col, header) in headers.iter().enumerate() {
            if let Some(ref format) = header_format {
                worksheet.write_string_with_format(0, col as u16, *header, format)?;
            } else {
                worksheet.write_string(0, col as u16, *header)?;
            }
        }

        // 視聴者データ
        for (row, viewer) in data.viewers.iter().enumerate() {
            let row_idx = (row + 1) as u32;

            worksheet.write_string(row_idx, 0, &viewer.channel_id)?;
            worksheet.write_string(row_idx, 1, &viewer.display_name)?;
            worksheet.write_string(
                row_idx,
                2,
                &viewer.first_seen.format("%Y-%m-%d %H:%M:%S").to_string(),
            )?;
            worksheet.write_string(
                row_idx,
                3,
                &viewer.last_seen.format("%Y-%m-%d %H:%M:%S").to_string(),
            )?;
            worksheet.write_number(row_idx, 4, viewer.total_messages as f64)?;
            worksheet.write_number(row_idx, 5, viewer.total_super_chat)?;
            worksheet.write_number(row_idx, 6, viewer.emoji_usage_rate)?;
            worksheet.write_number(row_idx, 7, viewer.average_message_length)?;
            worksheet.write_string(
                row_idx,
                8,
                if viewer.is_member {
                    "はい"
                } else {
                    "いいえ"
                },
            )?;
            worksheet.write_string(
                row_idx,
                9,
                if viewer.is_moderator {
                    "はい"
                } else {
                    "いいえ"
                },
            )?;
            worksheet.write_string(row_idx, 10, &viewer.tags.join(", "))?;
        }

        Ok(())
    }

    /// 感情分析シートを作成
    fn create_sentiment_sheet(
        &self,
        workbook: &mut Workbook,
        data: &SessionData,
    ) -> Result<(), ExportError> {
        let worksheet = workbook.add_worksheet().set_name("Sentiment")?;

        let header_format = if self.cell_formatting {
            Some(
                Format::new()
                    .set_bold()
                    .set_background_color(Color::RGB(0xFF6B9D))
                    .set_font_color(Color::White),
            )
        } else {
            None
        };

        // ヘッダー
        let headers = vec![
            "メッセージID",
            "感情タイプ",
            "信頼度",
            "ポジティブスコア",
            "ネガティブスコア",
            "中性スコア",
            "検出された感情",
        ];

        for (col, header) in headers.iter().enumerate() {
            if let Some(ref format) = header_format {
                worksheet.write_string_with_format(0, col as u16, *header, format)?;
            } else {
                worksheet.write_string(0, col as u16, *header)?;
            }
        }

        // 感情分析データ
        for (row, sentiment) in data.sentiment_analysis.iter().enumerate() {
            let row_idx = (row + 1) as u32;

            worksheet.write_string(row_idx, 0, &sentiment.message_id)?;
            worksheet.write_string(row_idx, 1, &sentiment.sentiment_type)?;
            worksheet.write_number(row_idx, 2, sentiment.confidence)?;
            worksheet.write_number(row_idx, 3, sentiment.positive_score)?;
            worksheet.write_number(row_idx, 4, sentiment.negative_score)?;
            worksheet.write_number(row_idx, 5, sentiment.neutral_score)?;
            worksheet.write_string(row_idx, 6, &sentiment.detected_emotions.join(", "))?;
        }

        Ok(())
    }

    /// メタデータシートを作成
    fn create_metadata_sheet(
        &self,
        workbook: &mut Workbook,
        data: &SessionData,
        _config: &ExportConfig,
    ) -> Result<(), ExportError> {
        let worksheet = workbook.add_worksheet().set_name("Metadata")?;

        let header_format = if self.cell_formatting {
            Some(
                Format::new()
                    .set_bold()
                    .set_background_color(Color::RGB(0x44546A))
                    .set_font_color(Color::White),
            )
        } else {
            None
        };

        // メタデータ情報
        let metadata = vec![
            ("セッションID", data.metadata.session_id.clone()),
            ("チャンネル名", data.metadata.channel_name.clone()),
            ("配信URL", data.metadata.stream_url.clone()),
            (
                "開始時刻",
                data.metadata
                    .start_time
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
            ),
            (
                "終了時刻",
                data.metadata.end_time.map_or("N/A".to_string(), |t| {
                    t.format("%Y-%m-%d %H:%M:%S UTC").to_string()
                }),
            ),
            (
                "継続時間（秒）",
                data.metadata
                    .duration_seconds
                    .map_or("N/A".to_string(), |d| d.to_string()),
            ),
            (
                "エクスポート時刻",
                data.metadata
                    .export_time
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
            ),
            (
                "エクスポートバージョン",
                data.metadata.export_version.clone(),
            ),
            ("liscovバージョン", data.metadata.liscov_version.clone()),
            (
                "総データポイント数",
                data.metadata.total_data_points.to_string(),
            ),
        ];

        if let Some(ref format) = header_format {
            worksheet.write_string_with_format(0, 0, "項目", format)?;
            worksheet.write_string_with_format(0, 1, "値", format)?;
        } else {
            worksheet.write_string(0, 0, "項目")?;
            worksheet.write_string(0, 1, "値")?;
        }

        for (row, (label, value)) in metadata.iter().enumerate() {
            let row_idx = (row + 1) as u32;
            worksheet.write_string(row_idx, 0, *label)?;
            worksheet.write_string(row_idx, 1, value)?;
        }

        // 適用されたフィルター
        let filter_start = metadata.len() + 3;
        if let Some(ref format) = header_format {
            worksheet.write_string_with_format(
                filter_start as u32,
                0,
                "適用されたフィルター",
                format,
            )?;
        } else {
            worksheet.write_string(filter_start as u32, 0, "適用されたフィルター")?;
        }

        for (idx, filter) in data.metadata.filters_applied.iter().enumerate() {
            let row_idx = (filter_start + idx + 1) as u32;
            worksheet.write_string(row_idx, 0, filter)?;
        }

        Ok(())
    }

    /// データをソート
    fn sort_data(&self, data: &mut SessionData, sort_order: SortOrder) {
        match sort_order {
            SortOrder::Chronological => {
                data.messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            }
            SortOrder::ReverseChronological => {
                data.messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            }
            SortOrder::ByAuthor => {
                data.messages.sort_by(|a, b| a.author.cmp(&b.author));
            }
            SortOrder::ByMessageType => {
                data.messages
                    .sort_by(|a, b| a.message_type.cmp(&b.message_type));
            }
            SortOrder::ByAmount => {
                data.messages.sort_by(|a, b| {
                    let amount_a = a.amount.unwrap_or(0.0);
                    let amount_b = b.amount.unwrap_or(0.0);
                    amount_b
                        .partial_cmp(&amount_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
    }

    /// フィルタリング適用
    fn apply_filters(&self, data: &mut SessionData, config: &ExportConfig) {
        // 日付範囲フィルタリング
        if let Some((start, end)) = config.date_range {
            data.messages
                .retain(|msg| msg.timestamp >= start && msg.timestamp <= end);
        }

        // システムメッセージフィルタリング
        if !config.include_system_messages {
            data.messages.retain(|msg| msg.message_type != "system");
        }

        // 削除されたメッセージフィルタリング
        if !config.include_deleted_messages {
            data.messages.retain(|msg| !msg.is_deleted);
        }

        // 最大レコード数制限
        if let Some(max_records) = config.max_records {
            data.messages.truncate(max_records);
        }
    }

    /// 列幅を計算
    fn calculate_column_width(&self, col_idx: usize, _headers: &[&str]) -> f64 {
        match col_idx {
            0 => 15.0,       // ID
            1 => 20.0,       // タイムスタンプ
            2 => 20.0,       // 投稿者
            3 => 15.0,       // 投稿者ID
            4 => 40.0,       // 内容
            5 => 12.0,       // タイプ
            6 => 10.0,       // 金額
            7 => 8.0,        // 通貨
            8 => 8.0,        // 絵文字数
            9 => 8.0,        // 文字数
            10..=13 => 10.0, // ブール値
            14 => 15.0,      // バッジ
            _ => 12.0,
        }
    }
}

impl FormatHandler for ExcelExporter {
    fn export(&self, data: &SessionData, config: &ExportConfig) -> Result<Vec<u8>, ExportError> {
        self.create_workbook(data, config)
    }

    fn file_extension(&self) -> &str {
        "xlsx"
    }

    fn supports_streaming(&self) -> bool {
        false // Excel形式はストリーミング非対応
    }
}

impl Default for ExcelExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Excelへの書き込みを抽象化するトレイト
trait WriteToExcel {
    fn write(&self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<(), XlsxError>;
    fn write_with_format(
        &self,
        worksheet: &mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<(), XlsxError>;
}

impl WriteToExcel for String {
    fn write(&self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<(), XlsxError> {
        worksheet.write_string(row, col, self)?;
        Ok(())
    }

    fn write_with_format(
        &self,
        worksheet: &mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<(), XlsxError> {
        worksheet.write_string_with_format(row, col, self, format)?;
        Ok(())
    }
}

// XlsxErrorをExportErrorに変換
impl From<XlsxError> for ExportError {
    fn from(error: XlsxError) -> Self {
        ExportError::Serialization(format!("Excel error: {}", error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::export::session_data::{ExportableData, SessionData};
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_session_data() -> SessionData {
        let mut data = SessionData::new(
            "test-session".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test Channel".to_string(),
            "test-channel-id".to_string(),
        );

        // テストメッセージを追加
        data.messages.push(ExportableData {
            id: "msg1".to_string(),
            timestamp: Utc::now(),
            author: "User1".to_string(),
            author_id: "user1".to_string(),
            content: "Hello Excel!".to_string(),
            message_type: "text".to_string(),
            amount: None,
            currency: None,
            emoji_count: 0,
            word_count: 2,
            is_deleted: false,
            is_moderator: false,
            is_member: false,
            is_verified: false,
            badges: vec![],
            metadata: HashMap::new(),
        });

        data
    }

    #[test]
    fn test_excel_export() {
        let exporter = ExcelExporter::new();
        let data = create_test_session_data();
        let config = ExportConfig::default();

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());

        let excel_bytes = result.unwrap();
        assert!(!excel_bytes.is_empty());
        // Excel形式の最初のバイトをチェック（ZIP形式の開始）
        assert_eq!(&excel_bytes[0..2], b"PK");
    }

    #[test]
    fn test_excel_export_single_sheet() {
        let exporter = ExcelExporter::new().with_multi_sheet(false);
        let data = create_test_session_data();
        let config = ExportConfig::default();

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_excel_export_without_formatting() {
        let exporter = ExcelExporter::new().with_cell_formatting(false);
        let data = create_test_session_data();
        let config = ExportConfig::default();

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());
    }
}
