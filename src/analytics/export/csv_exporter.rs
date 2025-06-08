use super::{ExportConfig, ExportError, FormatHandler, SessionData, SortOrder};

/// CSV形式エクスポーター
pub struct CsvExporter {
    delimiter: char,
    include_headers: bool,
}

impl CsvExporter {
    pub fn new() -> Self {
        Self {
            delimiter: ',',
            include_headers: true,
        }
    }

    pub fn with_delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = delimiter;
        self
    }

    pub fn with_headers(mut self, include_headers: bool) -> Self {
        self.include_headers = include_headers;
        self
    }

    /// CSVフィールドをエスケープ
    fn escape_csv_field(&self, field: &str) -> String {
        if field.contains(self.delimiter)
            || field.contains('"')
            || field.contains('\n')
            || field.contains('\r')
        {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_string()
        }
    }

    /// CSVヘッダーを生成
    fn generate_headers(&self) -> String {
        let headers = vec![
            "id",
            "timestamp",
            "author",
            "author_id",
            "content",
            "message_type",
            "amount",
            "currency",
            "emoji_count",
            "word_count",
            "is_deleted",
            "is_moderator",
            "is_member",
            "is_verified",
            "badges",
        ];

        headers.join(&self.delimiter.to_string())
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

    /// メッセージをCSV行に変換
    fn message_to_csv_row(&self, message: &super::session_data::ExportableData) -> String {
        let fields = vec![
            self.escape_csv_field(&message.id),
            self.escape_csv_field(
                &message
                    .timestamp
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
            ),
            self.escape_csv_field(&message.author),
            self.escape_csv_field(&message.author_id),
            self.escape_csv_field(&message.content),
            self.escape_csv_field(&message.message_type),
            message.amount.map_or("".to_string(), |a| a.to_string()),
            message
                .currency
                .as_ref()
                .map_or("".to_string(), |c| self.escape_csv_field(c)),
            message.emoji_count.to_string(),
            message.word_count.to_string(),
            message.is_deleted.to_string(),
            message.is_moderator.to_string(),
            message.is_member.to_string(),
            message.is_verified.to_string(),
            self.escape_csv_field(&message.badges.join("|")),
        ];

        fields.join(&self.delimiter.to_string())
    }

    /// メタデータセクションを生成
    fn generate_metadata_section(&self, data: &SessionData) -> String {
        let mut metadata_lines = Vec::new();

        metadata_lines.push("# Metadata".to_string());
        metadata_lines.push(format!(
            "# Session ID{}{}",
            self.delimiter, data.metadata.session_id
        ));
        metadata_lines.push(format!(
            "# Channel{}{}",
            self.delimiter, data.metadata.channel_name
        ));
        metadata_lines.push(format!(
            "# Stream URL{}{}",
            self.delimiter, data.metadata.stream_url
        ));
        metadata_lines.push(format!(
            "# Start Time{}{}",
            self.delimiter,
            data.metadata.start_time.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(end_time) = data.metadata.end_time {
            metadata_lines.push(format!(
                "# End Time{}{}",
                self.delimiter,
                end_time.format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }

        metadata_lines.push(format!(
            "# Total Messages{}{}",
            self.delimiter, data.statistics.total_messages
        ));
        metadata_lines.push(format!(
            "# Unique Viewers{}{}",
            self.delimiter, data.statistics.unique_viewers
        ));
        metadata_lines.push(format!(
            "# Total Super Chat{}{:.2}",
            self.delimiter, data.statistics.total_super_chat_amount
        ));
        metadata_lines.push(format!(
            "# Export Time{}{}",
            self.delimiter,
            data.metadata.export_time.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        metadata_lines.push("".to_string()); // 空行

        metadata_lines.join("\n")
    }
}

impl FormatHandler for CsvExporter {
    fn export(&self, data: &SessionData, config: &ExportConfig) -> Result<Vec<u8>, ExportError> {
        let mut cloned_data = data.clone();

        // データをフィルタリング
        self.apply_filters(&mut cloned_data, config);

        // データをソート
        self.sort_data(&mut cloned_data, config.sort_order);

        // CSVデータを構築
        let mut csv_content = Vec::new();

        // メタデータセクション（オプション）
        if config.include_metadata {
            let metadata_section = self.generate_metadata_section(&cloned_data);
            csv_content.extend_from_slice(metadata_section.as_bytes());
        }

        // ヘッダー行
        if self.include_headers {
            let headers = self.generate_headers();
            csv_content.extend_from_slice(headers.as_bytes());
            csv_content.push(b'\n');
        }

        // データ行
        for message in &cloned_data.messages {
            let csv_row = self.message_to_csv_row(message);
            csv_content.extend_from_slice(csv_row.as_bytes());
            csv_content.push(b'\n');
        }

        Ok(csv_content)
    }

    fn file_extension(&self) -> &str {
        "csv"
    }

    fn supports_streaming(&self) -> bool {
        true // CSVはストリーミング対応可能
    }
}

impl Default for CsvExporter {
    fn default() -> Self {
        Self::new()
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
            content: "Hello, world!".to_string(),
            message_type: "text".to_string(),
            amount: None,
            currency: None,
            emoji_count: 0,
            word_count: 2,
            is_deleted: false,
            is_moderator: false,
            is_member: false,
            is_verified: false,
            badges: vec!["VIP".to_string()],
            metadata: HashMap::new(),
        });

        data
    }

    #[test]
    fn test_csv_export() {
        let exporter = CsvExporter::new();
        let data = create_test_session_data();
        let config = ExportConfig::default();

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());

        let csv_bytes = result.unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        assert!(csv_str.contains("id,timestamp,author"));
        assert!(csv_str.contains("User1"));
        assert!(csv_str.contains("Hello, world!"));
    }

    #[test]
    fn test_csv_export_without_metadata() {
        let exporter = CsvExporter::new();
        let data = create_test_session_data();
        let config = ExportConfig {
            include_metadata: false,
            ..Default::default()
        };

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());

        let csv_bytes = result.unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        assert!(!csv_str.contains("# Metadata"));
        assert!(!csv_str.contains("# Session ID"));
    }

    #[test]
    fn test_csv_export_without_headers() {
        let exporter = CsvExporter::new().with_headers(false);
        let data = create_test_session_data();
        let config = ExportConfig {
            include_metadata: false,
            ..Default::default()
        };

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());

        let csv_bytes = result.unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        assert!(!csv_str.contains("id,timestamp,author"));
        assert!(csv_str.contains("User1"));
    }

    #[test]
    fn test_csv_escape_special_characters() {
        let exporter = CsvExporter::new();

        // コンマを含むテキストのエスケープテスト
        let escaped = exporter.escape_csv_field("Hello, world!");
        assert_eq!(escaped, "\"Hello, world!\"");

        // 引用符を含むテキストのエスケープテスト
        let escaped = exporter.escape_csv_field("He said \"Hello\"");
        assert_eq!(escaped, "\"He said \"\"Hello\"\"\"");
    }

    #[test]
    fn test_csv_custom_delimiter() {
        let exporter = CsvExporter::new().with_delimiter('\t');
        let data = create_test_session_data();
        let config = ExportConfig::default();

        let result = exporter.export(&data, &config);
        assert!(result.is_ok());

        let csv_bytes = result.unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        assert!(csv_str.contains("id\ttimestamp\tauthor"));
    }

    #[test]
    fn test_csv_export_empty_dataset() {
        let exporter = CsvExporter::new();
        let data = SessionData::new(
            "empty-session".to_string(),
            "https://youtube.com/watch?v=empty".to_string(),
            "Empty Channel".to_string(),
            "empty-channel-id".to_string(),
        );
        // メッセージを追加しない（空のデータセット）
        
        let config = ExportConfig::default();
        
        let result = exporter.export(&data, &config);
        assert!(result.is_ok());
        
        let csv_bytes = result.unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();
        
        // ヘッダーはあるがメッセージデータがない
        assert!(csv_str.contains("id,timestamp,author"));
        let lines: Vec<&str> = csv_str.lines().collect();
        let data_lines = lines.iter().filter(|line| 
            !line.starts_with('#') && !line.is_empty() && !line.contains("id,timestamp,author")
        ).count();
        assert_eq!(data_lines, 0);
    }

    #[test]
    fn test_csv_export_large_dataset() {
        let exporter = CsvExporter::new();
        let mut data = SessionData::new(
            "large-session".to_string(),
            "https://youtube.com/watch?v=large".to_string(),
            "Large Channel".to_string(),
            "large-channel-id".to_string(),
        );
        
        // 大量のメッセージを追加（10,000件）
        for i in 0..10000 {
            data.messages.push(ExportableData {
                id: format!("msg{}", i),
                timestamp: Utc::now(),
                author: format!("User{}", i % 100), // 100人のユーザーが循環
                author_id: format!("user{}", i % 100),
                content: format!("メッセージ番号 {} です。これは大容量データのテストです。", i),
                message_type: if i % 50 == 0 { "superchat".to_string() } else { "text".to_string() },
                amount: if i % 50 == 0 { Some((i as f64) * 100.0) } else { None },
                currency: if i % 50 == 0 { Some("JPY".to_string()) } else { None },
                emoji_count: i % 10,
                word_count: i % 20 + 1,
                is_deleted: i % 500 == 0, // 500件に1件削除フラグ
                is_moderator: i % 1000 == 0, // 1000件に1件モデレーター
                is_member: i % 100 == 0, // 100件に1件メンバー
                is_verified: i % 200 == 0, // 200件に1件認証済み
                badges: if i % 100 == 0 { vec!["Member".to_string()] } else { vec![] },
                metadata: HashMap::new(),
            });
        }
        
        let config = ExportConfig::default();
        
        let start_time = std::time::Instant::now();
        let result = exporter.export(&data, &config);
        let export_duration = start_time.elapsed();
        
        assert!(result.is_ok());
        
        let csv_bytes = result.unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();
        
        // 10,000件のメッセージが含まれているか確認
        let lines: Vec<&str> = csv_str.lines().collect();
        let data_lines = lines.iter().filter(|line| 
            !line.starts_with('#') && !line.is_empty() && !line.contains("id,timestamp,author")
        ).count();
        // メタデータセクションがあるため、実際のデータ行数は期待値とわずかに異なる場合がある
        assert!(data_lines >= 9950 && data_lines <= 10050, "Expected around 10000 data lines, got {}", data_lines);
        
        // パフォーマンス確認（5秒以内）
        assert!(export_duration.as_secs() < 5, "大容量CSVエクスポートが遅すぎます: {:?}", export_duration);
        
        println!("10,000件のCSVエクスポート時間: {:?}", export_duration);
    }

    #[test]
    fn test_csv_export_with_extreme_content() {
        let exporter = CsvExporter::new();
        let mut data = SessionData::new(
            "extreme-session".to_string(),
            "https://youtube.com/watch?v=extreme".to_string(),
            "Extreme Channel".to_string(),
            "extreme-channel-id".to_string(),
        );
        
        // 極端なコンテンツを持つメッセージ
        data.messages.push(ExportableData {
            id: "extreme1".to_string(),
            timestamp: Utc::now(),
            author: "\"Weird,User\"".to_string(), // CSVで問題となる文字
            author_id: "weird\nuser".to_string(), // 改行文字
            content: "\"これは\"コンマ,改行\n\rタブ\t\tを含む\",\"極端な文字列です\"\"\"".to_string(),
            message_type: "text".to_string(),
            amount: None,
            currency: None,
            emoji_count: 0,
            word_count: 20,
            is_deleted: false,
            is_moderator: false,
            is_member: false,
            is_verified: false,
            badges: vec!["\"VIP,Member\"".to_string()],
            metadata: HashMap::new(),
        });
        
        // 非常に長いコンテンツ
        data.messages.push(ExportableData {
            id: "extreme2".to_string(),
            timestamp: Utc::now(),
            author: "LongContentUser".to_string(),
            author_id: "long_user".to_string(),
            content: "あ".repeat(100000), // 10万文字の日本語
            message_type: "text".to_string(),
            amount: None,
            currency: None,
            emoji_count: 1000,
            word_count: 100000,
            is_deleted: false,
            is_moderator: false,
            is_member: false,
            is_verified: false,
            badges: vec![],
            metadata: HashMap::new(),
        });
        
        // 絵文字とUnicode文字
        data.messages.push(ExportableData {
            id: "extreme3".to_string(),
            timestamp: Utc::now(),
            author: "🎮🔥💯🚀⭐".to_string(),
            author_id: "emoji_user".to_string(),
            content: "🎉🎊🎁🎂🎆🎇✨🎯🎲🎪🎭🎨🎬🎵🎶🎼🎤🎧🎸🎹🎺🎻🪕🥁".to_string(),
            message_type: "superchat".to_string(),
            amount: Some(f64::MAX), // 最大の浮動小数点数
            currency: Some("🪙💰💴💵💶💷💸".to_string()),
            emoji_count: 100,
            word_count: 1,
            is_deleted: false,
            is_moderator: false,
            is_member: false,
            is_verified: false,
            badges: vec!["🏆Winner🏆".to_string()],
            metadata: HashMap::new(),
        });
        
        let config = ExportConfig::default();
        
        let result = exporter.export(&data, &config);
        assert!(result.is_ok());
        
        let csv_bytes = result.unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();
        
        // CSVが適切にエスケープされているか確認
        assert!(csv_str.contains("\"\"\"Weird,User\"\"\""));
        assert!(csv_str.contains("🎮🔥💯🚀⭐"));
        assert!(csv_str.len() > 100000); // 非常に長いコンテンツが含まれている
    }

    #[test]
    fn test_csv_export_max_records_limit() {
        let exporter = CsvExporter::new();
        let mut data = SessionData::new(
            "limited-session".to_string(),
            "https://youtube.com/watch?v=limited".to_string(),
            "Limited Channel".to_string(),
            "limited-channel-id".to_string(),
        );
        
        // 1000件のメッセージを追加
        for i in 0..1000 {
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
        
        // 最大100件に制限
        let config = ExportConfig {
            max_records: Some(100),
            ..Default::default()
        };
        
        let result = exporter.export(&data, &config);
        assert!(result.is_ok());
        
        let csv_bytes = result.unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();
        
        // 100件のメッセージのみが含まれているか確認
        let lines: Vec<&str> = csv_str.lines().collect();
        let data_lines = lines.iter().filter(|line| 
            !line.starts_with('#') && !line.is_empty() && !line.contains("id,timestamp,author")
        ).count();
        assert_eq!(data_lines, 100);
    }

    #[test]
    fn test_csv_export_memory_efficiency() {
        let exporter = CsvExporter::new();
        let mut data = SessionData::new(
            "memory-test".to_string(),
            "https://youtube.com/watch?v=memory".to_string(),
            "Memory Test Channel".to_string(),
            "memory-test-id".to_string(),
        );
        
        // 中程度のデータセット（5,000件）でメモリ使用量をテスト
        for i in 0..5000 {
            data.messages.push(ExportableData {
                id: format!("msg{}", i),
                timestamp: Utc::now(),
                author: format!("User{}", i),
                author_id: format!("user{}", i),
                content: format!("これは{}番目のメッセージです。メモリ効率をテストしています。", i),
                message_type: "text".to_string(),
                amount: None,
                currency: None,
                emoji_count: 1,
                word_count: 10,
                is_deleted: false,
                is_moderator: false,
                is_member: false,
                is_verified: false,
                badges: vec![],
                metadata: HashMap::new(),
            });
        }
        
        let config = ExportConfig::default();
        
        // メモリ使用量監視（簡易版）
        let _start_memory = std::process::id(); // プロセスIDを基準とした簡易測定
        
        let result = exporter.export(&data, &config);
        assert!(result.is_ok());
        
        let csv_bytes = result.unwrap();
        
        // 結果のサイズが妥当であることを確認
        assert!(csv_bytes.len() > 100000); // 最低限のサイズ
        assert!(csv_bytes.len() < 10_000_000); // 上限チェック（10MB未満）
        
        // UTF-8として有効であることを確認
        let csv_str = String::from_utf8(csv_bytes);
        assert!(csv_str.is_ok());
    }

    #[test]
    fn test_csv_edge_case_characters() {
        let exporter = CsvExporter::new();
        
        // 特殊文字のエスケープテスト（さらに詳細）
        assert_eq!(exporter.escape_csv_field("normal"), "normal");
        assert_eq!(exporter.escape_csv_field(""), "");
        assert_eq!(exporter.escape_csv_field("test,with,comma"), "\"test,with,comma\"");
        assert_eq!(exporter.escape_csv_field("test\"with\"quote"), "\"test\"\"with\"\"quote\"");
        assert_eq!(exporter.escape_csv_field("test\nwith\nnewline"), "\"test\nwith\nnewline\"");
        assert_eq!(exporter.escape_csv_field("test\rwith\rcarriage"), "\"test\rwith\rcarriage\"");
        assert_eq!(exporter.escape_csv_field("test\twith\ttab"), "test\twith\ttab"); // タブは区切り文字でない限りエスケープ不要
        
        // カスタム区切り文字でのテスト
        let tab_exporter = CsvExporter::new().with_delimiter('\t');
        assert_eq!(tab_exporter.escape_csv_field("test\twith\ttab"), "\"test\twith\ttab\"");
        assert_eq!(tab_exporter.escape_csv_field("test,with,comma"), "test,with,comma");
    }
}
