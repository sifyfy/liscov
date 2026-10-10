# WebSocket API機能

## 目的（Why）

OBSオーバーレイ、カスタムボット、外部分析ツールなど、外部アプリケーションにリアルタイムチャットデータを提供する。配信者の既存ワークフロー（OBSシーン等）にチャット情報を統合可能にする。

## 振る舞い（What）

### サーバーライフサイクル

| イベント | 結果 |
|---------|------|
| アプリ起動 | WebSocketサーバーがポート8765で自動起動。使用中の場合は8766〜8774を順に試行 |
| 全ポート使用中 | エラーログ出力。アプリは正常起動を継続（WebSocket機能のみ無効） |
| アプリ終了 | サーバーも自動終了 |
| 環境変数 `LISCOV_WEBSOCKET_PORT` を設定して起動（E2E テスト用） | 試すポートが変わる（下の「開始ポートの上書き」）。設定しなければ上の 8765〜8774 のまま |

### 開始ポートの上書き（E2E テスト用）

E2E のアプリが 8765 を取ると、本番の liscov 向けに 8765 へつなぎに来るクライアント（OBS のブラウザソースのオーバーレイなど）が
E2E のアプリにつながる。テストの架空のコメントが配信画面に流れうるうえ、接続数を数えるテストが落ちる。
これを防ぐため、E2E は `LISCOV_WEBSOCKET_PORT=0` でアプリを起動し、OS が選ぶ空きポートで WebSocket サーバーを動かす（ADR-005）。

| `LISCOV_WEBSOCKET_PORT` | 試すポート |
|-------------------------|-----------|
| 未設定 | 8765〜8774（本番の既定。上の「ポート自動選択」） |
| `0` | OS が選ぶ空きポート 1 つだけ。`actual_port` とステータス表示は実際に選ばれた番号（例: `WS:53124(0)`） |
| `9000` | 9000〜9009 を順に試す |
| `65530` | 65530〜65535 を順に試す（65535 で打ち切る） |
| `abc`・`-1`・`65536`・空文字 | 警告ログを出し、未設定と同じ（8765〜8774） |

- 制約「ポート範囲は8765〜8774」は、この環境変数を設定しないとき（本番）の約束。E2E がこの環境変数で範囲の外を使うのは、
  `LISCOV_APP_NAME` でデータの置き場所を分けるのと同じテストの分離で、本番の振る舞いは変わらない
- 本番の利用者に案内する設定ではない（オーバーレイ側の接続先を変える必要が出るため）。設定画面には出さない

### クライアント接続

| 操作 | 結果 |
|------|------|
| `ws://127.0.0.1:{port}` に接続 | `Connected` メッセージ（client_id付き）を受信 |
| 接続中にチャットメッセージ受信 | 全クライアントに `ChatMessage` をブロードキャスト |
| 接続中にギフト受信 | 全クライアントに `ChatMessage`（`message_type` = `Gift`）をブロードキャスト。新しいメッセージ種別は作らない |
| 接続中にライブリアクションの更新を受信 | 全クライアントに `Reaction` をブロードキャスト（0件の更新・再送は送らない） |
| `GetInfo` を送信 | `ServerInfo`（バージョン、接続クライアント数）を受信 |
| 接続直後 | **過去メッセージは送信されない**。接続後の新着メッセージのみ |

## 制約・不変条件（Boundaries）

| 制約 | 理由 |
|------|------|
| バインドアドレスは `127.0.0.1`（ローカルホストのみ） | 認証機構がないため、外部ネットワークからのアクセスを防ぐセキュリティ要件 |
| TLS/SSL非対応 | ローカル通信のため暗号化は不要 |
| 過去メッセージは送信しない | シンプルな設計を優先。履歴が必要な場合はDB API経由で取得 |
| WebSocketはコア `ChatMessage` 構造体をブロードキャストし、GUI向け `GuiChatMessage` とはフィールドが異なる | WebSocket APIとフロントエンドは別の消費者であり、それぞれに最適な形式を提供する |
| ポート範囲は8765〜8774（10ポート） | 複数インスタンス起動への対応と、ポート枯渇の妥当な上限 |
| クライアントは知らない `type` のメッセージを無視する（プロトコルの約束） | サーバーが新しい種類（例: `Reaction`）を足しても既存のクライアントが壊れないようにする |

