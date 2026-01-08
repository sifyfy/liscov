//! 棒読みちゃんバックエンド実装

use async_trait::async_trait;
use std::time::Duration;

use super::TtsBackend;
use crate::gui::plugins::tts_plugin::config::BouyomichanConfig;
use crate::gui::plugins::tts_plugin::error::TtsError;

/// 棒読みちゃんバックエンド
pub struct BouyomichanBackend {
    config: BouyomichanConfig,
    client: reqwest::Client,
}

impl BouyomichanBackend {
    /// 新しいインスタンスを作成
    pub fn new(config: BouyomichanConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("HTTPクライアントの作成に失敗");
        Self { config, client }
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: BouyomichanConfig) {
        self.config = config;
    }

    /// Talk APIのURLを構築
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
        // 棒読みちゃんに空のテキストを送って接続確認
        // 実際には何も読み上げられないが、接続は確認できる
        let url = format!("http://{}:{}/Talk?text=", self.config.host, self.config.port);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    tracing::info!("✅ 棒読みちゃん接続成功");
                    Ok(true)
                } else {
                    tracing::warn!(
                        "⚠️ 棒読みちゃん接続失敗: ステータス {}",
                        response.status()
                    );
                    Ok(false)
                }
            }
            Err(e) => {
                tracing::error!("❌ 棒読みちゃん接続エラー: {}", e);
                Err(TtsError::Connection(format!(
                    "棒読みちゃんに接続できません: {}",
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
        tracing::debug!("🔊 棒読みちゃんに送信: {}", text);

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            tracing::debug!("✅ 棒読みちゃん読み上げ成功");
            Ok(())
        } else {
            let status = response.status();
            Err(TtsError::Connection(format!(
                "棒読みちゃんがエラーを返しました: {}",
                status
            )))
        }
    }

    fn name(&self) -> &'static str {
        "棒読みちゃん"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_talk_url() {
        let config = BouyomichanConfig::default();
        let backend = BouyomichanBackend::new(config);

        let url = backend.build_talk_url("テスト");
        assert!(url.contains("text=%E3%83%86%E3%82%B9%E3%83%88"));
        assert!(url.contains("voice=0"));
        assert!(url.contains("volume=-1"));
        assert!(url.contains("speed=-1"));
        assert!(url.contains("tone=-1"));
    }

    #[test]
    fn test_url_encoding() {
        let config = BouyomichanConfig::default();
        let backend = BouyomichanBackend::new(config);

        // 日本語と特殊文字が正しくエンコードされるか確認
        let url = backend.build_talk_url("こんにちは！");
        assert!(url.contains("text="));
        assert!(!url.contains("こんにちは")); // エンコードされているはず
    }
}
