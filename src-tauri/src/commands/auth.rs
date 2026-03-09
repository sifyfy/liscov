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

// keyring_service のデフォルト値は paths モジュールで管理
const KEYRING_USER: &str = "youtube_credentials";

/// In-memory cache for credentials to work around keyring issues on Windows
/// The keyring crate may fail to read credentials from a new Entry instance
/// even immediately after writing, despite verification succeeding within
/// the same Entry instance. This cache provides a reliable fallback.
static CREDENTIALS_CACHE: RwLock<Option<YouTubeCookies>> = RwLock::new(None);

/// Storage type for credentials
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    Secure,
    Fallback,
}

/// Authentication status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub has_saved_credentials: bool,
    pub storage_type: StorageType,
    pub storage_error: Option<String>,
}

/// Session validity result
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    get_credentials_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

// =============================================================================
// Secure Storage (keyring) operations
// =============================================================================

/// Load cookies from secure storage (keyring)
fn load_cookies_from_secure_storage() -> Result<YouTubeCookies, String> {
    log::info!("📂 Loading from secure storage...");
    let entry = keyring::Entry::new(&crate::paths::keyring_service(), KEYRING_USER)
        .map_err(|e| format!("Failed to access secure storage: {}", e))?;

    let secret = entry.get_password()
        .map_err(|e| {
            let msg = match e {
                keyring::Error::NoEntry => "No credentials found in secure storage".to_string(),
                _ => format!("Failed to read from secure storage: {}", e),
            };
            log::info!("📂 Load error: {}", msg);
            msg
        })?;
    log::info!("📂 Load success");

    let json: CredentialsJson = serde_json::from_str(&secret)
        .map_err(|e| format!("Failed to parse credentials: {}", e))?;

    let cookies: YouTubeCookies = json.into();

    if cookies.sapisid.is_empty() {
        return Err("Invalid credentials: SAPISID is missing".to_string());
    }

    Ok(cookies)
}

/// Save cookies to secure storage (keyring)
fn save_cookies_to_secure_storage(cookies: &YouTubeCookies) -> Result<(), String> {
    log::info!("📝 Saving to secure storage...");
    let entry = keyring::Entry::new(&crate::paths::keyring_service(), KEYRING_USER)
        .map_err(|e| format!("Failed to access secure storage: {}", e))?;

    let json: CredentialsJson = cookies.into();
    let secret = serde_json::to_string(&json)
        .map_err(|e| format!("Failed to serialize credentials: {}", e))?;

    log::debug!("Credential JSON length: {} chars", secret.len());

    entry.set_password(&secret)
        .map_err(|e| format!("Failed to save to secure storage: {}", e))?;

    log::info!("Credentials saved to secure storage");

    // Verify the save immediately
    match entry.get_password() {
        Ok(read_back) => {
            if read_back == secret {
                log::info!("✅ Verified: credentials can be read back");
            } else {
                log::warn!("⚠️ Mismatch: written and read data differ");
            }
        }
        Err(e) => {
            log::error!("❌ Verification failed: could not read back: {}", e);
            return Err(format!("Credentials saved but cannot be read back: {}", e));
        }
    }

    Ok(())
}

/// Delete credentials from secure storage
fn delete_from_secure_storage() -> Result<(), String> {
    let entry = keyring::Entry::new(&crate::paths::keyring_service(), KEYRING_USER)
        .map_err(|e| format!("Failed to access secure storage: {}", e))?;

    match entry.delete_credential() {
        Ok(_) => {
            log::info!("Credentials deleted from secure storage");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            log::info!("No credentials to delete from secure storage");
            Ok(())
        }
        Err(e) => Err(format!("Failed to delete from secure storage: {}", e)),
    }
}

