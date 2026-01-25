//! Standalone HTTP mock server for YouTube InnerTube API
//!
//! Authentication endpoints:
//!   POST /youtubei/v1/account/account_menu  - Session validation check
//!   POST /set_auth_state                    - Control auth behavior
//!   GET  /auth_status                       - Get current auth state

use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::time::Instant;
use warp::Filter;

/// Detect chat mode from continuation token by parsing Protocol Buffer binary data
fn detect_chat_mode_from_token(token: &str) -> Option<String> {
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .or_else(|_| general_purpose::STANDARD.decode(token))
        .ok()?;

    // Pattern 1: Field 16 (0x82 0x01) + 1-byte length + 0x08 + chattype
    for i in 0..decoded.len().saturating_sub(4) {
        if decoded[i] == 0x82 && decoded[i + 1] == 0x01 {
            let len = decoded[i + 2] as usize;
            if decoded[i + 2] & 0x80 == 0 && i + 3 + len <= decoded.len() && decoded[i + 3] == 0x08 {
                let chattype = decoded[i + 4];
                return match chattype {
                    4 => Some("TopChat".to_string()),
                    1 => Some("AllChat".to_string()),
                    _ => None,
                };
            }
            // Pattern 2: 2-byte varint length
            if decoded[i + 2] & 0x80 != 0 && i + 5 < decoded.len() && decoded[i + 4] == 0x08 {
                let chattype = decoded[i + 5];
                return match chattype {
                    4 => Some("TopChat".to_string()),
                    1 => Some("AllChat".to_string()),
                    _ => None,
                };
            }
        }
    }

    // Pattern 3: 0x08 + chattype + 0x10
    for i in 0..decoded.len().saturating_sub(2) {
        if decoded[i] == 0x08 && decoded[i + 2] == 0x10 {
            let chattype = decoded[i + 1];
            if chattype == 0x01 || chattype == 0x04 {
                return match chattype {
                    4 => Some("TopChat".to_string()),
                    1 => Some("AllChat".to_string()),
                    _ => None,
                };
            }
        }
    }

    // Pattern 4: length-delimited field with 0x08 + chattype
    for i in 0..decoded.len().saturating_sub(2) {
        if decoded[i] == 0x08 {
            let chattype = decoded[i + 1];
            if (chattype == 0x01 || chattype == 0x04) && i >= 3 {
                let prev = decoded[i - 1];
                if prev == 0x02 || prev == 0x03 || prev == 0x04 {
                    return match chattype {
                        4 => Some("TopChat".to_string()),
                        1 => Some("AllChat".to_string()),
                        _ => None,
                    };
                }
            }
        }
    }

    None
}

/// Validate continuation token and return detailed validation result
fn validate_continuation_token(token: &str) -> TokenValidation {
    let preview = if token.len() > 50 {
        format!("{}...", &token[..50])
    } else {
        token.to_string()
    };

    // Try to decode
    let decoded = match general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .or_else(|_| general_purpose::STANDARD.decode(token))
    {
        Ok(d) => d,
        Err(_) => {
            return TokenValidation {
                received: true,
                decode_success: false,
                chat_mode_found: false,
                detected_mode: None,
                raw_token_preview: preview,
                decoded_length: 0,
                validation_count: 1,
            };
        }
    };

    let decoded_length = decoded.len();

    // Try to detect chat mode
    let detected_mode = detect_chat_mode_from_token(token);
    let chat_mode_found = detected_mode.is_some();

    TokenValidation {
        received: true,
        decode_success: true,
        chat_mode_found,
        detected_mode,
        raw_token_preview: preview,
        decoded_length,
        validation_count: 1,
    }
}

#[derive(Parser, Debug)]
#[command(name = "mock_server")]
#[command(about = "HTTP mock server for YouTube InnerTube API")]
struct Args {
    #[arg(short, long, default_value = "3456")]
    port: u16,
    #[arg(short, long)]
    file: Option<String>,
    #[arg(short, long, default_value = "1.0")]
    speed: f64,
    #[arg(short, long)]
    r#loop: bool,
    #[arg(long)]
    generate: Option<String>,
    #[arg(long, default_value = "mock_video_12345")]
    video_id: String,
    #[arg(long, default_value = "Mock Broadcaster")]
    channel_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponseEntry { timestamp: u64, response: Value }

/// Token validation result for testing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TokenValidation {
    /// Whether a continuation token was received in the request
    received: bool,
    /// Whether the token was successfully parsed (base64 decode succeeded)
    decode_success: bool,
    /// Whether the chat mode field was found in the token
    chat_mode_found: bool,
    /// The detected chat mode (TopChat or AllChat)
    detected_mode: Option<String>,
    /// Raw token for debugging (truncated to 50 chars)
    raw_token_preview: String,
    /// Token length in bytes after base64 decode
    decoded_length: usize,
    /// Number of tokens validated
    validation_count: u64,
}

