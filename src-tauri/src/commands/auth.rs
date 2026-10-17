//! Authentication commands
//!
//! Implements 01_auth.md specification

use crate::commands::auth_window;
use crate::commands::config::{ConfigState, StorageMode};
use crate::core::models::YouTubeCookies;
use crate::errors::CommandError;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use tauri::State;
use ts_rs::TS;

// keyring_service のデフォルト値は paths モジュールで管理
const KEYRING_USER: &str = "youtube_credentials";

// =============================================================================
// CredentialStorage トレイト
// =============================================================================

/// セキュアストレージの抽象化トレイト
/// テスト時に InMemoryStorage で差し替え可能にする
pub(crate) trait CredentialStorage: Send + Sync {
    fn load(&self) -> Result<YouTubeCookies, String>;
    fn save(&self, cookies: &YouTubeCookies) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
    fn is_available(&self) -> bool;
}

/// keyring の 1 件に入る長さ（keyring 3.6.3 の Windows 実装は UTF-16 で 1280 単位 = 2560 バイト）
const KEYRING_MAX_UTF16_UNITS: usize = 1280;

/// keyring の 1 件ずつの読み書き（分割保存の下の層。テスト時は差し替える）
pub(crate) trait SecretSlots: Send + Sync {
    /// 件が無ければ `Ok(None)`
    fn get(&self, user: &str) -> Result<Option<String>, String>;
    fn set(&self, user: &str, secret: &str) -> Result<(), String>;
    /// 件が無くても `Ok`
    fn remove(&self, user: &str) -> Result<(), String>;
}

/// Windows Credential Manager などの OS のセキュアストレージ（本番用）
struct KeyringSlots;

impl KeyringSlots {
    fn entry(user: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(&crate::paths::keyring_service(), user)
            .map_err(|e| format!("Failed to access secure storage: {}", e))
    }
}

impl SecretSlots for KeyringSlots {
    fn get(&self, user: &str) -> Result<Option<String>, String> {
        match Self::entry(user)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("Failed to read from secure storage: {}", e)),
        }
    }

    fn set(&self, user: &str, secret: &str) -> Result<(), String> {
        Self::entry(user)?
            .set_password(secret)
            .map_err(|e| format!("Failed to save to secure storage: {}", e))
    }

    fn remove(&self, user: &str) -> Result<(), String> {
        match Self::entry(user)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Failed to delete from secure storage: {}", e)),
        }
    }
}

/// 分けて保存したときの本体の件の中身（01_auth.md「1 件に入らないときの分割」）
#[derive(Debug, Serialize, Deserialize)]
struct PartsHeader {
    parts: usize,
}

/// 分けたときの i 番目（1 始まり）の件のユーザー名
fn part_user(i: usize) -> String {
    format!("{}.{}", KEYRING_USER, i)
}

