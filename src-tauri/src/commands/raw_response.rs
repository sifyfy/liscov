//! Raw response save configuration commands

use crate::core::raw_response::SaveConfig;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

/// Global save config state
pub struct SaveConfigState(pub Mutex<SaveConfig>);

impl Default for SaveConfigState {
    fn default() -> Self {
        Self(Mutex::new(SaveConfig::default()))
    }
}

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

/// Get current save config (spec: 05_raw_response.md)
#[tauri::command]
pub fn raw_response_get_config(state: State<'_, SaveConfigState>) -> Result<GuiSaveConfig, String> {
    let config = state.0.lock().map_err(|e| e.to_string())?;
    Ok(GuiSaveConfig::from(config.clone()))
}

/// Update save config (spec: 05_raw_response.md)
#[tauri::command]
pub fn raw_response_update_config(
    state: State<'_, SaveConfigState>,
    config: GuiSaveConfig,
) -> Result<(), String> {
    let mut current = state.0.lock().map_err(|e| e.to_string())?;
    *current = SaveConfig::from(config);
    tracing::info!("💾 Save config updated: enabled={}", current.enabled);
    Ok(())
}

/// Get resolved file path (resolves relative paths to data directory)
/// (spec: 05_raw_response.md)
#[tauri::command]
pub fn raw_response_resolve_path(file_path: String) -> Result<String, String> {
    use std::path::Path;

    if Path::new(&file_path).is_absolute() {
        Ok(file_path)
    } else {
        // Use app data directory
        if let Some(proj_dirs) = directories::ProjectDirs::from("dev", "sifyfy", "liscov") {
            let data_dir = proj_dirs.data_dir();
            std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
            Ok(data_dir.join(&file_path).to_string_lossy().to_string())
        } else {
            Ok(file_path)
        }
    }
}
