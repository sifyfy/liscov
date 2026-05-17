//! TTS backend implementations
//!
//! 新しいバックエンドを追加する場合:
//! 1. `TtsBackend` トレイトを実装した構造体を作成
//! 2. `create_backend` ファクトリ関数にマッチアームを追加
//! 3. `TtsBackendType` に新しいバリアントを追加

pub mod bouyomichan;
pub mod voicevox;

pub use bouyomichan::BouyomichanBackend;
pub use voicevox::VoicevoxBackend;

use crate::tts::config::{BouyomichanConfig, TtsBackendType, VoicevoxConfig};
use async_trait::async_trait;

/// TTS backend error
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Audio output error: {0}")]
    AudioOutput(String),

    #[error("Audio decode error: {0}")]
    AudioDecode(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

/// TTS バックエンドの共通インターフェース
///
/// 新しいバックエンドを追加する際はこのトレイトを実装し、
/// `create_backend` ファクトリ関数に登録する。
#[async_trait]
pub trait TtsBackend: Send + Sync {
    /// バックエンドへの接続テスト
    async fn test_connection(&self) -> Result<bool, TtsError>;
    /// テキストを読み上げる
    async fn speak(&self, text: &str) -> Result<(), TtsError>;
    /// バックエンド名を返す
    fn name(&self) -> &'static str;
}

/// 設定からバックエンドインスタンスを生成する
pub fn create_backend(
    backend_type: &TtsBackendType,
    bouyomichan: &BouyomichanConfig,
    voicevox: &VoicevoxConfig,
) -> Option<Box<dyn TtsBackend>> {
    match backend_type {
        TtsBackendType::None => None,
        TtsBackendType::Bouyomichan => Some(Box::new(BouyomichanBackend::new(bouyomichan.clone()))),
        TtsBackendType::Voicevox => Some(Box::new(VoicevoxBackend::new(voicevox.clone()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::config::{BouyomichanConfig, VoicevoxConfig};

    #[test]
    fn create_backend_none_returns_none() {
        let result = create_backend(
            &TtsBackendType::None,
            &BouyomichanConfig::default(),
            &VoicevoxConfig::default(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn create_backend_bouyomichan_returns_some() {
        let result = create_backend(
            &TtsBackendType::Bouyomichan,
            &BouyomichanConfig::default(),
            &VoicevoxConfig::default(),
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "Bouyomichan");
    }

    #[test]
    fn create_backend_voicevox_returns_some() {
        let result = create_backend(
            &TtsBackendType::Voicevox,
            &BouyomichanConfig::default(),
            &VoicevoxConfig::default(),
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "VOICEVOX");
    }
}
