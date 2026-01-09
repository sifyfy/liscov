pub mod get_live_chat;

use crate::api::auth::{generate_sapisidhash, YouTubeCookies};
use crate::api::continuation_builder::{detect_chat_mode, modify_continuation_mode};
use crate::api::innertube::get_live_chat::GetLiveChatResponse;
use crate::api::youtube::{ApiKey, ChatContinuations, ChatMode, ClientVersion, Continuation, VideoId};
use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Not found")]
    NotFound,
    #[error("Anyhow error: {0}")]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone, derive_more::Display, Serialize, Deserialize)]
pub struct ClientId(pub String);

#[derive(Debug, Clone)]
pub struct InnerTube {
    pub video_id: VideoId,
    pub api_key: ApiKey,
    pub is_replay: bool,
    pub client_version: ClientVersion,
    pub gl: String,
    pub hl: String,
    pub continuation: Continuation,
    pub client_id: ClientId,
    pub http_client: reqwest::Client,
    /// 両方のチャットモード用のcontinuation tokens
    pub chat_continuations: Option<ChatContinuations>,
    /// 現在選択されているチャットモード
    pub chat_mode: ChatMode,
    /// 認証情報（メンバー限定配信用）
    pub auth_cookies: Option<YouTubeCookies>,
    /// 配信者のYouTubeチャンネルID
    pub broadcaster_channel_id: Option<String>,
    /// 配信者のチャンネル名
    pub broadcaster_channel_name: Option<String>,
    /// 配信者のYouTubeハンドル (@xxx)
    pub broadcaster_handle: Option<String>,
}

impl InnerTube {
    pub fn new(
        video_id: VideoId,
        api_key: ApiKey,
        client_version: ClientVersion,
        client_id: ClientId,
    ) -> Self {
        Self {
            video_id,
            api_key,
            is_replay: false,
            client_version,
            gl: "US".to_string(),
            hl: "en".to_string(),
            continuation: Continuation("".to_string()),
            client_id,
            http_client: reqwest::Client::new(),
            chat_continuations: None,
            chat_mode: ChatMode::default(),
            auth_cookies: None,
            broadcaster_channel_id: None,
            broadcaster_channel_name: None,
            broadcaster_handle: None,
        }
    }

    /// 認証情報を設定
    pub fn set_auth(&mut self, cookies: YouTubeCookies) {
        self.auth_cookies = Some(cookies);
    }

    /// 認証情報をクリア
    pub fn clear_auth(&mut self) {
        self.auth_cookies = None;
    }

    /// 認証済みかどうかを確認
    pub fn is_authenticated(&self) -> bool {
        self.auth_cookies.is_some()
    }

    /// 認証ヘッダーを生成
    ///
    /// 認証情報が設定されている場合、以下のヘッダーを返す：
    /// - Authorization: SAPISIDHASH {hash}
    /// - Cookie: SID=...; HSID=...; ...
    /// - X-Origin: https://www.youtube.com
    /// - Origin: https://www.youtube.com
    fn build_auth_headers(&self) -> Option<Vec<(String, String)>> {
        let cookies = self.auth_cookies.as_ref()?;

        let sapisidhash = generate_sapisidhash(&cookies.sapisid);
        let cookie_header = cookies.to_cookie_header();

        Some(vec![
            ("Authorization".to_string(), format!("SAPISIDHASH {}", sapisidhash)),
            ("Cookie".to_string(), cookie_header),
            ("X-Origin".to_string(), "https://www.youtube.com".to_string()),
            ("Origin".to_string(), "https://www.youtube.com".to_string()),
        ])
    }

