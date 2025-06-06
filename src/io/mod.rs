//! I/O utilities for file processing and data handling.
//!
//! This module provides utilities for reading and processing various file formats,
//! particularly NDJSON files containing live chat data.

pub mod ndjson;

// Re-export commonly used types and functions
pub use ndjson::{parse_ndjson_file, parse_ndjson_file_legacy, LiveChatError};
