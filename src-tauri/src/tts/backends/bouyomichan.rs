//! Bouyomichan TTS backend

use std::sync::LazyLock;
use std::time::Duration;

use super::{TtsBackend, TtsError};
use crate::tts::config::BouyomichanConfig;
use async_trait::async_trait;
use regex::Regex;

/// 棒読みちゃんの教育・忘却コマンドを効かなくする
/// （04_tts.md「棒読みちゃんの辞書を視聴者に書き換えさせない」）
///
/// 棒読みちゃんは読み上げる文の中の `教育(単語=読み)` / `忘却(単語)` で辞書を書き換える。
/// 視聴者のコメントや名前がそのまま届くと誰でも配信者の辞書を変えられるので、
/// キーワード直後の括弧（半角・全角。あいだの空白と重ねた括弧も含む）を空白 1 つに置き換える。
/// 空白を差し込むだけにしないのは、棒読みちゃんがキーワードと括弧のあいだの空白を許す場合にも効かせるため。
fn neutralize_dictionary_commands(text: &str) -> String {
    static COMMAND_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(教育|忘却)[\s(（]*[(（]").expect("正規表現コンパイル失敗"));
    COMMAND_RE.replace_all(text, "${1} ").into_owned()
}

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
    ///
    /// 読み上げる文はどの経路から来ても必ず教育・忘却コマンドを効かなくしてから送る
    fn build_talk_url(&self, text: &str) -> String {
        format!(
            "http://{}:{}/Talk?text={}&voice={}&volume={}&speed={}&tone={}",
            self.config.host,
            self.config.port,
            urlencoding::encode(&neutralize_dictionary_commands(text)),
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
        let url = format!(
            "http://{}:{}/Talk?text=",
            self.config.host, self.config.port
        );

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    log::info!("Bouyomichan connection successful");
                    Ok(true)
                } else {
                    log::warn!(
                        "Bouyomichan connection failed: status {}",
                        response.status()
                    );
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

#[cfg(test)]
mod tests {
    use super::*;

    // 04_tts.md「棒読みちゃんの辞書を視聴者に書き換えさせない」の表
    #[test]
    fn neutralizes_teach_command_with_half_width_paren() {
        assert_eq!(
            neutralize_dictionary_commands("山田太郎さん、教育(liscov=りすこふ)"),
            "山田太郎さん、教育 liscov=りすこふ)"
        );
    }

    #[test]
    fn neutralizes_forget_command_with_full_width_paren() {
        assert_eq!(
            neutralize_dictionary_commands("山田太郎さん、忘却（liscov）"),
            "山田太郎さん、忘却 liscov）"
        );
    }

    #[test]
    fn neutralizes_command_with_space_before_paren() {
        assert_eq!(
            neutralize_dictionary_commands("山田太郎さん、教育 （liscov=りすこふ)"),
            "山田太郎さん、教育 liscov=りすこふ)"
        );
    }

    #[test]
    fn neutralizes_command_with_repeated_parens() {
        assert_eq!(
            neutralize_dictionary_commands("山田太郎さん、教育((liscov=りすこふ)"),
            "山田太郎さん、教育 liscov=りすこふ)"
        );
    }

    #[test]
    fn neutralizes_every_command_in_text() {
        assert_eq!(
            neutralize_dictionary_commands("山田太郎さん、教育(a=b)と忘却(c)"),
            "山田太郎さん、教育 a=b)と忘却 c)"
        );
    }

    #[test]
    fn neutralizes_command_in_author_name() {
        assert_eq!(
            neutralize_dictionary_commands("教育(liscov=りすこふ)さん、こんにちは"),
            "教育 liscov=りすこふ)さん、こんにちは"
        );
    }

    #[test]
    fn keeps_text_when_paren_does_not_follow_keyword() {
        assert_eq!(
            neutralize_dictionary_commands("山田太郎さん、教育について(雑談)"),
            "山田太郎さん、教育について(雑談)"
        );
        assert_eq!(
            neutralize_dictionary_commands("山田太郎さん、(教育)"),
            "山田太郎さん、(教育)"
        );
    }

    // 04_tts.md「棒読みちゃん連携」: text は教育・忘却コマンドを効かなくしてから URL エンコードする
    #[test]
    fn talk_url_sends_neutralized_text() {
        let backend = BouyomichanBackend::new(BouyomichanConfig::default());
        let url = backend.build_talk_url("山田太郎さん、教育(liscov=りすこふ)");
        let expected_text = urlencoding::encode("山田太郎さん、教育 liscov=りすこふ)");
        assert!(
            url.contains(&format!("/Talk?text={}&", expected_text)),
            "url: {}",
            url
        );
    }
}
