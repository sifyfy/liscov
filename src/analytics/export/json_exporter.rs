use super::{ExportConfig, ExportError, FormatHandler, SessionData, SortOrder};
use serde_json;

/// JSON形式エクスポーター
pub struct JsonExporter {
    pretty_print: bool,
}

impl JsonExporter {
    pub fn new() -> Self {
        Self { pretty_print: true }
    }

    pub fn with_pretty_print(mut self, pretty: bool) -> Self {
        self.pretty_print = pretty;
        self
    }

    /// データをソート順序に従って並べ替え
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

    /// JSON構造を最適化
    fn optimize_json_structure(
        &self,
        data: &SessionData,
        config: &ExportConfig,
    ) -> serde_json::Value {
        let mut json = serde_json::to_value(data)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

        // メタデータを除外する場合
        if !config.include_metadata {
            if let serde_json::Value::Object(ref mut map) = json {
                // 最上位のmetadataフィールドを削除
                map.remove("metadata");

                // メッセージ内のmetadataフィールドも削除
                if let Some(serde_json::Value::Array(ref mut messages)) = map.get_mut("messages") {
                    for message in messages {
                        if let serde_json::Value::Object(ref mut msg_map) = message {
                            msg_map.remove("metadata");
                        }
                    }
                }
            }
        }

        // 空の配列を除外して容量削減
        if let serde_json::Value::Object(ref mut map) = json {
            map.retain(|_, v| !matches!(v, serde_json::Value::Array(arr) if arr.is_empty()));
        }

        json
    }
}

impl FormatHandler for JsonExporter {
    fn export(&self, data: &SessionData, config: &ExportConfig) -> Result<Vec<u8>, ExportError> {
        let mut cloned_data = data.clone();

        // データをフィルタリング
        self.apply_filters(&mut cloned_data, config);

        // データをソート
        self.sort_data(&mut cloned_data, config.sort_order);

        // メタデータが含まれる場合のみ更新
        if config.include_metadata {
            // エクスポート時刻を更新
            cloned_data.metadata.export_time = chrono::Utc::now();

            // フィルター情報を記録
            cloned_data.metadata.filters_applied = vec![
                format!("sort_order: {:?}", config.sort_order),
                format!("include_metadata: {}", config.include_metadata),
                format!(
                    "include_system_messages: {}",
                    config.include_system_messages
                ),
                format!(
                    "include_deleted_messages: {}",
                    config.include_deleted_messages
                ),
            ];

            if let Some((start, end)) = config.date_range {
                cloned_data.metadata.filters_applied.push(format!(
                    "date_range: {} to {}",
                    start.format("%Y-%m-%d %H:%M:%S"),
                    end.format("%Y-%m-%d %H:%M:%S")
                ));
            }

            if let Some(max_records) = config.max_records {
                cloned_data
                    .metadata
                    .filters_applied
                    .push(format!("max_records: {}", max_records));
            }
        }

        // JSON構造を最適化
        let json_value = self.optimize_json_structure(&cloned_data, config);

        // JSONシリアライゼーション
        let json_bytes = if self.pretty_print {
            serde_json::to_vec_pretty(&json_value)
        } else {
            serde_json::to_vec(&json_value)
        }
        .map_err(|e| ExportError::Serialization(e.to_string()))?;

        Ok(json_bytes)
    }

    fn file_extension(&self) -> &str {
        "json"
    }

    fn supports_streaming(&self) -> bool {
        false // 大きなファイルの場合は将来的にストリーミング対応予定
    }
}

impl Default for JsonExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::export::session_data::ExportableData;
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
            content: "Hello".to_string(),
            message_type: "text".to_string(),
            amount: None,
            currency: None,
            emoji_count: 0,
            word_count: 1,
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
    fn test_json_export() {
        let exporter = JsonExporter::new();
        let data = create_test_session_data();
        let config = ExportConfig::default();

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());

        let json_bytes = result.unwrap();
        let json_str = String::from_utf8(json_bytes).unwrap();
        assert!(json_str.contains("test-session"));
        assert!(json_str.contains("Hello"));
    }

    #[test]
    fn test_json_export_without_metadata() {
        let exporter = JsonExporter::new();
        let data = create_test_session_data();
        let config = ExportConfig {
            include_metadata: false,
            ..Default::default()
        };

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());

        let json_bytes = result.unwrap();
        let json_str = String::from_utf8(json_bytes).unwrap();
        assert!(!json_str.contains("metadata"));
    }

    #[test]
    fn test_json_export_with_max_records() {
        let exporter = JsonExporter::new();
        let mut data = create_test_session_data();

        // 追加メッセージを作成
        for i in 2..=5 {
            data.messages.push(ExportableData {
                id: format!("msg{}", i),
                timestamp: Utc::now(),
                author: format!("User{}", i),
                author_id: format!("user{}", i),
                content: format!("Message {}", i),
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
        }

        let config = ExportConfig {
            max_records: Some(2),
            ..Default::default()
        };

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());

        let json_bytes = result.unwrap();
        let json_str = String::from_utf8(json_bytes).unwrap();

        // JSON をパースして messages 配列の長さを確認
        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let messages = json_value["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
    }
}