/// Auto-message generation state
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutoMessageState {
    enabled: bool,
    messages_per_poll: usize,
    total_generated: u64,
}

impl Default for AutoMessageState {
    fn default() -> Self {
        Self {
            enabled: false,
            messages_per_poll: 5,
            total_generated: 0,
        }
    }
}

struct ServerState {
    config: ServerConfig,
    message_queue: Mutex<VecDeque<Value>>,
    replay_state: Mutex<ReplayState>,
    request_count: AtomicU64,
    message_counter: AtomicU64,
    login_page_visits: AtomicU64,  // Track login page visits for cookie clearing verification
    auth_state: Mutex<AuthState>,
    stream_state: Mutex<StreamState>,
    last_chat_mode: Mutex<Option<String>>,  // Track chat mode from continuation token
    token_validation: Mutex<TokenValidation>,  // Detailed token validation results
    auto_message: Mutex<AutoMessageState>,  // Auto-generate messages for stress testing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamState {
    member_only: bool,           // Member-only stream
    require_auth: bool,          // Require authentication for chat
    title_override: Option<String>, // Override stream title for testing
    channel_id_override: Option<String>, // Override broadcaster channel ID for testing
    channel_name_override: Option<String>, // Override broadcaster channel name for testing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthState {
    session_valid: bool,
    expected_sapisid: Option<String>,
    simulate_error: bool,
    auth_channel_name: String,
    auth_channel_id: String,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            session_valid: true,
            expected_sapisid: None,
            simulate_error: false,
            auth_channel_name: "AuthenticatedUser".to_string(),
            auth_channel_id: "UC_authenticated_user".to_string(),
        }
    }
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            member_only: false,
            require_auth: false,
            title_override: None,
            channel_id_override: None,
            channel_name_override: None,
        }
    }
}

struct ServerConfig {
    video_id: String,
    channel_id: String,
    channel_name: String,
    stream_title: String,
    replay_entries: Vec<ResponseEntry>,
    replay_speed: f64,
    replay_loop: bool,
}

struct ReplayState {
    current_index: usize,
    start_time: Option<Instant>,
    base_timestamp: Option<u64>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Some(path) = args.generate {
        if let Err(e) = generate_sample_ndjson(&path) { eprintln!("Error: {}", e); std::process::exit(1); }
        println!("Generated: {}", path);
        return;
    }
    let replay_entries = if let Some(ref fp) = args.file {
        match load_ndjson(fp) {
            Ok(e) => { println!("Loaded {} entries", e.len()); e }
            Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
        }
    } else { Vec::new() };

    let state = Arc::new(ServerState {
        config: ServerConfig {
            video_id: args.video_id.clone(), channel_id: "UC_mock".into(),
            channel_name: args.channel_name.clone(), stream_title: "Mock Live".into(),
            replay_entries, replay_speed: args.speed, replay_loop: args.r#loop,
        },
        message_queue: Mutex::new(VecDeque::new()),
        replay_state: Mutex::new(ReplayState { current_index: 0, start_time: None, base_timestamp: None }),
        request_count: AtomicU64::new(0), message_counter: AtomicU64::new(0),
        login_page_visits: AtomicU64::new(0),
        auth_state: Mutex::new(AuthState::default()),
        stream_state: Mutex::new(StreamState::default()),
        last_chat_mode: Mutex::new(None),
        token_validation: Mutex::new(TokenValidation::default()),
        auto_message: Mutex::new(AutoMessageState::default()),
    });
    let routes = build_routes(state);
    let addr: SocketAddr = ([127, 0, 0, 1], args.port).into();
    println!("Mock server on http://{}", addr);
    warp::serve(routes).run(addr).await;
}

