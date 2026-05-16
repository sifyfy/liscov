//! TTS (Text-to-Speech) commands

use crate::errors::CommandError;
use crate::state::AppState;
use crate::tts::{TtsBackendType, TtsConfig, TtsPriority, TtsProcessManager, TtsQueueItem};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
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
    pub first_comment_prefix_enabled: bool,
    pub first_comment_prefix: String,
    pub first_comment_only: bool,
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
            first_comment_prefix_enabled: config.first_comment_prefix_enabled,
            first_comment_prefix: config.first_comment_prefix,
            first_comment_only: config.first_comment_only,
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
            first_comment_prefix_enabled: dto.first_comment_prefix_enabled,
            first_comment_prefix: dto.first_comment_prefix,
            first_comment_only: dto.first_comment_only,
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
) -> Result<(), CommandError> {
    let priority = parse_tts_priority(priority.as_deref());

    let item = TtsQueueItem {
        text,
        priority,
        author_name,
        amount,
        in_stream_comment_count: None,
    };

    state.tts_manager.enqueue(item).await;
    Ok(())
}

/// Speak text directly (bypasses queue)
#[tauri::command]
pub async fn tts_speak_direct(
    state: State<'_, AppState>,
    text: String,
) -> Result<(), CommandError> {
    state
        .tts_manager
        .speak_direct(&text)
        .await
        .map_err(CommandError::from)
}

/// Update TTS configuration
#[tauri::command]
pub async fn tts_update_config(
    state: State<'_, AppState>,
    config: TtsConfigDto,
) -> Result<(), CommandError> {
    let was_enabled = state.tts_manager.get_config().await.enabled;
    let will_be_enabled = config.enabled;

    state.tts_manager.update_config(config.into()).await;

    // Start/stop processing based on enabled state change
    match decide_processing_action(was_enabled, will_be_enabled) {
        Some(ProcessingAction::Start) => state.tts_manager.start_processing().await,
        Some(ProcessingAction::Stop) => state.tts_manager.stop_processing().await,
        None => {}
    }

    Ok(())
}

/// Get current TTS configuration
#[tauri::command]
pub async fn tts_get_config(state: State<'_, AppState>) -> Result<TtsConfigDto, CommandError> {
    let config = state.tts_manager.get_config().await;
    Ok(config.into())
}

/// Test TTS connection to backend
#[tauri::command]
pub async fn tts_test_connection(
    state: State<'_, AppState>,
    backend: Option<String>,
) -> Result<bool, CommandError> {
    if let Some(backend_str) = backend {
        let Some(backend_type) = parse_backend_type(&backend_str) else {
            return Ok(false);
        };
        state
            .tts_manager
            .test_backend_connection(backend_type)
            .await
            .map_err(CommandError::from)
    } else {
        state
            .tts_manager
            .test_connection()
            .await
            .map_err(CommandError::from)
    }
}

/// Start TTS queue processing
#[tauri::command]
pub async fn tts_start(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.tts_manager.start_processing().await;
    Ok(())
}

/// Stop TTS queue processing
#[tauri::command]
pub async fn tts_stop(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.tts_manager.stop_processing().await;
    Ok(())
}

/// Clear TTS queue
#[tauri::command]
pub async fn tts_clear_queue(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.tts_manager.clear_queue().await;
    Ok(())
}

/// Get TTS status
#[tauri::command]
pub async fn tts_get_status(state: State<'_, AppState>) -> Result<TtsStatus, CommandError> {
    Ok(TtsStatus {
        is_processing: state.tts_manager.is_processing().await,
        queue_size: state.tts_manager.queue_size().await,
        backend_name: state
            .tts_manager
            .backend_name()
            .await
            .map(|s| s.to_string()),
    })
}

/// TTS backend launch status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsLaunchStatus {
    pub bouyomichan_launched: bool,
    pub voicevox_launched: bool,
}

/// TTS処理の有効/無効切り替えアクション
#[derive(Debug, PartialEq)]
pub(crate) enum ProcessingAction {
    Start,
    Stop,
}

/// enabled状態の変化からProcessingActionを決定する
pub(crate) fn decide_processing_action(
    was_enabled: bool,
    will_be_enabled: bool,
) -> Option<ProcessingAction> {
    if !was_enabled && will_be_enabled {
        Some(ProcessingAction::Start)
    } else if was_enabled && !will_be_enabled {
        Some(ProcessingAction::Stop)
    } else {
        None
    }
}

/// バックエンド文字列をTtsBackendTypeに変換する
pub(crate) fn parse_backend_type(s: &str) -> Option<TtsBackendType> {
    match s {
        "bouyomichan" => Some(TtsBackendType::Bouyomichan),
        "voicevox" => Some(TtsBackendType::Voicevox),
        _ => None,
    }
}

/// 優先度文字列をTtsPriorityに変換する
pub(crate) fn parse_tts_priority(priority: Option<&str>) -> TtsPriority {
    match priority {
        Some("superchat") => TtsPriority::SuperChat,
        Some("membership") => TtsPriority::Membership,
        _ => TtsPriority::Normal,
    }
}

