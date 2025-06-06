pub mod analytics;
pub mod api;
pub mod chat_management;
pub mod database;
pub mod gui;
pub mod io;

pub use api::innertube::get_live_chat;

// Re-export the main error types for convenience
pub use api::innertube::FetchError;
pub use api::youtube::FetchError as YoutubeFetchError;
pub use io::LiveChatError;

// Re-export I/O utilities for convenience
pub use io::ndjson::{
    parse_ndjson_file, parse_ndjson_file_generic, parse_ndjson_file_legacy,
    parse_ndjson_file_timestamped, TimestampedEntry,
};

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