fn build_routes(state: Arc<ServerState>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // ログインページ（認証ウィンドウE2Eテスト用）
    // GET / または GET /login でアクセス
    let slp = Arc::clone(&state);
    let login_page = warp::path::end().or(warp::path("login")).unify().and(warp::get()).map(move || {
        slp.login_page_visits.fetch_add(1, Ordering::SeqCst);
        warp::reply::html(gen_login_html())
    });

    // ログイン処理（Cookieを設定してリダイレクト）
    // POST /do_login でCookieを設定してURLフラグメントにも含める
    let sdl = Arc::clone(&state);
    let do_login = warp::path("do_login").and(warp::post()).map(move || {
        // Set session_valid to true on login
        sdl.auth_state.lock().unwrap().session_valid = true;

        let cookies = [
            ("SID", "mock_sid_12345"),
            ("HSID", "mock_hsid_12345"),
            ("SSID", "mock_ssid_12345"),
            ("APISID", "mock_apisid_12345"),
            ("SAPISID", "mock_sapisid_12345"),
        ];

        // Cookieを文字列に変換してURLフラグメントに含める
        // URLフラグメントでスペースやセミコロンを含む場合はエンコードが必要
        let cookie_str = cookies.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        // URLエンコード: スペースを%20に、セミコロンを%3Bに変換
        let encoded_cookie_str = cookie_str
            .replace(' ', "%20")
            .replace(';', "%3B");

        // リダイレクトでCookieをフラグメントに含める
        let redirect_url = format!("/logged_in#LISCOV_AUTH:{}", encoded_cookie_str);

        let mut resp = warp::reply::Response::new(warp::hyper::Body::empty());
        *resp.status_mut() = warp::http::StatusCode::SEE_OTHER;
        resp.headers_mut().insert(
            "Location",
            redirect_url.parse().unwrap(),
        );

        // Set-Cookie ヘッダーを追加
        for (name, value) in cookies {
            resp.headers_mut().append(
                "Set-Cookie",
                format!("{}={}; Path=/; SameSite=Lax", name, value).parse().unwrap(),
            );
        }

        resp
    });

    // ログイン完了ページ
    let logged_in = warp::path("logged_in").and(warp::get()).map(|| {
        warp::reply::html(gen_logged_in_html())
    });

    let sw = Arc::clone(&state);
    let watch = warp::path("watch").and(warp::query::<WQ>()).map(move |q: WQ| {
        let vid = q.v.as_deref().unwrap_or(&sw.config.video_id);
        let stream_state = sw.stream_state.lock().unwrap();
        let title = stream_state.title_override.as_ref().unwrap_or(&sw.config.stream_title);
        let channel_id = stream_state.channel_id_override.as_ref().unwrap_or(&sw.config.channel_id);
        let channel_name = stream_state.channel_name_override.as_ref().unwrap_or(&sw.config.channel_name);
        warp::reply::html(gen_html(vid, channel_id, channel_name, title))
    });
    let sa = Arc::clone(&state);
    let chat = warp::path!("youtubei" / "v1" / "live_chat" / "get_live_chat").and(warp::post()).and(warp::body::json())
        .map(move |body: Value| {
            sa.request_count.fetch_add(1, Ordering::SeqCst);
            // Extract and parse continuation token to detect chat mode
            // Default to TopChat (4), but preserve from incoming token if available
            let mut chattype: u8 = 4;
            if let Some(cont) = body.get("continuation").and_then(|c| c.as_str()) {
                // Validate and record token details
                let validation = validate_continuation_token(cont);
                {
                    let mut tv = sa.token_validation.lock().unwrap();
                    tv.received = validation.received;
                    tv.decode_success = validation.decode_success;
                    tv.chat_mode_found = validation.chat_mode_found;
                    tv.detected_mode = validation.detected_mode.clone();
                    tv.raw_token_preview = validation.raw_token_preview;
                    tv.decoded_length = validation.decoded_length;
                    tv.validation_count += 1;
                }

                if let Some(ref mode) = validation.detected_mode {
                    let mut cm = sa.last_chat_mode.lock().unwrap();
                    *cm = Some(mode.clone());
                    chattype = if mode == "AllChat" { 1 } else { 4 };
                }
            } else {
                // No continuation token received
                let mut tv = sa.token_validation.lock().unwrap();
                tv.received = false;
                tv.decode_success = false;
                tv.chat_mode_found = false;
                tv.detected_mode = None;
                tv.raw_token_preview = String::new();
                tv.decoded_length = 0;
                tv.validation_count += 1;
            }
            warp::reply::json(&build_resp(get_actions(&sa), chattype))
        });
    let sac = Arc::clone(&state);
    let acct = warp::path!("youtubei" / "v1" / "account" / "account_menu").and(warp::post())
        .and(warp::header::optional::<String>("authorization")).and(warp::header::optional::<String>("cookie")).and(warp::body::json())
        .map(move |ah: Option<String>, ch: Option<String>, _: Value| {
            let a = sac.auth_state.lock().unwrap();
            if a.simulate_error { return warp::reply::with_status(warp::reply::json(&json!({"error":"Network error"})), warp::http::StatusCode::INTERNAL_SERVER_ERROR); }
            if let Some(ref exp) = a.expected_sapisid {
                let sv = ch.as_ref().map(|c| c.contains(&format!("SAPISID={}", exp))).unwrap_or(false);
                let av = ah.as_ref().map(|h| h.starts_with("SAPISIDHASH ")).unwrap_or(false);
                if !sv || !av { return warp::reply::with_status(warp::reply::json(&json!({"error":"Unauthorized"})), warp::http::StatusCode::UNAUTHORIZED); }
            }
            if !a.session_valid { return warp::reply::with_status(warp::reply::json(&json!({"error":"Session expired"})), warp::http::StatusCode::FORBIDDEN); }
            warp::reply::with_status(warp::reply::json(&json!({"responseContext":{},"actions":[{"openPopupAction":{"popup":{"multiPageMenuRenderer":{"header":{"activeAccountHeaderRenderer":{"accountName":{"simpleText":&a.auth_channel_name}}}}}}}]})), warp::http::StatusCode::OK)
        });
    let ssa = Arc::clone(&state);
    let setauth = warp::path("set_auth_state").and(warp::post()).and(warp::body::json())
        .map(move |b: SAR| { let mut a = ssa.auth_state.lock().unwrap();
            if let Some(v) = b.session_valid { a.session_valid = v; }
            if let Some(s) = b.expected_sapisid { a.expected_sapisid = if s.is_empty() { None } else { Some(s) }; }
            if let Some(e) = b.simulate_error { a.simulate_error = e; }
            if let Some(n) = b.auth_channel_name { a.auth_channel_name = n; }
            warp::reply::json(&json!({"status":"ok","auth":&*a}))
        });
    let sas = Arc::clone(&state);
    let authst = warp::path("auth_status").and(warp::get()).map(move || warp::reply::json(&*sas.auth_state.lock().unwrap()));
    let sst = Arc::clone(&state);
    let status = warp::path("status").and(warp::get()).map(move || {
        let c = sst.request_count.load(Ordering::SeqCst);
        let lpv = sst.login_page_visits.load(Ordering::SeqCst);
        let q = sst.message_queue.lock().unwrap().len();
        let r = sst.replay_state.lock().unwrap();
        let a = sst.auth_state.lock().unwrap();
        let ss = sst.stream_state.lock().unwrap();
        let rp = if !sst.config.replay_entries.is_empty() { Some(format!("{}/{}", r.current_index, sst.config.replay_entries.len())) } else { None };
        warp::reply::json(&json!({"request_count":c,"login_page_visits":lpv,"queued_messages":q,"replay_progress":rp,"video_id":sst.config.video_id,"auth":{"session_valid":a.session_valid},"stream":{"member_only":ss.member_only,"require_auth":ss.require_auth}}))
    });
    let sad = Arc::clone(&state);
    let add = warp::path("add_message").and(warp::post()).and(warp::body::json())
        .map(move |b: AMR| { sad.message_queue.lock().unwrap().push_back(gen_msg(&sad, &b)); warp::reply::json(&json!({"status":"ok"})) });
    // Set stream state (member_only, require_auth, channel_id, channel_name)
    let sss = Arc::clone(&state);
    let setstream = warp::path("set_stream_state").and(warp::post()).and(warp::body::json())
        .map(move |b: SSR| {
            let mut ss = sss.stream_state.lock().unwrap();
            if let Some(v) = b.member_only { ss.member_only = v; }
            if let Some(v) = b.require_auth { ss.require_auth = v; }
            if let Some(t) = b.title { ss.title_override = if t.is_empty() { None } else { Some(t) }; }
            if let Some(c) = b.channel_id { ss.channel_id_override = if c.is_empty() { None } else { Some(c) }; }
            if let Some(n) = b.channel_name { ss.channel_name_override = if n.is_empty() { None } else { Some(n) }; }
            warp::reply::json(&json!({"status":"ok","stream":&*ss}))
        });
    let scm = Arc::clone(&state);
    let chat_mode_status = warp::path("chat_mode_status").and(warp::get()).map(move || {
        let cm = scm.last_chat_mode.lock().unwrap();
        warp::reply::json(&json!({"chat_mode": *cm}))
    });
    let stv = Arc::clone(&state);
    let token_validation = warp::path("token_validation").and(warp::get()).map(move || {
        let tv = stv.token_validation.lock().unwrap();
        warp::reply::json(&*tv)
    });
    let srs = Arc::clone(&state);
    let reset = warp::path("reset").and(warp::post()).map(move || {
        let mut r = srs.replay_state.lock().unwrap(); r.current_index = 0; r.start_time = None; r.base_timestamp = None;
        srs.message_queue.lock().unwrap().clear(); *srs.auth_state.lock().unwrap() = AuthState::default();
        *srs.stream_state.lock().unwrap() = StreamState::default();
        *srs.last_chat_mode.lock().unwrap() = None;
        *srs.token_validation.lock().unwrap() = TokenValidation::default();
        *srs.auto_message.lock().unwrap() = AutoMessageState::default();
        srs.login_page_visits.store(0, Ordering::SeqCst);
        warp::reply::json(&json!({"status":"ok"}))
    });
    // Set auto-message generation settings
    let sam = Arc::clone(&state);
    let set_auto_message = warp::path("set_auto_message").and(warp::post()).and(warp::body::json())
        .map(move |b: AutoMsgReq| {
            let mut auto_msg = sam.auto_message.lock().unwrap();
            if let Some(enabled) = b.enabled { auto_msg.enabled = enabled; }
            if let Some(mpp) = b.messages_per_poll { auto_msg.messages_per_poll = mpp; }
            warp::reply::json(&json!({"status":"ok","auto_message":{"enabled":auto_msg.enabled,"messages_per_poll":auto_msg.messages_per_poll,"total_generated":auto_msg.total_generated}}))
        });
    // Get auto-message status
    let gam = Arc::clone(&state);
    let auto_message_status = warp::path("auto_message_status").and(warp::get()).map(move || {
        let auto_msg = gam.auto_message.lock().unwrap();
        warp::reply::json(&*auto_msg)
    });
    login_page.or(do_login).or(logged_in).or(watch).or(chat).or(acct).or(setauth).or(authst).or(status).or(add).or(setstream).or(chat_mode_status).or(token_validation).or(reset).or(set_auto_message).or(auto_message_status)
}

