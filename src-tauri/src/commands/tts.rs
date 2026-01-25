//! TTS (Text-to-Speech) commands

use crate::state::AppState;
use crate::tts::{TtsBackendType, TtsConfig, TtsProcessManager, TtsPriority, TtsQueueItem};
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

/// TTS configuration for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfigDto {
    pub enabled: bool,
    pub backend: String, // "none", "bouyomichan", "voicevox"
    pub read_author_name: bool,
    pub add_honorific: bool,
    pub strip_at_prefix: bool,
    pub strip_handle_suffix: bool,
    pub read_superchat_amount: bool,
    pub max_text_length: usize,
    pub queue_size_limit: usize,
    // Bouyomichan settings
    pub bouyomichan_host: String,
    pub bouyomichan_port: u16,
    pub bouyomichan_voice: i32,
    pub bouyomichan_volume: i32,
    pub bouyomichan_speed: i32,
    pub bouyomichan_tone: i32,
    pub bouyomichan_auto_launch: bool,
    pub bouyomichan_exe_path: Option<String>,
    pub bouyomichan_auto_close: bool,
    // VOICEVOX settings
    pub voicevox_host: String,
    pub voicevox_port: u16,
    pub voicevox_speaker_id: i32,
    pub voicevox_volume_scale: f32,
    pub voicevox_speed_scale: f32,
    pub voicevox_pitch_scale: f32,
    pub voicevox_intonation_scale: f32,
    pub voicevox_auto_launch: bool,
    pub voicevox_exe_path: Option<String>,
    pub voicevox_auto_close: bool,
}

impl From<TtsConfig> for TtsConfigDto {
    fn from(config: TtsConfig) -> Self {
        Self {
            enabled: config.enabled,
            backend: match config.backend {
                TtsBackendType::None => "none".to_string(),
                TtsBackendType::Bouyomichan => "bouyomichan".to_string(),
                TtsBackendType::Voicevox => "voicevox".to_string(),
            },
            read_author_name: config.read_author_name,
            add_honorific: config.add_honorific,
            strip_at_prefix: config.strip_at_prefix,
            strip_handle_suffix: config.strip_handle_suffix,
            read_superchat_amount: config.read_superchat_amount,
            max_text_length: config.max_text_length,
            queue_size_limit: config.queue_size_limit,
            bouyomichan_host: config.bouyomichan.host,
            bouyomichan_port: config.bouyomichan.port,
            bouyomichan_voice: config.bouyomichan.voice,
            bouyomichan_volume: config.bouyomichan.volume,
            bouyomichan_speed: config.bouyomichan.speed,
            bouyomichan_tone: config.bouyomichan.tone,
            bouyomichan_auto_launch: config.bouyomichan.auto_launch,
            bouyomichan_exe_path: config.bouyomichan.exe_path,
            bouyomichan_auto_close: config.bouyomichan.auto_close,
            voicevox_host: config.voicevox.host,
            voicevox_port: config.voicevox.port,
            voicevox_speaker_id: config.voicevox.speaker_id,
            voicevox_volume_scale: config.voicevox.volume_scale,
            voicevox_speed_scale: config.voicevox.speed_scale,
            voicevox_pitch_scale: config.voicevox.pitch_scale,
            voicevox_intonation_scale: config.voicevox.intonation_scale,
            voicevox_auto_launch: config.voicevox.auto_launch,
            voicevox_exe_path: config.voicevox.exe_path,
            voicevox_auto_close: config.voicevox.auto_close,
        }
    }
}

impl From<TtsConfigDto> for TtsConfig {
    fn from(dto: TtsConfigDto) -> Self {
        use crate::tts::{BouyomichanConfig, VoicevoxConfig};

        Self {
            enabled: dto.enabled,
            backend: match dto.backend.as_str() {
                "bouyomichan" => TtsBackendType::Bouyomichan,
                "voicevox" => TtsBackendType::Voicevox,
                _ => TtsBackendType::None,
            },
            bouyomichan: BouyomichanConfig {
                host: dto.bouyomichan_host,
                port: dto.bouyomichan_port,
                voice: dto.bouyomichan_voice,
                volume: dto.bouyomichan_volume,
                speed: dto.bouyomichan_speed,
                tone: dto.bouyomichan_tone,
                auto_launch: dto.bouyomichan_auto_launch,
                exe_path: dto.bouyomichan_exe_path,
                auto_close: dto.bouyomichan_auto_close,
            },
            voicevox: VoicevoxConfig {
                host: dto.voicevox_host,
                port: dto.voicevox_port,
                speaker_id: dto.voicevox_speaker_id,
                volume_scale: dto.voicevox_volume_scale,
                speed_scale: dto.voicevox_speed_scale,
                pitch_scale: dto.voicevox_pitch_scale,
                intonation_scale: dto.voicevox_intonation_scale,
                auto_launch: dto.voicevox_auto_launch,
                exe_path: dto.voicevox_exe_path,
                auto_close: dto.voicevox_auto_close,
            },
            read_author_name: dto.read_author_name,
            add_honorific: dto.add_honorific,
            strip_at_prefix: dto.strip_at_prefix,
            strip_handle_suffix: dto.strip_handle_suffix,
            read_superchat_amount: dto.read_superchat_amount,
            max_text_length: dto.max_text_length,
            queue_size_limit: dto.queue_size_limit,
        }
    }
}

impl Default for TtsConfigDto {
    fn default() -> Self {
        TtsConfig::default().into()
    }
}

/// TTS status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsStatus {
    pub is_processing: bool,
    pub queue_size: usize,
    pub backend_name: Option<String>,
}

