//! Authentication commands
//!
//! Implements 01_auth.md specification

use crate::commands::auth_window;
use crate::commands::config::{ConfigState, StorageMode};
use crate::core::models::YouTubeCookies;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::State;

const KEYRING_SERVICE: &str = "liscov";
const KEYRING_USER: &str = "youtube_credentials";

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
}

impl From<&YouTubeCookies> for CredentialsJson {
    fn from(cookies: &YouTubeCookies) -> Self {
        Self {
            sid: cookies.sid.clone(),
            hsid: cookies.hsid.clone(),
            ssid: cookies.ssid.clone(),
            apisid: cookies.apisid.clone(),
            sapisid: cookies.sapisid.clone(),
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
            raw_cookies: None,
        }
    }
}

/// Get credentials file path (for fallback mode)
fn get_credentials_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Failed to determine config directory".to_string())?;
    Ok(config_dir.join("liscov").join("credentials.toml"))
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
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Failed to access secure storage: {}", e))?;

    let secret = entry.get_password()
        .map_err(|e| match e {
            keyring::Error::NoEntry => "No credentials found in secure storage".to_string(),
            _ => format!("Failed to read from secure storage: {}", e),
        })?;

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
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Failed to access secure storage: {}", e))?;

    let json: CredentialsJson = cookies.into();
    let secret = serde_json::to_string(&json)
        .map_err(|e| format!("Failed to serialize credentials: {}", e))?;

    entry.set_password(&secret)
        .map_err(|e| format!("Failed to save to secure storage: {}", e))?;

    log::info!("Credentials saved to secure storage");
    Ok(())
}