#[derive(Debug, Deserialize)] struct WQ { v: Option<String> }
#[derive(Debug, Deserialize)] struct AMR { message_type: String, author: String, #[serde(default = "dcid")] channel_id: String, #[serde(default)] content: String, amount: Option<String>, tier: Option<String>, #[serde(default)] is_member: bool, milestone_months: Option<u32>, gift_count: Option<u32> }
#[derive(Debug, Deserialize)] struct SAR { session_valid: Option<bool>, expected_sapisid: Option<String>, simulate_error: Option<bool>, auth_channel_name: Option<String>, auth_channel_id: Option<String> }
#[derive(Debug, Deserialize)] struct SSR { member_only: Option<bool>, require_auth: Option<bool>, title: Option<String>, channel_id: Option<String>, channel_name: Option<String> }
#[derive(Debug, Deserialize)] struct AutoMsgReq { enabled: Option<bool>, messages_per_poll: Option<usize> }
fn dcid() -> String { format!("UC_user_{}", rand::random::<u32>() % 1000) }

fn get_actions(s: &ServerState) -> Vec<Value> {
    // First, drain any queued messages
    { let mut q = s.message_queue.lock().unwrap(); if !q.is_empty() { return q.drain(..).collect(); } }

    // Check if auto-message generation is enabled
    {
        let mut auto_msg = s.auto_message.lock().unwrap();
        if auto_msg.enabled {
            let mut msgs = Vec::new();
            for i in 0..auto_msg.messages_per_poll {
                let msg_num = auto_msg.total_generated + i as u64;
                let id = format!("auto_msg_{}", s.message_counter.fetch_add(1, Ordering::SeqCst));
                let ts = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros());
                let author = format!("AutoUser{}", msg_num % 50);
                let content = format!("Auto message #{} - simulating real YouTube chat flow", msg_num);
                let channel_id = format!("UC_auto_{}", msg_num % 50);
                msgs.push(json!({"addChatItemAction":{"item":{"liveChatTextMessageRenderer":{"id":id,"timestampUsec":ts,"authorName":{"simpleText":author},"authorPhoto":{"thumbnails":[{"url":"https://example.com/av.png"}]},"authorExternalChannelId":channel_id,"message":{"runs":[{"text":content}]},"authorBadges":[]}}}}));
            }
            auto_msg.total_generated += auto_msg.messages_per_poll as u64;
            return msgs;
        }
    }

    // Fall back to replay entries
    if s.config.replay_entries.is_empty() { return Vec::new(); }
    let mut rs = s.replay_state.lock().unwrap();
    if rs.start_time.is_none() { rs.start_time = Some(Instant::now()); rs.base_timestamp = Some(s.config.replay_entries[0].timestamp); }
    let st = rs.start_time.unwrap(); let bt = rs.base_timestamp.unwrap();
    let es = if s.config.replay_speed > 0.0 { (st.elapsed().as_secs_f64() * s.config.replay_speed) as u64 } else { u64::MAX };
    let ct = bt.saturating_add(es);
    let mut acts = Vec::new();
    while rs.current_index < s.config.replay_entries.len() {
        let e = &s.config.replay_entries[rs.current_index];
        if e.timestamp > ct { break; }
        if let Some(ea) = e.response.pointer("/continuationContents/liveChatContinuation/actions").and_then(|v| v.as_array()) { acts.extend(ea.clone()); }
        rs.current_index += 1; if acts.len() >= 20 { break; }
    }
    if rs.current_index >= s.config.replay_entries.len() && s.config.replay_loop {
        rs.current_index = 0; rs.start_time = Some(Instant::now()); rs.base_timestamp = Some(s.config.replay_entries[0].timestamp);
    }
    acts
}