## サーバー設定

### バインドアドレス

- **アドレス**: `127.0.0.1`（ローカルホストのみ）
- **デフォルトポート**: `8765`
- **ポート範囲**: `8765` 〜 `8774`（自動フォールバック用）

### ポート自動選択

アプリ起動時、指定ポートが使用中の場合は自動的に次のポートを試行する。

```
1. ポート 8765 を試行
   ├─ 成功 → 8765 で起動
   └─ 失敗（使用中）
        ↓
2. ポート 8766 を試行
   ├─ 成功 → 8766 で起動
   └─ 失敗（使用中）
        ↓
   ... 繰り返し ...
        ↓
10. ポート 8774 を試行
    ├─ 成功 → 8774 で起動
    └─ 失敗 → エラーログ出力（アプリは起動継続）
```

### 開始ポートの決め方（実装）

- `commands/websocket.rs` の `websocket_start_port(value: Option<&str>) -> u16` が `LISCOV_WEBSOCKET_PORT` の値から開始ポートを決める
  （未設定・不正なら `8765`。不正なときの警告ログもここで出す）。`start_websocket_server_auto` はこの結果で `WebSocketServer::new` を呼ぶ
- `WebSocketServer::start` は開始ポートから 10 ポート（`u16` の上限で打ち切る）を試す。開始ポートが `0` のときは 1 回だけ bind し、
  返すポート番号は bind した listener の `local_addr()` から読む（`0` のまま返さない）

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `websocket_get_status` | なし | `WebSocketStatus` | 状態取得 |

### WebSocketStatus

```rust
pub struct WebSocketStatus {
    pub is_running: bool,
    pub actual_port: Option<u16>,
    pub connected_clients: u32,
}
```

## 自動起動

### 起動タイミング

- アプリケーション起動時（`setup` フック内）
- バックグラウンドでサーバーを起動

### 起動フロー

```
1. アプリケーション起動
        ↓
2. setup フックで WebSocket サーバー起動タスクをスポーン
        ↓
3. 開始ポート（既定 8765。「開始ポートの上書き」）から順に試行
        ↓
4. 成功: サーバー稼働開始、ポート番号を AppState に保存
   失敗: エラーログ出力、アプリは正常起動継続
```

### 終了時

- アプリケーション終了時に自動的にサーバーも終了

## Tauriイベント

| イベント | ペイロード | 発火タイミング |
|---------|-----------|---------------|
| `websocket-client-connected` | `{ client_id: u64 }` | クライアント接続時 |
| `websocket-client-disconnected` | `{ client_id: u64 }` | クライアント切断時 |

## メッセージプロトコル

### サーバー → クライアント（ServerMessage）

すべてのメッセージは `type` と `data` フィールドを持つJSON形式。

#### Connected

接続確立時に送信。

```json
{
  "type": "Connected",
  "data": {
    "client_id": 1
  }
}
```

#### ChatMessage

チャットメッセージ受信時にブロードキャスト。

> **注意**: WebSocketはコアの `ChatMessage` 構造体をブロードキャストする。フロントエンド向けTauriイベント（`chat:message`）で送られる `GuiChatMessage` とはフィールドが異なる（`connection_id`, `platform`, `broadcaster_name` はWebSocketには含まれない）。runs の直列化形式も異なる（コア: 外部タグ `{ "Text": { ... } }` / GUI: 内部タグ `{ "type": "Text", ... }`）。

