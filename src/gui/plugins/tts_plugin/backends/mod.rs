//! TTSバックエンド実装

pub mod bouyomichan;
pub mod voicevox;

use async_trait::async_trait;

use super::error::TtsError;

pub use bouyomichan::BouyomichanBackend;
pub use voicevox::VoicevoxBackend;

/// TTSバックエンドトレイト
#[async_trait]
pub trait TtsBackend: Send + Sync {
    /// 接続テスト
    async fn test_connection(&self) -> Result<bool, TtsError>;

    /// テキストを読み上げ
    async fn speak(&self, text: &str) -> Result<(), TtsError>;

    /// バックエンド名を取得
    fn name(&self) -> &'static str;
}
