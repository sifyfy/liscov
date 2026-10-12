//! プッシュ通知の受信（接続ごとに 1 本。02_chat.md「プッシュ通知」、ADR-008）
//!
//! 通知が届いたら、最後に受けた通知の発行時刻を watch チャネルで監視ループに渡す（監視ループはそれを合図に取りに行く）。
//! 壊れたら間を空けて最初からつなぎ直す。取得はポーリングで続いているので、ここで諦めたり切断したりはしない。

use std::time::Duration;
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::core::api::{FrameDecoder, SignalerClient, SignalerEvent};

/// 受信の接続でこれだけ何も届かなければ壊れたとみなす（02_chat.md「設定値」）
const SILENCE_LIMIT: Duration = Duration::from_secs(60);
/// 資格の更新の間隔（02_chat.md「設定値」）
const REFRESH_INTERVAL: Duration = Duration::from_secs(240);
/// つなぎ直すまでの最初の待ち（02_chat.md「設定値」）
const RECONNECT_FIRST_WAIT: Duration = Duration::from_secs(30);
/// つなぎ直すまでの待ちの上限（02_chat.md「設定値」）
const RECONNECT_MAX_WAIT: Duration = Duration::from_secs(300);

/// 壊れたときに、つなぎ直すまでの待ち（02_chat.md「壊れたと判断する条件とつなぎ直し」）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReconnectBackoff {
    wait: Duration,
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self {
            wait: RECONNECT_FIRST_WAIT,
        }
    }
}

impl ReconnectBackoff {
    /// 次につなぎ直すまでの待ち
    fn wait(&self) -> Duration {
        self.wait
    }

    /// つなぎ直しに失敗した後の状態（待ちを倍にし、上限で止める）
    #[must_use]
    fn after_failure(self) -> Self {
        Self {
            wait: (self.wait * 2).min(RECONNECT_MAX_WAIT),
        }
    }
}

/// プッシュ通知を受け続ける。`cancel` で止まる
///
/// `enabled` が false の間は受けない（設定 `chat_fetch.push_notification`。切り替えはすぐ効く）。
/// 無効にしたときは `notices` を None に戻し、取得のリクエストに発行時刻を付けないようにする。
pub async fn run_push_watch(
    client: SignalerClient,
    mut enabled: watch::Receiver<bool>,
    cancel: CancellationToken,
    notices: watch::Sender<Option<String>>,
    connection_id: u64,
) {
    let mut backoff = ReconnectBackoff::default();
    loop {
        if !*enabled.borrow_and_update() {
            notices.send_replace(None);
            tracing::info!("プッシュ通知は無効 connection_id: {}", connection_id);
            tokio::select! {
                _ = cancel.cancelled() => return,
                changed = enabled.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    continue;
                }
            }
        }

        let error = tokio::select! {
            _ = cancel.cancelled() => return,
            _ = until_disabled(&mut enabled) => continue,
            error = watch_session(&client, &notices, &mut backoff, connection_id) => error,
        };
        tracing::warn!(
            "プッシュ通知が壊れた（{:?} 後につなぎ直す。取得はポーリングで続く） connection_id: {}: {:#}",
            backoff.wait(),
            connection_id,
            error
        );

        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = until_disabled(&mut enabled) => continue,
            _ = tokio::time::sleep(backoff.wait()) => {}
        }
        backoff = backoff.after_failure();
    }
}

/// 設定が無効になるまで待つ（送り手が無くなったら待ち続ける）
async fn until_disabled(enabled: &mut watch::Receiver<bool>) {
    loop {
        if enabled.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
        if !*enabled.borrow_and_update() {
            return;
        }
    }
}

/// 1 つのセッションで受け続ける。戻るのは壊れたときだけ
async fn watch_session(
    client: &SignalerClient,
    notices: &watch::Sender<Option<String>>,
    backoff: &mut ReconnectBackoff,
    connection_id: u64,
) -> anyhow::Error {
    let session = match client.open().await {
        Ok(session) => session,
        Err(e) => return e.context("通知のチャンネルを開けない"),
    };
    let mut aid = 0;
    let mut token: Option<String> = None;
    let mut next_refresh = Instant::now() + REFRESH_INTERVAL;

    // 受信の接続はサーバーが約 4 分で閉じる。閉じたら同じセッションのまま開き直す
    loop {
        let mut response = match client.receive(&session, aid).await {
            Ok(response) => response,
            Err(e) => return e.context("通知の受信を開けない"),
        };
        let mut decoder = FrameDecoder::default();
        let mut last_received = Instant::now();
        loop {
            tokio::select! {
                chunk = response.chunk() => match chunk {
                    Ok(Some(bytes)) => {
                        last_received = Instant::now();
                        let events = match decoder.push(&bytes) {
                            Ok(events) => events,
                            Err(e) => return e,
                        };
                        for received in events {
                            aid = received.aid;
                            match received.event {
                                SignalerEvent::CredentialToken(t) => token = Some(t),
                                SignalerEvent::Subscribed => {
                                    *backoff = ReconnectBackoff::default();
                                    tracing::info!("プッシュ通知を受け始めた connection_id: {}", connection_id);
                                }
                                SignalerEvent::Notified { publish_at_usec } => {
                                    notices.send_replace(Some(publish_at_usec));
                                }
                                SignalerEvent::Alive => {}
                                SignalerEvent::Closed => {
                                    return anyhow::anyhow!("通知のサーバーが閉じた");
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => return anyhow::Error::from(e).context("通知の受信が切れた"),
                },
                _ = tokio::time::sleep_until(last_received + SILENCE_LIMIT) => {
                    return anyhow::anyhow!("通知の受信で {:?} 何も届かない", SILENCE_LIMIT);
                }
                _ = tokio::time::sleep_until(next_refresh), if token.is_some() => {
                    if let Some(t) = &token
                        && let Err(e) = client.refresh_credentials(&session, t).await
                    {
                        return e.context("通知の資格を更新できない");
                    }
                    next_refresh = Instant::now() + REFRESH_INTERVAL;
                }
            }
        }
        tracing::debug!(
            "通知の受信をサーバーが閉じたので開き直す connection_id: {} AID: {}",
            connection_id,
            aid
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waits_after_failures(count: usize) -> Vec<u64> {
        std::iter::successors(Some(ReconnectBackoff::default()), |b| {
            Some(b.after_failure())
        })
        .take(count)
        .map(|b| b.wait().as_secs())
        .collect()
    }

    // 02_chat.md「壊れたと判断する条件とつなぎ直し」: 30 → 60 → 120 → 240 → 300 → 300 …秒
    #[test]
    fn reconnect_wait_doubles_up_to_five_minutes() {
        assert_eq!(
            waits_after_failures(7),
            vec![30, 60, 120, 240, 300, 300, 300]
        );
    }
}