/// Discover executable path for a TTS backend
#[tauri::command]
pub async fn tts_discover_exe(backend: String) -> Result<Option<String>, CommandError> {
    let Some(backend_type) = parse_backend_type(&backend) else {
        return Ok(None);
    };
    Ok(TtsProcessManager::discover_exe(&backend_type))
}

/// Select executable file via dialog
#[tauri::command]
pub async fn tts_select_exe(app: tauri::AppHandle) -> Result<Option<String>, CommandError> {
    let file = app
        .dialog()
        .file()
        .add_filter("実行ファイル", &["exe"])
        .blocking_pick_file();

    Ok(file.map(|f| f.to_string()))
}

/// バックエンドプロセス起動のビジネスロジック
pub(crate) async fn launch_backend_impl(
    process_manager: &TtsProcessManager,
    backend: &str,
    exe_path: Option<&str>,
) -> Result<u32, CommandError> {
    let backend_type = parse_backend_type(backend)
        .ok_or_else(|| CommandError::InvalidInput("Invalid backend type".to_string()))?;
    process_manager
        .launch(backend_type, exe_path)
        .await
        .map_err(CommandError::TtsError)
}

/// バックエンドプロセス停止のビジネスロジック
pub(crate) async fn kill_backend_impl(
    process_manager: &TtsProcessManager,
    backend: &str,
) -> Result<(), CommandError> {
    let backend_type = parse_backend_type(backend)
        .ok_or_else(|| CommandError::InvalidInput("Invalid backend type".to_string()))?;
    process_manager
        .kill(&backend_type)
        .await
        .map_err(CommandError::TtsError)
}

/// Launch a TTS backend process
#[tauri::command]
pub async fn tts_launch_backend(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    backend: String,
    exe_path: Option<String>,
) -> Result<u32, CommandError> {
    let result =
        launch_backend_impl(&state.tts_process_manager, &backend, exe_path.as_deref()).await;

    if result.is_ok() {
        let _ = app.emit("tts:process_launched", &backend);
    }

    result
}

/// Kill a TTS backend process
#[tauri::command]
pub async fn tts_kill_backend(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    backend: String,
) -> Result<(), CommandError> {
    let result = kill_backend_impl(&state.tts_process_manager, &backend).await;

    if result.is_ok() {
        let _ = app.emit("tts:process_stopped", &backend);
    }

    result
}