/// Split title into runs, separating hashtags (mimics YouTube's behavior)
fn split_title_into_runs(title: &str) -> Vec<serde_json::Value> {
    let mut runs = Vec::new();
    let mut current = String::new();
    let mut chars = title.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '#' {
            // Push current text as a run if not empty
            if !current.is_empty() {
                runs.push(json!({"text": current}));
                current = String::new();
            }
            // Collect the hashtag
            let mut hashtag = String::from("#");
            while let Some(&next_c) = chars.peek() {
                if next_c.is_whitespace() {
                    break;
                }
                hashtag.push(chars.next().unwrap());
            }
            runs.push(json!({"text": hashtag}));
        } else {
            current.push(c);
        }
    }
    // Push remaining text
    if !current.is_empty() {
        runs.push(json!({"text": current}));
    }
    // If no runs were created (empty title), add empty run
    if runs.is_empty() {
        runs.push(json!({"text": ""}));
    }
    runs
}

fn gen_html(_vid: &str, cid: &str, cn: &str, t: &str) -> String {
    // Generate a proper continuation token with TopChat mode (chattype=4)
    let ct = generate_mock_continuation_token(4);
    let title_runs = split_title_into_runs(t);
    let d = json!({"contents":{"twoColumnWatchNextResults":{"results":{"results":{"contents":[{"videoPrimaryInfoRenderer":{"title":{"runs":title_runs}}},{"videoSecondaryInfoRenderer":{"owner":{"videoOwnerRenderer":{"title":{"runs":[{"text":cn}]},"navigationEndpoint":{"browseEndpoint":{"browseId":cid}}}}}}]}},"conversationBar":{"liveChatRenderer":{"continuations":[{"reloadContinuationData":{"continuation":ct}}],"isReplay":false}}}}});
    format!("<!DOCTYPE html><html><head><title>{}</title></head><body><script>var ytInitialData = {};</script></body></html>", t, serde_json::to_string(&d).unwrap())
}

