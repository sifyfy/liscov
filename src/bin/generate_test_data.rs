use liscov::api::innertube::get_live_chat::*;
use serde_json;
use std::fs::File;
use std::io::Write;

fn main() -> anyhow::Result<()> {
    // 実際のAPIレスポンス構造に基づいたテストデータを生成
    let test_entries = vec![
        ResponseEntry {
            timestamp: 1640995200,
            response: GetLiveChatResponse {
                continuation_contents: ContinuationContents {
                    live_chat_continuation: LiveChatContinuation {
                        continuation: Some(Continuation(
                            "EkwKGgh0ZXN0X3RvY2VuXzEyMzQ1Njc4OTAzMBABGAEiAggBIAE".to_string(),
                        )),
                        actions: vec![Action::AddChatItem(AddChatItemActionWrapper {
                            action: AddChatItemAction::UserMessage {
                                client_id: "client_123".to_string(),
                                item: ChatItem::TextMessage {
                                    renderer: LiveChatTextMessageRenderer {
                                        id: "msg_001".to_string(),
                                        message: Message {
                                            runs: vec![MessageRun {
                                                text: Some("Hello, world!".to_string()),
                                                emoji: None,
                                            }],
                                            simple_text: None,
                                        },
                                        author_name: AuthorName {
                                            simple_text: "TestUser1".to_string(),
                                        },
                                        author_photo: AuthorPhoto { thumbnails: vec![] },
                                        timestamp_usec: "1640995200000000".to_string(),
                                        author_external_channel_id: "UC_test_123".to_string(),
                                        author_badges: vec![],
                                        tracking_params: Some("tracking_001".to_string()),
                                        context_menu_endpoint: None,
                                        context_menu_accessibility: None,
                                    },
                                },
                                click_tracking_params: Some("click_001".to_string()),
                            },
                            click_tracking_params: Some("wrapper_click_001".to_string()),
                        })],
                        continuations: vec![],
                    },
                },
            },
        },
        ResponseEntry {
            timestamp: 1640995220,
            response: GetLiveChatResponse {
                continuation_contents: ContinuationContents {
                    live_chat_continuation: LiveChatContinuation {
                        continuation: Some(Continuation(
                            "EkwKGgh0ZXN0X3RvY2VuXzIyMzQ1Njc4OTAzMBABGAEiAggBIAE".to_string(),
                        )),
                        actions: vec![Action::AddChatItem(AddChatItemActionWrapper {
                            action: AddChatItemAction::UserMessage {
                                client_id: "client_456".to_string(),
                                item: ChatItem::PaidMessage {
                                    renderer: LiveChatPaidMessageRenderer {
                                        id: "msg_002".to_string(),
                                        message: Some(Message {
                                            runs: vec![MessageRun {
                                                text: Some("Thanks for the stream!".to_string()),
                                                emoji: None,
                                            }],
                                            simple_text: None,
                                        }),
                                        author_name: AuthorName {
                                            simple_text: "Supporter1".to_string(),
                                        },
                                        author_photo: AuthorPhoto { thumbnails: vec![] },
                                        timestamp_usec: "1640995220000000".to_string(),
                                        author_external_channel_id: "UC_test_456".to_string(),
                                        purchase_amount_text: SimpleText {
                                            simple_text: "¥500".to_string(),
                                        },
                                        author_badges: vec![],
                                        tracking_params: "tracking_002".to_string(),
                                        header_background_color: 4294901760,
                                        header_text_color: 4294967295,
                                        body_background_color: 4294967040,
                                        body_text_color: 4278190080,
                                    },
                                },
                                click_tracking_params: Some("click_002".to_string()),
                            },
                            click_tracking_params: Some("wrapper_click_002".to_string()),
                        })],
                        continuations: vec![],
                    },
                },
            },
        },
    ];

    // tests/data/ディレクトリを作成
    std::fs::create_dir_all("tests/data")?;

    // ndjsonファイルに書き込み
    let mut file = File::create("tests/data/live_chat.ndjson")?;
    for entry in test_entries {
        let json_line = serde_json::to_string(&entry)?;
        writeln!(file, "{}", json_line)?;
    }

    println!("✅ テストデータファイルを生成しました: tests/data/live_chat.ndjson");
    Ok(())
}
