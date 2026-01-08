//! VOICEVOXバックエンド実装

use async_trait::async_trait;
use std::time::Duration;

use super::TtsBackend;
use crate::gui::plugins::tts_plugin::config::VoicevoxConfig;
use crate::gui::plugins::tts_plugin::error::TtsError;

/// VOICEVOXバックエンド
pub struct VoicevoxBackend {
    config: VoicevoxConfig,
    client: reqwest::Client,
}

impl VoicevoxBackend {
    /// 新しいインスタンスを作成
    pub fn new(config: VoicevoxConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("HTTPクライアントの作成に失敗");

        Self { config, client }
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: VoicevoxConfig) {
        self.config = config;
    }

    /// audio_queryを取得
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
                "audio_queryに失敗: ステータス {}",
                response.status()
            )));
        }

        let query: serde_json::Value = response.json().await?;
        Ok(query)
    }

    /// 音声合成を実行
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
                "synthesisに失敗: ステータス {}",
                response.status()
            )));
        }

        let wav_bytes = response.bytes().await?.to_vec();
        Ok(wav_bytes)
    }

    /// WAVデータを再生（ブロッキング）
    fn play_wav_blocking(wav_bytes: Vec<u8>) -> Result<(), TtsError> {
        use rodio::{Decoder, OutputStream, Sink};
        use std::io::Cursor;

        // 音声出力ストリームを作成
        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| TtsError::AudioOutput(format!("音声出力の初期化に失敗: {}", e)))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| TtsError::AudioOutput(format!("音声シンクの作成に失敗: {}", e)))?;

        // WAVをデコード
        let cursor = Cursor::new(wav_bytes);
        let source = Decoder::new(cursor)
            .map_err(|e| TtsError::AudioDecode(format!("WAVデコードに失敗: {}", e)))?;

        // 再生
        sink.append(source);
        sink.sleep_until_end();

        Ok(())
    }
}

#[async_trait]
impl TtsBackend for VoicevoxBackend {
    async fn test_connection(&self) -> Result<bool, TtsError> {
        // VOICEVOXのバージョン情報を取得して接続確認
        let url = format!("http://{}:{}/version", self.config.host, self.config.port);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(version) = response.text().await {
                        tracing::info!("✅ VOICEVOX接続成功 (バージョン: {})", version.trim());
                    } else {
                        tracing::info!("✅ VOICEVOX接続成功");
                    }
                    Ok(true)
                } else {
                    tracing::warn!("⚠️ VOICEVOX接続失敗: ステータス {}", response.status());
                    Ok(false)
                }
            }
            Err(e) => {
                tracing::error!("❌ VOICEVOX接続エラー: {}", e);
                Err(TtsError::Connection(format!(
                    "VOICEVOXに接続できません: {}",
                    e
                )))
            }
        }
    }

    async fn speak(&self, text: &str) -> Result<(), TtsError> {
        if text.is_empty() {
            return Ok(());
        }

        tracing::debug!("🔊 VOICEVOXに送信: {}", text);

        // 1. audio_queryを取得
        let mut audio_query = self.get_audio_query(text).await?;

        // 2. 音声パラメータを適用
        if let Some(obj) = audio_query.as_object_mut() {
            // 音量
            obj.insert(
                "volumeScale".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(self.config.volume_scale as f64)
                        .unwrap_or_else(|| serde_json::Number::from_f64(1.0).unwrap()),
                ),
            );
            // 話速
            obj.insert(
                "speedScale".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(self.config.speed_scale as f64)
                        .unwrap_or_else(|| serde_json::Number::from_f64(1.0).unwrap()),
                ),
            );
            // 音高
            obj.insert(
                "pitchScale".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(self.config.pitch_scale as f64)
                        .unwrap_or_else(|| serde_json::Number::from_f64(0.0).unwrap()),
                ),
            );
            // 抑揚
            obj.insert(
                "intonationScale".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(self.config.intonation_scale as f64)
                        .unwrap_or_else(|| serde_json::Number::from_f64(1.0).unwrap()),
                ),
            );
        }

        // 3. 音声合成
        let wav_bytes = self.synthesize(&audio_query).await?;

        // 4. 再生（spawn_blockingでブロッキングタスクとして実行）
        tokio::task::spawn_blocking(move || Self::play_wav_blocking(wav_bytes))
            .await
            .map_err(|e| TtsError::AudioOutput(format!("再生タスクエラー: {}", e)))??;

        tracing::debug!("✅ VOICEVOX読み上げ完了");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "VOICEVOX"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VoicevoxConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 50021);
        assert_eq!(config.speaker_id, 1);
    }
}
