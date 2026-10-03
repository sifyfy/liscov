//! VOICEVOX TTS backend

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::{TtsBackend, TtsError};
use crate::tts::config::VoicevoxConfig;
use async_trait::async_trait;

/// 再生を打ち切るまでの、音声の長さに足す余裕（04_tts.md「音声再生」）
const PLAYBACK_MARGIN: Duration = Duration::from_secs(5);
/// 音声の長さが分からないときに再生を打ち切るまでの時間
const PLAYBACK_LIMIT_UNKNOWN_LENGTH: Duration = Duration::from_secs(60);
/// 再生の終わり・停止要求を確かめる間隔
const PLAYBACK_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// 再生の終わり方
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackEnd {
    Finished,
    TimedOut,
    Stopped,
}

/// 再生を打ち切るまでの時間
fn playback_limit(length: Option<Duration>) -> Duration {
    length.map_or(PLAYBACK_LIMIT_UNKNOWN_LENGTH, |length| {
        length + PLAYBACK_MARGIN
    })
}

/// 再生の終わりを待つ。`limit` を過ぎるか `stop` が立ったら待つのをやめる
fn wait_for_playback(
    is_finished: impl Fn() -> bool,
    limit: Duration,
    stop: &AtomicBool,
    poll: Duration,
) -> PlaybackEnd {
    let started = Instant::now();
    loop {
        if is_finished() {
            return PlaybackEnd::Finished;
        }
        if stop.load(Ordering::Relaxed) {
            return PlaybackEnd::Stopped;
        }
        if started.elapsed() >= limit {
            return PlaybackEnd::TimedOut;
        }
        std::thread::sleep(poll);
    }
}

/// drop されたら停止要求を立てる。speak の future が捨てられた（キュー処理の停止）ときに再生も止めるため
struct StopOnDrop(Arc<AtomicBool>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

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
    ///
    /// 音声の長さ + 5秒で打ち切り、`stop` が立ったら止める（04_tts.md「音声再生」）。
    fn play_wav_blocking(wav_bytes: Vec<u8>, stop: &AtomicBool) -> Result<(), TtsError> {
        use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
        use std::io::Cursor;

        let stream = OutputStreamBuilder::open_default_stream().map_err(|e| {
            TtsError::AudioOutput(format!("Failed to initialize audio output: {}", e))
        })?;

        let sink = Sink::connect_new(stream.mixer());

        let cursor = Cursor::new(wav_bytes);
        let source = Decoder::new(cursor)
            .map_err(|e| TtsError::AudioDecode(format!("Failed to decode WAV: {}", e)))?;

        let limit = playback_limit(source.total_duration());
        sink.append(source);
        let end = wait_for_playback(|| sink.empty(), limit, stop, PLAYBACK_POLL_INTERVAL);
        sink.stop();

        match end {
            PlaybackEnd::Finished | PlaybackEnd::Stopped => Ok(()),
            PlaybackEnd::TimedOut => Err(TtsError::AudioOutput(format!(
                "再生が {:?} で終わらないため打ち切った",
                limit
            ))),
        }
    }
}

#[async_trait]
impl TtsBackend for VoicevoxBackend {
    async fn test_connection(&self) -> Result<bool, TtsError> {
        let url = format!("http://{}:{}/version", self.config.host, self.config.port);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(version) = response.text().await {
                        log::info!(
                            "VOICEVOX connection successful (version: {})",
                            version.trim()
                        );
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

    async fn speak(&self, text: &str) -> Result<(), TtsError> {
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
        // この future が捨てられたら（キュー処理の停止）_stop_on_drop が再生を止める
        let stop = Arc::new(AtomicBool::new(false));
        let _stop_on_drop = StopOnDrop(Arc::clone(&stop));
        tokio::task::spawn_blocking(move || Self::play_wav_blocking(wav_bytes, &stop))
            .await
            .map_err(|e| TtsError::AudioOutput(format!("Playback task error: {}", e)))??;

        log::debug!("VOICEVOX speak completed");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "VOICEVOX"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    const POLL: Duration = Duration::from_millis(5);

    // 04_tts.md 音声再生: 長さ + 5秒で打ち切る。長さが分からなければ 60 秒
    #[test]
    fn playback_limit_is_length_plus_five_seconds() {
        assert_eq!(
            playback_limit(Some(Duration::from_secs(3))),
            Duration::from_secs(8)
        );
        assert_eq!(playback_limit(None), Duration::from_secs(60));
    }

    #[test]
    fn playback_finishes_when_sink_becomes_empty() {
        let stop = AtomicBool::new(false);
        assert_eq!(
            wait_for_playback(|| true, Duration::from_secs(1), &stop, POLL),
            PlaybackEnd::Finished
        );
    }

    // 04_tts.md: 長さ + 5秒たっても終わらない → 打ち切る
    #[test]
    fn playback_times_out_when_it_never_finishes() {
        let stop = AtomicBool::new(false);
        assert_eq!(
            wait_for_playback(|| false, Duration::from_millis(30), &stop, POLL),
            PlaybackEnd::TimedOut
        );
    }

    // 04_tts.md: キュー処理を止めたら再生中の音声も止める
    #[test]
    fn playback_stops_when_requested() {
        let stop = AtomicBool::new(true);
        assert_eq!(
            wait_for_playback(|| false, Duration::from_secs(10), &stop, POLL),
            PlaybackEnd::Stopped
        );
    }
}