    /// チャットモードを変更し、continuation tokenを更新する
    ///
    /// メインcontinuation tokenのバイナリを変更してモードを切り替える。
    /// reload tokenは使用しない（reload tokenはAPIで直接使えないため）。
    ///
    /// # Arguments
    /// * `mode` - 新しいチャットモード
    ///
    /// # Returns
    /// * `true` - モード変更成功
    /// * `false` - モード変更失敗（トークンが空または変更できない場合）
    pub fn set_chat_mode(&mut self, mode: ChatMode) -> bool {
        // 既に同じモードの場合は何もしない
        if self.chat_mode == mode {
            tracing::debug!("Chat mode already set to {:?}", mode);
            return true;
        }

        // continuation tokenが空の場合は変更不可
        if self.continuation.0.is_empty() {
            tracing::warn!("Cannot change chat mode: continuation token is empty");
            return false;
        }

        // continuation tokenをバイナリ変換してモードを変更
        if let Some(new_token) = modify_continuation_mode(&self.continuation, mode) {
            tracing::info!(
                "🔄 Chat mode changed: {:?} -> {:?} (token length: {})",
                self.chat_mode,
                mode,
                new_token.0.len()
            );
            self.continuation = new_token;
            self.chat_mode = mode;
            true
        } else {
            tracing::warn!("Failed to modify continuation token for mode {:?}", mode);
            false
        }
    }

    /// 現在のチャットモードを取得
    pub fn current_chat_mode(&self) -> ChatMode {
        self.chat_mode
    }

    /// 利用可能なチャットモードを取得
    ///
    /// continuation tokenが有効な場合、両方のモードが利用可能
    pub fn available_chat_modes(&self) -> Vec<ChatMode> {
        if self.continuation.0.is_empty() {
            vec![self.chat_mode]
        } else {
            // 有効なtokenがあれば両方のモードが利用可能
            vec![ChatMode::TopChat, ChatMode::AllChat]
        }
    }

    /// continuation tokenから現在のチャットモードを検出
    pub fn detect_current_mode(&self) -> Option<ChatMode> {
        if self.continuation.0.is_empty() {
            None
        } else {
            detect_chat_mode(&self.continuation)
        }
    }

    /// チャットモードを非同期で切り替える
    ///
    /// reload tokenを使ってlive_chatページを再取得し、
    /// 新しいモード用のmain continuation tokenを取得する。
    ///
    /// # Arguments
    /// * `mode` - 切り替え先のチャットモード
    ///
    /// # Returns
    /// * `Ok(true)` - 切り替え成功
    /// * `Ok(false)` - reload tokenが利用できない
    /// * `Err(_)` - ページ取得失敗
    pub async fn switch_chat_mode(&mut self, mode: ChatMode) -> Result<bool> {
        // 既に同じモードの場合は何もしない
        if self.chat_mode == mode {
            tracing::debug!("Chat mode already set to {:?}", mode);
            return Ok(true);
        }

        // reload tokenを取得
        let reload_token = if let Some(ref continuations) = self.chat_continuations {
            if let Some(token) = continuations.get_for_mode(mode) {
                token.clone()
            } else {
                tracing::warn!("No reload token available for mode {:?}", mode);
                return Ok(false);
            }
        } else {
            tracing::warn!("No chat_continuations available");
            return Ok(false);
        };

        tracing::info!(
            "🔄 Switching chat mode: {:?} -> {:?}",
            self.chat_mode,
            mode
        );

        // reload tokenを使ってlive_chatページを再取得
        let url = format!(
            "https://www.youtube.com/live_chat?continuation={}",
            urlencoding::encode(&reload_token.0)
        );

        tracing::debug!("📋 Fetching live_chat page with reload token");

        let response = self
            .http_client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(
                "❌ Page fetch failed with status: {}\nResponse: {}",
                status,
                &error_body[..200.min(error_body.len())]
            );
            return Err(anyhow::anyhow!("Page fetch failed with status: {}", status));
        }

        let html = response.text().await?;
        tracing::debug!("📄 Received HTML response: {} chars", html.len());

        // 新しいmain continuation tokenを抽出
        if let Some(new_continuation) = crate::api::youtube::extract_continuation(&html) {
            tracing::info!(
                "✅ Chat mode switched: {:?} -> {:?} (new token length: {})",
                self.chat_mode,
                mode,
                new_continuation.0.len()
            );
            self.continuation = new_continuation;
            self.chat_mode = mode;

            // 新しいreload tokensも更新
            let new_continuations = crate::api::youtube::extract_chat_continuations(&html);
            if new_continuations.has_any() {
                self.chat_continuations = Some(new_continuations);
            }

            Ok(true)
        } else {
            tracing::warn!("⚠️ No continuation token found in response");
            // フォールバック: バイナリ変換を試みる
            if self.set_chat_mode(mode) {
                tracing::info!("✅ Fallback: Chat mode switched using binary modification");
                Ok(true)
            } else {
                Err(anyhow::anyhow!("Failed to extract continuation token"))
            }
        }
    }
}

