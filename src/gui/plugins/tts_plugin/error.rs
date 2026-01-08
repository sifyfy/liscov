//! TTSプラグイン用エラー型

use thiserror::Error;

/// TTSエラー型
#[derive(Debug, Error)]
pub enum TtsError {
    #[error("接続に失敗しました: {0}")]
    Connection(String),

    #[error("HTTPリクエストに失敗しました: {0}")]
    Http(#[from] reqwest::Error),

    #[error("音声出力エラー: {0}")]
    AudioOutput(String),

    #[error("音声デコードエラー: {0}")]
    AudioDecode(String),

    #[error("設定エラー: {0}")]
    Config(String),

    #[error("バックエンドが利用できません: {0}")]
    BackendUnavailable(String),

    #[error("キューが満杯です")]
    QueueFull,

    #[error("JSONパースエラー: {0}")]
    JsonParse(#[from] serde_json::Error),
}

impl From<TtsError> for crate::LiscovError {
    fn from(err: TtsError) -> Self {
        crate::LiscovError::General(anyhow::anyhow!("{}", err))
    }
}
