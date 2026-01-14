//! VOICEVOX TTS backend

use std::time::Duration;

use super::TtsError;
use crate::tts::config::VoicevoxConfig;

/// VOICEVOX backend
pub struct VoicevoxBackend {
    config: VoicevoxConfig,
    client: reqwest::Client,
}

impl VoicevoxBackend {
    /// Create a new instance
    pub fn new(config: VoicevoxConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Update configuration
    pub fn update_config(&mut self, config: VoicevoxConfig) {
        self.config = config;
    }

    /// Get audio query
    async fn get_audio_query(&self, text: &str) -> Result<serde_json::Value, TtsError> {
        let url = format!(
            "http://{}:{}/audio_query?speaker={}&text={}",
            self.config.host,
            self.config.port,
            self.config.speaker_id,
            urlencoding::encode(text),
        );

        let response = self.client.post(&url).send().await?;

        if !response.status().is_success() {
            return Err(TtsError::Connection(format!(
                "audio_query failed: status {}",
                response.status()
            )));
        }

        let query: serde_json::Value = response.json().await?;
        Ok(query)
    }

    /// Synthesize audio
    async fn synthesize(&self, audio_query: &serde_json::Value) -> Result<Vec<u8>, TtsError> {
        let url = format!(
            "http://{}:{}/synthesis?speaker={}",
            self.config.host, self.config.port, self.config.speaker_id,
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(audio_query)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(TtsError::Connection(format!(
                "synthesis failed: status {}",
                response.status()
            )));
        }

        let wav_bytes = response.bytes().await?.to_vec();
        Ok(wav_bytes)
    }

    /// Play WAV data (blocking)
    fn play_wav_blocking(wav_bytes: Vec<u8>) -> Result<(), TtsError> {
        use rodio::{Decoder, OutputStreamBuilder, Sink};
        use std::io::Cursor;

        let stream = OutputStreamBuilder::open_default_stream()
            .map_err(|e| TtsError::AudioOutput(format!("Failed to initialize audio output: {}", e)))?;

        let sink = Sink::connect_new(&stream.mixer());

        let cursor = Cursor::new(wav_bytes);
        let source = Decoder::new(cursor)
            .map_err(|e| TtsError::AudioDecode(format!("Failed to decode WAV: {}", e)))?;

        sink.append(source);
        sink.sleep_until_end();

        Ok(())
    }

    /// Test connection to the backend
    pub async fn test_connection(&self) -> Result<bool, TtsError> {
        let url = format!("http://{}:{}/version", self.config.host, self.config.port);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(version) = response.text().await {
                        log::info!("VOICEVOX connection successful (version: {})", version.trim());
                    } else {
                        log::info!("VOICEVOX connection successful");
                    }
                    Ok(true)
                } else {
                    log::warn!("VOICEVOX connection failed: status {}", response.status());
                    Ok(false)
                }
            }
            Err(e) => {
                log::error!("VOICEVOX connection error: {}", e);
                Err(TtsError::Connection(format!(
                    "Cannot connect to VOICEVOX: {}",
                    e
                )))
            }
        }
    }

    /// Speak the given text
    pub async fn speak(&self, text: &str) -> Result<(), TtsError> {
        if text.is_empty() {
            return Ok(());
        }

        log::debug!("Sending to VOICEVOX: {}", text);

        // 1. Get audio query
        let mut audio_query = self.get_audio_query(text).await?;

        // 2. Apply audio parameters
        if let Some(obj) = audio_query.as_object_mut() {
            obj.insert(
                "volumeScale".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(self.config.volume_scale as f64)
                        .unwrap_or_else(|| serde_json::Number::from_f64(1.0).unwrap()),
                ),
            );
            obj.insert(
                "speedScale".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(self.config.speed_scale as f64)
                        .unwrap_or_else(|| serde_json::Number::from_f64(1.0).unwrap()),
                ),
            );
            obj.insert(
                "pitchScale".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(self.config.pitch_scale as f64)
                        .unwrap_or_else(|| serde_json::Number::from_f64(0.0).unwrap()),
                ),
            );
            obj.insert(
                "intonationScale".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(self.config.intonation_scale as f64)
                        .unwrap_or_else(|| serde_json::Number::from_f64(1.0).unwrap()),
                ),
            );
        }

        // 3. Synthesize
        let wav_bytes = self.synthesize(&audio_query).await?;

        // 4. Play (spawn_blocking for blocking task)
        tokio::task::spawn_blocking(move || Self::play_wav_blocking(wav_bytes))
            .await
            .map_err(|e| TtsError::AudioOutput(format!("Playback task error: {}", e)))??;

        log::debug!("VOICEVOX speak completed");
        Ok(())
    }
}
