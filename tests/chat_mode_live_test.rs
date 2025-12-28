//! ライブ配信でのチャットモード切替テスト

use liscov::api::innertube::{fetch_live_chat_page_with_mode, fetch_live_chat_messages};
use liscov::api::youtube::ChatMode;

/// ライブ配信でモード切替が実際に動作するか確認
#[tokio::test]
async fn test_live_chat_mode_switching() {
    let url = "https://www.youtube.com/watch?v=6c1_dRgZmrI";

    println!("\n🔍 Live Chat Mode Switching Test");
    println!("URL: {}\n", url);

    // 1. TopChatモードで接続
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("1️⃣ Connect with TopChat mode");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut inner_tube = match fetch_live_chat_page_with_mode(url, ChatMode::TopChat).await {
        Ok(client) => {
            println!("✅ Connected successfully");
            println!("   Mode: {:?}", client.current_chat_mode());
            println!("   Available modes: {:?}", client.available_chat_modes());
            println!("   Detected from token: {:?}", client.detect_current_mode());
            client
        }
        Err(e) => {
            println!("❌ Connection failed: {}", e);
            return;
        }
    };

    // TopChatでメッセージ取得
    let top_chat_response = match fetch_live_chat_messages(&inner_tube).await {
        Ok(response) => {
            let action_count = response.continuation_contents.live_chat_continuation.actions.len();
            println!("📬 TopChat: {} actions", action_count);
            Some(action_count)
        }
        Err(e) => {
            println!("❌ Failed to fetch TopChat messages: {}", e);
            None
        }
    };

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 2. AllChatモードに切り替え
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("2️⃣ Switch to AllChat mode");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let switched = inner_tube.set_chat_mode(ChatMode::AllChat);
    println!("Mode switch result: {}", if switched { "✅ Success" } else { "❌ Failed" });
    println!("Current mode: {:?}", inner_tube.current_chat_mode());
    println!("Detected from token: {:?}", inner_tube.detect_current_mode());

    // AllChatでメッセージ取得
    let all_chat_response = match fetch_live_chat_messages(&inner_tube).await {
        Ok(response) => {
            let action_count = response.continuation_contents.live_chat_continuation.actions.len();
            println!("📬 AllChat: {} actions", action_count);
            Some(action_count)
        }
        Err(e) => {
            println!("❌ Failed to fetch AllChat messages: {}", e);
            None
        }
    };

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 3. 再度TopChatモードに戻す
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("3️⃣ Switch back to TopChat mode");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let switched_back = inner_tube.set_chat_mode(ChatMode::TopChat);
    println!("Mode switch result: {}", if switched_back { "✅ Success" } else { "❌ Failed" });
    println!("Current mode: {:?}", inner_tube.current_chat_mode());
    println!("Detected from token: {:?}", inner_tube.detect_current_mode());

    // 結果サマリー
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TopChat actions: {:?}", top_chat_response);
    println!("AllChat actions: {:?}", all_chat_response);

    if switched && switched_back {
        println!("\n✅ Mode switching works correctly!");
    } else {
        println!("\n⚠️ Mode switching had issues");
    }

    // アサーション
    assert!(switched, "Should be able to switch to AllChat");
    assert!(switched_back, "Should be able to switch back to TopChat");
    assert!(top_chat_response.is_some(), "Should get TopChat response");
    assert!(all_chat_response.is_some(), "Should get AllChat response");
}
