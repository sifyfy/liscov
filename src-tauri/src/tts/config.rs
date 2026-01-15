//! TTS configuration

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// TTS backend type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TtsBackendType {
    #[default]
    None,
    Bouyomichan,
    Voicevox,
}

/// Bouyomichan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BouyomichanConfig {
    pub host: String,
    pub port: u16,
    pub voice: i32,
    pub volume: i32,
    pub speed: i32,
    pub tone: i32,
}

impl Default for BouyomichanConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 50080,
            voice: 0,
            volume: -1,
            speed: -1,
            tone: -1,
        }
    }
}

/// VOICEVOX configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicevoxConfig {
    pub host: String,
    pub port: u16,
    pub speaker_id: i32,
    pub volume_scale: f32,
    pub speed_scale: f32,
    pub pitch_scale: f32,
    pub intonation_scale: f32,
}

impl Default for VoicevoxConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 50021,
            speaker_id: 1,
            volume_scale: 1.0,
            speed_scale: 1.0,
            pitch_scale: 0.0,
            intonation_scale: 1.0,
        }
    }
}

/// TTS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub enabled: bool,
    pub backend: TtsBackendType,
    pub bouyomichan: BouyomichanConfig,
    pub voicevox: VoicevoxConfig,
    pub read_author_name: bool,
    pub add_honorific: bool,
    pub strip_at_prefix: bool,
    pub strip_handle_suffix: bool,
    pub read_superchat_amount: bool,
    pub max_text_length: usize,
    pub queue_size_limit: usize,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: TtsBackendType::None,
            bouyomichan: BouyomichanConfig::default(),
            voicevox: VoicevoxConfig::default(),
            read_author_name: true,
            add_honorific: true,
            strip_at_prefix: true,
            strip_handle_suffix: true,
            read_superchat_amount: true,
            max_text_length: 200,
            queue_size_limit: 50,
        }
    }
}

impl TtsConfig {
    /// Get the app name for directory paths (can be overridden via LISCOV_APP_NAME env var for testing)
    fn get_app_name() -> String {
        std::env::var("LISCOV_APP_NAME").unwrap_or_else(|_| "liscov-tauri".to_string())
    }

    /// Get the config file path
    fn config_path() -> Result<PathBuf, String> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Failed to determine config directory".to_string())?;
        Ok(config_dir.join(Self::get_app_name()).join("tts_config.toml"))
    }

    /// Load config from file, or return default if file doesn't exist
    pub fn load() -> Self {
        match Self::config_path() {
            Ok(path) => {
                if path.exists() {
                    match fs::read_to_string(&path) {
                        Ok(content) => {
                            match toml::from_str(&content) {
                                Ok(config) => {
                                    log::info!("TTS config loaded from {:?}", path);
                                    return config;
                                }
                                Err(e) => {
                                    log::warn!("Failed to parse TTS config: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to read TTS config: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to get config path: {}", e);
            }
        }
        Self::default()
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path()?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize TTS config: {}", e))?;

        fs::write(&path, toml_str)
            .map_err(|e| format!("Failed to write TTS config: {}", e))?;

        log::info!("TTS config saved to {:?}", path);
        Ok(())
    }
}
