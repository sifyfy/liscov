//! プッシュ通知の受信（接続ごとに 1 本。02_chat.md「プッシュ通知」、ADR-008）
//!
//! 通知が届いたら、最後に受けた通知の発行時刻を watch チャネルで監視ループに渡す（監視ループはそれを合図に取りに行く）。
//! 壊れたら間を空けて最初からつなぎ直す。取得はポーリングで続いているので、ここで諦めたり切断したりはしない。
//! セッションは約 18 分で黙って通知を出さなくなるので、15 分で待たずに最初からつなぎ直す（張り直し。ADR-008 の追記）。

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
/// 通知のセッションを張り直す間隔（02_chat.md「設定値」）
const RENEW_AFTER: Duration = Duration::from_secs(15 * 60);

/// 1 つのセッションの終わり方
enum SessionEnd {
    /// 張り直しの時刻になった。そのセッションで受けた通知の件数を持つ
    Renew { notices: u64 },
    /// 壊れた
    Broken(anyhow::Error),
}

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
    enabled: watch::Receiver<bool>,
    cancel: CancellationToken,
    notices: watch::Sender<Option<String>>,
    connection_id: u64,
) {
    watch_until_cancelled(client, enabled, cancel, notices, connection_id, RENEW_AFTER).await;
}

/// `run_push_watch` の本体。張り直しの間隔を受け取る（テストで短くするため）
async fn watch_until_cancelled(
    client: SignalerClient,
    mut enabled: watch::Receiver<bool>,
    cancel: CancellationToken,
    notices: watch::Sender<Option<String>>,
    connection_id: u64,
    renew_after: Duration,
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

        let end = tokio::select! {
            _ = cancel.cancelled() => return,
            _ = until_disabled(&mut enabled) => continue,
            end = watch_session(&client, &notices, &mut backoff, connection_id, renew_after) => end,
        };
        let error = match end {
            SessionEnd::Renew { notices } => {
                tracing::info!(
                    "通知のセッションを張り直す（{:?} たった。受けた通知 {} 件） connection_id: {}",
                    renew_after,
                    notices,
                    connection_id
                );
                continue;
            }
            SessionEnd::Broken(error) => error,
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

/// 1 つのセッションで受け続ける。戻るのは張り直しの時刻になったときと、壊れたときだけ
async fn watch_session(
    client: &SignalerClient,
    notices: &watch::Sender<Option<String>>,
    backoff: &mut ReconnectBackoff,
    connection_id: u64,
    renew_after: Duration,
) -> SessionEnd {
    let renew_at = Instant::now() + renew_after;
    let session = match client.open().await {
        Ok(session) => session,
        Err(e) => return SessionEnd::Broken(e.context("通知のチャンネルを開けない")),
    };
    let mut received_notices: u64 = 0;
    let mut aid = 0;
    let mut token: Option<String> = None;
    let mut next_refresh = Instant::now() + REFRESH_INTERVAL;

    // 受信の接続はサーバーが約 4 分で閉じる。閉じたら同じセッションのまま開き直す
    loop {
        let mut response = match client.receive(&session, aid).await {
            Ok(response) => response,
            Err(e) => return SessionEnd::Broken(e.context("通知の受信を開けない")),
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
                            Err(e) => return SessionEnd::Broken(e),
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
                                    received_notices += 1;
                                    notices.send_replace(Some(publish_at_usec));
                                }
                                SignalerEvent::Alive => {}
                                SignalerEvent::Closed => {
                                    return SessionEnd::Broken(anyhow::anyhow!("通知のサーバーが閉じた"));
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        return SessionEnd::Broken(anyhow::Error::from(e).context("通知の受信が切れた"));
                    }
                },
                _ = tokio::time::sleep_until(renew_at) => {
                    return SessionEnd::Renew { notices: received_notices };
                }
                _ = tokio::time::sleep_until(last_received + SILENCE_LIMIT) => {
                    return SessionEnd::Broken(anyhow::anyhow!(
                        "通知の受信で {:?} 何も届かない",
                        SILENCE_LIMIT
                    ));
                }
                _ = tokio::time::sleep_until(next_refresh), if token.is_some() => {
                    if let Some(t) = &token
                        && let Err(e) = client.refresh_credentials(&session, t).await
                    {
                        return SessionEnd::Broken(e.context("通知の資格を更新できない"));
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

    // 02_chat.md「設定値」: 通知のセッションを張り直す間隔は 15分
    #[test]
    fn renew_interval_is_fifteen_minutes() {
        assert_eq!(RENEW_AFTER, Duration::from_secs(15 * 60));
    }

    /// 偽の通知のサーバー。サーバーを選んだ回数を数え、開いている受信に通知を送れる
    mod fake_signaler {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::{TcpListener, TcpStream};
        use tokio::sync::broadcast;

        pub struct FakeSignaler {
            pub base_url: String,
            choose_server: Arc<AtomicUsize>,
            notify: broadcast::Sender<String>,
        }

        fn frame(json: &str) -> String {
            format!("{}\n{}", json.len(), json)
        }

        impl FakeSignaler {
            pub async fn start() -> Self {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let base_url = format!("http://{}/punctual", listener.local_addr().unwrap());
                let choose_server = Arc::new(AtomicUsize::new(0));
                let (notify, _) = broadcast::channel(16);
                let (count, tx) = (Arc::clone(&choose_server), notify.clone());
                tokio::spawn(async move {
                    loop {
                        let (stream, _) = listener.accept().await.unwrap();
                        tokio::spawn(serve(stream, Arc::clone(&count), tx.subscribe()));
                    }
                });
                Self {
                    base_url,
                    choose_server,
                    notify,
                }
            }

            /// サーバーを選んだ回数（= 最初からつないだ回数）
            pub fn choose_server_count(&self) -> usize {
                self.choose_server.load(Ordering::SeqCst)
            }

            /// 開いている受信すべてに通知を送る
            pub fn send_notification(&self, publish_at: &str) {
                let _ = self.notify.send(publish_at.to_string());
            }
        }

        async fn serve(
            mut stream: TcpStream,
            choose_server: Arc<AtomicUsize>,
            mut notify: broadcast::Receiver<String>,
        ) {
            // ヘッダーと本文を読み切る（本文は使わない）
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            let header_end = loop {
                let n = stream.read(&mut chunk).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
            };
            let head = String::from_utf8_lossy(&buf[..header_end]).to_lowercase();
            let content_length: usize = head
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
            while buf.len() < header_end + content_length {
                let n = stream.read(&mut chunk).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            let request_line = head.lines().next().unwrap_or_default().to_string();

            let reply = |body: String| {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
            };
            if request_line.starts_with("post /punctual/v1/chooseserver") {
                choose_server.fetch_add(1, Ordering::SeqCst);
                let _ = stream
                    .write_all(reply(r#"["gsession",3,null,"1","1"]"#.to_string()).as_bytes())
                    .await;
            } else if request_line.starts_with("post /punctual/multi-watch/channel") {
                let body = frame(r#"[[0,["c","sid","",8,15,30000]]]"#);
                let _ = stream.write_all(reply(body).as_bytes()).await;
            } else if request_line.starts_with("post /punctual/v1/refreshcreds") {
                let _ = stream.write_all(reply("[]".to_string()).as_bytes()).await;
            } else if request_line.starts_with("get /punctual/multi-watch/channel") {
                // 受信: 開いたまま、トークンと購読の開始の後に通知を流す
                let first = frame(r#"[[1,[[null,null,["token"]]]],[2,[[[["1",[["1"]]]]]]]]"#);
                let head = "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n";
                if stream
                    .write_all(format!("{head}{first}").as_bytes())
                    .await
                    .is_err()
                {
                    return;
                }
                let mut aid = 3;
                while let Ok(publish_at) = notify.recv().await {
                    let entry = format!(
                        r#"[[{aid},[[[["1",[null,[[[null,"v","{publish_at}","0",[],[],0,0]]]]]]]]]]"#
                    );
                    if stream.write_all(frame(&entry).as_bytes()).await.is_err() {
                        return;
                    }
                    aid += 1;
                }
            }
        }
    }

    /// 通知の合図が `publish_at` になるまで待つ（2 秒で諦める）
    async fn wait_for_notice(rx: &mut watch::Receiver<Option<String>>, publish_at: &str) -> bool {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if rx.borrow_and_update().as_deref() == Some(publish_at) {
                    return;
                }
                rx.changed().await.unwrap();
            }
        })
        .await
        .is_ok()
    }

    /// 本番と同じ受信処理を、張り直しの間隔だけ短くして偽のサーバーに向けて回す
    async fn start_watch(
        server: &fake_signaler::FakeSignaler,
        renew_after: Duration,
    ) -> (CancellationToken, watch::Receiver<Option<String>>) {
        let client = SignalerClient::with_base_url("key", "video", server.base_url.clone());
        let (_enabled_tx, enabled_rx) = watch::channel(true);
        let (notices_tx, notices_rx) = watch::channel(None);
        let cancel = CancellationToken::new();
        tokio::spawn(watch_until_cancelled(
            client,
            enabled_rx,
            cancel.clone(),
            notices_tx,
            1,
            renew_after,
        ));
        // 設定の送り手を落としても、受信は続く（送り手が無くなったら待ち続ける）
        (cancel, notices_rx)
    }

    // 02_chat.md「壊れたと判断する条件とつなぎ直し」:
    // 通知のセッションを開いてから張り直しの間隔がたつと、待たずに（30 秒待たずに）最初からつなぎ直す
    #[tokio::test]
    async fn renews_session_without_waiting_after_interval() {
        let server = fake_signaler::FakeSignaler::start().await;
        let (cancel, mut notices) = start_watch(&server, Duration::from_secs(1)).await;

        server.send_notification("100");
        // 受信を開く前に送った通知は届かないので、届くまで送り直す
        let mut delivered = false;
        for _ in 0..20 {
            if wait_for_notice(&mut notices, "100").await {
                delivered = true;
                break;
            }
            server.send_notification("100");
        }
        assert!(delivered, "張り直しの前は通知が届く");

        tokio::time::sleep(Duration::from_millis(2500)).await;
        assert!(
            server.choose_server_count() >= 2,
            "1 秒ごとに最初からつなぎ直す（壊れたときの 30 秒の待ちを挟まない）"
        );
        cancel.cancel();
    }

    // 02_chat.md「プッシュ通知」: 張り直した後のセッションでも通知が届き、監視タスクへの合図になる
    #[tokio::test]
    async fn notifications_arrive_after_renewal() {
        let server = fake_signaler::FakeSignaler::start().await;
        let (cancel, mut notices) = start_watch(&server, Duration::from_secs(1)).await;

        // 2 回目のつなぎ直し（= 張り直し）が済むまで待つ
        tokio::time::timeout(Duration::from_secs(5), async {
            while server.choose_server_count() < 2 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("張り直す");

        let mut delivered = false;
        for _ in 0..20 {
            server.send_notification("200");
            if wait_for_notice(&mut notices, "200").await {
                delivered = true;
                break;
            }
        }
        assert!(delivered, "張り直した後も通知が届く");
        cancel.cancel();
    }
}
