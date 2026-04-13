//! Bouyomichan TTS backend

use std::time::Duration;

use async_trait::async_trait;
use super::{TtsBackend, TtsError};
use crate::tts::config::BouyomichanConfig;

/// Bouyomichan backend
pub struct BouyomichanBackend {
    config: BouyomichanConfig,
    client: reqwest::Client,
}

impl BouyomichanBackend {
    /// Create a new instance
    pub fn new(config: BouyomichanConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");
        Self { config, client }
    }

    /// Build Talk API URL
    fn build_talk_url(&self, text: &str) -> String {
        format!(
            "http://{}:{}/Talk?text={}&voice={}&volume={}&speed={}&tone={}",
            self.config.host,
            self.config.port,
            urlencoding::encode(text),
            self.config.voice,
            self.config.volume,
            self.config.speed,
            self.config.tone,
        )
    }
}

#[async_trait]
impl TtsBackend for BouyomichanBackend {
    async fn test_connection(&self) -> Result<bool, TtsError> {
        let url = format!("http://{}:{}/Talk?text=", self.config.host, self.config.port);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    log::info!("Bouyomichan connection successful");
                    Ok(true)
                } else {
                    log::warn!("Bouyomichan connection failed: status {}", response.status());
                    Ok(false)
                }
            }
            Err(e) => {
                log::error!("Bouyomichan connection error: {}", e);
                Err(TtsError::Connection(format!(
                    "Cannot connect to Bouyomichan: {}",
                    e
                )))
            }
        }
    }

    async fn speak(&self, text: &str) -> Result<(), TtsError> {
        if text.is_empty() {
            return Ok(());
        }

        let url = self.build_talk_url(text);
        log::debug!("Sending to Bouyomichan: {}", text);

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            log::debug!("Bouyomichan speak successful");
            Ok(())
        } else {
            let status = response.status();
            Err(TtsError::Connection(format!(
                "Bouyomichan returned error: {}",
                status
            )))
        }
    }

    fn name(&self) -> &'static str {
        "Bouyomichan"
    }
}
