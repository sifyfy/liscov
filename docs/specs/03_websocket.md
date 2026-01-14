# WebSocket API機能

## 概要

外部アプリケーション（OBS、カスタムオーバーレイ、ボット等）にリアルタイムチャットデータを提供するローカルWebSocketサーバー。

## サーバー設定

### バインドアドレス

- **アドレス**: `127.0.0.1`（ローカルホストのみ）
- **デフォルトポート**: `8765`
- **ポート範囲**: `8765` 〜 `8774`（自動フォールバック用）

### ポート自動選択

指定ポートが使用中の場合、自動的に次のポートを試行する。

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
    └─ 失敗 → エラー返却
```

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `websocket_start` | `port: Option<u16>` | `{ actual_port: u16 }` | サーバー起動（デフォルト: 8765） |
| `websocket_stop` | なし | `()` | サーバー停止 |
| `websocket_get_status` | なし | `WebSocketStatus` | 状態取得 |

### WebSocketStatus

```rust
pub struct WebSocketStatus {
    pub is_running: bool,
    pub actual_port: Option<u16>,
    pub connected_clients: u32,
}
```

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
      { "type": "text", "text": "こんにちは！" }
    ],
    "metadata": {
      "amount": null,
      "badges": ["Member (6 months)"],
      "is_moderator": false,
      "is_verified": false
    },
    "is_member": true,
    "comment_count": 5
  }
}
```

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
| `System` | システムメッセージ | なし |

## サーバー状態

```rust
pub enum WebSocketState {
    Stopped,    // 停止中
    Starting,   // 起動処理中
    Running,    // 稼働中
    Stopping,   // 停止処理中
}
```

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
| 全ポート使用中 | エラーを返却、サーバー起動失敗 |
| クライアント切断 | 自動クリーンアップ、他クライアントに影響なし |
| メッセージ送信失敗 | 対象クライアントをスキップ、他は継続 |
| 不正なメッセージ受信 | 警告ログ、接続は維持 |

## フロントエンド

### UI要素

| 要素 | 説明 |
|-----|------|
| 起動/停止ボタン | サーバーの起動・停止を切り替え |
| ステータス表示 | 稼働状態（Running/Stopped） |
| ポート番号表示 | 実際に使用中のポート |
| 接続数表示 | 現在接続中のクライアント数 |

### 操作

| ユーザー操作 | 期待動作 |
|-------------|---------|
| 「Start Server」クリック | WebSocketサーバー起動、ポート番号表示 |
| 「Stop Server」クリック | サーバー停止、ボタンが「Start Server」に戻る |

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