/// デバッグ用HTMLファイル保存
fn save_debug_html(html: &str, reason: &str) {
    if let Ok(temp_dir) = std::env::var("TEMP").or_else(|_| std::env::var("TMP")) {
        let path = format!("{}/liscov_debug_html_{}.txt", temp_dir, reason);
        if let Err(e) = std::fs::write(&path, html) {
            tracing::error!("Failed to save debug HTML: {}", e);
        } else {
            tracing::info!("📁 Debug HTML saved to: {}", path);
        }
    }
}

/// デフォルトのチャットモード（TopChat）でライブチャットページを取得
pub async fn fetch_live_chat_page(url: &str) -> Result<InnerTube> {
    fetch_live_chat_page_with_auth(url, ChatMode::default(), None).await
}

/// 指定したチャットモードでライブチャットページを取得
///
/// 注意: YouTubeのチャットモード切替はreload continuation tokenを使用する。
/// 初回接続時はメインのcontinuation tokenを使用し、モード切替用のtokenは
/// chat_continuationsに保存される。
pub async fn fetch_live_chat_page_with_mode(url: &str, preferred_mode: ChatMode) -> Result<InnerTube> {
    fetch_live_chat_page_with_auth(url, preferred_mode, None).await
}

/// 認証情報付きでライブチャットページを取得
///
/// メンバー限定配信など、認証が必要なコンテンツにアクセスする場合に使用。
pub async fn fetch_live_chat_page_with_auth(
    url: &str,
    preferred_mode: ChatMode,
    cookies: Option<&YouTubeCookies>,
) -> Result<InnerTube> {
    tracing::info!("🌐 Fetching live chat page from URL: {} (mode: {})", url, preferred_mode);

    if cookies.is_some() {
        tracing::info!("🔐 Using authentication cookies for page fetch");
    }

    let client = reqwest::Client::new();

    // URLからビデオIDを抽出
    let video_id_from_url = crate::gui::utils::extract_video_id(url);

    // 認証がある場合、まず動画ページから配信者チャンネルIDを取得
    let mut broadcaster_channel_id_prefetch: Option<String> = None;
    if cookies.is_some() {
        if let Some(ref vid) = video_id_from_url {
            let video_page_url = format!("https://www.youtube.com/watch?v={}", vid);
            tracing::info!("📺 Pre-fetching video page to get broadcaster channel ID: {}", video_page_url);

            match client
                .get(&video_page_url)
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                )
                .send()
                .await
            {
                Ok(response) => {
                    if let Ok(video_html) = response.text().await {
                        broadcaster_channel_id_prefetch = crate::api::youtube::extract_broadcaster_channel_id(&video_html);
                        if let Some(ref id) = broadcaster_channel_id_prefetch {
                            tracing::info!("📺 Pre-fetched broadcaster channel ID: {}", id);
                        } else {
                            tracing::warn!("⚠️ Could not extract broadcaster channel ID from video page");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to pre-fetch video page for broadcaster ID: {}", e);
                }
            }
        }
    }

    // live_chatページを直接取得するかどうかを決定
    let fetch_url = if let Some(ref vid) = video_id_from_url {
        // 認証がある場合はlive_chatポップアップページを直接取得
        if cookies.is_some() {
            // is_popout=1を追加してポップアップチャットウィンドウとして取得
            let chat_url = format!("https://www.youtube.com/live_chat?is_popout=1&v={}", vid);
            tracing::info!("🔄 Fetching live_chat popup page directly: {}", chat_url);
            chat_url
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    };

    let mut request = client
        .get(&fetch_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .header("Accept-Language", "ja,en-US;q=0.7,en;q=0.3")
        // Note: Accept-Encoding は設定しない（reqwestにgzip featureがないため圧縮レスポンスを処理できない）
        .header("Sec-Fetch-Dest", "document")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-Site", "none")
        .header("Sec-Fetch-User", "?1")
        .header("Upgrade-Insecure-Requests", "1");

    // 認証Cookieを追加（ページナビゲーションではAuthorizationヘッダーは不要）
    // ブラウザと同様にCookieのみを送信
    if let Some(auth_cookies) = cookies {
        let cookie_header = auth_cookies.to_cookie_header();

        request = request.header("Cookie", cookie_header.clone());

        tracing::info!("🍪 Added authentication cookies (length: {} chars)", cookie_header.len());

        // Cookieの主要な値をログ（デバッグ用）
        if cookie_header.contains("SAPISID=") {
            tracing::info!("✅ SAPISID cookie is present");
        }
        if cookie_header.contains("LOGIN_INFO=") {
            tracing::info!("✅ LOGIN_INFO cookie is present");
        }
        if cookie_header.contains("__Secure-1PSID=") || cookie_header.contains("__Secure-3PSID=") {
            tracing::info!("✅ Secure PSID cookies are present");
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to fetch URL: {}", e);
            e
        })?;

    tracing::debug!(
        "📄 Received HTTP response with status: {}",
        response.status()
    );

    let html = response.text().await.map_err(|e| {
        tracing::error!("❌ Failed to read response text: {}", e);
        e
    })?;

    tracing::debug!("📄 HTML response length: {} chars", html.len());

    // live_chatポップアップページの場合はURLからvideo_idを使用（HTMLからは抽出できない）
    let video_id = if let Some(ref vid) = video_id_from_url {
        if cookies.is_some() {
            // live_chatポップアップページからはvideo_idを直接抽出できないため、URLから取得したものを使用
            tracing::info!("🎬 Using video_id from URL for live_chat popup: {}", vid);
            crate::api::youtube::VideoId(vid.clone())
        } else {
            crate::api::youtube::extract_video_id(&html).ok_or_else(|| {
                tracing::error!("❌ video_id not found in HTML");
                // デバッグ用：HTMLをファイルに保存
                save_debug_html(&html, "video_id_not_found");
                anyhow::anyhow!("video_id not found")
            })?
        }
    } else {
        crate::api::youtube::extract_video_id(&html).ok_or_else(|| {
            tracing::error!("❌ video_id not found in HTML");
            save_debug_html(&html, "video_id_not_found");
            anyhow::anyhow!("video_id not found")
        })?
    };
    tracing::info!("🎬 Extracted video_id: {}", video_id);

    let api_key = crate::api::youtube::extract_api_key(&html).ok_or_else(|| {
        tracing::error!("❌ api_key not found in HTML");
        save_debug_html(&html, "api_key_not_found");
        anyhow::anyhow!("api_key not found")
    })?;
    tracing::info!(
        "🔑 Extracted api_key: {}...",
        &api_key.to_string()[..10.min(api_key.to_string().len())]
    );

    let client_version = crate::api::youtube::extract_client_version(&html).ok_or_else(|| {
        tracing::error!("❌ client_version not found in HTML");
        anyhow::anyhow!("client_version not found")
    })?;
    tracing::info!("📱 Extracted client_version: {}", client_version);

    // メインのcontinuation token（長い形式、メッセージ取得に使用）
    let main_continuation = crate::api::youtube::extract_continuation(&html).ok_or_else(|| {
        tracing::error!("❌ continuation not found in HTML");
        save_debug_html(&html, "continuation_not_found");
        anyhow::anyhow!("continuation not found")
    })?;
    tracing::info!(
        "🔄 Extracted main continuation token (length: {}): {}...",
        main_continuation.0.len(),
        &main_continuation.to_string()[..30.min(main_continuation.to_string().len())]
    );

    // モード切替用のreload tokensを抽出（subMenuItemsから）
    let chat_continuations = crate::api::youtube::extract_chat_continuations(&html);

    let chat_continuations_option = if chat_continuations.has_any() {
        tracing::info!(
            "📋 Mode switch tokens available: TopChat={}, AllChat={}",
            chat_continuations.top_chat.is_some(),
            chat_continuations.all_chat.is_some()
        );
        Some(chat_continuations)
    } else {
        tracing::warn!("⚠️ No mode switch tokens found in HTML");
        None
    };

    // 配信者情報を抽出（事前取得したチャンネルIDを優先）
    let broadcaster_info = crate::api::youtube::extract_broadcaster_info(&html);
    let broadcaster_channel_id = if broadcaster_channel_id_prefetch.is_some() {
        tracing::info!("📺 Using pre-fetched broadcaster channel ID");
        broadcaster_channel_id_prefetch
    } else {
        if let Some(ref id) = broadcaster_info.channel_id {
            tracing::info!("📺 Extracted broadcaster channel ID from chat page: {}", id);
        } else {
            tracing::warn!("⚠️ Could not extract broadcaster channel ID from HTML");
        }
        broadcaster_info.channel_id.clone()
    };

    // チャンネル名とハンドルもログ出力
    if let Some(ref name) = broadcaster_info.channel_name {
        tracing::info!("📺 Broadcaster channel name: {}", name);
    }
    if let Some(ref handle) = broadcaster_info.handle {
        tracing::info!("📺 Broadcaster handle: {}", handle);
    }

    let mut inner_tube =
        InnerTube::new(video_id, api_key, client_version, ClientId("1".to_string()));

    // メインcontinuation tokenを設定
    inner_tube.continuation = main_continuation;
    inner_tube.chat_continuations = chat_continuations_option;
    inner_tube.broadcaster_channel_id = broadcaster_channel_id;
    inner_tube.broadcaster_channel_name = broadcaster_info.channel_name;
    inner_tube.broadcaster_handle = broadcaster_info.handle;

    // トークンから現在のモードを検出
    let detected_mode = inner_tube.detect_current_mode().unwrap_or(ChatMode::TopChat);
    inner_tube.chat_mode = detected_mode;
    tracing::info!("🔍 Detected chat mode from token: {:?}", detected_mode);

    // 希望するモードと異なる場合は非同期で切り替え
    if preferred_mode != detected_mode {
        match inner_tube.switch_chat_mode(preferred_mode).await {
            Ok(true) => {
                tracing::info!("🔄 Switched chat mode to: {:?}", preferred_mode);
            }
            Ok(false) => {
                tracing::warn!("⚠️ Could not switch to preferred mode {:?}, using {:?}", preferred_mode, detected_mode);
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to switch to preferred mode {:?}: {}, using {:?}", preferred_mode, e, detected_mode);
            }
        }
    }

    tracing::info!("✅ Successfully initialized InnerTube client (mode: {:?})", inner_tube.chat_mode);
    Ok(inner_tube)
}

pub async fn fetch_live_chat_messages(inner_tube: &InnerTube) -> Result<GetLiveChatResponse> {
    let url = format!(
        "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
        inner_tube.api_key
    );

    tracing::debug!(
        "📡 Making API request to: {}",
        if tracing::level_enabled!(tracing::Level::DEBUG) {
            &url[..60.min(url.len())]
        } else {
            ""
        }
    );

    let payload = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": inner_tube.client_version.to_string()
            }
        },
        "continuation": inner_tube.continuation.to_string(),
    });

    if tracing::level_enabled!(tracing::Level::DEBUG) {
        tracing::debug!(
            "📋 Request payload size: {} bytes",
            serde_json::to_string(&payload).unwrap_or_default().len()
        );
    }

    // リクエストビルダーを構築
    let mut request = inner_tube
        .http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        );

    // 認証ヘッダーを追加（メンバー限定配信用）
    if let Some(auth_headers) = inner_tube.build_auth_headers() {
        tracing::debug!("🔐 Adding authentication headers for member-only content");
        for (name, value) in auth_headers {
            // Cookie値はログに出力しない
            if name != "Cookie" {
                tracing::trace!("  {}: {}", name, value);
            }
            request = request.header(&name, &value);
        }
    }

    let response = request
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("❌ HTTP request failed: {}", e);
            e
        })?;

    let status = response.status();
    if tracing::level_enabled!(tracing::Level::DEBUG) {
        tracing::debug!("📡 API response status: {}", status);
    }

    if !status.is_success() {
        let error_msg = format!("HTTP request failed with status: {}", status);
        tracing::error!("❌ {}", error_msg);
        return Err(anyhow::anyhow!(error_msg));
    }

    let response_text = response.text().await.map_err(|e| {
        tracing::error!("❌ Failed to read response text: {}", e);
        e
    })?;

    if tracing::level_enabled!(tracing::Level::DEBUG) {
        tracing::debug!("📄 Response text length: {} chars", response_text.len());
    }

    let live_chat_response: GetLiveChatResponse =
        serde_json::from_str(&response_text).map_err(|e| {
            tracing::error!("❌ Failed to parse JSON response: {}", e);
            tracing::debug!(
                "🔍 Response text preview: {}",
                &response_text[..200.min(response_text.len())]
            );
            e
        })?;

    if tracing::level_enabled!(tracing::Level::DEBUG) {
        tracing::debug!("✅ Successfully parsed live chat response");
    }
    Ok(live_chat_response)
}

