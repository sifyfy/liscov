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

/// Validate file path for security (spec: 05_raw_response.md パス検証)
fn validate_file_path(file_path: &str) -> Result<(), String> {
    // Null文字
    if file_path.contains('\0') {
        return Err("Path contains null character".to_string());
    }

    // ディレクトリトラバーサル
    if file_path.contains("../") || file_path.contains("..\\") {
        return Err("Directory traversal not allowed".to_string());
    }

    // Windows危険文字 (ファイル名部分のみチェック)
    let dangerous_chars = ['<', '>', '"', '|', '?', '*'];
    if file_path.chars().any(|c| dangerous_chars.contains(&c)) {
        return Err("Path contains dangerous characters".to_string());
    }

    // パス長超過
    if file_path.len() > 4096 {
        return Err("Path exceeds maximum length (4096)".to_string());
    }

    // システムディレクトリ
    let lower = file_path.to_lowercase().replace('/', "\\");
    if lower.starts_with("c:\\windows") || lower.starts_with("c:\\program files") {
        return Err("System directory not allowed".to_string());
    }

    Ok(())
}

/// Get resolved file path (resolves relative paths to data directory)
/// (spec: 05_raw_response.md)
#[tauri::command]
pub fn raw_response_resolve_path(file_path: String) -> Result<String, String> {
    use std::path::Path;

    validate_file_path(&file_path)?;

    if Path::new(&file_path).is_absolute() {
        Ok(file_path)
    } else {
        // 相対パスの場合はアプリデータディレクトリを基準に解決する
        match crate::paths::data_dir() {
            Ok(data_dir) => {
                std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
                Ok(data_dir.join(&file_path).to_string_lossy().to_string())
            }
            Err(_) => Ok(file_path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_directory_traversal() {
        assert!(validate_file_path("../etc/passwd").is_err());
        assert!(validate_file_path("..\\secret").is_err());
    }

    #[test]
    fn validate_rejects_null_char() {
        assert!(validate_file_path("file\0.ndjson").is_err());
    }

    #[test]
    fn validate_rejects_dangerous_chars() {
        assert!(validate_file_path("file<>.ndjson").is_err());
        assert!(validate_file_path("file|name").is_err());
    }

    #[test]
    fn validate_rejects_system_dirs() {
        assert!(validate_file_path("C:\\Windows\\test.ndjson").is_err());
        assert!(validate_file_path("C:\\Program Files\\test.ndjson").is_err());
    }

    #[test]
    fn validate_rejects_long_paths() {
        let long_path = "a".repeat(4097);
        assert!(validate_file_path(&long_path).is_err());
    }

    #[test]
    fn validate_accepts_normal_paths() {
        assert!(validate_file_path("raw_responses.ndjson").is_ok());
        assert!(validate_file_path("D:\\data\\responses.ndjson").is_ok());
    }
}
