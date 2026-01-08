//! TTS設定構造体

use serde::{Deserialize, Serialize};

/// TTSバックエンドの種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TtsBackendType {
    /// 無効
    #[default]
    None,
    /// 棒読みちゃん
    Bouyomichan,
    /// VOICEVOX
    Voicevox,
}

impl std::fmt::Display for TtsBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtsBackendType::None => write!(f, "無効"),
            TtsBackendType::Bouyomichan => write!(f, "棒読みちゃん"),
            TtsBackendType::Voicevox => write!(f, "VOICEVOX"),
        }
    }
}

/// 棒読みちゃん固有設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BouyomichanConfig {
    /// ホスト名
    pub host: String,
    /// ポート番号
    pub port: u16,
    /// 声質 (0=デフォルト, 1=女性1, 2=女性2, 3=男性1, 4=男性2, ...)
    pub voice: i32,
    /// 音量 (-1=デフォルト)
    pub volume: i32,
    /// 速度 (-1=デフォルト)
    pub speed: i32,
    /// トーン (-1=デフォルト)
    pub tone: i32,
    /// 自動起動を有効化
    #[serde(default)]
    pub auto_launch: bool,
    /// 実行ファイルパス（Noneなら自動検出）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    /// アプリ終了時に一緒に終了する
    #[serde(default)]
    pub auto_close_on_exit: bool,
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
            auto_launch: false,
            executable_path: None,
            auto_close_on_exit: false,
        }
    }
}

/// VOICEVOX固有設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicevoxConfig {
    /// ホスト名
    pub host: String,
    /// ポート番号
    pub port: u16,
    /// 話者ID
    pub speaker_id: i32,
    /// 音量スケール (0.0〜2.0、デフォルト1.0)
    #[serde(default = "default_volume_scale")]
    pub volume_scale: f32,
    /// 話速スケール (0.5〜2.0、デフォルト1.0)
    #[serde(default = "default_speed_scale")]
    pub speed_scale: f32,
    /// 音高スケール (-0.15〜0.15、デフォルト0.0)
    #[serde(default = "default_pitch_scale")]
    pub pitch_scale: f32,
    /// 抑揚スケール (0.0〜2.0、デフォルト1.0)
    #[serde(default = "default_intonation_scale")]
    pub intonation_scale: f32,
    /// 自動起動を有効化
    #[serde(default)]
    pub auto_launch: bool,
    /// 実行ファイルパス（Noneなら自動検出）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    /// アプリ終了時に一緒に終了する
    #[serde(default)]
    pub auto_close_on_exit: bool,
}

fn default_volume_scale() -> f32 {
    1.0
}

fn default_speed_scale() -> f32 {
    1.0
}

fn default_pitch_scale() -> f32 {
    0.0
}

fn default_intonation_scale() -> f32 {
    1.0
}

impl Default for VoicevoxConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 50021,
            speaker_id: 1, // 四国めたん（ノーマル）
            volume_scale: 1.0,
            speed_scale: 1.0,
            pitch_scale: 0.0,
            intonation_scale: 1.0,
            auto_launch: false,
            executable_path: None,
            auto_close_on_exit: false,
        }
    }
}

/// TTSプラグイン設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// 有効/無効
    pub enabled: bool,
    /// 使用するバックエンド
    pub backend: TtsBackendType,
    /// 棒読みちゃん設定
    pub bouyomichan: BouyomichanConfig,
    /// VOICEVOX設定
    pub voicevox: VoicevoxConfig,
    /// 投稿者名を読み上げるか
    pub read_author_name: bool,
    /// 投稿者名に「さん」を付けるか
    #[serde(default = "default_true")]
    pub add_honorific: bool,
    /// 投稿者名の先頭の@を除去するか（YouTubeハンドル対応）
    #[serde(default = "default_true")]
    pub strip_at_prefix: bool,
    /// 投稿者名末尾の -xxx (3文字のsuffix) を除去するか
    #[serde(default = "default_true")]
    pub strip_handle_suffix: bool,
    /// スーパーチャット金額を読み上げるか
    pub read_superchat_amount: bool,
    /// 最大読み上げ文字数
    pub max_text_length: usize,
    /// キューサイズ上限
    pub queue_size_limit: usize,
}

fn default_true() -> bool {
    true
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