```json
{
  "type": "ChatMessage",
  "data": {
    "id": "CjkKGkNQVG...",
    "timestamp": "12:34:56",
    "timestamp_usec": "1234567890000000",
    "message_type": "Text",
    "author": "視聴者名",
    "author_icon_url": "https://yt4.ggpht.com/...",
    "channel_id": "UCxxxxxxxxxxxx",
    "content": "こんにちは！",
    "runs": [
      { "Text": { "content": "こんにちは！" } }
    ],
    "metadata": {
      "amount": null,
      "badges": ["Member (6 months)"],
      "badge_info": [{ "badge_type": "member", "label": "Member", "tooltip": "Member (6 months)", "icon_url": "https://..." }],
      "color": null,
      "is_moderator": false,
      "is_verified": false,
      "superchat_colors": null
    },
    "is_member": true,
    "is_first_time_viewer": false,
    "in_stream_comment_count": 5
  }
}
```

**ギフトの例**（付加情報のある種別は `{ "種別": { ... } }` の形になる。`content` はギフト名）

```json
{
  "type": "ChatMessage",
  "data": {
    "message_type": { "Gift": { "gift_name": "Press F", "gift_image_url": "https://www.gstatic.com/youtube/img/pdg/gift/assets/press_f.png=w480-h480", "jewel_count": 10 } },
    "author": "@viewer-a1b",
    "channel_id": "UCxxxxxxxxxxxx",
    "content": "Press F",
    "runs": [],
    "is_member": false,
    "is_first_time_viewer": false,
    "in_stream_comment_count": 2
  }
}
```

視聴者を特定できないギフトは `channel_id` が `""`、`in_stream_comment_count` が null になる。

**runs フォーマット（InnerTube API準拠）**

```json
// テキスト
{ "Text": { "content": "テキスト内容" } }

// 絵文字
{ "Emoji": { "emoji_id": "🎉", "image_url": "https://...", "alt_text": ":party:" } }
```

#### Reaction

