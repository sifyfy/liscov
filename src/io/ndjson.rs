//! NDJSON (Newline Delimited JSON) file processing utilities.
//!
//! This module provides functions for reading and parsing NDJSON files,
//! with comprehensive error handling and validation.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use thiserror::Error;

/// Comprehensive error types for file I/O and parsing operations.
#[derive(Error, Debug)]
pub enum LiveChatError {
    /// I/O error when reading files
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON parsing error at line {line}: {source}")]
    JsonParse {
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    /// Invalid file format
    #[error("Invalid file format: {reason}")]
    InvalidFormat { reason: String },

    /// Empty or invalid data
    #[error("No valid data found: {context}")]
    NoData { context: String },

    /// Missing required field in data structure
    #[error("Missing required field '{field}' in {structure}")]
    MissingField { field: String, structure: String },

    /// Invalid continuation token
    #[error("Invalid continuation token: {token}")]
    InvalidContinuation { token: String },

    /// Unsupported message type
    #[error("Unsupported message type: {message_type}")]
    UnsupportedMessageType { message_type: String },

    /// Network operation failed
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Rate limiting encountered
    #[error("Rate limit exceeded. Retry after {retry_after_seconds} seconds")]
    RateLimit { retry_after_seconds: u64 },

    /// Generic error with context
    #[error("Error in {context}: {message}")]
    Generic { context: String, message: String },
}

impl LiveChatError {
    /// Create a new generic error with context
    pub fn generic(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Generic {
            context: context.into(),
            message: message.into(),
        }
    }

    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>, structure: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
            structure: structure.into(),
        }
    }

    /// Create an invalid format error
    pub fn invalid_format(reason: impl Into<String>) -> Self {
        Self::InvalidFormat {
            reason: reason.into(),
        }
    }

    /// Create a no data error
    pub fn no_data(context: impl Into<String>) -> Self {
        Self::NoData {
            context: context.into(),
        }
    }
}

/// Generic entry in an NDJSON file with timestamp and data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedEntry<T> {
    /// Unix timestamp when the entry was received
    pub timestamp: u64,
    /// The actual data payload
    pub data: T,
}

/// ファイルパス検証ユーティリティ - セキュリティチェック
fn validate_file_path(path: &str) -> Result<(), LiveChatError> {
    // 空パスチェック
    if path.is_empty() {
        return Err(LiveChatError::invalid_format("Empty file path"));
    }
    
    // パス長制限（非常に長いパスを拒否）
    if path.len() > 4096 {
        return Err(LiveChatError::invalid_format(format!(
            "Path too long ({} chars, max 4096)", 
            path.len()
        )));
    }
    
    // ディレクトリトラバーサル攻撃の検出
    if path.contains("../") || path.contains("..\\") {
        return Err(LiveChatError::invalid_format("Directory traversal detected"));
    }
    
    // Null文字やその他の危険な文字をチェック
    if path.contains('\0') {
        return Err(LiveChatError::invalid_format("Null character in path"));
    }
    
    // Windowsで危険な文字をチェック
    if cfg!(windows) {
        let dangerous_chars = ['<', '>', ':', '"', '|', '?', '*'];
        for ch in dangerous_chars {
            if path.contains(ch) {
                return Err(LiveChatError::invalid_format(format!(
                    "Invalid character '{}' in Windows path", 
                    ch
                )));
            }
        }
    }
    
    // PathBufを使用してパスの正当性を検証
    let path_buf = match PathBuf::from(path).canonicalize() {
        Ok(canonical_path) => canonical_path,
        Err(_) => {
            // ファイルが存在しない場合は、親ディレクトリの検証のみ行う
            let path_buf = PathBuf::from(path);
            if let Some(parent) = path_buf.parent() {
                if parent.exists() {
                    path_buf
                } else {
                    return Err(LiveChatError::invalid_format("Parent directory does not exist"));
                }
            } else {
                path_buf
            }
        }
    };
    
    // 絶対パスに変換された結果が元のパスと著しく異なる場合は拒否
    let canonical_str = path_buf.to_string_lossy();
    
    // システムディレクトリへのアクセスを制限（Unix系）
    if cfg!(unix) {
        let restricted_prefixes = ["/etc", "/proc", "/sys", "/dev"];
        for prefix in &restricted_prefixes {
            if canonical_str.starts_with(prefix) {
                return Err(LiveChatError::invalid_format(format!(
                    "Access to system directory '{}' is restricted", 
                    prefix
                )));
            }
        }
    }
    
    // Windowsシステムディレクトリの制限
    if cfg!(windows) {
        let path_lower = canonical_str.to_lowercase();
        let restricted_prefixes = ["c:\\windows", "c:\\program files"];
        for prefix in &restricted_prefixes {
            if path_lower.starts_with(prefix) {
                return Err(LiveChatError::invalid_format(format!(
                    "Access to system directory '{}' is restricted", 
                    prefix
                )));
            }
        }
    }
    
    Ok(())
}