/// Delete credentials from secure storage
fn delete_from_secure_storage() -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
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
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
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
fn load_cookies(storage_mode: &StorageMode) -> Result<YouTubeCookies, String> {
    match storage_mode {
        StorageMode::Fallback => load_cookies_from_file(),
        StorageMode::Secure => {
            // Try secure storage first
            match load_cookies_from_secure_storage() {
                Ok(cookies) => Ok(cookies),
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
fn save_cookies(cookies: &YouTubeCookies, storage_mode: &StorageMode) -> Result<(), String> {
    match storage_mode {
        StorageMode::Fallback => save_cookies_to_file(cookies),
        StorageMode::Secure => save_cookies_to_secure_storage(cookies),
    }
}

/// Delete credentials based on storage mode
fn delete_credentials(storage_mode: &StorageMode) -> Result<(), String> {
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

/// Generate SAPISIDHASH for authentication
fn generate_sapisidhash(sapisid: &str) -> String {
    use sha1::{Sha1, Digest};

    let timestamp = Utc::now().timestamp();
    let origin = "https://www.youtube.com";
    let input = format!("{} {} {}", timestamp, sapisid, origin);

    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let hash = hex::encode(hasher.finalize());

    format!("{}_{}", timestamp, hash)
}

/// Check session validity by making a test request to YouTube API
async fn check_session_validity_internal(cookies: &YouTubeCookies) -> SessionValidity {
    let checked_at = Utc::now().to_rfc3339();

    // Generate authentication header
    let sapisidhash = generate_sapisidhash(&cookies.sapisid);

    // Build cookie string
    let cookie_string = format!(
        "SID={}; HSID={}; SSID={}; APISID={}; SAPISID={}",
        cookies.sid, cookies.hsid, cookies.ssid, cookies.apisid, cookies.sapisid
    );

    // Make request to YouTube InnerTube API
    let client = reqwest::Client::new();
    let result = client
        .post("https://www.youtube.com/youtubei/v1/account/account_menu")
        .header("Authorization", format!("SAPISIDHASH {}", sapisidhash))
        .header("Cookie", cookie_string)
        .header("X-Origin", "https://www.youtube.com")
        .header("Origin", "https://www.youtube.com")
        .header("Content-Type", "application/json")
        .body(r#"{"context":{"client":{"clientName":"WEB","clientVersion":"2.20231219.04.00"}}}"#)
        .send()
        .await;

    match result {
        Ok(response) => {
            let status = response.status();
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
        Err(e) => SessionValidity {
            is_valid: false,
            checked_at,
            error: Some(format!("Network error: {}", e)),
        },
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
) -> Result<AuthStatus, String> {
    let config = config_state.get();
    let storage_mode = &config.storage.mode;

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
) -> Result<bool, String> {
    let config = config_state.get();
    let storage_mode = &config.storage.mode;

    match load_cookies(storage_mode) {
        Ok(_) => {
            log::info!("Credentials loaded successfully");
            Ok(true)
        }
        Err(e) => {
            log::warn!("Failed to load credentials: {}", e);
            Err(e)
        }
    }
}

/// Save credentials (from raw cookie string)
#[tauri::command]
pub async fn auth_save_raw_cookies(
    raw_cookies: String,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    if raw_cookies.is_empty() {
        return Err("Cookie string is empty".to_string());
    }

    if !raw_cookies.contains("SAPISID=") {
        return Err("Cookie string must contain SAPISID".to_string());
    }

    let cookies = parse_raw_cookies(&raw_cookies);
    let config = config_state.get();
    save_cookies(&cookies, &config.storage.mode)?;

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
) -> Result<(), String> {
    if sapisid.is_empty() {
        return Err("SAPISID is required".to_string());
    }

    let cookies = YouTubeCookies {
        sid,
        hsid,
        ssid,
        apisid,
        sapisid,
    };

    let config = config_state.get();
    save_cookies(&cookies, &config.storage.mode)?;

    log::info!("Credentials saved");
    Ok(())
}

/// Delete saved credentials
#[tauri::command]
pub async fn auth_delete_credentials(
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    let config = config_state.get();
    delete_credentials(&config.storage.mode)?;
    log::info!("Credentials deleted");
    Ok(())
}

/// Validate current credentials (local check only)
#[tauri::command]
pub async fn auth_validate_credentials(
    config_state: State<'_, ConfigState>,
) -> Result<bool, String> {
    let config = config_state.get();
    let cookies = load_cookies(&config.storage.mode)?;

    // Simple validation: check that SAPISID is present and non-empty
    Ok(!cookies.sapisid.is_empty())
}

/// Check session validity by testing with YouTube API
#[tauri::command]
pub async fn auth_check_session_validity(
    config_state: State<'_, ConfigState>,
) -> Result<SessionValidity, String> {
    let config = config_state.get();
    let cookies = load_cookies(&config.storage.mode)?;

    Ok(check_session_validity_internal(&cookies).await)
}

/// Switch to fallback storage mode
#[tauri::command]
pub async fn auth_use_fallback_storage(
    config_state: State<'_, ConfigState>,
) -> Result<bool, String> {
    let mut config = config_state.get();

    // Check if we have credentials in secure storage to migrate
    let credentials_to_migrate = if config.storage.mode == StorageMode::Secure {
        load_cookies_from_secure_storage().ok()
    } else {
        None
    };

    // Switch to fallback mode
    config.storage.mode = StorageMode::Fallback;
    config_state.set(config.clone());

    // Save the config
    use crate::commands::config::save_config_to_file;
    if let Err(e) = save_config_to_file(&config) {
        log::error!("Failed to save config: {}", e);
    }

    // Migrate credentials if they existed
    if let Some(cookies) = credentials_to_migrate {
        if let Err(e) = save_cookies_to_file(&cookies) {
            log::error!("Failed to migrate credentials to file: {}", e);
            return Err(format!("Failed to migrate credentials: {}", e));
        }
        log::info!("Credentials migrated to file storage");
    }

    log::info!("Switched to fallback storage mode");
    Ok(true)
}

/// Open authentication window (WebView-based login)
#[tauri::command]
pub async fn auth_open_window(app: tauri::AppHandle) -> Result<(), String> {
    match auth_window::open_auth_window(app).await {
        Ok(_cookies) => {
            log::info!("Authentication successful via WebView");
            Ok(())
        }
        Err(e) => {
            log::warn!("Authentication failed: {}", e);
            Err(e.to_string())
        }
    }
}

// =============================================================================