pub fn get_next_continuation(response: &GetLiveChatResponse) -> Option<String> {
    response
        .continuation_contents
        .live_chat_continuation
        .continuations
        .first()
        .and_then(|v| {
            v.get("invalidationContinuationData")
                .or_else(|| v.get("timedContinuationData"))
                .or_else(|| v.get("reloadContinuationData"))
        })
        .and_then(|v| v.get("continuation"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// 継続情報（トークンとポーリング間隔）
#[derive(Debug, Clone)]
pub struct ContinuationInfo {
    /// 継続トークン
    pub continuation: String,
    /// 推奨ポーリング間隔（ミリ秒）
    pub timeout_ms: Option<u64>,
}

/// 継続トークンとtimeoutMs（推奨ポーリング間隔）を取得
///
/// YouTubeのAPIはチャットの活発さに応じて推奨ポーリング間隔を返す。
/// 活発な時は短い間隔（数百ms）、静かな時は長い間隔（数秒）となる。
pub fn get_next_continuation_with_timeout(response: &GetLiveChatResponse) -> Option<ContinuationInfo> {
    response
        .continuation_contents
        .live_chat_continuation
        .continuations
        .first()
        .and_then(|v| {
            // 優先順位: invalidationContinuationData > timedContinuationData > reloadContinuationData
            v.get("invalidationContinuationData")
                .or_else(|| v.get("timedContinuationData"))
                .or_else(|| v.get("reloadContinuationData"))
        })
        .and_then(|data| {
            let continuation = data.get("continuation")?.as_str()?.to_string();
            let timeout_ms = data.get("timeoutMs").and_then(|v| v.as_u64());
            Some(ContinuationInfo {
                continuation,
                timeout_ms,
            })
        })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub author: String,
    pub message: String,
    pub timestamp: u64,
}

impl ChatMessage {
    pub fn new(id: String, author: String, message: String, timestamp: u64) -> Self {
        Self {
            id,
            author,
            message,
            timestamp,
        }
    }

    pub fn datetime(&self) -> String {
        use std::time::SystemTime;
        let datetime = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp);
        format!("{:?}", datetime)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseEntry {
    pub timestamp: u64,
    pub response: GetLiveChatResponse,
}

impl ResponseEntry {
    pub fn new(timestamp: u64, response: GetLiveChatResponse) -> Self {
        Self {
            timestamp,
            response,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inner_tube_creation() {
        let inner_tube = InnerTube::new(
            VideoId("test_video_id".to_string()),
            ApiKey::new("test_api_key".to_string()),
            ClientVersion::new("2.0".to_string()),
            ClientId("1".to_string()),
        );

        assert_eq!(inner_tube.video_id.0, "test_video_id");
        assert_eq!(inner_tube.api_key.to_string(), "test_api_key");
        assert!(!inner_tube.is_replay);
        assert_eq!(inner_tube.client_version.to_string(), "2.0");
        assert_eq!(inner_tube.gl, "US");
        assert_eq!(inner_tube.hl, "en");
        assert_eq!(inner_tube.continuation.to_string(), "");
        assert_eq!(inner_tube.client_id.0, "1");
    }

    #[test]
    fn test_chat_message_creation() {
        let message = ChatMessage::new(
            "msg_123".to_string(),
            "TestUser".to_string(),
            "Hello World!".to_string(),
            1234567890,
        );

        assert_eq!(message.id, "msg_123");
        assert_eq!(message.author, "TestUser");
        assert_eq!(message.message, "Hello World!");
        assert_eq!(message.timestamp, 1234567890);
    }

    #[test]
    fn test_response_entry_creation() {
        use crate::api::innertube::get_live_chat::*;

        let response = GetLiveChatResponse {
            continuation_contents: ContinuationContents {
                live_chat_continuation: LiveChatContinuation {
                    continuation: None,
                    actions: vec![],
                    continuations: vec![],
                },
            },
        };

        let entry = super::ResponseEntry::new(1234567890, response);
        assert_eq!(entry.timestamp, 1234567890);
    }

    #[test]
    fn test_fetch_error_display() {
        let error = FetchError::NotFound;
        assert!(format!("{}", error).contains("Not found"));
    }

    #[test]
    fn test_client_id_wrapper() {
        let client_id = ClientId("1".to_string());
        assert_eq!(client_id.0, "1");
    }

    #[test]
    fn test_fetch_error_types() {
        // FetchErrorの各バリアントをテスト
        let error = FetchError::NotFound;
        assert!(format!("{}", error).contains("Not found"));

        let anyhow_error = anyhow::anyhow!("Test error");
        let fetch_error = FetchError::from(anyhow_error);
        match fetch_error {
            FetchError::Other(_) => (), // 期待される
            _ => panic!("Expected FetchError::Other variant"),
        }
    }

    #[test]
    fn test_fetch_error_from_serde() {
        // JSON解析エラーからの変換をテスト
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let fetch_error = FetchError::from(json_error);

        match fetch_error {
            FetchError::Serialization(_) => (), // 期待される
            _ => panic!("Expected FetchError::Serialization variant"),
        }
    }

    #[test]
    fn test_fetch_error_from_anyhow() {
        // anyhowエラーからの変換をテスト
        let anyhow_error = anyhow::anyhow!("Test error");
        let fetch_error = FetchError::from(anyhow_error);

        match fetch_error {
            FetchError::Other(_) => (), // 期待される
            _ => panic!("Expected FetchError::Other variant"),
        }
    }

    #[test]
    fn test_inner_tube_with_invalid_continuation() {
        // 無効な継続トークンでのテスト
        let mut inner_tube = InnerTube::new(
            VideoId("test_video_id".to_string()),
            ApiKey::new("test_api_key".to_string()),
            ClientVersion::new("2.0".to_string()),
            ClientId("1".to_string()),
        );

        // 空の継続トークンを設定
        inner_tube.continuation = Continuation("".to_string());
        assert_eq!(inner_tube.continuation.to_string(), "");

        // 無効な継続トークンを設定
        inner_tube.continuation = Continuation("invalid_token".to_string());
        assert_eq!(inner_tube.continuation.to_string(), "invalid_token");
    }

    #[test]
    fn test_chat_message_edge_cases() {
        // 空のメッセージ
        let empty_message = ChatMessage::new(
            "msg_empty".to_string(),
            "TestUser".to_string(),
            "".to_string(),
            0,
        );
        assert_eq!(empty_message.message, "");
        assert_eq!(empty_message.timestamp, 0);

        // 極端に長いメッセージ
        let long_message = "a".repeat(1000);
        let message = ChatMessage::new(
            "msg_long".to_string(),
            "TestUser".to_string(),
            long_message.clone(),
            u64::MAX,
        );
        assert_eq!(message.message, long_message);
        assert_eq!(message.timestamp, u64::MAX);

        // 特殊文字を含むメッセージ
        let special_message = ChatMessage::new(
            "msg_special".to_string(),
            "TestUser".to_string(),
            "🎮🔥 テスト メッセージ with emojis and 日本語!".to_string(),
            1234567890,
        );
        assert!(special_message.message.contains("🎮"));
        assert!(special_message.message.contains("日本語"));
    }

    #[test]
    fn test_chat_message_datetime_formatting() {
        // 有効なタイムスタンプでの日時フォーマット
        let message = ChatMessage::new(
            "msg_time".to_string(),
            "TestUser".to_string(),
            "Time test".to_string(),
            1609459200, // 2021-01-01 00:00:00 UTC
        );

        let datetime_str = message.datetime();
        assert!(!datetime_str.is_empty());

        // タイムスタンプ0での処理
        let zero_message = ChatMessage::new(
            "msg_zero".to_string(),
            "TestUser".to_string(),
            "Zero timestamp".to_string(),
            0,
        );

        let zero_datetime = zero_message.datetime();
        assert!(!zero_datetime.is_empty());
    }

    #[test]
    fn test_get_next_continuation_edge_cases() {
        use crate::api::innertube::get_live_chat::*;
        use serde_json::json;

        // 空の継続リストの場合
        let empty_response = GetLiveChatResponse {
            continuation_contents: ContinuationContents {
                live_chat_continuation: LiveChatContinuation {
                    continuation: None,
                    actions: vec![],
                    continuations: vec![],
                },
            },
        };
        assert!(get_next_continuation(&empty_response).is_none());

        // 無効な継続データの場合
        let invalid_continuation = json!({
            "invalidKey": "invalidValue"
        });
        let invalid_response = GetLiveChatResponse {
            continuation_contents: ContinuationContents {
                live_chat_continuation: LiveChatContinuation {
                    continuation: None,
                    actions: vec![],
                    continuations: vec![invalid_continuation],
                },
            },
        };
        assert!(get_next_continuation(&invalid_response).is_none());
    }

    #[test]
    fn test_client_id_display() {
        let client_id = ClientId("test_client_123".to_string());
        let display_str = format!("{}", client_id);
        assert_eq!(display_str, "test_client_123");
    }

    #[test]
    fn test_inner_tube_auth_default() {
        let inner_tube = InnerTube::new(
            VideoId("test".to_string()),
            ApiKey::new("key".to_string()),
            ClientVersion::new("1.0".to_string()),
            ClientId("1".to_string()),
        );

        assert!(!inner_tube.is_authenticated());
        assert!(inner_tube.auth_cookies.is_none());
    }

    #[test]
    fn test_inner_tube_set_auth() {
        let mut inner_tube = InnerTube::new(
            VideoId("test".to_string()),
            ApiKey::new("key".to_string()),
            ClientVersion::new("1.0".to_string()),
            ClientId("1".to_string()),
        );

        let cookies = YouTubeCookies::new(
            "sid".to_string(),
            "hsid".to_string(),
            "ssid".to_string(),
            "apisid".to_string(),
            "sapisid".to_string(),
        );

        inner_tube.set_auth(cookies);

        assert!(inner_tube.is_authenticated());
        assert!(inner_tube.auth_cookies.is_some());
    }

    #[test]
    fn test_inner_tube_clear_auth() {
        let mut inner_tube = InnerTube::new(
            VideoId("test".to_string()),
            ApiKey::new("key".to_string()),
            ClientVersion::new("1.0".to_string()),
            ClientId("1".to_string()),
        );

        let cookies = YouTubeCookies::new(
            "sid".to_string(),
            "hsid".to_string(),
            "ssid".to_string(),
            "apisid".to_string(),
            "sapisid".to_string(),
        );

        inner_tube.set_auth(cookies);
        assert!(inner_tube.is_authenticated());

        inner_tube.clear_auth();
        assert!(!inner_tube.is_authenticated());
    }

    #[test]
    fn test_inner_tube_build_auth_headers() {
        let mut inner_tube = InnerTube::new(
            VideoId("test".to_string()),
            ApiKey::new("key".to_string()),
            ClientVersion::new("1.0".to_string()),
            ClientId("1".to_string()),
        );

        // 認証なしの場合はNone
        assert!(inner_tube.build_auth_headers().is_none());

        // 認証設定後はSome
        let cookies = YouTubeCookies::new(
            "sid".to_string(),
            "hsid".to_string(),
            "ssid".to_string(),
            "apisid".to_string(),
            "sapisid".to_string(),
        );
        inner_tube.set_auth(cookies);

        let headers = inner_tube.build_auth_headers();
        assert!(headers.is_some());

        let headers = headers.unwrap();
        assert_eq!(headers.len(), 4);

        // Authorizationヘッダーの確認
        let auth_header = headers.iter().find(|(k, _)| k == "Authorization");
        assert!(auth_header.is_some());
        assert!(auth_header.unwrap().1.starts_with("SAPISIDHASH "));

        // Cookieヘッダーの確認
        let cookie_header = headers.iter().find(|(k, _)| k == "Cookie");
        assert!(cookie_header.is_some());
        assert!(cookie_header.unwrap().1.contains("SID="));
        assert!(cookie_header.unwrap().1.contains("SAPISID="));

        // X-Originヘッダーの確認
        let origin_header = headers.iter().find(|(k, _)| k == "X-Origin");
        assert!(origin_header.is_some());
        assert_eq!(origin_header.unwrap().1, "https://www.youtube.com");
    }
}