/// ログインページのHTML生成（E2Eテスト用）
fn gen_login_html() -> String {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>Mock YouTube Login</title>
    <style>
        body { font-family: Arial, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #f1f1f1; }
        .login-box { background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); text-align: center; }
        h1 { color: #c4302b; margin-bottom: 20px; }
        button { background: #c4302b; color: white; border: none; padding: 12px 24px; font-size: 16px; border-radius: 4px; cursor: pointer; }
        button:hover { background: #a02520; }
        #auto-login { margin-top: 20px; font-size: 12px; color: #666; }
    </style>
</head>
<body>
    <div class="login-box">
        <h1>Mock YouTube Login</h1>
        <p>E2Eテスト用のモックログインページです</p>
        <form action="/do_login" method="POST">
            <button type="submit" id="login-button">ログイン</button>
        </form>
        <p id="auto-login">自動ログインが有効な場合、このページは自動的にログインします</p>
    </div>
    <script>
        // E2Eテスト用: auto_loginクエリパラメータがあれば自動ログイン
        const params = new URLSearchParams(window.location.search);
        if (params.get('auto_login') === 'true') {
            document.querySelector('form').submit();
        }
    </script>
</body>
</html>"#.to_string()
}

/// ログイン完了後のHTML生成（E2Eテスト用）
fn gen_logged_in_html() -> String {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>Mock YouTube - Logged In</title>
    <style>
        body { font-family: Arial, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #f1f1f1; }
        .success-box { background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); text-align: center; }
        h1 { color: #4CAF50; margin-bottom: 20px; }
        .cookies { background: #f5f5f5; padding: 10px; border-radius: 4px; font-family: monospace; font-size: 12px; text-align: left; margin-top: 20px; }
    </style>
</head>
<body>
    <div class="success-box">
        <h1>✓ ログイン完了</h1>
        <p>認証情報が設定されました</p>
        <div class="cookies">
            <strong>設定されたCookie:</strong><br>
            SID=mock_sid_12345<br>
            HSID=mock_hsid_12345<br>
            SSID=mock_ssid_12345<br>
            APISID=mock_apisid_12345<br>
            SAPISID=mock_sapisid_12345
        </div>
    </div>
    <script>
        // 認証情報検出用: CookieをページのtitleにLISCOV_AUTH:プレフィックス付きで設定
        // これによりTauriのauth_windowがCookieを検出できる
        const cookies = document.cookie;
        if (cookies && cookies.includes('SAPISID=')) {
            document.title = 'LISCOV_AUTH:' + cookies;
        }
    </script>
</body>
</html>"#.to_string()
}

/// Generate a mock continuation token with proper Protocol Buffer structure
/// that includes the chattype field (4=TopChat, 1=AllChat)
fn generate_mock_continuation_token(chattype: u8) -> String {
    // Build a minimal Protocol Buffer structure:
    // Field 16 (0x82 0x01) + length(2) + Field 1 (0x08) + chattype
    // Plus some random data to make each token unique
    let random_data: u32 = rand::random();
    let bytes: Vec<u8> = vec![
        0xd2, 0x87, 0xcc, 0xc8, 0x03,           // Some header bytes
        0x10, (random_data & 0xFF) as u8,       // Field 2 with random value
        0x82, 0x01, 0x02, 0x08, chattype,       // Field 16 with chattype
        0x20, ((random_data >> 8) & 0xFF) as u8, // Trailing field
    ];
    general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

fn build_resp(acts: Vec<Value>, chattype: u8) -> Value {
    // Preserve the chat mode from the incoming request
    let token = generate_mock_continuation_token(chattype);
    json!({"continuationContents":{"liveChatContinuation":{"continuations":[{"invalidationContinuationData":{"continuation":token,"timeoutMs":5000}}],"actions":acts}}})
}

fn gen_msg(s: &ServerState, r: &AMR) -> Value {
    let id = format!("mock_msg_{}", s.message_counter.fetch_add(1, Ordering::SeqCst));
    let ts = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros());
    // Member badge for member messages
    let member_badge = if r.is_member {
        json!([{"liveChatAuthorBadgeRenderer":{"customThumbnail":{"thumbnails":[{"url":"https://example.com/member_badge.png"}]},"tooltip":"Member"}}])
    } else {
        json!([])
    };
    match r.message_type.as_str() {
        "superchat" => json!({"addChatItemAction":{"item":{"liveChatPaidMessageRenderer":{"id":id,"timestampUsec":ts,"authorName":{"simpleText":&r.author},"authorPhoto":{"thumbnails":[{"url":"https://example.com/av.png"}]},"authorExternalChannelId":&r.channel_id,"purchaseAmountText":{"simpleText":r.amount.as_deref().unwrap_or("¥500")},"message":{"runs":[{"text":&r.content}]},"headerBackgroundColor":tier_col(r.tier.as_deref()),"headerTextColor":0xFFFFFF,"bodyBackgroundColor":tier_col(r.tier.as_deref()),"bodyTextColor":0xFFFFFF,"authorBadges":member_badge}}}}),
        "supersticker" => json!({"addChatItemAction":{"item":{"liveChatPaidStickerRenderer":{"id":id,"timestampUsec":ts,"authorName":{"simpleText":&r.author},"authorPhoto":{"thumbnails":[{"url":"https://example.com/av.png"}]},"authorExternalChannelId":&r.channel_id,"purchaseAmountText":{"simpleText":r.amount.as_deref().unwrap_or("¥500")},"sticker":{"thumbnails":[{"url":"https://example.com/sticker.png"}]},"moneyChipBackgroundColor":tier_col(r.tier.as_deref()),"moneyChipTextColor":0xFFFFFF,"authorBadges":member_badge}}}}),
        "membership" => json!({"addChatItemAction":{"item":{"liveChatMembershipItemRenderer":{"id":id,"timestampUsec":ts,"authorName":{"simpleText":&r.author},"authorPhoto":{"thumbnails":[{"url":"https://example.com/av.png"}]},"authorExternalChannelId":&r.channel_id,"headerSubtext":{"runs":[{"text":"Welcome to "},{"text":"Channel"},{"text":"!"}]},"authorBadges":[{"liveChatAuthorBadgeRenderer":{"tooltip":"New member","customThumbnail":{"thumbnails":[{"url":"https://example.com/badge.png"}]}}}]}}}}),
        "membership_milestone" => {
            let months = r.milestone_months.unwrap_or(6);
            // Actual YouTube format: months in badge tooltip, not headerSubtext
            json!({"addChatItemAction":{"item":{"liveChatMembershipItemRenderer":{"id":id,"timestampUsec":ts,"authorName":{"simpleText":&r.author},"authorPhoto":{"thumbnails":[{"url":"https://example.com/av.png"}]},"authorExternalChannelId":&r.channel_id,"headerSubtext":{"runs":[{"text":"Welcome to "},{"text":"Channel"},{"text":"!"}]},"message":{"runs":[{"text":&r.content}]},"authorBadges":[{"liveChatAuthorBadgeRenderer":{"tooltip":format!("Member ({} months)", months),"customThumbnail":{"thumbnails":[{"url":"https://example.com/badge.png"}]}}}]}}}})
        },
        "membership_gift" => {
            let count = r.gift_count.unwrap_or(5);
            // Actual YouTube format: authorExternalChannelId at root level
            json!({"addChatItemAction":{"item":{"liveChatSponsorshipsGiftPurchaseAnnouncementRenderer":{"id":id,"timestampUsec":ts,"authorExternalChannelId":&r.channel_id,"header":{"liveChatSponsorshipsHeaderRenderer":{"authorName":{"simpleText":&r.author},"authorPhoto":{"thumbnails":[{"url":"https://example.com/av.png"}]},"primaryText":{"runs":[{"text":"Sent "},{"text":format!("{}", count)},{"text":" "},{"text":"Channel"},{"text":" gift memberships"}]}}}}}}})
        },
        "system" => json!({"addChatItemAction":{"item":{"liveChatTextMessageRenderer":{"id":id,"timestampUsec":ts,"authorName":{"simpleText":"System"},"authorExternalChannelId":"system","message":{"runs":[{"text":&r.content}]}}}}}),
        _ => json!({"addChatItemAction":{"item":{"liveChatTextMessageRenderer":{"id":id,"timestampUsec":ts,"authorName":{"simpleText":&r.author},"authorPhoto":{"thumbnails":[{"url":"https://example.com/av.png"}]},"authorExternalChannelId":&r.channel_id,"message":{"runs":[{"text":&r.content}]},"authorBadges":member_badge}}}})
    }
}

fn tier_col(t: Option<&str>) -> u32 { match t { Some("blue")=>0x1565C0, Some("cyan")=>0x00B8D4, Some("green")=>0x00BFA5, Some("yellow")=>0xFFB300, Some("orange")=>0xE65100, Some("magenta")=>0xC2185B, Some("red")=>0xD00000, _=>0x00BFA5 } }

fn load_ndjson(p: &str) -> Result<Vec<ResponseEntry>, String> {
    let f = File::open(p).map_err(|e| format!("Open error: {}", e))?;
    let mut ents: Vec<ResponseEntry> = Vec::new();
    for (i, lr) in BufReader::new(f).lines().enumerate() {
        let l = lr.map_err(|e| format!("Line {}: {}", i+1, e))?;
        if l.trim().is_empty() { continue; }
        ents.push(serde_json::from_str(&l).map_err(|e| format!("Line {}: {}", i+1, e))?);
    }
    ents.sort_by_key(|e| e.timestamp); Ok(ents)
}

fn generate_sample_ndjson(p: &str) -> Result<(), String> {
    let mut f = File::create(p).map_err(|e| format!("Create error: {}", e))?;
    let bt = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    for (i, (a, m)) in [("User1","Hello!"),("User2","Hi!"),("User3","Thanks!")].iter().enumerate() {
        let ts = bt + (i as u64 * 2);
        let e = json!({"timestamp":ts,"response":{"continuationContents":{"liveChatContinuation":{"actions":[{"addChatItemAction":{"item":{"liveChatTextMessageRenderer":{"id":format!("msg_{}",i),"timestampUsec":format!("{}",ts*1000000),"authorName":{"simpleText":a},"message":{"runs":[{"text":m}]},"authorBadges":[]}}}}]}}}});
        writeln!(f, "{}", serde_json::to_string(&e).unwrap()).map_err(|e| format!("Write error: {}", e))?;
    }
    Ok(())
}
