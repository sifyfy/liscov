//! TTS読み上げキュー

use std::sync::Arc;
use tokio::sync::mpsc;

use super::backends::TtsBackend;
use super::error::TtsError;

/// 読み上げメッセージ
#[derive(Debug, Clone)]
pub struct TtsMessage {
    /// 読み上げテキスト
    pub text: String,
    /// 優先度
    pub priority: TtsPriority,
}

/// 読み上げ優先度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TtsPriority {
    /// 通常
    Normal = 0,
    /// スーパーチャット
    SuperChat = 1,
    /// メンバーシップ
    Membership = 2,
}

/// TTSキュー
pub struct TtsQueue {
    sender: mpsc::Sender<TtsMessage>,
}

impl TtsQueue {
    /// 新しいキューを作成し、処理タスクを開始
    pub fn new(
        backend: Arc<dyn TtsBackend>,
        queue_size: usize,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        let (sender, receiver) = mpsc::channel(queue_size);

        let handle = tokio::spawn(Self::process_queue(receiver, backend));

        (Self { sender }, handle)
    }

    /// メッセージをキューに追加
    pub async fn enqueue(&self, message: TtsMessage) -> Result<(), TtsError> {
        self.sender
            .send(message)
            .await
            .map_err(|_| TtsError::QueueFull)?;
        Ok(())
    }

    /// キューが満杯かどうか（非同期チェック不可のため常にfalseを返す）
    pub fn is_full(&self) -> bool {
        // mpscではcapacityチェックが直接できないため、送信時にエラーで判断
        false
    }

    /// キュー処理タスク
    async fn process_queue(
        mut receiver: mpsc::Receiver<TtsMessage>,
        backend: Arc<dyn TtsBackend>,
    ) {
        tracing::info!("🔊 TTS読み上げキュー処理を開始");

        while let Some(message) = receiver.recv().await {
            tracing::debug!(
                "📢 読み上げ開始: {:?} - {}",
                message.priority,
                &message.text[..message.text.len().min(50)]
            );

            match backend.speak(&message.text).await {
                Ok(()) => {
                    tracing::debug!("✅ 読み上げ完了");
                }
                Err(e) => {
                    tracing::error!("❌ 読み上げエラー: {}", e);
                }
            }
        }

        tracing::info!("🔊 TTS読み上げキュー処理を終了");
    }
}

impl Clone for TtsQueue {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(TtsPriority::Normal < TtsPriority::SuperChat);
        assert!(TtsPriority::SuperChat < TtsPriority::Membership);
    }

    #[test]
    fn test_tts_message_creation() {
        let msg = TtsMessage {
            text: "テスト".to_string(),
            priority: TtsPriority::Normal,
        };
        assert_eq!(msg.text, "テスト");
        assert_eq!(msg.priority, TtsPriority::Normal);
    }
}