/// Get TTS backend launch status
#[tauri::command]
pub async fn tts_get_launch_status(
    state: State<'_, AppState>,
) -> Result<TtsLaunchStatus, CommandError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::{TtsBackendType, TtsConfig, TtsPriority};

    // ========================================================================
    // TtsConfig → TtsConfigDto 変換（L50のmutantをkill）
    // ========================================================================

    #[test]
    fn tts_config_to_dto_backend_none() {
        // spec: TtsBackendType::None → dto.backend == "none"
        let config = TtsConfig {
            backend: TtsBackendType::None,
            ..TtsConfig::default()
        };
        let dto = TtsConfigDto::from(config);
        assert_eq!(dto.backend, "none");
    }

    #[test]
    fn tts_config_to_dto_backend_bouyomichan() {
        // spec: TtsBackendType::Bouyomichan → dto.backend == "bouyomichan"
        let config = TtsConfig {
            enabled: true,
            backend: TtsBackendType::Bouyomichan,
            first_comment_only: true,
            ..TtsConfig::default()
        };
        let dto = TtsConfigDto::from(config);
        assert_eq!(dto.backend, "bouyomichan");
        assert!(dto.enabled);
        assert!(dto.first_comment_only);
    }

    #[test]
    fn tts_config_to_dto_backend_voicevox() {
        // spec: TtsBackendType::Voicevox → dto.backend == "voicevox"
        let config = TtsConfig {
            backend: TtsBackendType::Voicevox,
            ..TtsConfig::default()
        };
        let dto = TtsConfigDto::from(config);
        assert_eq!(dto.backend, "voicevox");
    }

    // ========================================================================
    // TtsConfigDto → TtsConfig 変換（L92, L97, L98のmutantをkill）
    // ========================================================================

    #[test]
    fn dto_to_config_backend_bouyomichan() {
        // spec: dto.backend == "bouyomichan" → TtsBackendType::Bouyomichan
        let dto = TtsConfigDto {
            backend: "bouyomichan".to_string(),
            ..TtsConfigDto::default()
        };
        let config = TtsConfig::from(dto);
        assert_eq!(config.backend, TtsBackendType::Bouyomichan);
    }

    #[test]
    fn dto_to_config_backend_voicevox() {
        // spec: dto.backend == "voicevox" → TtsBackendType::Voicevox
        let dto = TtsConfigDto {
            backend: "voicevox".to_string(),
            ..TtsConfigDto::default()
        };
        let config = TtsConfig::from(dto);
        assert_eq!(config.backend, TtsBackendType::Voicevox);
    }

    #[test]
    fn dto_to_config_backend_none_string() {
        // spec: dto.backend == "none" → TtsBackendType::None
        let dto = TtsConfigDto {
            backend: "none".to_string(),
            ..TtsConfigDto::default()
        };
        let config = TtsConfig::from(dto);
        assert_eq!(config.backend, TtsBackendType::None);
    }

    #[test]
    fn dto_to_config_backend_invalid_string_falls_back_to_none() {
        // spec: 無効な文字列 → TtsBackendType::None にフォールバック
        let dto = TtsConfigDto {
            backend: "invalid_backend".to_string(),
            ..TtsConfigDto::default()
        };
        let config = TtsConfig::from(dto);
        assert_eq!(config.backend, TtsBackendType::None);
    }

    // ========================================================================
    // parse_backend_type テスト（バックエンド文字列→TtsBackendType変換）
    // ========================================================================

    #[test]
    fn parse_backend_type_bouyomichan() {
        assert_eq!(
            parse_backend_type("bouyomichan"),
            Some(TtsBackendType::Bouyomichan)
        );
    }

    #[test]
    fn parse_backend_type_voicevox() {
        assert_eq!(
            parse_backend_type("voicevox"),
            Some(TtsBackendType::Voicevox)
        );
    }

    #[test]
    fn parse_backend_type_unknown_returns_none() {
        assert_eq!(parse_backend_type("unknown"), None);
    }

    #[test]
    fn parse_backend_type_empty_returns_none() {
        assert_eq!(parse_backend_type(""), None);
    }

    // ========================================================================
    // parse_tts_priority テスト（優先度文字列→TtsPriority変換）
    // ========================================================================

    #[test]
    fn parse_tts_priority_superchat() {
        assert_eq!(
            parse_tts_priority(Some("superchat")),
            TtsPriority::SuperChat
        );
    }

    #[test]
    fn parse_tts_priority_membership() {
        assert_eq!(
            parse_tts_priority(Some("membership")),
            TtsPriority::Membership
        );
    }

    #[test]
    fn parse_tts_priority_none_returns_normal() {
        assert_eq!(parse_tts_priority(None), TtsPriority::Normal);
    }

    #[test]
    fn parse_tts_priority_unknown_returns_normal() {
        assert_eq!(parse_tts_priority(Some("unknown")), TtsPriority::Normal);
    }

    // ========================================================================
    // decide_processing_action テスト（enabled変化→ProcessingAction決定）
    // ========================================================================

    #[test]
    fn decide_processing_action_disabled_to_enabled_starts() {
        assert_eq!(
            decide_processing_action(false, true),
            Some(ProcessingAction::Start)
        );
    }

    #[test]
    fn decide_processing_action_enabled_to_disabled_stops() {
        assert_eq!(
            decide_processing_action(true, false),
            Some(ProcessingAction::Stop)
        );
    }

    #[test]
    fn decide_processing_action_both_disabled_returns_none() {
        assert_eq!(decide_processing_action(false, false), None);
    }

    #[test]
    fn decide_processing_action_both_enabled_returns_none() {
        assert_eq!(decide_processing_action(true, true), None);
    }

    #[test]
    fn dto_to_config_first_comment_fields_are_preserved() {
        // spec: first_comment_prefix_enabled/first_comment_only が正しく変換される
        let dto = TtsConfigDto {
            first_comment_prefix_enabled: true,
            first_comment_only: true,
            first_comment_prefix: "初コメ！".to_string(),
            ..TtsConfigDto::default()
        };
        let config = TtsConfig::from(dto);
        assert!(config.first_comment_prefix_enabled);
        assert!(config.first_comment_only);
        assert_eq!(config.first_comment_prefix, "初コメ！");
    }

    // ========================================================================
    // launch_backend_impl / kill_backend_impl テスト
    // ========================================================================

    #[tokio::test]
    async fn launch_backend_impl_rejects_invalid_backend() {
        // spec: 無効なバックエンド文字列 → InvalidInput エラー
        let pm = crate::tts::TtsProcessManager::new();
        let result = launch_backend_impl(&pm, "invalid", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn launch_backend_impl_fails_without_exe() {
        // spec: exe なし + 探索失敗 → エラー（Ok(0) / Ok(1) mutant をkill）
        let pm = crate::tts::TtsProcessManager::new();
        let result = launch_backend_impl(&pm, "bouyomichan", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn kill_backend_impl_rejects_invalid_backend() {
        // spec: 無効なバックエンド文字列 → InvalidInput エラー
        let pm = crate::tts::TtsProcessManager::new();
        let result = kill_backend_impl(&pm, "invalid").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn kill_backend_impl_fails_when_not_launched() {
        // spec: 起動していないバックエンド → エラー（Ok(()) mutant をkill）
        let pm = crate::tts::TtsProcessManager::new();
        let result = kill_backend_impl(&pm, "bouyomichan").await;
        assert!(result.is_err());
    }
}
