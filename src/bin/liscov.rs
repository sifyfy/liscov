use std::{thread, time::Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let video_id = std::env::args().nth(1).expect("video_id is required");
    let url = format!("https://www.youtube.com/watch?v={video_id}");

    let mut inner_tube = liscov::api::youtube::fetch_live_chat_page(&url).await?;

    loop {
        let get_live_chat = liscov::api::youtube::fetch_live_chat_messages(&inner_tube).await?;
        let live_chat_continuation = get_live_chat.continuation_contents.live_chat_continuation;

        inner_tube.continuation = live_chat_continuation.continuation;

        println!("{:?}", live_chat_continuation.actions);

        // 指定ミリ秒待機
        thread::sleep(Duration::from_secs(1));
    }

    // let secret = yup_oauth2::read_application_secret("client_secret.json")
    //     .await
    //     .expect("clientsecret.json");

    // let mut auth =
    //     InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
    //         .persist_tokens_to_disk("tokencache.json")
    //         .build()
    //         .await
    //         .unwrap();

    // let scopes = &["https://www.googleapis.com/auth/youtube.readonly"];

    // let access_token = auth.token(scopes).await?;

    // // ライブストリーミングのIDを指定
    // let video_id = "8x1k5Z9_qIU";

    // async fn video_id_to_live_chat_id(
    //     video_id: &str,
    //     token: &AccessToken,
    // ) -> anyhow::Result<String> {
    //     let url = format!(
    //         "https://www.googleapis.com/youtube/v3/videos?part=liveStreamingDetails&id={video_id}"
    //     );
    //     let client = reqwest::Client::new();
    //     let res = client
    //         .get(&url)
    //         .bearer_auth(token.token().unwrap())
    //         .send()
    //         .await?;
    //     let text = res.text().await?;
    //     let json: serde_json::Value = serde_json::from_str(&text)?;

    //     let live_chat_id = json
    //         .get("items")
    //         .and_then(|v| {
    //             v.as_array()?
    //                 .get(0)?
    //                 .get("liveStreamingDetails")?
    //                 .get("activeLiveChatId")?
    //                 .as_str()
    //                 .map(|id| id.to_string())
    //         })
    //         .ok_or_else(|| anyhow::anyhow!("error"))?;

    //     Ok(live_chat_id)
    // }

    // let live_chat_id = video_id_to_live_chat_id(video_id, &access_token).await?;

    // let client = reqwest::Client::new();
    // let mut next_page_token = String::new();
    // loop {
    //     // ライブチャットコメントを取得
    //     let url = format!("https://www.googleapis.com/youtube/v3/liveChat/messages?liveChatId={live_chat_id}&pageToken={next_page_token}&part=snippet,authorDetails");
    //     let res = client
    //         .get(&url)
    //         .bearer_auth(access_token.token().unwrap())
    //         .send()
    //         .await?;

    //     let data: serde_json::Value = serde_json::from_slice(&res.bytes().await?)?;

    //     let polling_interval_millis = data["pollingIntervalMillis"].as_u64().unwrap_or(5000);
    //     if let Some(new_next_page_token) = data["nextPageToken"].as_str() {
    //         next_page_token = new_next_page_token.to_string();
    //     }
    //     let num_of_comments = data["items"]
    //         .as_array()
    //         .map(|xs| xs.len() as i64)
    //         .unwrap_or(-1);

    //     println!("items: {num_of_comments}, next_page_token: {next_page_token:?}, polling_interval_millis: {polling_interval_millis}");

    //     // コメントを出力
    //     // if let Some(items) = data["items"].as_array() {
    //     //     println!("{}", items.len());
    //     //     // for item in items {
    //     //     //     let snippet = &item["snippet"];
    //     //     //     let author_details = &item["authorDetails"];
    //     //     //     println!(
    //     //     //         "{}: {}",
    //     //     //         author_details["displayName"].as_str().unwrap_or(""),
    //     //     //         snippet["displayMessage"].as_str().unwrap_or("")
    //     //     //     );
    //     //     // }
    //     // }

    //     // 指定ミリ秒待機
    //     thread::sleep(Duration::from_millis(polling_interval_millis));
    // }

    Ok(())
}