/// Check if secure storage is available
fn is_secure_storage_available() -> bool {
    match keyring::Entry::new(&crate::paths::keyring_service(), KEYRING_USER) {
        Ok(entry) => {
            // Try to access the entry (read or write test)
            match entry.get_password() {
                Ok(_) => true,
                Err(keyring::Error::NoEntry) => true, // No entry is fine, storage is available
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
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

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read credentials file: {}", e))?;

    let config: CredentialsConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse credentials file: {}", e))?;

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

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config = CredentialsConfig {
        youtube: cookies.into(),
    };

    let toml_string = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize credentials: {}", e))?;

    fs::write(&path, toml_string)
        .map_err(|e| format!("Failed to write credentials file: {}", e))?;

    log::info!("Credentials saved to {:?}", path);
    Ok(())
}

/// Delete credentials file
fn delete_credentials_file() -> Result<(), String> {
    let path = get_credentials_path()?;

    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete credentials file: {}", e))?;
        log::info!("Credentials file deleted");
    }

    Ok(())
}

// =============================================================================
// Combined operations (based on storage mode)
// =============================================================================

/// Load cookies based on storage mode
pub(crate) fn load_cookies(storage_mode: &StorageMode) -> Result<YouTubeCookies, String> {
    // First, check in-memory cache (workaround for keyring Windows issues)
    if let Ok(cache) = CREDENTIALS_CACHE.read() {
        if let Some(ref cached_cookies) = *cache {
            log::info!("📦 Returning credentials from memory cache");
            return Ok(cached_cookies.clone());
        }
    }

    match storage_mode {
        StorageMode::Fallback => {
            let cookies = load_cookies_from_file()?;
            // Update cache
            if let Ok(mut cache) = CREDENTIALS_CACHE.write() {
                *cache = Some(cookies.clone());
            }
            Ok(cookies)
        }
        StorageMode::Secure => {
            // Try secure storage first
            match load_cookies_from_secure_storage() {
                Ok(cookies) => {
                    // Update cache
                    if let Ok(mut cache) = CREDENTIALS_CACHE.write() {
                        *cache = Some(cookies.clone());
                    }
                    Ok(cookies)
                }
                Err(e) => {
                    // Check if it's a "no entry" error vs storage failure
                    if e.contains("No credentials found") {
                        // Check for migration opportunity
                        if credentials_file_exists() {
                            log::info!("Migrating credentials from file to secure storage");
                            let cookies = load_cookies_from_file()?;
                            // Try to migrate
                            if save_cookies_to_secure_storage(&cookies).is_ok() {
                                // Successfully migrated, delete the file
                                let _ = delete_credentials_file();
                            }
                            // Update cache
                            if let Ok(mut cache) = CREDENTIALS_CACHE.write() {
                                *cache = Some(cookies.clone());
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

/// Save cookies based on storage mode
/// Secure modeでkeyringの容量制限に引っかかった場合、自動的にfile storageにフォールバックする
fn save_cookies(cookies: &YouTubeCookies, storage_mode: &StorageMode) -> Result<(), String> {
    // Always update in-memory cache first (workaround for keyring Windows issues)
    if let Ok(mut cache) = CREDENTIALS_CACHE.write() {
        *cache = Some(cookies.clone());
        log::info!("📦 Credentials cached in memory");
    }

    match storage_mode {
        StorageMode::Fallback => save_cookies_to_file(cookies),
        StorageMode::Secure => {
            match save_cookies_to_secure_storage(cookies) {
                Ok(()) => Ok(()),
                Err(e) if e.contains("platform limit") || e.contains("2560") => {
                    log::warn!("⚠️ Secure storage size limit exceeded, falling back to file storage: {}", e);
                    save_cookies_to_file(cookies)
                }
                Err(e) => Err(e),
            }
        }
    }
}

/// Delete credentials based on storage mode
fn delete_credentials(storage_mode: &StorageMode) -> Result<(), String> {
    // Clear in-memory cache
    if let Ok(mut cache) = CREDENTIALS_CACHE.write() {
        *cache = None;
        log::info!("📦 Credentials cache cleared");
    }

    match storage_mode {
        StorageMode::Fallback => delete_credentials_file(),
        StorageMode::Secure => {
            // Delete from both locations to be safe
            let _ = delete_from_secure_storage();
            let _ = delete_credentials_file();
            Ok(())
        }
    }
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

    log::info!("🌐 Making session validity check request to: {}", session_check_url);

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
            } else if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
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

    // Check for storage errors in secure mode
    let storage_error = if *storage_mode == StorageMode::Secure && !is_secure_storage_available() {
        Some("Secure storage is not available".to_string())
    } else {
        None
    };

    let storage_type = match storage_mode {
        StorageMode::Secure => StorageType::Secure,
        StorageMode::Fallback => StorageType::Fallback,
    };

    // Try to load credentials
    let credentials_result = load_cookies(storage_mode);
    let has_saved = credentials_result.is_ok();
    let is_authenticated = has_saved;

    log::info!(
        "🔐 Auth status: is_authenticated={}, has_saved={}, storage_error={:?}",
        is_authenticated,
        has_saved,
        storage_error
    );

    Ok(AuthStatus {
        is_authenticated,
        has_saved_credentials: has_saved,
        storage_type,
        storage_error,
    })
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
    if raw_cookies.is_empty() {
        return Err(CommandError::InvalidInput("Cookie string is empty".to_string()));
    }

    if !raw_cookies.contains("SAPISID=") {
        return Err(CommandError::InvalidInput("Cookie string must contain SAPISID".to_string()));
    }

    let cookies = parse_raw_cookies(&raw_cookies);
    let config = config_state.get();
    save_cookies(&cookies, &config.storage.mode)
        .map_err(|e| CommandError::StorageError(e))?;

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
        return Err(CommandError::InvalidInput("SAPISID is required".to_string()));
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
    save_cookies(&cookies, &config.storage.mode)
        .map_err(|e| CommandError::StorageError(e))?;

    log::info!("Credentials saved");
    Ok(())
}

/// Delete saved credentials
#[tauri::command]
pub async fn auth_delete_credentials(
    config_state: State<'_, ConfigState>,
) -> Result<(), CommandError> {
    let config = config_state.get();
    delete_credentials(&config.storage.mode)
        .map_err(|e| CommandError::StorageError(e))?;
    log::info!("Credentials deleted");
    Ok(())
}

/// Clear WebView cookies (logout from YouTube)
#[tauri::command]
pub async fn auth_clear_webview_cookies(
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
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
    let cookies = load_cookies(&config.storage.mode)
        .map_err(|e| CommandError::AuthRequired(e))?;

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
    let cookies = load_cookies(&config.storage.mode)
        .map_err(|e| CommandError::AuthRequired(e))?;
    log::info!("🔍 Checking session validity...");

    let result = check_session_validity_internal(&cookies).await;
    log::info!("🔍 Session validity result: is_valid={}, error={:?}", result.is_valid, result.error);
    Ok(result)
}

/// Switch to fallback storage mode
#[tauri::command]
pub async fn auth_use_fallback_storage(
    config_state: State<'_, ConfigState>,
) -> Result<bool, CommandError> {
    let mut config = config_state.get();

    // セキュアストレージからの移行対象クレデンシャルを確認
    let credentials_to_migrate = if config.storage.mode == StorageMode::Secure {
        load_cookies_from_secure_storage().ok()
    } else {
        None
    };

    // フォールバックモードに切り替え
    config.storage.mode = StorageMode::Fallback;
    config_state.set(config.clone());

    // 設定ファイルを保存
    use crate::commands::config::save_config_to_file;
    if let Err(e) = save_config_to_file(&config) {
        log::error!("Failed to save config: {}", e);
    }

    // クレデンシャルが存在する場合は移行
    if let Some(cookies) = credentials_to_migrate {
        if let Err(e) = save_cookies_to_file(&cookies) {
            log::error!("Failed to migrate credentials to file: {}", e);
            return Err(CommandError::StorageError(format!("Failed to migrate credentials: {}", e)));
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
            save_cookies(&cookies, &config.storage.mode)
                .map_err(|e| CommandError::StorageError(e))?;

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
        assert_eq!(restored_cookies.raw_cookie_string, cookies.raw_cookie_string);
    }
}

// =============================================================================