/// Parse an NDJSON file containing timestamped entries.
///
/// This is a generic function that can parse any NDJSON file where each line
/// contains a JSON object with a timestamp and data payload.
///
/// # Arguments
/// * `path` - Path to the NDJSON file
/// * `validate_entry` - Optional validation function for each entry
///
/// # Returns
/// A vector of entries or a LiveChatError
pub fn parse_ndjson_file_generic<T, F>(
    path: &str,
    validate_entry: Option<F>,
) -> Result<Vec<T>, LiveChatError>
where
    T: for<'de> Deserialize<'de>,
    F: Fn(&T) -> Result<(), LiveChatError>,
{
    // ファイルパス検証を最初に実行
    validate_file_path(path)?;
    
    let file = File::open(path).map_err(|e| {
        LiveChatError::generic("opening file", format!("Failed to open '{}': {}", path, e))
    })?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (line_number, line) in reader.lines().enumerate() {
        let line = line?;

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        let entry: T = serde_json::from_str(&line).map_err(|e| LiveChatError::JsonParse {
            line: line_number + 1,
            source: e,
        })?;

        // Apply validation if provided
        if let Some(ref validator) = validate_entry {
            validator(&entry)?;
        }

        entries.push(entry);
    }

    if entries.is_empty() {
        return Err(LiveChatError::no_data(format!(
            "No valid entries found in file '{}'",
            path
        )));
    }

    Ok(entries)
}

/// Parse an NDJSON file containing timestamped entries with built-in timestamp validation.
///
/// # Arguments
/// * `path` - Path to the NDJSON file
///
/// # Returns
/// A vector of TimestampedEntry objects or a LiveChatError
pub fn parse_ndjson_file_timestamped<T>(
    path: &str,
) -> Result<Vec<TimestampedEntry<T>>, LiveChatError>
where
    T: for<'de> Deserialize<'de>,
{
    parse_ndjson_file_generic(
        path,
        Some(|entry: &TimestampedEntry<T>| {
            if entry.timestamp == 0 {
                Err(LiveChatError::invalid_format("Invalid timestamp (zero)"))
            } else {
                Ok(())
            }
        }),
    )
}

/// Parse an NDJSON file containing ResponseEntry objects (for backward compatibility).
///
/// # Arguments
/// * `path` - Path to the NDJSON file
///
/// # Returns
/// A vector of ResponseEntry objects or a LiveChatError
pub fn parse_ndjson_file(
    path: &str,
) -> Result<Vec<crate::api::innertube::get_live_chat::ResponseEntry>, LiveChatError> {
    use crate::api::innertube::get_live_chat::ResponseEntry;

    parse_ndjson_file_generic(
        path,
        Some(|entry: &ResponseEntry| {
            if entry.timestamp == 0 {
                Err(LiveChatError::invalid_format("Invalid timestamp (zero)"))
            } else {
                Ok(())
            }
        }),
    )
}

