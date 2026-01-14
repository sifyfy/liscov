//! TTS backend implementations

pub mod bouyomichan;
pub mod voicevox;

pub use bouyomichan::BouyomichanBackend;
pub use voicevox::VoicevoxBackend;

use crate::tts::config::{BouyomichanConfig, TtsBackendType, VoicevoxConfig};

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

/// TTS backend enum (static dispatch)
pub enum TtsBackendEnum {
    Bouyomichan(BouyomichanBackend),
    Voicevox(VoicevoxBackend),
}

impl TtsBackendEnum {
    /// Create a backend from config
    pub fn from_config(
        backend_type: &TtsBackendType,
        bouyomichan: &BouyomichanConfig,
        voicevox: &VoicevoxConfig,
    ) -> Option<Self> {
        match backend_type {
            TtsBackendType::None => None,
            TtsBackendType::Bouyomichan => {
                Some(Self::Bouyomichan(BouyomichanBackend::new(bouyomichan.clone())))
            }
            TtsBackendType::Voicevox => {
                Some(Self::Voicevox(VoicevoxBackend::new(voicevox.clone())))
            }
        }
    }

    /// Test connection to the backend
    pub async fn test_connection(&self) -> Result<bool, TtsError> {
        match self {
            Self::Bouyomichan(b) => b.test_connection().await,
            Self::Voicevox(b) => b.test_connection().await,
        }
    }

    /// Speak the given text
    pub async fn speak(&self, text: &str) -> Result<(), TtsError> {
        match self {
            Self::Bouyomichan(b) => b.speak(text).await,
            Self::Voicevox(b) => b.speak(text).await,
        }
    }

    /// Get the backend name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bouyomichan(_) => "Bouyomichan",
            Self::Voicevox(_) => "VOICEVOX",
        }
    }
}