/// Speak text using TTS
#[tauri::command]
pub async fn tts_speak(
    state: State<'_, AppState>,
    text: String,
    priority: Option<String>,
    author_name: Option<String>,
    amount: Option<String>,
) -> Result<(), String> {
    let priority = match priority.as_deref() {
        Some("superchat") => TtsPriority::SuperChat,
        Some("membership") => TtsPriority::Membership,
        _ => TtsPriority::Normal,
    };

    let item = TtsQueueItem {
        text,
        priority,
        author_name,
        amount,
    };

    state.tts_manager.enqueue(item).await;
    Ok(())
}

/// Speak text directly (bypasses queue)
#[tauri::command]
pub async fn tts_speak_direct(state: State<'_, AppState>, text: String) -> Result<(), String> {
    state
        .tts_manager
        .speak_direct(&text)
        .await
        .map_err(|e| e.to_string())
}

/// Update TTS configuration
#[tauri::command]
pub async fn tts_update_config(
    state: State<'_, AppState>,
    config: TtsConfigDto,
) -> Result<(), String> {
    let was_enabled = state.tts_manager.get_config().await.enabled;
    let will_be_enabled = config.enabled;

    state.tts_manager.update_config(config.into()).await;

    // Start/stop processing based on enabled state change
    if !was_enabled && will_be_enabled {
        state.tts_manager.start_processing().await;
    } else if was_enabled && !will_be_enabled {
        state.tts_manager.stop_processing().await;
    }

    Ok(())
}

/// Get current TTS configuration
#[tauri::command]
pub async fn tts_get_config(state: State<'_, AppState>) -> Result<TtsConfigDto, String> {
    let config = state.tts_manager.get_config().await;
    Ok(config.into())
}

/// Test TTS connection to backend
#[tauri::command]
pub async fn tts_test_connection(
    state: State<'_, AppState>,
    backend: Option<String>,
) -> Result<bool, String> {
    if let Some(backend_str) = backend {
        let backend_type = match backend_str.as_str() {
            "bouyomichan" => TtsBackendType::Bouyomichan,
            "voicevox" => TtsBackendType::Voicevox,
            _ => return Ok(false),
        };
        state
            .tts_manager
            .test_backend_connection(backend_type)
            .await
            .map_err(|e| e.to_string())
    } else {
        state
            .tts_manager
            .test_connection()
            .await
            .map_err(|e| e.to_string())
    }
}

/// Start TTS queue processing
#[tauri::command]
pub async fn tts_start(state: State<'_, AppState>) -> Result<(), String> {
    state.tts_manager.start_processing().await;
    Ok(())
}

/// Stop TTS queue processing
#[tauri::command]
pub async fn tts_stop(state: State<'_, AppState>) -> Result<(), String> {
    state.tts_manager.stop_processing().await;
    Ok(())
}

/// Clear TTS queue
#[tauri::command]
pub async fn tts_clear_queue(state: State<'_, AppState>) -> Result<(), String> {
    state.tts_manager.clear_queue().await;
    Ok(())
}

/// Get TTS status
#[tauri::command]
pub async fn tts_get_status(state: State<'_, AppState>) -> Result<TtsStatus, String> {
    Ok(TtsStatus {
        is_processing: state.tts_manager.is_processing().await,
        queue_size: state.tts_manager.queue_size().await,
        backend_name: state.tts_manager.backend_name().await.map(|s| s.to_string()),
    })
}

/// TTS backend launch status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsLaunchStatus {
    pub bouyomichan_launched: bool,
    pub voicevox_launched: bool,
}

/// Discover executable path for a TTS backend
#[tauri::command]
pub async fn tts_discover_exe(backend: String) -> Result<Option<String>, String> {
    let backend_type = match backend.as_str() {
        "bouyomichan" => TtsBackendType::Bouyomichan,
        "voicevox" => TtsBackendType::Voicevox,
        _ => return Ok(None),
    };
    Ok(TtsProcessManager::discover_exe(&backend_type))
}

/// Select executable file via dialog
#[tauri::command]
pub async fn tts_select_exe(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file = app
        .dialog()
        .file()
        .add_filter("実行ファイル", &["exe"])
        .blocking_pick_file();

    Ok(file.map(|f| f.to_string()))
}

/// Launch a TTS backend process
#[tauri::command]
pub async fn tts_launch_backend(
    state: State<'_, AppState>,
    backend: String,
    exe_path: Option<String>,
) -> Result<u32, String> {
    let backend_type = match backend.as_str() {
        "bouyomichan" => TtsBackendType::Bouyomichan,
        "voicevox" => TtsBackendType::Voicevox,
        _ => return Err("Invalid backend type".to_string()),
    };
    state
        .tts_process_manager
        .launch(backend_type, exe_path.as_deref())
        .await
}

/// Kill a TTS backend process
#[tauri::command]
pub async fn tts_kill_backend(state: State<'_, AppState>, backend: String) -> Result<(), String> {
    let backend_type = match backend.as_str() {
        "bouyomichan" => TtsBackendType::Bouyomichan,
        "voicevox" => TtsBackendType::Voicevox,
        _ => return Err("Invalid backend type".to_string()),
    };
    state.tts_process_manager.kill(&backend_type).await
}

/// Get TTS backend launch status
#[tauri::command]
pub async fn tts_get_launch_status(state: State<'_, AppState>) -> Result<TtsLaunchStatus, String> {
    Ok(TtsLaunchStatus {
        bouyomichan_launched: state
            .tts_process_manager
            .is_launched(&TtsBackendType::Bouyomichan)
            .await,
        voicevox_launched: state
            .tts_process_manager
            .is_launched(&TtsBackendType::Voicevox)
            .await,
    })
}