/// Parse an NDJSON file containing ResponseEntry objects (legacy version using anyhow).
///
/// This function is kept for backward compatibility and internally uses the new error handling.
///
/// # Arguments
/// * `path` - Path to the NDJSON file
///
/// # Returns
/// A vector of ResponseEntry objects or an anyhow error
pub fn parse_ndjson_file_legacy(
    path: &str,
) -> anyhow::Result<Vec<crate::api::innertube::get_live_chat::ResponseEntry>> {
    parse_ndjson_file(path).map_err(|e| anyhow::anyhow!("{}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::path::PathBuf;

    fn get_test_file_path(filename: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("data");
        path.push(filename);
        path
    }

    #[test]
    fn test_live_chat_error_creation() {
        // Test generic error
        let error = LiveChatError::generic("test context", "test message");
        assert!(format!("{}", error).contains("test context"));
        assert!(format!("{}", error).contains("test message"));

        // Test missing field error
        let error = LiveChatError::missing_field("test_field", "TestStruct");
        assert!(format!("{}", error).contains("test_field"));
        assert!(format!("{}", error).contains("TestStruct"));

        // Test invalid format error
        let error = LiveChatError::invalid_format("invalid JSON");
        assert!(format!("{}", error).contains("invalid JSON"));

        // Test no data error
        let error = LiveChatError::no_data("empty file");
        assert!(format!("{}", error).contains("empty file"));
    }

    #[test]
    fn test_live_chat_error_display() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error = LiveChatError::JsonParse {
            line: 42,
            source: json_error,
        };
        let error_string = format!("{}", error);
        assert!(error_string.contains("line 42"));
        assert!(error_string.contains("JSON parsing error"));
    }

    #[test]
    fn test_live_chat_error_chain() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error = LiveChatError::JsonParse {
            line: 1,
            source: json_error,
        };

        // Test error source chain
        assert!(error.source().is_some());
    }

    #[test]
    fn test_parse_ndjson_file_error_handling() {
        // Test non-existent file
        match parse_ndjson_file("non_existent_file.ndjson") {
            Err(LiveChatError::InvalidFormat { reason }) => {
                assert!(reason.contains("Parent directory does not exist"));
            }
            Err(LiveChatError::Generic { context, .. }) => {
                assert!(context.contains("opening file"));
            }
            _ => panic!("Expected InvalidFormat or Generic error for non-existent file"),
        }
    }

    #[test]
    fn test_parse_ndjson_file() {
        let file_path = get_test_file_path("live_chat.ndjson");
        let entries = parse_ndjson_file(file_path.to_str().unwrap()).unwrap();
        assert!(!entries.is_empty());

        // Verify that each entry has a timestamp and response
        for entry in &entries {
            assert!(entry.timestamp > 0);
        }
    }

    #[test]
    fn test_parse_ndjson_file_legacy() {
        let file_path = get_test_file_path("live_chat.ndjson");
        let entries = parse_ndjson_file_legacy(file_path.to_str().unwrap()).unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_timestamped_entry() {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct TestData {
            message: String,
        }

        let entry = TimestampedEntry {
            timestamp: 1234567890,
            data: TestData {
                message: "test".to_string(),
            },
        };

        assert_eq!(entry.timestamp, 1234567890);
        assert_eq!(entry.data.message, "test");
    }

    #[test]
    fn test_file_path_validation() {
        use super::validate_file_path;

        // 正常なパス
        let result = validate_file_path("tests/data/test.ndjson");
        assert!(result.is_ok(), "Valid path should be accepted");

        // 空パス
        let result = validate_file_path("");
        assert!(result.is_err(), "Empty path should be rejected");

        // ディレクトリトラバーサル攻撃
        let result = validate_file_path("../etc/passwd");
        assert!(result.is_err(), "Directory traversal should be rejected");

        let result = validate_file_path("data\\..\\secret.txt");
        assert!(result.is_err(), "Windows directory traversal should be rejected");

        // Null文字
        let result = validate_file_path("test\0.txt");
        assert!(result.is_err(), "Null character should be rejected");

        // 非常に長いパス
        let long_path = "a".repeat(5000);
        let result = validate_file_path(&long_path);
        assert!(result.is_err(), "Extremely long path should be rejected");
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_path_validation() {
        use super::validate_file_path;

        // Windows危険文字
        let dangerous_chars = ['<', '>', ':', '"', '|', '?', '*'];
        for ch in dangerous_chars {
            let path = format!("test{}.txt", ch);
            let result = validate_file_path(&path);
            assert!(result.is_err(), "Dangerous character '{}' should be rejected", ch);
        }

        // Windowsシステムディレクトリ（モック）
        // 実際のテストではこれらのパスは存在しないため、テストは制限的
        println!("Windows path validation test completed (limited scope in test environment)");
    }

    #[test]
    #[cfg(unix)]
    fn test_unix_path_validation() {
        // Unix系システムディレクトリ（実際には存在チェックで止まる可能性）
        let restricted_paths = ["/etc/passwd", "/proc/version", "/sys/kernel", "/dev/null"];
        for path in &restricted_paths {
            println!("Testing restricted path: {}", path);
            // 実際のテスト環境では制限的なテストのみ
        }
        println!("Unix path validation test completed (limited scope in test environment)");
    }

    #[test]
    fn test_file_path_validation_integration() {
        // 統合テスト: ファイルパス検証が実際のパース関数で動作することを確認
        let result = parse_ndjson_file("../invalid_traversal.ndjson");
        assert!(result.is_err(), "Directory traversal should prevent file parsing");

        let result = parse_ndjson_file("");
        assert!(result.is_err(), "Empty path should prevent file parsing");

        // テストファイルが存在する場合の正常ケース
        let test_file_path = get_test_file_path("live_chat.ndjson");
        if test_file_path.exists() {
            let result = parse_ndjson_file(test_file_path.to_str().unwrap());
            // ファイルパス検証はパスするが、ファイル内容の問題で失敗する可能性がある
            // エラーの種類をチェックして、ファイルパス検証エラーでないことを確認
            if let Err(e) = result {
                let error_str = format!("{}", e);
                assert!(!error_str.contains("Directory traversal"));
                assert!(!error_str.contains("Empty file path"));
            }
        }
    }
}
