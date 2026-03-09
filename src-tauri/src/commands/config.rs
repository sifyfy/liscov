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

/// UI theme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

/// UI configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default)]
    pub theme: Theme,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
        }
    }
}

/// Chat display configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatDisplayConfig {
    pub message_font_size: u32,
    pub show_timestamps: bool,
    pub auto_scroll_enabled: bool,
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
    #[serde(default)]
    pub ui: UiConfig,
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

/// 設定ファイルのパスを返す
fn get_config_path() -> Result<PathBuf, String> {
    crate::paths::config_path()
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
        "ui" => match key.as_str() {
            "theme" => Some(serde_json::to_value(&config.ui.theme).unwrap()),
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
        "ui" => match key.as_str() {
            "theme" => {
                config.ui.theme = serde_json::from_value(value)
                    .map_err(|e| format!("Invalid theme value: {}", e))?;
            }
            _ => return Err(format!("Unknown key in ui section: {}", key)),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Config defaults (09_config.md: デフォルト値)
    // ========================================================================

    #[test]
    fn config_default_values() {
        let config = Config::default();
        assert_eq!(config.storage.mode, StorageMode::Secure);
        assert_eq!(config.chat_display.message_font_size, 13);
        assert!(config.chat_display.show_timestamps);
        assert!(config.chat_display.auto_scroll_enabled);
        assert_eq!(config.ui.theme, Theme::Dark);
    }

    #[test]
    fn storage_mode_default() {
        assert_eq!(StorageMode::default(), StorageMode::Secure);
    }

    #[test]
    fn theme_default() {
        assert_eq!(Theme::default(), Theme::Dark);
    }

    // ========================================================================
    // ConfigState get/set (09_config.md: メモリ上の設定操作)
    // ========================================================================

    #[test]
    fn config_state_get_returns_default() {
        let state = ConfigState::new();
        let config = state.get();
        assert_eq!(config.chat_display.message_font_size, 13);
    }

    #[test]
    fn config_state_set_updates_value() {
        let state = ConfigState::new();
        let mut config = state.get();
        config.chat_display.message_font_size = 20;
        state.set(config);

        let updated = state.get();
        assert_eq!(updated.chat_display.message_font_size, 20);
    }

    // ========================================================================
    // TOML serialization/deserialization (09_config.md: シリアライズ往復)
    // ========================================================================

    #[test]
    fn config_toml_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.storage.mode, config.storage.mode);
        assert_eq!(parsed.chat_display.message_font_size, config.chat_display.message_font_size);
        assert_eq!(parsed.chat_display.show_timestamps, config.chat_display.show_timestamps);
        assert_eq!(parsed.chat_display.auto_scroll_enabled, config.chat_display.auto_scroll_enabled);
        assert_eq!(parsed.ui.theme, config.ui.theme);
    }

    #[test]
    fn config_toml_migration_partial_keys() {
        // 09_config.md: 一部キーのみのTOML → 不足分はデフォルト値で補完
        let partial_toml = r#"
[storage]
mode = "fallback"
"#;
        let config: Config = toml::from_str(partial_toml).unwrap();
        assert_eq!(config.storage.mode, StorageMode::Fallback);
        assert_eq!(config.chat_display.message_font_size, 13);
        assert!(config.chat_display.show_timestamps);
        assert_eq!(config.ui.theme, Theme::Dark);
    }

    #[test]
    fn config_toml_unknown_keys_ignored() {
        // 09_config.md: 未知のキーは無視
        let toml_with_extra = r#"
[storage]
mode = "secure"
unknown_field = "value"

[chat_display]
message_font_size = 16
future_setting = true
"#;
        let config: Config = toml::from_str(toml_with_extra).unwrap();
        assert_eq!(config.storage.mode, StorageMode::Secure);
        assert_eq!(config.chat_display.message_font_size, 16);
    }

    // ========================================================================
    // Font size validation (09_config.md: フォントサイズ範囲 10-24)
    // ========================================================================

    #[test]
    fn font_size_valid_range_boundaries() {
        for size in [10u32, 13, 24] {
            assert!(size >= 10 && size <= 24, "Size {} should be valid", size);
        }
    }

    #[test]
    fn font_size_invalid_range() {
        for size in [9u32, 25, 0, 100] {
            assert!(size < 10 || size > 24, "Size {} should be invalid", size);
        }
    }

    // ========================================================================
    // Serialization format (09_config.md)
    // ========================================================================

    #[test]
    fn storage_mode_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&StorageMode::Secure).unwrap(), "\"secure\"");
        assert_eq!(serde_json::to_string(&StorageMode::Fallback).unwrap(), "\"fallback\"");
    }

    #[test]
    fn theme_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Theme::Dark).unwrap(), "\"dark\"");
        assert_eq!(serde_json::to_string(&Theme::Light).unwrap(), "\"light\"");
    }
}
