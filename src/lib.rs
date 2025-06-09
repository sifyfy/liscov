pub mod analytics;
pub mod api;
pub mod chat_management;
pub mod database;
pub mod gui;
pub mod io;

pub use api::innertube::get_live_chat;

// Unified error handling system
use thiserror::Error;

/// 統一エラー型 - アプリケーション全体で使用される階層化エラー
#[derive(Error, Debug)]
pub enum LiscovError {
    /// API関連エラー（YouTube、InnerTube）
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    
    /// データベース操作エラー
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    
    /// ファイルI/O・データ処理エラー
    #[error("I/O error: {0}")]
    Io(#[from] IoError),
    
    /// GUI・設定関連エラー
    #[error("GUI error: {0}")]
    Gui(#[from] GuiError),
    
    /// アナリティクス・エクスポート関連エラー
    #[error("Analytics error: {0}")]
    Analytics(#[from] AnalyticsError),
    
    /// ネットワーク・HTTP関連エラー
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    /// JSON処理エラー
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// 標準I/Oエラー
    #[error("IO error: {0}")]
    StdIo(#[from] std::io::Error),
    
    /// 設定関連エラー
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// 汎用エラー（既存のanyhowエラーをラップ）
    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
}

/// API関連の具体的エラー
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Request failed: {0}")]
    Request(#[source] reqwest::Error),
    
    #[error("JSON parsing failed: {0}")]
    JsonParse(#[source] serde_json::Error),
    
    #[error("Resource not found")]
    NotFound,
    
    #[error("Authentication failed")]
    Authentication,
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Invalid response format")]
    InvalidFormat,
}

/// データベース関連の具体的エラー
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(String),
    
    #[error("Query execution failed: {0}")]
    Query(String),
    
    #[error("Migration failed: {0}")]
    Migration(String),
    
    #[error("Transaction failed: {0}")]
    Transaction(String),
    
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// ファイルI/O・データ処理の具体的エラー
#[derive(Error, Debug)]
pub enum IoError {
    #[error("File read error: {0}")]
    FileRead(String),
    
    #[error("File write error: {0}")]
    FileWrite(String),
    
    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },
    
    #[error("Invalid file format: {0}")]
    InvalidFormat(String),
    
    #[error("Path error: {0}")]
    Path(String),
}

/// GUI関連の具体的エラー
#[derive(Error, Debug)]
pub enum GuiError {
    #[error("Component initialization failed: {0}")]
    ComponentInit(String),
    
    #[error("State management error: {0}")]
    StateManagement(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Window operation failed: {0}")]
    WindowOperation(String),
    
    #[error("Plugin error: {0}")]
    PluginError(String),
    
    #[error("Service error: {0}")]
    Service(String),
}

/// アナリティクス・エクスポート関連の具体的エラー
#[derive(Error, Debug)]
pub enum AnalyticsError {
    #[error("Export failed: {0}")]
    Export(String),
    
    #[error("Data processing error: {0}")]
    DataProcessing(String),
    
    #[error("Format conversion error: {0}")]
    FormatConversion(String),
}

// 旧エラー型から新エラー型への変換実装
impl From<api::innertube::FetchError> for LiscovError {
    fn from(err: api::innertube::FetchError) -> Self {
        match err {
            api::innertube::FetchError::Request(e) => LiscovError::Network(e),
            api::innertube::FetchError::Serialization(e) => LiscovError::Json(e),
            api::innertube::FetchError::NotFound => LiscovError::Api(ApiError::NotFound),
            api::innertube::FetchError::Other(e) => LiscovError::General(e),
        }
    }
}

impl From<api::youtube::FetchError> for LiscovError {
    fn from(err: api::youtube::FetchError) -> Self {
        match err {
            api::youtube::FetchError::Request(e) => LiscovError::Network(e),
            api::youtube::FetchError::Parse(e) => LiscovError::Json(e),
            api::youtube::FetchError::NotFound => LiscovError::Api(ApiError::NotFound),
        }
    }
}

impl From<io::ndjson::LiveChatError> for LiscovError {
    fn from(err: io::ndjson::LiveChatError) -> Self {
        match err {
            io::ndjson::LiveChatError::Io(e) => LiscovError::StdIo(e),
            io::ndjson::LiveChatError::JsonParse { line, source } => {
                LiscovError::Io(IoError::Parse { 
                    line, 
                    message: source.to_string() 
                })
            },
            io::ndjson::LiveChatError::InvalidFormat { reason } => {
                LiscovError::Io(IoError::InvalidFormat(reason))
            },
            io::ndjson::LiveChatError::NoData { context } => {
                LiscovError::Io(IoError::InvalidFormat(context))
            },
            io::ndjson::LiveChatError::MissingField { field, structure } => {
                LiscovError::Io(IoError::InvalidFormat(format!("Missing field '{}' in {}", field, structure)))
            },
            io::ndjson::LiveChatError::InvalidContinuation { token } => {
                LiscovError::Io(IoError::InvalidFormat(format!("Invalid continuation: {}", token)))
            },
            io::ndjson::LiveChatError::UnsupportedMessageType { message_type } => {
                LiscovError::Io(IoError::InvalidFormat(format!("Unsupported message type: {}", message_type)))
            },
            io::ndjson::LiveChatError::Network(e) => LiscovError::Network(e),
            io::ndjson::LiveChatError::RateLimit { retry_after_seconds: _ } => {
                LiscovError::Api(ApiError::RateLimit)
            },
            io::ndjson::LiveChatError::Generic { context, message } => {
                LiscovError::Io(IoError::InvalidFormat(format!("{}: {}", context, message)))
            },
        }
    }
}

// 便利な結果型エイリアス
pub type LiscovResult<T> = Result<T, LiscovError>;

// Unified error types are defined in this module and available directly

// Re-export the main error types for convenience (legacy compatibility)
pub use api::innertube::FetchError;
pub use api::youtube::FetchError as YoutubeFetchError;
pub use io::ndjson::LiveChatError;

// Re-export I/O utilities for convenience
pub use io::ndjson::{
    parse_ndjson_file, parse_ndjson_file_generic, parse_ndjson_file_legacy,
    parse_ndjson_file_timestamped, TimestampedEntry,
};

// Re-export raw response saver
pub use io::{RawResponseSaver, SaveConfig};

// Re-export InnerTube HTTP client functions
pub use api::innertube::{fetch_live_chat_messages, fetch_live_chat_page, InnerTube};

// Re-export analytics modules
pub use analytics::{ContributorInfo, HourlyRevenue, RevenueAnalytics, RevenueSummary};

// Re-export database modules
pub use database::{LiscovDatabase, Question, Session, ViewerProfile};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Test that the main modules are accessible
        assert!(std::any::type_name::<api::innertube::InnerTube>().contains("InnerTube"));
        assert!(std::any::type_name::<api::youtube::VideoId>().contains("VideoId"));
    }

    #[test]
    fn test_get_live_chat_reexport() {
        // Test that get_live_chat module is properly re-exported
        let _: Option<get_live_chat::ResponseEntry> = None;
        let _: Option<get_live_chat::GetLiveChatResponse> = None;
        let _: Option<get_live_chat::ChatItem> = None;
        let _: Option<get_live_chat::Action> = None;
    }

    #[test]
    fn test_public_api_availability() {
        // Test that key public functions are available
        use get_live_chat::*;

        // Test function signatures - these should compile without errors
        let _result: Result<Vec<ResponseEntry>, LiveChatError> = parse_ndjson_file("test.ndjson");
        let _result2: anyhow::Result<Vec<ResponseEntry>> = parse_ndjson_file_legacy("test.ndjson");

        // Test other function signatures
        let dummy_entries: Vec<ResponseEntry> = vec![];
        let _counts = count_actions_by_type(&dummy_entries);
        let _counts2 = count_renderers_by_type(&dummy_entries);

        // Test get_next_continuation with dummy data
        let dummy_response = GetLiveChatResponse {
            continuation_contents: ContinuationContents {
                live_chat_continuation: LiveChatContinuation {
                    continuation: Some(Continuation("test".to_string())),
                    actions: vec![],
                    continuations: vec![],
                },
            },
        };
        let _continuation = get_next_continuation(&dummy_response);
    }

    #[test]
    fn test_data_structures_creation() {
        use get_live_chat::*;

        // Test that we can create basic data structures
        let continuation = Continuation("test".to_string());
        assert_eq!(continuation.0, "test");

        let message = Message { runs: vec![] };
        assert!(message.runs.is_empty());

        let simple_text = SimpleText {
            simple_text: "test".to_string(),
        };
        assert_eq!(simple_text.simple_text, "test");
    }

    #[test]
    fn test_error_types() {
        use api::innertube::FetchError;
        use api::youtube::FetchError as YoutubeFetchError;

        // Test that error types are accessible and can be created
        let _innertube_error = FetchError::NotFound;
        let _youtube_error = YoutubeFetchError::NotFound;

        // Test new LiveChatError
        let _live_chat_error = LiveChatError::generic("test", "message");
    }

    #[test]
    fn test_error_types_re_exported() {
        // Test that error types are available from the crate root
        let _live_chat_error = LiveChatError::generic("test", "message");
        let _fetch_error = FetchError::NotFound;
        let _youtube_error = YoutubeFetchError::NotFound;
    }

    #[test]
    fn test_io_module_availability() {
        // Test that I/O utilities are available
        use io::ndjson::*;

        let _error = LiveChatError::generic("test", "test");

        // Test TimestampedEntry
        let _entry: TimestampedEntry<String> = TimestampedEntry {
            timestamp: 123456,
            data: "test data".to_string(),
        };
    }

    #[test]
    fn test_ndjson_functions_re_exported() {
        // Test that NDJSON functions are available from the crate root
        use get_live_chat::ResponseEntry;

        let _result: Result<Vec<ResponseEntry>, LiveChatError> = parse_ndjson_file("test.ndjson");
        let _result2: anyhow::Result<Vec<ResponseEntry>> = parse_ndjson_file_legacy("test.ndjson");
    }
}
