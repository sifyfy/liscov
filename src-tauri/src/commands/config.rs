//! Configuration commands
//!
//! Implements 09_config.md specification

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use tauri::State;

/// Storage mode for credentials
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageMode {
    Secure,
    Fallback,
}

impl Default for StorageMode {
    fn default() -> Self {
        StorageMode::Secure
    }
}

/// Storage configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default)]
    pub mode: StorageMode,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            mode: StorageMode::Secure,
        }
    }
}

/// Chat display configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDisplayConfig {
    #[serde(default = "default_font_size")]
    pub message_font_size: u32,
    #[serde(default = "default_true")]
    pub show_timestamps: bool,
    #[serde(default = "default_true")]
    pub auto_scroll_enabled: bool,
}

fn default_font_size() -> u32 {
    13
}

fn default_true() -> bool {
    true
}

impl Default for ChatDisplayConfig {
    fn default() -> Self {
        Self {
            message_font_size: 13,
            show_timestamps: true,
            auto_scroll_enabled: true,
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub chat_display: ChatDisplayConfig,
}

/// Configuration state for managing in-memory config
pub struct ConfigState {
    config: RwLock<Config>,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            config: RwLock::new(Config::default()),
        }
    }
}

impl ConfigState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a clone of the current config
    pub fn get(&self) -> Config {
        self.config.read().unwrap().clone()
    }

    /// Update the config
    pub fn set(&self, config: Config) {
        *self.config.write().unwrap() = config;
    }
}

/// Get the app name for directory paths (can be overridden via LISCOV_APP_NAME env var for testing)
fn get_app_name() -> String {
    std::env::var("LISCOV_APP_NAME").unwrap_or_else(|_| "liscov".to_string())
}

/// Get config file path
fn get_config_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Failed to determine config directory".to_string())?;
    Ok(config_dir.join(get_app_name()).join("config.toml"))
}

/// Load config from file
fn load_config_from_file() -> Config {
    let path = match get_config_path() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("Failed to get config path: {}", e);
            return Config::default();
        }
    };

    if !path.exists() {
        log::info!("Config file not found, using defaults");
        return Config::default();
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read config file: {}", e);
            return Config::default();
        }
    };

    match toml::from_str(&content) {
        Ok(config) => {
            log::info!("Config loaded from {:?}", path);
            config
        }
        Err(e) => {
            log::warn!("Failed to parse config file: {}", e);
            Config::default()
        }
    }
}

/// Save config to file
pub fn save_config_to_file(config: &Config) -> Result<(), String> {
    let path = get_config_path()?;

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&path, toml_string)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    log::info!("Config saved to {:?}", path);
    Ok(())
}

/// Load configuration
#[tauri::command]
pub async fn config_load(state: State<'_, ConfigState>) -> Result<Config, String> {
    let config = load_config_from_file();
    state.set(config.clone());
    Ok(config)
}

/// Save configuration
#[tauri::command]
pub async fn config_save(config: Config, state: State<'_, ConfigState>) -> Result<(), String> {
    state.set(config.clone());
    save_config_to_file(&config)
}

/// Get a specific configuration value
#[tauri::command]
pub async fn config_get_value(
    section: String,
    key: String,
    state: State<'_, ConfigState>,
) -> Result<Option<Value>, String> {
    let config = state.get();

    let value = match section.as_str() {
        "storage" => match key.as_str() {
            "mode" => Some(serde_json::to_value(&config.storage.mode).unwrap()),
            _ => None,
        },
        "chat_display" => match key.as_str() {
            "message_font_size" => Some(serde_json::to_value(config.chat_display.message_font_size).unwrap()),
            "show_timestamps" => Some(serde_json::to_value(config.chat_display.show_timestamps).unwrap()),
            "auto_scroll_enabled" => Some(serde_json::to_value(config.chat_display.auto_scroll_enabled).unwrap()),
            _ => None,
        },
        _ => None,
    };

    Ok(value)
}

/// Set a specific configuration value and save
#[tauri::command]
pub async fn config_set_value(
    section: String,
    key: String,
    value: Value,
    state: State<'_, ConfigState>,
) -> Result<(), String> {
    let mut config = state.get();

    match section.as_str() {
        "storage" => match key.as_str() {
            "mode" => {
                config.storage.mode = serde_json::from_value(value)
                    .map_err(|e| format!("Invalid storage mode value: {}", e))?;
            }
            _ => return Err(format!("Unknown key in storage section: {}", key)),
        },
        "chat_display" => match key.as_str() {
            "message_font_size" => {
                let size: u32 = serde_json::from_value(value)
                    .map_err(|e| format!("Invalid font size value: {}", e))?;
                // Validate range (10-24)
                if size < 10 || size > 24 {
                    return Err(format!("Font size must be between 10 and 24, got {}", size));
                }
                config.chat_display.message_font_size = size;
            }
            "show_timestamps" => {
                config.chat_display.show_timestamps = serde_json::from_value(value)
                    .map_err(|e| format!("Invalid show_timestamps value: {}", e))?;
            }
            "auto_scroll_enabled" => {
                config.chat_display.auto_scroll_enabled = serde_json::from_value(value)
                    .map_err(|e| format!("Invalid auto_scroll_enabled value: {}", e))?;
            }
            _ => return Err(format!("Unknown key in chat_display section: {}", key)),
        },
        _ => return Err(format!("Unknown section: {}", section)),
    }

    state.set(config.clone());

    // Save to file, but continue even if save fails
    if let Err(e) = save_config_to_file(&config) {
        log::error!("Failed to save config: {}", e);
        // Memory is updated, so we continue without error
    }

    Ok(())
}