/// 文字の境目で、UTF-16 で `max_units` 単位以下ずつに分ける
fn split_secret(secret: &str, max_units: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut units = 0;
    for c in secret.chars() {
        if units + c.len_utf16() > max_units {
            parts.push(std::mem::take(&mut current));
            units = 0;
        }
        current.push(c);
        units += c.len_utf16();
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// `start` 番目から、見つからなくなるまでの部分を消す
fn remove_parts_from(slots: &dyn SecretSlots, start: usize) -> Result<(), String> {
    let mut i = start;
    while slots.get(&part_user(i))?.is_some() {
        slots.remove(&part_user(i))?;
        i += 1;
    }
    Ok(())
}

/// 1 件に入らなければ分けて書き、前回の余った部分を消す
fn write_split_secret(slots: &dyn SecretSlots, secret: &str) -> Result<(), String> {
    let parts = if secret.encode_utf16().count() <= KEYRING_MAX_UTF16_UNITS {
        slots.set(KEYRING_USER, secret)?;
        0
    } else {
        let parts = split_secret(secret, KEYRING_MAX_UTF16_UNITS);
        for (i, part) in parts.iter().enumerate() {
            slots.set(&part_user(i + 1), part)?;
        }
        let header = serde_json::to_string(&PartsHeader { parts: parts.len() })
            .map_err(|e| format!("Failed to serialize credentials: {}", e))?;
        slots.set(KEYRING_USER, &header)?;
        parts.len()
    };
    remove_parts_from(slots, parts + 1)
}

/// 本体の件が見出しなら部分をつないで返す。件が無ければ `Ok(None)`
fn read_split_secret(slots: &dyn SecretSlots) -> Result<Option<String>, String> {
    let Some(main) = slots.get(KEYRING_USER)? else {
        return Ok(None);
    };
    let Ok(header) = serde_json::from_str::<PartsHeader>(&main) else {
        return Ok(Some(main));
    };
    (1..=header.parts)
        .map(|i| {
            slots
                .get(&part_user(i))?
                .ok_or_else(|| format!("Secure storage is missing part {} of {}", i, header.parts))
        })
        .collect::<Result<String, String>>()
        .map(Some)
}

/// keyring を使用するセキュアストレージ実装。1 件に入らない認証情報は分けて保存する
struct KeyringStorage<S>(S);

/// 本番のセキュアストレージ
const KEYRING_STORAGE: KeyringStorage<KeyringSlots> = KeyringStorage(KeyringSlots);

impl<S: SecretSlots> CredentialStorage for KeyringStorage<S> {
    fn load(&self) -> Result<YouTubeCookies, String> {
        log::info!("📂 Loading from secure storage...");
        let secret = read_split_secret(&self.0)
            .and_then(|secret| {
                secret.ok_or_else(|| "No credentials found in secure storage".to_string())
            })
            .inspect_err(|msg| log::info!("📂 Load error: {}", msg))?;
        log::info!("📂 Load success");

        let json: CredentialsJson = serde_json::from_str(&secret)
            .map_err(|e| format!("Failed to parse credentials: {}", e))?;

        let cookies: YouTubeCookies = json.into();

        if cookies.sapisid.is_empty() {
            return Err("Invalid credentials: SAPISID is missing".to_string());
        }

        Ok(cookies)
    }

    fn save(&self, cookies: &YouTubeCookies) -> Result<(), String> {
        log::info!("📝 Saving to secure storage...");
        let json: CredentialsJson = cookies.into();
        let secret = serde_json::to_string(&json)
            .map_err(|e| format!("Failed to serialize credentials: {}", e))?;

        log::debug!("Credential JSON length: {} chars", secret.len());

        write_split_secret(&self.0, &secret)?;

        log::info!("Credentials saved to secure storage");

        // 保存直後に読み戻して検証
        match read_split_secret(&self.0) {
            Ok(Some(read_back)) if read_back == secret => {
                log::info!("✅ Verified: credentials can be read back");
            }
            Ok(_) => log::warn!("⚠️ Mismatch: written and read data differ"),
            Err(e) => {
                log::error!("❌ Verification failed: could not read back: {}", e);
                return Err(format!("Credentials saved but cannot be read back: {}", e));
            }
        }

        Ok(())
    }

    fn delete(&self) -> Result<(), String> {
        self.0.remove(KEYRING_USER)?;
        remove_parts_from(&self.0, 1)?;
        log::info!("Credentials deleted from secure storage");
        Ok(())
    }

    fn is_available(&self) -> bool {
        // エントリへのアクセスを試行（読み込みテスト）
        // エントリなしでもストレージ自体は利用可能
        self.0.get(KEYRING_USER).is_ok()
    }
}

/// In-memory cache for credentials to work around keyring issues on Windows
/// The keyring crate may fail to read credentials from a new Entry instance
/// even immediately after writing, despite verification succeeding within
/// the same Entry instance. This cache provides a reliable fallback.
static CREDENTIALS_CACHE: RwLock<Option<YouTubeCookies>> = RwLock::new(None);

/// テスト用: CREDENTIALS_CACHE をクリアする
/// テスト間のキャッシュ汚染を防ぐためにのみ使用する
#[doc(hidden)]
pub fn clear_credentials_cache_for_test() {
    if let Ok(mut guard) = CREDENTIALS_CACHE.write() {
        *guard = None;
    }
}

/// Storage type for credentials
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub enum StorageType {
    Secure,
    Fallback,
}

/// Authentication status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub has_saved_credentials: bool,
    pub storage_type: StorageType,
    pub storage_error: Option<String>,
}

/// Session validity result
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct SessionValidity {
    pub is_valid: bool,
    pub checked_at: String,
    pub error: Option<String>,
}

/// Credentials for JSON storage in keyring
#[derive(Debug, Serialize, Deserialize)]
struct CredentialsJson {
    sid: String,
    hsid: String,
    ssid: String,
    apisid: String,
    sapisid: String,
    #[serde(default)]
    raw_cookie_string: Option<String>,
}

impl From<&YouTubeCookies> for CredentialsJson {
    fn from(cookies: &YouTubeCookies) -> Self {
        Self {
            sid: cookies.sid.clone(),
            hsid: cookies.hsid.clone(),
            ssid: cookies.ssid.clone(),
            apisid: cookies.apisid.clone(),
            sapisid: cookies.sapisid.clone(),
            raw_cookie_string: cookies.raw_cookie_string.clone(),
        }
    }
}

impl From<CredentialsJson> for YouTubeCookies {
    fn from(json: CredentialsJson) -> Self {
        Self {
            sid: json.sid,
            hsid: json.hsid,
            ssid: json.ssid,
            apisid: json.apisid,
            sapisid: json.sapisid,
            raw_cookie_string: json.raw_cookie_string,
        }
    }
}

/// Credentials configuration file structure (for fallback mode)
#[derive(Debug, Serialize, Deserialize)]
struct CredentialsConfig {
    youtube: YouTubeCookiesConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct YouTubeCookiesConfig {
    #[serde(default)]
    sid: String,
    #[serde(default)]
    hsid: String,
    #[serde(default)]
    ssid: String,
    #[serde(default)]
    apisid: String,
    #[serde(default)]
    sapisid: String,
    #[serde(default)]
    raw_cookies: Option<String>,
}

impl From<YouTubeCookiesConfig> for YouTubeCookies {
    fn from(config: YouTubeCookiesConfig) -> Self {
        Self {
            sid: config.sid,
            hsid: config.hsid,
            ssid: config.ssid,
            apisid: config.apisid,
            sapisid: config.sapisid,
            raw_cookie_string: config.raw_cookies,
        }
    }
}

impl From<&YouTubeCookies> for YouTubeCookiesConfig {
    fn from(cookies: &YouTubeCookies) -> Self {
        Self {
            sid: cookies.sid.clone(),
            hsid: cookies.hsid.clone(),
            ssid: cookies.ssid.clone(),
            apisid: cookies.apisid.clone(),
            sapisid: cookies.sapisid.clone(),
            raw_cookies: cookies.raw_cookie_string.clone(),
        }
    }
}

/// 認証情報ファイルのパスを返す（フォールバックモード用）
fn get_credentials_path() -> Result<PathBuf, String> {
    crate::paths::credentials_path()
}

/// Check if credentials file exists
fn credentials_file_exists() -> bool {
    get_credentials_path().map(|p| p.exists()).unwrap_or(false)
}

// =============================================================================
// Secure Storage (keyring) operations — KeyringStorage への委譲ラッパー
// =============================================================================

/// Load cookies from secure storage (keyring)
fn load_cookies_from_secure_storage() -> Result<YouTubeCookies, String> {
    KEYRING_STORAGE.load()
}

/// Check if secure storage is available
fn is_secure_storage_available() -> bool {
    KEYRING_STORAGE.is_available()
}

// =============================================================================
// File Storage (fallback) operations
// =============================================================================

/// Load cookies from file (fallback mode)
fn load_cookies_from_file() -> Result<YouTubeCookies, String> {
    let path = get_credentials_path()?;

    if !path.exists() {
        return Err("Credentials file not found".to_string());
    }

    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read credentials file: {}", e))?;

    let config: CredentialsConfig =
        toml::from_str(&content).map_err(|e| format!("Failed to parse credentials file: {}", e))?;

    // Handle raw_cookies if present
    if let Some(ref raw) = config.youtube.raw_cookies {
        if !raw.is_empty() {
            return Ok(parse_raw_cookies(raw));
        }
    }

    let cookies: YouTubeCookies = config.youtube.into();

    if cookies.sapisid.is_empty() {
        return Err("Invalid credentials: SAPISID is missing".to_string());
    }

    Ok(cookies)
}

/// Parse raw cookie string
fn parse_raw_cookies(raw: &str) -> YouTubeCookies {
    let extract = |name: &str| -> String {
        for part in raw.split(';') {
            let part = part.trim();
            if let Some(pos) = part.find('=') {
                let key = &part[..pos];
                if key == name {
                    return part[pos + 1..].to_string();
                }
            }
        }
        String::new()
    };

    YouTubeCookies {
        sid: extract("SID"),
        hsid: extract("HSID"),
        ssid: extract("SSID"),
        apisid: extract("APISID"),
        sapisid: extract("SAPISID"),
        raw_cookie_string: Some(raw.to_string()),
    }
}

/// Save cookies to file (fallback mode)
fn save_cookies_to_file(cookies: &YouTubeCookies) -> Result<(), String> {
    let path = get_credentials_path()?;

    let config = CredentialsConfig {
        youtube: cookies.into(),
    };

    let toml_string = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize credentials: {}", e))?;

    crate::atomic_file::write(&path, &toml_string)
        .map_err(|e| format!("Failed to write credentials file: {}", e))?;

    log::info!("Credentials saved to {:?}", path);
    Ok(())
}

/// Delete credentials file
fn delete_credentials_file() -> Result<(), String> {
    let path = get_credentials_path()?;

    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete credentials file: {}", e))?;
        log::info!("Credentials file deleted");
    }

    Ok(())
}

/// secure モードで残っている credentials.toml を消す
///
/// secure モードのファイルは、旧版が容量超過のときに同意なしに書いた平文の Cookie
/// （01_auth.md「secure モードの保存先」）。消せなくても読み書きは成功とする
fn remove_leftover_credentials_file() {
    if !credentials_file_exists() {
        return;
    }
    match delete_credentials_file() {
        Ok(()) => log::info!("Removed leftover plaintext credentials file in secure mode"),
        Err(e) => log::warn!("⚠️ Failed to remove leftover credentials file: {}", e),
    }
}

// =============================================================================
// Combined operations (based on storage mode)
// =============================================================================

/// ストレージモードに基づいてCookieをロードする（CredentialStorage差し替え可能版）
///
/// ロジック:
/// 1. インメモリキャッシュを確認
/// 2. Fallbackモード → ファイルから読み込み
/// 3. Secureモード → secure_storage.load() を試行
///    - 成功 → 残っている credentials.toml を消し、キャッシュ更新して返却
///    - "No credentials found" → ファイルからのマイグレーションを試行
///    - その他のエラー → そのまま返却
pub(crate) fn load_cookies_with_storage(
    storage_mode: &StorageMode,
    secure_storage: &dyn CredentialStorage,
    cache: &RwLock<Option<YouTubeCookies>>,
) -> Result<YouTubeCookies, String> {
    // まずインメモリキャッシュを確認（keyring Windows問題のワークアラウンド）
    if let Ok(guard) = cache.read() {
        if let Some(ref cached_cookies) = *guard {
            log::info!("📦 Returning credentials from memory cache");
            return Ok(cached_cookies.clone());
        }
    }

    match storage_mode {
        StorageMode::Fallback => {
            let cookies = load_cookies_from_file()?;
            // キャッシュ更新
            if let Ok(mut guard) = cache.write() {
                *guard = Some(cookies.clone());
            }
            Ok(cookies)
        }
        StorageMode::Secure => {
            // セキュアストレージから読み込みを試行
            match secure_storage.load() {
                Ok(cookies) => {
                    remove_leftover_credentials_file();
                    // キャッシュ更新
                    if let Ok(mut guard) = cache.write() {
                        *guard = Some(cookies.clone());
                    }
                    Ok(cookies)
                }
                Err(e) => {
                    // "no entry" エラーとストレージ障害を区別
                    if e.contains("No credentials found") {
                        // マイグレーション: ファイルにデータがあればセキュアストレージへ移行
                        if credentials_file_exists() {
                            log::info!("Migrating credentials from file to secure storage");
                            let cookies = load_cookies_from_file()?;
                            // 移行を試行
                            if secure_storage.save(&cookies).is_ok() {
                                // 移行成功 → ファイル削除
                                let _ = delete_credentials_file();
                            }
                            // キャッシュ更新
                            if let Ok(mut guard) = cache.write() {
                                *guard = Some(cookies.clone());
                            }
                            return Ok(cookies);
                        }
                    }
                    Err(e)
                }
            }
        }
    }
}

/// Cookieを保存する（CredentialStorage差し替え可能版）
///
/// Secureモードで保存に失敗したら（容量超過も含む）エラーを返し、ファイルには書かない
/// （01_auth.md「制約・不変条件」: 同意なしにファイル保存へフォールバックしない）。
/// 大きい Cookie は secure_storage が分けて保存する
pub(crate) fn save_cookies_with_storage(
    cookies: &YouTubeCookies,
    storage_mode: &StorageMode,
    secure_storage: &dyn CredentialStorage,
    cache: &RwLock<Option<YouTubeCookies>>,
) -> Result<(), String> {
    // まずインメモリキャッシュを更新（keyring Windows問題のワークアラウンド）
    if let Ok(mut guard) = cache.write() {
        *guard = Some(cookies.clone());
        log::info!("📦 Credentials cached in memory");
    }

    match storage_mode {
        StorageMode::Fallback => save_cookies_to_file(cookies),
        StorageMode::Secure => {
            secure_storage.save(cookies)?;
            remove_leftover_credentials_file();
            Ok(())
        }
    }
}

/// 認証情報を削除する（CredentialStorage差し替え可能版）
pub(crate) fn delete_credentials_with_storage(
    storage_mode: &StorageMode,
    secure_storage: &dyn CredentialStorage,
    cache: &RwLock<Option<YouTubeCookies>>,
) -> Result<(), String> {
    // インメモリキャッシュをクリア
    if let Ok(mut guard) = cache.write() {
        *guard = None;
        log::info!("📦 Credentials cache cleared");
    }

    match storage_mode {
        StorageMode::Fallback => delete_credentials_file(),
        StorageMode::Secure => {
            // 安全のため両方から削除
            let _ = secure_storage.delete();
            let _ = delete_credentials_file();
            Ok(())
        }
    }
}

/// Load cookies based on storage mode（本番用ラッパー）
pub(crate) fn load_cookies(storage_mode: &StorageMode) -> Result<YouTubeCookies, String> {
    load_cookies_with_storage(storage_mode, &KEYRING_STORAGE, &CREDENTIALS_CACHE)
}

/// Save cookies based on storage mode（本番用ラッパー）
fn save_cookies(cookies: &YouTubeCookies, storage_mode: &StorageMode) -> Result<(), String> {
    save_cookies_with_storage(cookies, storage_mode, &KEYRING_STORAGE, &CREDENTIALS_CACHE)
}

/// Delete credentials based on storage mode（本番用ラッパー）
fn delete_credentials(storage_mode: &StorageMode) -> Result<(), String> {
    delete_credentials_with_storage(storage_mode, &KEYRING_STORAGE, &CREDENTIALS_CACHE)
}

// =============================================================================
// Session validity check
// =============================================================================

/// Check session validity by making a test request to YouTube API
async fn check_session_validity_internal(cookies: &YouTubeCookies) -> SessionValidity {
    use crate::core::api::build_auth_headers;
    use std::time::Duration;

    let checked_at = Utc::now().to_rfc3339();

    // Allow overriding the session check URL for E2E tests
    let session_check_url = std::env::var("LISCOV_SESSION_CHECK_URL")
        .unwrap_or_else(|_| "https://www.youtube.com/youtubei/v1/account/account_menu".to_string());

    log::info!(
        "🌐 Making session validity check request to: {}",
        session_check_url
    );

    // G4: API接続と同じCookie・認証ヘッダーを使用（build_auth_headersで統一）
    let auth_headers = build_auth_headers(cookies);

    // Make request to YouTube InnerTube API with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut request = client
        .post(&session_check_url)
        .header("Content-Type", "application/json")
        .body(r#"{"context":{"client":{"clientName":"WEB","clientVersion":"2.20231219.04.00"}}}"#);

    for (name, value) in &auth_headers {
        request = request.header(name.as_str(), value.as_str());
    }

    let result = request.send().await;

    match result {
        Ok(response) => {
            let status = response.status();
            log::info!("🌐 YouTube API response: {}", status);
            if status.is_success() {
                SessionValidity {
                    is_valid: true,
                    checked_at,
                    error: None,
                }
            } else if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                SessionValidity {
                    is_valid: false,
                    checked_at,
                    error: Some(format!("Authentication failed: {}", status)),
                }
            } else {
                SessionValidity {
                    is_valid: false,
                    checked_at,
                    error: Some(format!("Unexpected status: {}", status)),
                }
            }
        }
        Err(e) => {
            log::error!("🌐 YouTube API error: {}", e);
            SessionValidity {
                is_valid: false,
                checked_at,
                error: Some(format!("Network error: {}", e)),
            }
        }
    }
}

// =============================================================================
// 純粋関数（テスト可能なロジック）
// =============================================================================

/// Raw cookie文字列のバリデーション（純粋関数）
pub(crate) fn validate_raw_cookies(raw_cookies: &str) -> Result<(), CommandError> {
    if raw_cookies.is_empty() {
        return Err(CommandError::InvalidInput(
            "Cookie string is empty".to_string(),
        ));
    }
    if !raw_cookies.contains("SAPISID=") {
        return Err(CommandError::InvalidInput(
            "Cookie string must contain SAPISID".to_string(),
        ));
    }
    Ok(())
}

/// ストレージモードとcredentials有無からAuthStatusを構築する純粋関数
pub(crate) fn build_auth_status(
    storage_mode: &StorageMode,
    secure_storage_available: bool,
    has_credentials: bool,
) -> AuthStatus {
    let storage_type = match storage_mode {
        StorageMode::Secure => StorageType::Secure,
        StorageMode::Fallback => StorageType::Fallback,
    };

    let storage_error = (*storage_mode == StorageMode::Secure && !secure_storage_available)
        .then(|| "Secure storage is not available".to_string());

    AuthStatus {
        is_authenticated: has_credentials,
        has_saved_credentials: has_credentials,
        storage_type,
        storage_error,
    }
}

// =============================================================================
// Tauri Commands
// =============================================================================

/// Get authentication status
#[tauri::command]
pub async fn auth_get_status(
    _state: State<'_, AppState>,
    config_state: State<'_, ConfigState>,
) -> Result<AuthStatus, CommandError> {
    log::info!("🔍 auth_get_status called");
    let config = config_state.get();
    let storage_mode = &config.storage.mode;
    log::info!("📦 Storage mode: {:?}", storage_mode);

    let secure_storage_available = is_secure_storage_available();
    let has_credentials = load_cookies(storage_mode).is_ok();

    let status = build_auth_status(storage_mode, secure_storage_available, has_credentials);

    log::info!(
        "🔐 Auth status: is_authenticated={}, has_saved={}, storage_error={:?}",
        status.is_authenticated,
        status.has_saved_credentials,
        status.storage_error
    );

    Ok(status)
}

/// Load credentials from storage
#[tauri::command]
pub async fn auth_load_credentials(
    config_state: State<'_, ConfigState>,
) -> Result<bool, CommandError> {
    let config = config_state.get();
    let storage_mode = &config.storage.mode;

    match load_cookies(storage_mode) {
        Ok(_) => {
            log::info!("Credentials loaded successfully");
            Ok(true)
        }
        Err(e) => {
            log::warn!("Failed to load credentials: {}", e);
            Err(CommandError::AuthRequired(e))
        }
    }
}

/// Save credentials (from raw cookie string)
#[tauri::command]
pub async fn auth_save_raw_cookies(
    raw_cookies: String,
    config_state: State<'_, ConfigState>,
) -> Result<(), CommandError> {
    validate_raw_cookies(&raw_cookies)?;

    let cookies = parse_raw_cookies(&raw_cookies);
    let config = config_state.get();
    save_cookies(&cookies, &config.storage.mode).map_err(CommandError::StorageError)?;

    log::info!("Credentials saved from raw cookies");
    Ok(())
}

/// Save credentials (from individual values)
#[tauri::command]
pub async fn auth_save_credentials(
    sid: String,
    hsid: String,
    ssid: String,
    apisid: String,
    sapisid: String,
    config_state: State<'_, ConfigState>,
) -> Result<(), CommandError> {
    if sapisid.is_empty() {
        return Err(CommandError::InvalidInput(
            "SAPISID is required".to_string(),
        ));
    }

    let cookies = YouTubeCookies {
        sid,
        hsid,
        ssid,
        apisid,
        sapisid,
        raw_cookie_string: None,
    };

    let config = config_state.get();
    save_cookies(&cookies, &config.storage.mode).map_err(CommandError::StorageError)?;

    log::info!("Credentials saved");
    Ok(())
}

/// Delete saved credentials
#[tauri::command]
pub async fn auth_delete_credentials(
    config_state: State<'_, ConfigState>,
) -> Result<(), CommandError> {
    let config = config_state.get();
    delete_credentials(&config.storage.mode).map_err(CommandError::StorageError)?;
    log::info!("Credentials deleted");
    Ok(())
}

/// Clear WebView cookies (logout from YouTube)
#[tauri::command]
pub async fn auth_clear_webview_cookies(app: tauri::AppHandle) -> Result<(), CommandError> {
    use tauri::Manager;

    log::info!("🧹 Clearing WebView cookies...");

    // メインのWebViewウィンドウを取得
    if let Some(window) = app.get_webview_window("main") {
        // ブラウジングデータ（Cookieを含む）をすべてクリア
        window
            .clear_all_browsing_data()
            .map_err(|e| CommandError::Internal(format!("Failed to clear browsing data: {}", e)))?;
        log::info!("✅ WebView cookies cleared successfully");
    } else {
        log::warn!("⚠️ Main window not found, skipping WebView cookie clear");
    }

    Ok(())
}

/// Validate current credentials (local check only)
#[tauri::command]
pub async fn auth_validate_credentials(
    config_state: State<'_, ConfigState>,
) -> Result<bool, CommandError> {
    let config = config_state.get();
    let cookies = load_cookies(&config.storage.mode).map_err(CommandError::AuthRequired)?;

    // SAPISIDが存在し空でないことをチェック
    Ok(!cookies.sapisid.is_empty())
}

/// Check session validity by testing with YouTube API
#[tauri::command]
pub async fn auth_check_session_validity(
    config_state: State<'_, ConfigState>,
) -> Result<SessionValidity, CommandError> {
    log::info!("🔍 auth_check_session_validity called");
    let config = config_state.get();
    let cookies = load_cookies(&config.storage.mode).map_err(CommandError::AuthRequired)?;
    log::info!("🔍 Checking session validity...");

    let result = check_session_validity_internal(&cookies).await;
    log::info!(
        "🔍 Session validity result: is_valid={}, error={:?}",
        result.is_valid,
        result.error
    );
    Ok(result)
}

/// Switch to fallback storage mode
#[tauri::command]
pub async fn auth_use_fallback_storage(
    config_state: State<'_, ConfigState>,
) -> Result<bool, CommandError> {
    let config = config_state.get();

    // セキュアストレージからの移行対象クレデンシャルを確認
    let credentials_to_migrate = if config.storage.mode == StorageMode::Secure {
        load_cookies_from_secure_storage().ok()
    } else {
        None
    };

    // フォールバックモードに切り替えて保存（他の項目は今の設定のまま。09_config.md）
    let updated = config_state.update(|current| {
        let mut next = current.clone();
        next.storage.mode = StorageMode::Fallback;
        Ok(next)
    })?;
    if let Err(e) = updated.saved {
        log::error!("Failed to save config: {}", e);
    }

    // クレデンシャルが存在する場合は移行
    if let Some(cookies) = credentials_to_migrate {
        if let Err(e) = save_cookies_to_file(&cookies) {
            log::error!("Failed to migrate credentials to file: {}", e);
            return Err(CommandError::StorageError(format!(
                "Failed to migrate credentials: {}",
                e
            )));
        }
        log::info!("Credentials migrated to file storage");
    }

    log::info!("Switched to fallback storage mode");
    Ok(true)
}

/// Open authentication window (WebView-based login)
#[tauri::command]
pub async fn auth_open_window(
    app: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
) -> Result<(), CommandError> {
    match auth_window::open_auth_window(app).await {
        Ok(cookies) => {
            log::info!("Authentication successful via WebView");

            // 現在のストレージモードでCookieを保存
            let config = config_state.get();
            save_cookies(&cookies, &config.storage.mode).map_err(CommandError::StorageError)?;

            // Windows資格情報マネージャーへの永続化を待機
            // 参照: https://docs.rs/keyring/latest/x86_64-pc-windows-msvc/keyring/windows/index.html
            // "setting a password on one thread and then immediately spawning another
            // to get the password may return a NoEntry error"
            log::info!("⏳ Waiting 500ms for credential persistence...");
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            log::info!("⏳ Wait complete");

            log::info!("Credentials saved after WebView authentication");
            Ok(())
        }
        Err(e) => {
            log::warn!("Authentication failed: {}", e);
            Err(CommandError::AuthFailed(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn parse_raw_cookies_preserves_raw_string() {
        let raw = "SID=s; HSID=h; SSID=ss; APISID=a; SAPISID=sa; __Secure-1PSID=sec1; YSC=ysc";
        let cookies = parse_raw_cookies(raw);
        assert_eq!(cookies.raw_cookie_string.as_deref(), Some(raw));
    }

    #[test]
    fn parse_raw_cookies_extracts_five_fields() {
        let raw = "SID=sid_val; HSID=hsid_val; SSID=ssid_val; APISID=apisid_val; SAPISID=sapisid_val; OTHER=x";
        let cookies = parse_raw_cookies(raw);
        assert_eq!(cookies.sid, "sid_val");
        assert_eq!(cookies.hsid, "hsid_val");
        assert_eq!(cookies.ssid, "ssid_val");
        assert_eq!(cookies.apisid, "apisid_val");
        assert_eq!(cookies.sapisid, "sapisid_val");
    }

    #[test]
    fn parse_raw_cookies_missing_field_returns_empty_string() {
        let raw = "SID=s; SAPISID=sa";
        let cookies = parse_raw_cookies(raw);
        assert_eq!(cookies.hsid, "");
        assert_eq!(cookies.ssid, "");
        assert_eq!(cookies.apisid, "");
    }

    #[test]
    fn credentials_json_roundtrip_with_raw_cookie_string() {
        let cookies = YouTubeCookies {
            sid: "s".to_string(),
            hsid: "h".to_string(),
            ssid: "ss".to_string(),
            apisid: "a".to_string(),
            sapisid: "sa".to_string(),
            raw_cookie_string: Some("SID=s; __Secure-1PSID=sec1".to_string()),
        };
        let json: CredentialsJson = (&cookies).into();
        let serialized = serde_json::to_string(&json).unwrap();
        let deserialized: CredentialsJson = serde_json::from_str(&serialized).unwrap();
        let restored: YouTubeCookies = deserialized.into();
        assert_eq!(restored.raw_cookie_string, cookies.raw_cookie_string);
        assert_eq!(restored.sapisid, cookies.sapisid);
    }

    #[test]
    fn credentials_json_backwards_compatible_without_raw() {
        // raw_cookie_string が無い古い形式のJSONからもデシリアライズできる
        let json_str = r#"{"sid":"s","hsid":"h","ssid":"ss","apisid":"a","sapisid":"sa"}"#;
        let cred: CredentialsJson = serde_json::from_str(json_str).unwrap();
        let cookies: YouTubeCookies = cred.into();
        assert!(cookies.raw_cookie_string.is_none());
        assert_eq!(cookies.sapisid, "sa");
    }

    #[test]
    fn youtube_cookies_config_roundtrip_with_raw_cookies() {
        let cookies = YouTubeCookies {
            sid: "s".to_string(),
            hsid: "h".to_string(),
            ssid: "ss".to_string(),
            apisid: "a".to_string(),
            sapisid: "sa".to_string(),
            raw_cookie_string: Some("SID=s; YSC=ysc; __Secure-1PSID=sec1".to_string()),
        };
        let config: YouTubeCookiesConfig = (&cookies).into();
        assert_eq!(config.raw_cookies, cookies.raw_cookie_string);

        let restored: YouTubeCookies = config.into();
        assert_eq!(restored.raw_cookie_string, cookies.raw_cookie_string);
        assert_eq!(restored.to_cookie_string(), cookies.to_cookie_string());
    }

    #[test]
    fn credentials_config_toml_roundtrip_with_raw_cookies() {
        let cookies = YouTubeCookies {
            sid: "s".to_string(),
            hsid: "h".to_string(),
            ssid: "ss".to_string(),
            apisid: "a".to_string(),
            sapisid: "sa".to_string(),
            raw_cookie_string: Some("SID=s; SAPISID=sa; __Secure-1PSID=sec1".to_string()),
        };
        let config = CredentialsConfig {
            youtube: (&cookies).into(),
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let restored: CredentialsConfig = toml::from_str(&toml_str).unwrap();
        let restored_cookies: YouTubeCookies = restored.youtube.into();
        assert_eq!(
            restored_cookies.raw_cookie_string,
            cookies.raw_cookie_string
        );
    }

    // =========================================================================
    // validate_raw_cookies テスト
    // =========================================================================

    #[test]
    fn validate_raw_cookies_empty_string_returns_error() {
        let result = validate_raw_cookies("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_raw_cookies_without_sapisid_returns_error() {
        let result = validate_raw_cookies("SID=xxx; HSID=yyy");
        assert!(result.is_err());
    }

    #[test]
    fn validate_raw_cookies_with_sapisid_returns_ok() {
        let result = validate_raw_cookies("SID=xxx; SAPISID=yyy; HSID=zzz");
        assert!(result.is_ok());
    }

    // =========================================================================
    // build_auth_status テスト
    // =========================================================================

    #[test]
    fn build_auth_status_secure_available_with_credentials() {
        let status = build_auth_status(&StorageMode::Secure, true, true);
        assert!(status.is_authenticated);
        assert!(status.has_saved_credentials);
        assert_eq!(status.storage_type, StorageType::Secure);
        assert!(status.storage_error.is_none());
    }

    #[test]
    fn build_auth_status_secure_unavailable_without_credentials() {
        let status = build_auth_status(&StorageMode::Secure, false, false);
        assert!(!status.is_authenticated);
        assert!(!status.has_saved_credentials);
        assert_eq!(status.storage_type, StorageType::Secure);
        assert!(status.storage_error.is_some());
    }

    #[test]
    fn build_auth_status_fallback_with_credentials() {
        let status = build_auth_status(&StorageMode::Fallback, false, true);
        assert!(status.is_authenticated);
        assert!(status.has_saved_credentials);
        assert_eq!(status.storage_type, StorageType::Fallback);
        assert!(status.storage_error.is_none());
    }

    #[test]
    fn build_auth_status_fallback_without_credentials() {
        let status = build_auth_status(&StorageMode::Fallback, false, false);
        assert!(!status.is_authenticated);
        assert!(!status.has_saved_credentials);
        assert_eq!(status.storage_type, StorageType::Fallback);
        assert!(status.storage_error.is_none());
    }

    // =========================================================================
    // InMemoryStorage（テスト用 CredentialStorage 実装）
    // =========================================================================

    /// テスト用のインメモリストレージ実装
    struct InMemoryStorage {
        data: std::sync::RwLock<Option<YouTubeCookies>>,
        available: bool,
    }

    impl InMemoryStorage {
        /// 初期データありで作成
        fn with_data(cookies: YouTubeCookies) -> Self {
            Self {
                data: std::sync::RwLock::new(Some(cookies)),
                available: true,
            }
        }

        /// 空（データなし）で作成
        fn empty() -> Self {
            Self {
                data: std::sync::RwLock::new(None),
                available: true,
            }
        }

        /// ストレージに保存されているデータを取得（テストアサーション用）
        fn get_stored(&self) -> Option<YouTubeCookies> {
            self.data.read().unwrap().clone()
        }
    }

    impl CredentialStorage for InMemoryStorage {
        fn load(&self) -> Result<YouTubeCookies, String> {
            self.data
                .read()
                .unwrap()
                .clone()
                .ok_or_else(|| "No credentials found in secure storage".to_string())
        }

        fn save(&self, cookies: &YouTubeCookies) -> Result<(), String> {
            *self.data.write().unwrap() = Some(cookies.clone());
            Ok(())
        }

        fn delete(&self) -> Result<(), String> {
            *self.data.write().unwrap() = None;
            Ok(())
        }

        fn is_available(&self) -> bool {
            self.available
        }
    }

    /// load()呼び出し回数を追跡できるストレージ（振る舞い検証用）
    struct CountingStorage {
        inner: InMemoryStorage,
        load_calls: std::sync::atomic::AtomicU32,
    }

    impl CountingStorage {
        fn new(data: Option<YouTubeCookies>) -> Self {
            let inner = match data {
                Some(cookies) => InMemoryStorage::with_data(cookies),
                None => InMemoryStorage::empty(),
            };
            Self {
                inner,
                load_calls: std::sync::atomic::AtomicU32::new(0),
            }
        }

        fn load_count(&self) -> u32 {
            self.load_calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    impl CredentialStorage for CountingStorage {
        fn load(&self) -> Result<YouTubeCookies, String> {
            self.load_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.inner.load()
        }

        fn save(&self, cookies: &YouTubeCookies) -> Result<(), String> {
            self.inner.save(cookies)
        }

        fn delete(&self) -> Result<(), String> {
            self.inner.delete()
        }

        fn is_available(&self) -> bool {
            self.inner.is_available()
        }
    }

    /// テスト用のサンプルCookieを作成するヘルパー
    fn sample_cookies() -> YouTubeCookies {
        YouTubeCookies {
            sid: "test_sid".to_string(),
            hsid: "test_hsid".to_string(),
            ssid: "test_ssid".to_string(),
            apisid: "test_apisid".to_string(),
            sapisid: "test_sapisid".to_string(),
            raw_cookie_string: None,
        }
    }

    // =========================================================================
    // load_cookies_with_storage テスト
    // =========================================================================

    #[test]
    fn load_cookies_with_storage_secure_mode_returns_stored_cookies() {
        // Secureモード + ストレージにデータあり → cookies返却
        let cookies = sample_cookies();
        let storage = InMemoryStorage::with_data(cookies.clone());
        let cache = RwLock::new(None);

        let result = load_cookies_with_storage(&StorageMode::Secure, &storage, &cache);
        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.sapisid, cookies.sapisid);
        assert_eq!(loaded.sid, cookies.sid);
    }

    #[test]
    #[serial(liscov_env)]
    fn load_cookies_with_storage_secure_mode_no_entry() {
        // Secureモード + ストレージにデータなし + ファイルもなし → Err
        let storage = InMemoryStorage::empty();
        let cache = RwLock::new(None);

        let result = load_cookies_with_storage(&StorageMode::Secure, &storage, &cache);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No credentials found"));
    }

    #[test]
    #[serial(liscov_env)]
    fn load_cookies_with_storage_fallback_mode_ignores_secure_storage() {
        // Fallbackモード → セキュアストレージにデータがあっても使用せずファイルパスに委譲する
        // ここではFallbackモード時にsecure_storage.load()が呼ばれない（= ファイルパス経由）ことを
        // load呼び出し回数の追跡で検証する。
        // load()呼び出しを追跡するストレージ
        let storage = CountingStorage::new(Some(sample_cookies()));
        let cache = RwLock::new(None);

        let _result = load_cookies_with_storage(&StorageMode::Fallback, &storage, &cache);

        // Fallbackモードではsecure_storage.load()は呼ばれない
        assert_eq!(
            storage.load_count(),
            0,
            "Fallbackモードではsecure_storage.load()は呼ばれない"
        );
    }

    // =========================================================================
    // save_cookies_with_storage テスト
    // =========================================================================

    #[test]
    fn save_cookies_with_storage_secure_mode() {
        // Secureモード → InMemoryStorage に保存確認
        let cookies = sample_cookies();
        let storage = InMemoryStorage::empty();
        let cache = RwLock::new(None);

        let result = save_cookies_with_storage(&cookies, &StorageMode::Secure, &storage, &cache);
        assert!(result.is_ok());

        // ストレージに保存されていることを確認
        let stored = storage.get_stored();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().sapisid, cookies.sapisid);
    }

    /// Fallbackモードテストが実ファイルシステムに書き込むため、
    /// panic時にもクリーンアップを保証するRAIIガード
    struct CredentialsFileGuard;
    impl Drop for CredentialsFileGuard {
        fn drop(&mut self) {
            let _ = delete_credentials_file();
        }
    }

    #[test]
    #[serial(liscov_env)]
    fn save_cookies_with_storage_fallback_mode() {
        // Fallbackモード → ファイルパスに委譲
        // ファイルI/Oが発生するため、ストレージには委譲されないことを確認
        let _guard = CredentialsFileGuard;
        let cookies = sample_cookies();
        let storage = InMemoryStorage::empty();
        let cache = RwLock::new(None);

        // Fallbackモードではsecure_storageは使用されない
        // ファイルパスへの保存が試みられる（パスが存在しない環境ではエラーになりうる）
        let _result = save_cookies_with_storage(&cookies, &StorageMode::Fallback, &storage, &cache);

        // InMemoryStorageには保存されないことを確認（Fallbackモードはファイルに委譲）
        assert!(storage.get_stored().is_none());
    }

    // =========================================================================
    // delete_credentials_with_storage テスト
    // =========================================================================

    #[test]
    #[serial(liscov_env)]
    fn delete_credentials_with_storage_secure_mode() {
        // Secureモード → InMemoryStorage から削除確認
        let cookies = sample_cookies();
        let storage = InMemoryStorage::with_data(cookies);
        let cache = RwLock::new(None);

        // 削除前にデータがあることを確認
        assert!(storage.get_stored().is_some());

        let result = delete_credentials_with_storage(&StorageMode::Secure, &storage, &cache);
        assert!(result.is_ok());

        // ストレージからデータが削除されていることを確認
        assert!(storage.get_stored().is_none());
    }

    #[test]
    #[serial(liscov_env)]
    fn delete_credentials_with_storage_clears_cache() {
        // 削除時にインメモリキャッシュもクリアされることを確認
        let cookies = sample_cookies();
        // キャッシュにデータをセット
        let cache = RwLock::new(Some(cookies.clone()));
        let storage = InMemoryStorage::with_data(cookies);

        let result = delete_credentials_with_storage(&StorageMode::Secure, &storage, &cache);
        assert!(result.is_ok());

        // キャッシュがクリアされていることを確認
        assert!(cache.read().unwrap().is_none());
    }

    // =========================================================================
    // キャッシュパラメータ化の振る舞い検証テスト
    // =========================================================================

    #[test]
    fn load_cookies_with_storage_cache_hit() {
        // キャッシュにデータあり → storage.load() を呼ばない
        let cookies = sample_cookies();
        let cache = RwLock::new(Some(cookies.clone()));
        // CountingStorage: load() が呼ばれたかを追跡
        let storage = CountingStorage::new(None);

        let result = load_cookies_with_storage(&StorageMode::Secure, &storage, &cache);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().sapisid, cookies.sapisid);
        // キャッシュヒットのためstorage.load()は呼ばれない
        assert_eq!(
            storage.load_count(),
            0,
            "キャッシュヒット時はstorage.load()を呼ばない"
        );
    }

    #[test]
    fn load_cookies_with_storage_cache_miss_updates_cache() {
        // キャッシュ空 → storage.load()成功後にキャッシュが更新される
        let cookies = sample_cookies();
        let cache = RwLock::new(None);
        let storage = InMemoryStorage::with_data(cookies.clone());

        let result = load_cookies_with_storage(&StorageMode::Secure, &storage, &cache);

        assert!(result.is_ok());
        // ロード後にキャッシュが更新されていることを確認
        let cached = cache.read().unwrap().clone();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().sapisid, cookies.sapisid);
    }

    #[test]
    fn save_cookies_with_storage_updates_cache() {
        // save後にキャッシュが更新される
        let cookies = sample_cookies();
        let cache = RwLock::new(None);
        let storage = InMemoryStorage::empty();

        let result = save_cookies_with_storage(&cookies, &StorageMode::Secure, &storage, &cache);

        assert!(result.is_ok());
        // save後にキャッシュが更新されていることを確認
        let cached = cache.read().unwrap().clone();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().sapisid, cookies.sapisid);
    }

    #[test]
    fn delete_credentials_with_storage_clears_cache_verified() {
        // delete後にキャッシュがNoneになる
        let cookies = sample_cookies();
        let cache = RwLock::new(Some(cookies.clone()));
        let storage = InMemoryStorage::with_data(cookies);

        // delete前はキャッシュにデータがある
        assert!(cache.read().unwrap().is_some());

        let result = delete_credentials_with_storage(&StorageMode::Secure, &storage, &cache);

        assert!(result.is_ok());
        // delete後にキャッシュがNoneになっていることを確認
        assert!(cache.read().unwrap().is_none());
    }

    // =========================================================================
    // secure モードの保存先（01_auth.md「secure モードの保存先」「1 件に入らないときの分割」）
    // =========================================================================

    /// keyring の 1 件ずつを模したテスト用の格納先。keyring 3.6.3 の Windows 実装と同じ上限で断る
    #[derive(Default)]
    struct InMemorySlots {
        entries: std::sync::RwLock<std::collections::BTreeMap<String, String>>,
    }

    impl InMemorySlots {
        fn users(&self) -> Vec<String> {
            self.entries.read().unwrap().keys().cloned().collect()
        }

        fn value(&self, user: &str) -> Option<String> {
            self.entries.read().unwrap().get(user).cloned()
        }

        fn remove_entry(&self, user: &str) {
            self.entries.write().unwrap().remove(user);
        }

        /// 本体の件の見出し {"parts":N} の N
        fn header_parts(&self) -> usize {
            let header: PartsHeader =
                serde_json::from_str(&self.value(KEYRING_USER).unwrap()).unwrap();
            header.parts
        }
    }

    impl SecretSlots for InMemorySlots {
        fn get(&self, user: &str) -> Result<Option<String>, String> {
            Ok(self.value(user))
        }

        fn set(&self, user: &str, secret: &str) -> Result<(), String> {
            if secret.encode_utf16().count() > KEYRING_MAX_UTF16_UNITS {
                return Err(
                    "Attribute 'password' is longer than platform limit of 2560 chars".to_string(),
                );
            }
            self.entries
                .write()
                .unwrap()
                .insert(user.to_string(), secret.to_string());
            Ok(())
        }

        fn remove(&self, user: &str) -> Result<(), String> {
            self.remove_entry(user);
            Ok(())
        }
    }

    /// 保存がいつも容量超過で失敗するセキュアストレージ（分割前の版の keyring 相当）
    struct TooLongStorage;

    impl CredentialStorage for TooLongStorage {
        fn load(&self) -> Result<YouTubeCookies, String> {
            Err("No credentials found in secure storage".to_string())
        }

        fn save(&self, _cookies: &YouTubeCookies) -> Result<(), String> {
            Err("Failed to save to secure storage: Attribute 'password' is longer than platform limit of 2560 chars".to_string())
        }

        fn delete(&self) -> Result<(), String> {
            Ok(())
        }

        fn is_available(&self) -> bool {
            true
        }
    }

    /// WebView ログイン相当の大きい Cookie（raw_cookie_string が約 len 文字）
    fn large_cookies(len: usize) -> YouTubeCookies {
        let mut raw = "SID=test_sid; HSID=test_hsid; SSID=test_ssid; APISID=test_apisid; SAPISID=test_sapisid".to_string();
        let mut i = 0;
        while raw.len() < len {
            raw.push_str(&format!("; __Secure-C{}={}", i, "x".repeat(40)));
            i += 1;
        }
        parse_raw_cookies(&raw)
    }

    /// テストの前後で credentials.toml を消す（liscov-test-unit の名前空間）
    fn remove_credentials_file_for_test() -> CredentialsFileGuard {
        let _ = delete_credentials_file();
        CredentialsFileGuard
    }

    #[test]
    fn keyring_storage_saves_small_credentials_as_single_entry() {
        // 1280 単位以下 → youtube_credentials に JSON をそのまま保存（分割前の版と同じ形）
        let storage = KeyringStorage(InMemorySlots::default());

        storage.save(&sample_cookies()).unwrap();

        assert_eq!(storage.0.users(), vec![KEYRING_USER.to_string()]);
        let json: CredentialsJson =
            serde_json::from_str(&storage.0.value(KEYRING_USER).unwrap()).unwrap();
        assert_eq!(json.sapisid, "test_sapisid");
    }

    #[test]
    fn keyring_storage_splits_3000_char_cookie_and_reads_it_back() {
        // 例: 3000 文字の Cookie → .1〜.N に分け、本体の件は見出し {"parts":N}
        let cookies = large_cookies(3000);
        let storage = KeyringStorage(InMemorySlots::default());

        storage.save(&cookies).unwrap();

        let parts = storage.0.header_parts();
        assert!(parts >= 3, "3000 文字超の JSON は 3 件以上: {}", parts);
        let mut expected_users: Vec<String> = std::iter::once(KEYRING_USER.to_string())
            .chain((1..=parts).map(|i| format!("{}.{}", KEYRING_USER, i)))
            .collect();
        expected_users.sort();
        assert_eq!(storage.0.users(), expected_users);
        assert!((1..=parts).all(|i| {
            let part = storage.0.value(&format!("{}.{}", KEYRING_USER, i)).unwrap();
            part.encode_utf16().count() <= KEYRING_MAX_UTF16_UNITS
        }));

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.raw_cookie_string, cookies.raw_cookie_string);
        assert_eq!(loaded.sapisid, "test_sapisid");
    }

    #[test]
    fn keyring_storage_removes_leftover_parts_when_new_value_is_shorter() {
        // 保存: 前回の保存の余った部分（.N+1 以降）を消す
        let storage = KeyringStorage(InMemorySlots::default());
        storage.save(&large_cookies(5000)).unwrap();
        storage.save(&large_cookies(2000)).unwrap();

        assert_eq!(storage.0.users().len(), storage.0.header_parts() + 1);
        assert_eq!(
            storage.load().unwrap().raw_cookie_string,
            large_cookies(2000).raw_cookie_string
        );

        storage.save(&sample_cookies()).unwrap();
        assert_eq!(storage.0.users(), vec![KEYRING_USER.to_string()]);
    }

    #[test]
    fn keyring_storage_delete_removes_all_parts() {
        // 削除: youtube_credentials と .1 から順に見つからなくなるまでの部分を消す
        let storage = KeyringStorage(InMemorySlots::default());
        storage.save(&large_cookies(3000)).unwrap();

        storage.delete().unwrap();

        assert!(storage.0.users().is_empty());
        assert!(storage.load().unwrap_err().contains("No credentials found"));
    }

    #[test]
    fn keyring_storage_missing_part_is_not_reported_as_no_entry() {
        // 読み込み: 部分が欠けていたら読み込み失敗（「認証情報なし」とは区別する）
        let storage = KeyringStorage(InMemorySlots::default());
        storage.save(&large_cookies(3000)).unwrap();
        storage.0.remove_entry(&format!("{}.2", KEYRING_USER));

        let err = storage.load().unwrap_err();
        assert!(!err.contains("No credentials found"), "{}", err);
    }

    #[test]
    fn split_secret_keeps_chars_whole_within_limit() {
        // 文字の境目で分ける（UTF-16 で 2 単位の文字を途中で切らない）
        let secret = "ab🎵cd🎵🎵e";
        let parts = split_secret(secret, 3);

        assert!(parts.iter().all(|p| p.encode_utf16().count() <= 3));
        assert_eq!(parts.concat(), secret);
    }

    #[test]
    #[serial(liscov_env)]
    fn secure_mode_webview_sized_login_is_saved_without_file() {
        // 例: WebView ログインの 3000 文字の Cookie → セキュアストレージに保存し、ファイルには書かない
        let _guard = remove_credentials_file_for_test();
        let cookies = large_cookies(3000);
        let storage = KeyringStorage(InMemorySlots::default());
        let cache = RwLock::new(None);

        save_cookies_with_storage(&cookies, &StorageMode::Secure, &storage, &cache).unwrap();

        assert!(!credentials_file_exists());
        let reloaded =
            load_cookies_with_storage(&StorageMode::Secure, &storage, &RwLock::new(None)).unwrap();
        assert_eq!(reloaded.raw_cookie_string, cookies.raw_cookie_string);
    }

    #[test]
    #[serial(liscov_env)]
    fn secure_mode_save_failure_does_not_write_file() {
        // セキュアストレージへの保存に失敗（容量超過も含む）→ エラーを返し、ファイルには書かない
        let _guard = remove_credentials_file_for_test();
        let cache = RwLock::new(None);

        let result = save_cookies_with_storage(
            &large_cookies(3000),
            &StorageMode::Secure,
            &TooLongStorage,
            &cache,
        );

        assert!(result.is_err());
        assert!(!credentials_file_exists());
    }

    #[test]
    #[serial(liscov_env)]
    fn secure_mode_load_removes_leftover_file_and_uses_keyring_value() {
        // セキュアストレージから読めたときに credentials.toml が残っている → 削除し、セキュアストレージの値を使う
        let _guard = remove_credentials_file_for_test();
        save_cookies_to_file(&large_cookies(3000)).unwrap();
        let storage = InMemoryStorage::with_data(sample_cookies());
        let cache = RwLock::new(None);

        let loaded = load_cookies_with_storage(&StorageMode::Secure, &storage, &cache).unwrap();

        assert_eq!(loaded.raw_cookie_string, None);
        assert_eq!(loaded.sapisid, "test_sapisid");
        assert!(!credentials_file_exists());
    }

    #[test]
    #[serial(liscov_env)]
    fn secure_mode_save_removes_leftover_file() {
        // セキュアストレージに保存できたときに credentials.toml が残っている → 削除する
        let _guard = remove_credentials_file_for_test();
        save_cookies_to_file(&large_cookies(3000)).unwrap();
        let storage = InMemoryStorage::empty();
        let cache = RwLock::new(None);

        save_cookies_with_storage(&sample_cookies(), &StorageMode::Secure, &storage, &cache)
            .unwrap();

        assert!(!credentials_file_exists());
    }
}

// =============================================================================
