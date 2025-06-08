//! NDJSON (Newline Delimited JSON) file processing utilities.
//!
//! This module provides functions for reading and parsing NDJSON files,
//! with comprehensive error handling and validation.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
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
            Err(LiveChatError::Generic { context, .. }) => {
                assert!(context.contains("opening file"));
            }
            _ => panic!("Expected generic error for non-existent file"),
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
}
