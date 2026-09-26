//! Raw response save configuration commands
//!
//! 保存設定は ConfigState（config.toml の [raw_response]）が正本（05_raw_response.md）

use crate::commands::config::{Config, ConfigState, save_config_to_file};
use crate::core::raw_response::{SaveConfig, resolve_save_path, validate_file_path};
use crate::errors::CommandError;
use serde::{Deserialize, Serialize};
use tauri::State;

/// GUI-friendly save config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSaveConfig {
    pub enabled: bool,
    pub file_path: String,
    pub max_file_size_mb: u64,
    pub enable_rotation: bool,
    pub max_backup_files: u32,
}

impl From<SaveConfig> for GuiSaveConfig {
    fn from(config: SaveConfig) -> Self {
        Self {
            enabled: config.enabled,
            file_path: config.file_path,
            max_file_size_mb: config.max_file_size_mb,
            enable_rotation: config.enable_rotation,
            max_backup_files: config.max_backup_files,
        }
    }
}

impl From<GuiSaveConfig> for SaveConfig {
    fn from(config: GuiSaveConfig) -> Self {
        Self {
            enabled: config.enabled,
            file_path: config.file_path,
            max_file_size_mb: config.max_file_size_mb,
            enable_rotation: config.enable_rotation,
            max_backup_files: config.max_backup_files,
        }
    }
}

/// Config に保存設定を適用する純粋関数。パス検証に失敗したら Err を返す。
pub(crate) fn config_apply_raw_response(
    config: &Config,
    save_config: SaveConfig,
) -> Result<Config, CommandError> {
    validate_file_path(&save_config.file_path).map_err(CommandError::InvalidInput)?;
    Ok(Config {
        raw_response: save_config,
        ..config.clone()
    })
}

/// Get current save config (spec: 05_raw_response.md)
#[tauri::command]
pub fn raw_response_get_config(state: State<'_, ConfigState>) -> GuiSaveConfig {
    GuiSaveConfig::from(state.get().raw_response)
}

/// Update save config (spec: 05_raw_response.md)
#[tauri::command]
pub fn raw_response_update_config(
    state: State<'_, ConfigState>,
    config: GuiSaveConfig,
) -> Result<(), CommandError> {
    let new_config = config_apply_raw_response(&state.get(), SaveConfig::from(config))?;
    state.set(new_config.clone());
    tracing::info!(
        "💾 Save config updated: enabled={}",
        new_config.raw_response.enabled
    );

    // ファイル保存を試行。失敗してもメモリ上の変更は維持（09_config.md）
    if let Err(e) = save_config_to_file(&new_config) {
        tracing::error!("Failed to save config: {}", e);
    }
    Ok(())
}

/// Get resolved file path (resolves relative paths to data directory)
/// (spec: 05_raw_response.md)
#[tauri::command]
pub fn raw_response_resolve_path(file_path: String) -> Result<String, CommandError> {
    validate_file_path(&file_path).map_err(CommandError::InvalidInput)?;

    let Ok(data_dir) = crate::paths::data_dir() else {
        return Ok(file_path);
    };
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| CommandError::IoError(format!("Failed to create data dir: {}", e)))?;
    Ok(resolve_save_path(&file_path, &data_dir)
        .to_string_lossy()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: 05_raw_response.md - From<GuiSaveConfig> for SaveConfig は全フィールドを保持する
    #[test]
    fn from_gui_save_config_preserves_all_fields() {
        let gui = GuiSaveConfig {
            enabled: true,
            file_path: "custom.ndjson".to_string(),
            max_file_size_mb: 50,
            enable_rotation: false,
            max_backup_files: 10,
        };
        let config = SaveConfig::from(gui);
        assert!(config.enabled);
        assert_eq!(config.file_path, "custom.ndjson");
        assert_eq!(config.max_file_size_mb, 50);
        assert!(!config.enable_rotation);
        assert_eq!(config.max_backup_files, 10);
    }

    // spec: 05_raw_response.md 設定の変更 - 有効化した設定が Config に入る
    #[test]
    fn apply_raw_response_updates_config() {
        let save_config = SaveConfig {
            enabled: true,
            ..SaveConfig::default()
        };
        let config = config_apply_raw_response(&Config::default(), save_config).unwrap();
        assert!(config.raw_response.enabled);
    }

    // spec: 05_raw_response.md 設定の変更 - パス検証に失敗する file_path はエラー
    #[test]
    fn apply_raw_response_rejects_invalid_path() {
        let save_config = SaveConfig {
            enabled: true,
            file_path: "../secret.txt".to_string(),
            ..SaveConfig::default()
        };
        let result = config_apply_raw_response(&Config::default(), save_config);
        assert!(matches!(result, Err(CommandError::InvalidInput(_))));
    }
}