ライブリアクションの更新（[02_chat.md](02_chat.md#ライブリアクション)）を受けたときにブロードキャスト。前回の更新からの差分で、累計ではない。

```json
{
  "type": "Reaction",
  "data": {
    "broadcaster_channel_id": "UCxxxxxxxxxxxx",
    "update_time_usec": 1790422357025983,
    "duration_seconds": 2,
    "counts": { "❤": 3, "🎉": 5 },
    "total": 8
  }
}
```

| フィールド | 説明 |
|-----------|------|
| `broadcaster_channel_id` | どの配信のリアクションか（多接続時の区別用。分からなければ null） |
| `update_time_usec` | YouTube 側の更新時刻（マイクロ秒、数値） |
| `duration_seconds` | この更新が表す秒数 |
| `counts` | 絵文字 → 件数。0件の絵文字は含めない |
| `total` | `counts` の合計 |

#### ServerInfo

`GetInfo` リクエストへの応答。

```json
{
  "type": "ServerInfo",
  "data": {
    "version": "0.1.0",
    "connected_clients": 3
  }
}
```

#### Error

エラー発生時に送信。

```json
{
  "type": "Error",
  "data": {
    "message": "エラーの詳細"
  }
}
```

### クライアント → サーバー（ClientMessage）

#### Ping

接続維持確認。サーバーは `Pong` フレームで応答。

```json
{ "type": "Ping" }
```

#### GetInfo

サーバー情報をリクエスト。

```json
{ "type": "GetInfo" }
```

## データモデル

### ServerMessage（Rust）

```rust
#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Connected { client_id: u64 },
    ChatMessage(GuiChatMessage),
    Reaction { broadcaster_channel_id: Option<String>, #[serde(flatten)] update: ReactionUpdate },
    ServerInfo { version: String, connected_clients: u32 },
    Error { message: String },
}
```

### ClientMessage（Rust）

```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Ping,
    GetInfo,
}
```

### MessageType

チャットメッセージの種別。

| 値 | 説明 | 付加情報 |
|----|------|---------|
| `Text` | 通常メッセージ | なし |
| `SuperChat` | スーパーチャット | `amount` |
| `SuperSticker` | スーパーステッカー | `amount` |
| `Membership` | メンバーシップ | `milestone_months` |
| `MembershipGift` | ギフトメンバーシップ | `gift_count` |
| `Gift` | ジュエルで送るギフト | `gift_name`、`gift_image_url`（無ければ null）、`jewel_count`（取れなければ null） |
| `System` | システムメッセージ | なし |

## クライアント管理

### 接続フロー

```
1. クライアントが ws://127.0.0.1:8765 に接続
        ↓
2. WebSocket ハンドシェイク完了
        ↓
3. クライアントIDを発行（自動インクリメント）
        ↓
4. Connected メッセージを送信
        ↓
5. ブロードキャストチャネルに登録
        ↓
6. メッセージ受信ループ開始
```

### 切断処理

```
1. クライアントから Close フレーム受信
   または 接続エラー検出
        ↓
2. メッセージ受信ループ終了
        ↓
3. クライアントリストから削除
        ↓
4. websocket-client-disconnected イベント発火
```

### 複数クライアント

- 複数クライアントの同時接続をサポート
- 各クライアントに一意のIDを割り当て
- すべてのクライアントにメッセージをブロードキャスト

## メッセージ配信

### ブロードキャスト

チャットメッセージ受信時、接続中のすべてのクライアントに配信。

```
Chat Service
    │
    ├─→ クライアント A (client_id: 1)
    ├─→ クライアント B (client_id: 2)
    └─→ クライアント C (client_id: 3)
```

### 初回接続時の動作

- 過去メッセージは送信しない
- 接続後に受信したメッセージのみ配信

## エラーハンドリング

| エラー | 動作 |
|-------|------|
| ポート使用中 | 次のポートを自動試行（8765→8774） |
| 全ポート使用中 | エラーログ出力、アプリは起動継続（WebSocket機能のみ無効） |
| クライアント切断 | 自動クリーンアップ、他クライアントに影響なし |
| メッセージ送信失敗 | 対象クライアントをスキップ、他は継続 |
| 不正なメッセージ受信 | 警告ログ、接続は維持 |

## フロントエンド

### UI要素

| 要素 | 説明 |
|-----|------|
| ステータス表示 | ヘッダーに `WS:ポート番号(接続数)` 形式で表示 |

### 表示例

- 稼働中: `WS:8765(2)` （ポート8765で2クライアント接続中）
- 停止中: 非表示

### Store状態

```typescript
interface WebSocketStore {
    isRunning: boolean;
    actualPort: number | null;
    connectedClients: number;
}
```

## 使用例（外部クライアント）

### JavaScript

```javascript
const ws = new WebSocket('ws://127.0.0.1:8765');

ws.onopen = () => {
    console.log('Connected');
};

ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);

    switch (msg.type) {
        case 'Connected':
            console.log(`Client ID: ${msg.data.client_id}`);
            break;
        case 'ChatMessage':
            console.log(`${msg.data.author}: ${msg.data.content}`);
            break;
    }
};

// サーバー情報をリクエスト
ws.send(JSON.stringify({ type: 'GetInfo' }));
```

### Python

```python
import asyncio
import websockets
import json

async def connect():
    async with websockets.connect('ws://127.0.0.1:8765') as ws:
        async for message in ws:
            data = json.loads(message)
            if data['type'] == 'ChatMessage':
                print(f"{data['data']['author']}: {data['data']['content']}")

asyncio.run(connect())
```

## 制限事項

| 制限 | 理由 |
|------|------|
| ローカルホストのみ | セキュリティ（認証機構なし） |
| TLS/SSL非対応 | ローカル通信のため不要 |
| 過去メッセージ送信なし | シンプルな設計を優先 |
| クライアント認証なし | ローカル環境での使用を想定 |
