//! Configuration commands
//!
//! Implements 09_config.md specification

use crate::errors::CommandError;
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

/// 指定パスから設定を読み込む純粋関数。ファイル不在・パースエラー時はデフォルト値を返す。
fn load_config_from_path(path: &std::path::Path) -> Config {
    if !path.exists() {
        log::info!("Config file not found, using defaults");
        return Config::default();
    }

    let content = match fs::read_to_string(path) {
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

/// 指定パスへ設定を書き込む純粋関数。親ディレクトリが存在しない場合は自動作成する。
fn save_config_to_path(path: &std::path::Path, config: &Config) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(path, toml_string)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    log::info!("Config saved to {:?}", path);
    Ok(())
}

/// Load config from file
fn load_config_from_file() -> Config {
    match get_config_path() {
        Ok(p) => load_config_from_path(&p),
        Err(e) => {
            log::warn!("Failed to get config path: {}", e);
            Config::default()
        }
    }
}

/// Save config to file
pub fn save_config_to_file(config: &Config) -> Result<(), String> {
    let path = get_config_path()?;
    save_config_to_path(&path, config)
}

/// Load configuration
#[tauri::command]
pub async fn config_load(state: State<'_, ConfigState>) -> Result<Config, CommandError> {
    let config = load_config_from_file();
    state.set(config.clone());
    Ok(config)
}

/// Save configuration
#[tauri::command]
pub async fn config_save(config: Config, state: State<'_, ConfigState>) -> Result<(), CommandError> {
    state.set(config.clone());
    save_config_to_file(&config)
        .map_err(|e| CommandError::IoError(e))
}

/// Config構造体から section/key で値を取得する純粋関数
pub(crate) fn config_lookup(config: &Config, section: &str, key: &str) -> Option<Value> {
    match section {
        "storage" => match key {
            "mode" => Some(serde_json::to_value(&config.storage.mode).unwrap()),
            _ => None,
        },
        "chat_display" => match key {
            "message_font_size" => Some(serde_json::to_value(config.chat_display.message_font_size).unwrap()),
            "show_timestamps" => Some(serde_json::to_value(config.chat_display.show_timestamps).unwrap()),
            "auto_scroll_enabled" => Some(serde_json::to_value(config.chat_display.auto_scroll_enabled).unwrap()),
            _ => None,
        },
        "ui" => match key {
            "theme" => Some(serde_json::to_value(&config.ui.theme).unwrap()),
            _ => None,
        },
        _ => None,
    }
}

/// Get a specific configuration value
#[tauri::command]
pub async fn config_get_value(
    section: String,
    key: String,
    state: State<'_, ConfigState>,
) -> Result<Option<Value>, CommandError> {
    let config = state.get();
    Ok(config_lookup(&config, &section, &key))
}

/// Config構造体に section/key/value を適用する純粋関数
/// バリデーション込み。成功時は更新後のConfigを返す。
pub(crate) fn config_apply_value(
    config: &Config,
    section: &str,
    key: &str,
    value: Value,
) -> Result<Config, CommandError> {
    let mut new_config = config.clone();

    match section {
        "storage" => match key {
            "mode" => {
                new_config.storage.mode = serde_json::from_value(value)
                    .map_err(|e| CommandError::InvalidInput(format!("Invalid storage mode value: {}", e)))?;
            }
            _ => return Err(CommandError::InvalidInput(format!("Unknown key in storage section: {}", key))),
        },
        "chat_display" => match key {
            "message_font_size" => {
                let size: u32 = serde_json::from_value(value)
                    .map_err(|e| CommandError::InvalidInput(format!("Invalid font size value: {}", e)))?;
                // 有効範囲チェック (10-24)
                if !(10..=24).contains(&size) {
                    return Err(CommandError::InvalidInput(format!("Font size must be between 10 and 24, got {}", size)));
                }
                new_config.chat_display.message_font_size = size;
            }
            "show_timestamps" => {
                new_config.chat_display.show_timestamps = serde_json::from_value(value)
                    .map_err(|e| CommandError::InvalidInput(format!("Invalid show_timestamps value: {}", e)))?;
            }
            "auto_scroll_enabled" => {
                new_config.chat_display.auto_scroll_enabled = serde_json::from_value(value)
                    .map_err(|e| CommandError::InvalidInput(format!("Invalid auto_scroll_enabled value: {}", e)))?;
            }
            _ => return Err(CommandError::InvalidInput(format!("Unknown key in chat_display section: {}", key))),
        },
        "ui" => match key {
            "theme" => {
                new_config.ui.theme = serde_json::from_value(value)
                    .map_err(|e| CommandError::InvalidInput(format!("Invalid theme value: {}", e)))?;
            }
            _ => return Err(CommandError::InvalidInput(format!("Unknown key in ui section: {}", key))),
        },
        _ => return Err(CommandError::InvalidInput(format!("Unknown section: {}", section))),
    }

    Ok(new_config)
}

/// Set a specific configuration value and save
#[tauri::command]
pub async fn config_set_value(
    section: String,
    key: String,
    value: Value,
    state: State<'_, ConfigState>,
) -> Result<(), CommandError> {
    let config = state.get();
    let new_config = config_apply_value(&config, &section, &key, value)?;

    state.set(new_config.clone());

    // ファイル保存を試行。失敗してもメモリ上の変更は維持
    if let Err(e) = save_config_to_file(&new_config) {
        log::error!("Failed to save config: {}", e);
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

    // ========================================================================
    // config_lookup (09_config.md: 個別値取得)
    // ========================================================================

    #[test]
    fn config_lookup_storage_mode_default() {
        let config = Config::default();
        let val = config_lookup(&config, "storage", "mode");
        assert_eq!(val, Some(serde_json::json!("secure")));
    }

    #[test]
    fn config_lookup_chat_display_message_font_size_default() {
        let config = Config::default();
        let val = config_lookup(&config, "chat_display", "message_font_size");
        assert_eq!(val, Some(serde_json::json!(13)));
    }

    #[test]
    fn config_lookup_chat_display_show_timestamps_default() {
        let config = Config::default();
        let val = config_lookup(&config, "chat_display", "show_timestamps");
        assert_eq!(val, Some(serde_json::json!(true)));
    }

    #[test]
    fn config_lookup_chat_display_auto_scroll_enabled_default() {
        let config = Config::default();
        let val = config_lookup(&config, "chat_display", "auto_scroll_enabled");
        assert_eq!(val, Some(serde_json::json!(true)));
    }

    #[test]
    fn config_lookup_ui_theme_default() {
        let config = Config::default();
        let val = config_lookup(&config, "ui", "theme");
        assert_eq!(val, Some(serde_json::json!("dark")));
    }

    #[test]
    fn config_lookup_unknown_section_returns_none() {
        let config = Config::default();
        assert_eq!(config_lookup(&config, "nonexistent", "key"), None);
    }

    #[test]
    fn config_lookup_unknown_key_returns_none() {
        let config = Config::default();
        assert_eq!(config_lookup(&config, "storage", "nonexistent"), None);
        assert_eq!(config_lookup(&config, "chat_display", "nonexistent"), None);
        assert_eq!(config_lookup(&config, "ui", "nonexistent"), None);
    }

    // ========================================================================
    // config_apply_value (09_config.md: 個別値設定・バリデーション)
    // ========================================================================

    #[test]
    fn config_apply_value_storage_mode_fallback() {
        let config = Config::default();
        let result = config_apply_value(&config, "storage", "mode", serde_json::json!("fallback"));
        let new_config = result.unwrap();
        assert_eq!(new_config.storage.mode, StorageMode::Fallback);
    }

    #[test]
    fn config_apply_value_font_size_valid() {
        let config = Config::default();
        let new_config = config_apply_value(
            &config, "chat_display", "message_font_size", serde_json::json!(20),
        ).unwrap();
        assert_eq!(new_config.chat_display.message_font_size, 20);
    }

    #[test]
    fn config_apply_value_font_size_too_small() {
        let config = Config::default();
        let result = config_apply_value(
            &config, "chat_display", "message_font_size", serde_json::json!(9),
        );
        assert!(result.is_err());
    }

    #[test]
    fn config_apply_value_font_size_too_large() {
        let config = Config::default();
        let result = config_apply_value(
            &config, "chat_display", "message_font_size", serde_json::json!(25),
        );
        assert!(result.is_err());
    }

    #[test]
    fn config_apply_value_show_timestamps_false() {
        let config = Config::default();
        let new_config = config_apply_value(
            &config, "chat_display", "show_timestamps", serde_json::json!(false),
        ).unwrap();
        assert!(!new_config.chat_display.show_timestamps);
    }

    #[test]
    fn config_apply_value_auto_scroll_enabled_false() {
        let config = Config::default();
        let new_config = config_apply_value(
            &config, "chat_display", "auto_scroll_enabled", serde_json::json!(false),
        ).unwrap();
        assert!(!new_config.chat_display.auto_scroll_enabled);
    }

    #[test]
    fn config_apply_value_ui_theme_light() {
        let config = Config::default();
        let new_config = config_apply_value(
            &config, "ui", "theme", serde_json::json!("light"),
        ).unwrap();
        assert_eq!(new_config.ui.theme, Theme::Light);
    }

    #[test]
    fn config_apply_value_unknown_section_error() {
        let config = Config::default();
        let result = config_apply_value(
            &config, "nonexistent", "key", serde_json::json!("value"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn config_apply_value_unknown_key_error() {
        let config = Config::default();
        assert!(config_apply_value(
            &config, "storage", "nonexistent", serde_json::json!("value"),
        ).is_err());
        assert!(config_apply_value(
            &config, "chat_display", "nonexistent", serde_json::json!("value"),
        ).is_err());
        assert!(config_apply_value(
            &config, "ui", "nonexistent", serde_json::json!("value"),
        ).is_err());
    }

    #[test]
    fn config_apply_value_does_not_mutate_original() {
        // 元のConfigが変更されないことを確認（immutability）
        let config = Config::default();
        let _ = config_apply_value(
            &config, "chat_display", "message_font_size", serde_json::json!(20),
        ).unwrap();
        assert_eq!(config.chat_display.message_font_size, 13);
    }

    // ========================================================================
    // load_config_from_path / save_config_to_path (09_config.md: ファイルI/O)
    // ========================================================================

    /// テスト用一時ディレクトリパスを生成する（std::env::temp_dir() + UUID風サフィックス）
    fn temp_config_path(suffix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("liscov_test_config_{}.toml", suffix))
    }

    #[test]
    fn load_config_from_path_file_not_exists_returns_default() {
        // 存在しないパスを渡すとデフォルト値が返る
        let path = temp_config_path("not_exists_abc123");
        // 万一残っていたら削除
        let _ = fs::remove_file(&path);

        let config = load_config_from_path(&path);
        assert_eq!(config.storage.mode, StorageMode::Secure);
        assert_eq!(config.chat_display.message_font_size, 13);
        assert_eq!(config.ui.theme, Theme::Dark);
    }

    #[test]
    fn load_config_from_path_valid_toml_returns_parsed() {
        // 有効なTOMLファイルを書き込んで読み込めることを確認
        let path = temp_config_path("valid_toml_def456");
        let toml = r#"
[storage]
mode = "fallback"

[chat_display]
message_font_size = 18
show_timestamps = false
auto_scroll_enabled = false

[ui]
theme = "light"
"#;
        fs::write(&path, toml).unwrap();

        let config = load_config_from_path(&path);
        assert_eq!(config.storage.mode, StorageMode::Fallback);
        assert_eq!(config.chat_display.message_font_size, 18);
        assert!(!config.chat_display.show_timestamps);
        assert!(!config.chat_display.auto_scroll_enabled);
        assert_eq!(config.ui.theme, Theme::Light);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_config_from_path_invalid_toml_returns_default() {
        // 不正なTOMLはデフォルト値にフォールバックする
        let path = temp_config_path("invalid_toml_ghi789");
        fs::write(&path, "this is not valid toml ][[[").unwrap();

        let config = load_config_from_path(&path);
        assert_eq!(config.storage.mode, StorageMode::Secure);
        assert_eq!(config.chat_display.message_font_size, 13);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_config_from_path_partial_keys_fills_defaults() {
        // 一部キーのみのTOML → 不足分はデフォルト値で補完する
        let path = temp_config_path("partial_keys_jkl012");
        let toml = r#"
[storage]
mode = "fallback"
"#;
        fs::write(&path, toml).unwrap();

        let config = load_config_from_path(&path);
        assert_eq!(config.storage.mode, StorageMode::Fallback);
        assert_eq!(config.chat_display.message_font_size, 13);
        assert!(config.chat_display.show_timestamps);
        assert_eq!(config.ui.theme, Theme::Dark);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn save_config_to_path_creates_parent_and_writes() {
        // 親ディレクトリが存在しない場合でも自動作成してファイルを書き込める
        let parent = std::env::temp_dir().join("liscov_test_nested_mno345");
        let path = parent.join("config.toml");
        // 念のためディレクトリを削除しておく
        let _ = fs::remove_dir_all(&parent);

        let config = Config::default();
        save_config_to_path(&path, &config).unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("mode"));

        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn save_load_config_roundtrip() {
        // save → load でフィールド値が保持される（往復一貫性）
        let path = temp_config_path("roundtrip_pqr678");

        let mut config = Config::default();
        config.storage.mode = StorageMode::Fallback;
        config.chat_display.message_font_size = 20;
        config.chat_display.show_timestamps = false;
        config.ui.theme = Theme::Light;

        save_config_to_path(&path, &config).unwrap();
        let loaded = load_config_from_path(&path);

        assert_eq!(loaded.storage.mode, StorageMode::Fallback);
        assert_eq!(loaded.chat_display.message_font_size, 20);
        assert!(!loaded.chat_display.show_timestamps);
        assert_eq!(loaded.ui.theme, Theme::Light);

        let _ = fs::remove_file(&path);
    }
}
