use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub mod csv_exporter;
pub mod excel_exporter;
pub mod json_exporter;
pub mod session_data;

pub use csv_exporter::CsvExporter;
pub use excel_exporter::ExcelExporter;
pub use json_exporter::JsonExporter;
pub use session_data::{ExportableData, SessionData};

/// エクスポート形式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Json,
    Excel,
}

impl ExportFormat {
    pub fn file_extension(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Excel => "xlsx",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "text/csv",
            ExportFormat::Json => "application/json",
            ExportFormat::Excel => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            }
        }
    }
}

/// エクスポートエラー
#[derive(Error, Debug)]
pub enum ExportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Unsupported format: {format:?}")]
    UnsupportedFormat { format: ExportFormat },

    #[error("Invalid data: {message}")]
    InvalidData { message: String },

    #[error("File access error: {path}")]
    FileAccess { path: String },
}

/// エクスポート設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub include_metadata: bool,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub include_system_messages: bool,
    pub include_deleted_messages: bool,
    pub max_records: Option<usize>,
    pub sort_order: SortOrder,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            include_metadata: true,
            date_range: None,
            include_system_messages: false,
            include_deleted_messages: false,
            max_records: None,
            sort_order: SortOrder::Chronological,
        }
    }
}

/// ソート順序
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Chronological,
    ReverseChronological,
    ByAuthor,
    ByMessageType,
    ByAmount,
}

/// フォーマットハンドラートレイト
pub trait FormatHandler: Send + Sync {
    fn export(&self, data: &SessionData, config: &ExportConfig) -> Result<Vec<u8>, ExportError>;
    fn file_extension(&self) -> &str;
    fn supports_streaming(&self) -> bool {
        false
    }
}

/// エクスポートマネージャー
pub struct ExportManager {
    format_handlers: HashMap<ExportFormat, Box<dyn FormatHandler>>,
}

impl ExportManager {
    /// 新しいエクスポートマネージャーを作成
    pub fn new() -> Self {
        let mut manager = Self {
            format_handlers: HashMap::new(),
        };

        // デフォルトハンドラーを登録
        manager.register_handler(ExportFormat::Csv, Box::new(CsvExporter::new()));
        manager.register_handler(ExportFormat::Json, Box::new(JsonExporter::new()));
        manager.register_handler(ExportFormat::Excel, Box::new(ExcelExporter::new()));

        manager
    }

    /// フォーマットハンドラーを登録
    pub fn register_handler(&mut self, format: ExportFormat, handler: Box<dyn FormatHandler>) {
        self.format_handlers.insert(format, handler);
    }

    /// データをエクスポート
    pub fn export(
        &self,
        data: &SessionData,
        config: &ExportConfig,
    ) -> Result<Vec<u8>, ExportError> {
        let handler =
            self.format_handlers
                .get(&config.format)
                .ok_or(ExportError::UnsupportedFormat {
                    format: config.format,
                })?;

        handler.export(data, config)
    }

    /// サポートされている形式を取得
    pub fn supported_formats(&self) -> Vec<ExportFormat> {
        self.format_handlers.keys().copied().collect()
    }

    /// 設定の妥当性を検証
    pub fn validate_config(&self, config: &ExportConfig) -> Result<(), ExportError> {
        if !self.format_handlers.contains_key(&config.format) {
            return Err(ExportError::UnsupportedFormat {
                format: config.format,
            });
        }

        if let Some(max_records) = config.max_records {
            if max_records == 0 {
                return Err(ExportError::InvalidData {
                    message: "max_records must be greater than 0".to_string(),
                });
            }
        }

        if let Some((start, end)) = config.date_range {
            if start >= end {
                return Err(ExportError::InvalidData {
                    message: "date range start must be before end".to_string(),
                });
            }
        }

        Ok(())
    }

    /// エクスポート可能なデータサイズを推定
    pub fn estimate_export_size(
        &self,
        data: &SessionData,
        config: &ExportConfig,
    ) -> Result<usize, ExportError> {
        self.validate_config(config)?;

        let filtered_message_count = self.count_filtered_messages(data, config);

        // 形式に応じたサイズ推定
        let estimated_size_per_message = match config.format {
            ExportFormat::Csv => 150,   // 平均150バイト/メッセージ
            ExportFormat::Json => 300,  // 平均300バイト/メッセージ（構造化データのため）
            ExportFormat::Excel => 100, // 平均100バイト/メッセージ（バイナリ形式のため効率的）
        };

        let base_size = if config.include_metadata { 2048 } else { 512 };

        Ok(base_size + (filtered_message_count * estimated_size_per_message))
    }

    /// フィルタリング後のメッセージ数をカウント
    fn count_filtered_messages(&self, data: &SessionData, config: &ExportConfig) -> usize {
        data.messages
            .iter()
            .filter(|msg| self.message_matches_filter(msg, config))
            .count()
    }

    /// メッセージがフィルタリング条件に一致するかチェック
    fn message_matches_filter(&self, message: &ExportableData, config: &ExportConfig) -> bool {
        // システムメッセージのフィルタリング
        if !config.include_system_messages && message.message_type == "system" {
            return false;
        }

        // 削除されたメッセージのフィルタリング
        if !config.include_deleted_messages && message.is_deleted {
            return false;
        }

        // 日付範囲のフィルタリング
        if let Some((start, end)) = config.date_range {
            if message.timestamp < start || message.timestamp > end {
                return false;
            }
        }

        true
    }
}

impl Default for ExportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::export::{CsvExporter, JsonExporter};

    #[test]
    fn test_export_format_extensions() {
        assert_eq!(ExportFormat::Csv.file_extension(), "csv");
        assert_eq!(ExportFormat::Json.file_extension(), "json");
        assert_eq!(ExportFormat::Excel.file_extension(), "xlsx");
    }

    #[test]
    fn test_export_config_validation() {
        let manager = ExportManager::new();

        let valid_config = ExportConfig::default();
        assert!(manager.validate_config(&valid_config).is_ok());

        let invalid_config = ExportConfig {
            max_records: Some(0),
            ..Default::default()
        };
        assert!(manager.validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_unsupported_format() {
        // カスタムマネージャーを作成し、一部のハンドラーのみを登録
        let mut manager = ExportManager {
            format_handlers: HashMap::new(),
        };
        // CSVとJSONのみを登録
        manager.register_handler(ExportFormat::Csv, Box::new(CsvExporter::new()));
        manager.register_handler(ExportFormat::Json, Box::new(JsonExporter::new()));

        let data = SessionData::default();
        let config = ExportConfig {
            format: ExportFormat::Excel, // Excelハンドラーが未登録
            ..Default::default()
        };

        let result = manager.export(&data, &config);
        assert!(matches!(result, Err(ExportError::UnsupportedFormat { .. })));
    }
}
