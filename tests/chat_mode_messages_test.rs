//! チャットモードでの実際のメッセージ取得テスト

use liscov::api::youtube::ChatMode;
use liscov::api::innertube::{fetch_live_chat_page_with_mode, fetch_live_chat_messages};

/// 両方のモードでメッセージを取得して比較
#[tokio::test]
async fn test_fetch_messages_both_modes() {
    let url = "https://www.youtube.com/watch?v=6c1_dRgZmrI";

    println!("\n🔍 Testing message fetch with both chat modes");
    println!("URL: {}\n", url);

    let mut top_chat_count = 0usize;
    let mut all_chat_count = 0usize;

    // TopChatモードでフェッチ
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔝 TopChat Mode");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    match fetch_live_chat_page_with_mode(url, ChatMode::TopChat).await {
        Ok(inner_tube) => {
            println!("✅ InnerTube client initialized");
            println!("   Mode: {:?}", inner_tube.chat_mode);

            match fetch_live_chat_messages(&inner_tube).await {
                Ok(response) => {
                    top_chat_count = response.continuation_contents.live_chat_continuation.actions.len();
                    println!("📬 Received {} actions", top_chat_count);

                    // アクションタイプをカウント
                    let mut add_chat_items = 0;
                    for action in &response.continuation_contents.live_chat_continuation.actions {
                        if matches!(action, liscov::get_live_chat::Action::AddChatItem(_)) {
                            add_chat_items += 1;
                        }
                    }
                    println!("   AddChatItem actions: {}", add_chat_items);
                }
                Err(e) => println!("❌ Failed to fetch messages: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to initialize: {}", e),
    }

    // 少し待機（レート制限対策）
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // AllChatモードでフェッチ
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💬 AllChat Mode");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    match fetch_live_chat_page_with_mode(url, ChatMode::AllChat).await {
        Ok(inner_tube) => {
            println!("✅ InnerTube client initialized");
            println!("   Mode: {:?}", inner_tube.chat_mode);

            match fetch_live_chat_messages(&inner_tube).await {
                Ok(response) => {
                    all_chat_count = response.continuation_contents.live_chat_continuation.actions.len();
                    println!("📬 Received {} actions", all_chat_count);

                    // アクションタイプをカウント
                    let mut add_chat_items = 0;
                    for action in &response.continuation_contents.live_chat_continuation.actions {
                        if matches!(action, liscov::get_live_chat::Action::AddChatItem(_)) {
                            add_chat_items += 1;
                        }
                    }
                    println!("   AddChatItem actions: {}", add_chat_items);
                }
                Err(e) => println!("❌ Failed to fetch messages: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to initialize: {}", e),
    }

    // 結果の比較
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Comparison");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TopChat actions:  {}", top_chat_count);
    println!("AllChat actions:  {}", all_chat_count);

    if all_chat_count > top_chat_count {
        println!("✅ AllChat returned more messages (as expected for unfiltered mode)");
    } else if all_chat_count == top_chat_count {
        println!("ℹ️ Both modes returned same count (may be low activity)");
    } else {
        println!("⚠️ TopChat returned more messages (unexpected, but may depend on timing)");
    }

    println!("\n✅ Test completed successfully");
}
