# WebSocket API ドキュメント

liscovはWebSocket APIを提供し、外部アプリケーションがリアルタイムでチャットメッセージを受信できるようにします。

## 概要

| 項目 | 値 |
|------|-----|
| エンドポイント | `ws://127.0.0.1:8765` |
| プロトコル | WebSocket (RFC 6455) |
| メッセージ形式 | JSON |
| 認証 | なし（ローカルホストのみ） |

## 接続

### 接続例

```javascript
const ws = new WebSocket('ws://127.0.0.1:8765');

ws.onopen = () => {
  console.log('Connected to liscov');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);
};

ws.onclose = () => {
  console.log('Disconnected from liscov');
};
```

### 接続時の応答

接続が成功すると、サーバーは `Connected` メッセージを送信します：

```json
{
  "type": "Connected",
  "data": {
    "client_id": 1
  }
}
```

---

## サーバー → クライアント メッセージ

### ChatMessage

チャットメッセージを受信したときに送信されます。

```json
{
  "type": "ChatMessage",
  "data": {
    "id": "ChwKGkNQSFMxLXdVc2RfWV9...",
    "timestamp": "15:30:45",
    "timestamp_usec": "1735432245123456",
    "message_type": "Text",
    "author": "視聴者A",
    "author_icon_url": "https://yt4.ggpht.com/...",
    "channel_id": "UCxxxxxxxxxxxx",
    "content": "こんにちは！",
    "runs": [
      { "Text": { "content": "こんにちは！" } }
    ],
    "metadata": {
      "amount": null,
      "badges": ["メンバー（1年）"],
      "badge_info": [
        {
          "tooltip": "メンバー（1年）",
          "image_url": "https://yt3.ggpht.com/..."
        }
      ],
      "color": null,
      "is_moderator": false,
      "is_verified": false
    },
    "is_member": true,
    "comment_count": 5
  }
}
```

#### フィールド説明

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `id` | string | メッセージの一意識別子。重複排除に使用 |
| `timestamp` | string | 表示用タイムスタンプ (HH:MM:SS) |
| `timestamp_usec` | string | マイクロ秒精度のUnix時間。ソート用 |
| `message_type` | string/object | メッセージ種別（後述） |
| `author` | string | 投稿者名 |
| `author_icon_url` | string? | 投稿者アイコンURL |
| `channel_id` | string | 投稿者のYouTubeチャンネルID |
| `content` | string | メッセージ本文（プレーンテキスト） |
| `runs` | array | メッセージ要素（テキスト/絵文字）の配列 |
| `metadata` | object? | メタデータ（バッジ、金額等） |
| `is_member` | boolean | メンバーシップ加入者か |
| `comment_count` | number? | この配信での投稿者のコメント回数 |

#### message_type の種類

| 値 | 説明 |
|----|------|
| `"Text"` | 通常のテキストメッセージ |
| `{ "SuperChat": { "amount": "¥500" } }` | スーパーチャット |
| `{ "SuperSticker": { "amount": "¥200" } }` | スーパーステッカー |
| `"Membership"` | 新規メンバーシップ加入 |
| `"System"` | システムメッセージ |

#### runs の構造

```json
// テキスト
{ "Text": { "content": "こんにちは" } }

// 絵文字/スタンプ
{
  "Emoji": {
    "emoji_id": "UC...",
    "image_url": "https://...",
    "alt_text": ":smile:"
  }
}
```

#### metadata の構造

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `amount` | string? | スパチャ/ステッカーの金額（例: "¥500"） |
| `badges` | string[] | バッジ名のリスト |
| `badge_info` | array | バッジ詳細情報 |
| `color` | string? | メッセージの色（スパチャ等） |
| `is_moderator` | boolean | モデレーターか |
| `is_verified` | boolean | 認証済みアカウントか |

### Connected

クライアント接続時に送信されます。

```json
{
  "type": "Connected",
  "data": {
    "client_id": 1
  }
}
```

### ServerInfo

`GetInfo` リクエストへの応答として送信されます。

```json
{
  "type": "ServerInfo",
  "data": {
    "version": "0.1.0",
    "connected_clients": 3
  }
}
```

### Error

エラー発生時に送信されます。

```json
{
  "type": "Error",
  "data": {
    "message": "エラーの説明"
  }
}
```

---

## クライアント → サーバー メッセージ

### Ping

接続確認用。サーバーはPongフレームで応答します。

```json
{ "type": "Ping" }
```

### GetInfo

サーバー情報をリクエストします。

```json
{ "type": "GetInfo" }
```

---

## 使用例

### Python

```python
import asyncio
import json
import websockets

async def monitor_chat():
    uri = "ws://127.0.0.1:8765"

    async with websockets.connect(uri) as ws:
        async for message in ws:
            data = json.loads(message)

            if data['type'] == 'ChatMessage':
                msg = data['data']
                print(f"[{msg['timestamp']}] {msg['author']}: {msg['content']}")

                # スパチャ検出
                if isinstance(msg['message_type'], dict):
                    if 'SuperChat' in msg['message_type']:
                        amount = msg['message_type']['SuperChat']['amount']
                        print(f"  💰 スーパーチャット: {amount}")

                # メンバー検出
                if msg['is_member']:
                    print(f"  ⭐ メンバー")

asyncio.run(monitor_chat())
```

### JavaScript/Node.js

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://127.0.0.1:8765');

ws.on('message', (data) => {
  const message = JSON.parse(data);

  if (message.type === 'ChatMessage') {
    const { author, content, message_type, is_member, metadata } = message.data;

    console.log(`${author}: ${content}`);

    // スパチャ判定
    if (message_type?.SuperChat) {
      console.log(`  💰 ${message_type.SuperChat.amount}`);
    }

    // メンバーランク取得（バッジから）
    if (is_member && metadata?.badges) {
      const memberBadge = metadata.badges.find(b => b.includes('メンバー'));
      if (memberBadge) {
        console.log(`  ⭐ ${memberBadge}`);
      }
    }
  }
});
```

### 参加型配信での使用例

```python
import asyncio
import json
import websockets

participants = {}  # channel_id -> 参加情報

async def participation_manager():
    uri = "ws://127.0.0.1:8765"

    async with websockets.connect(uri) as ws:
        async for message in ws:
            data = json.loads(message)

            if data['type'] != 'ChatMessage':
                continue

            msg = data['data']
            channel_id = msg['channel_id']
            content = msg['content'].lower()

            # 参加コマンド検出
            if '参加' in content or '!join' in content:
                # メンバー限定チェック
                if not msg['is_member']:
                    print(f"{msg['author']} はメンバーではないため参加できません")
                    continue

                participants[channel_id] = {
                    'name': msg['author'],
                    'joined_at': msg['timestamp_usec'],
                    'is_member': msg['is_member']
                }
                print(f"✅ {msg['author']} が参加しました")

            # 離脱コマンド検出
            elif '離脱' in content or '!leave' in content:
                if channel_id in participants:
                    del participants[channel_id]
                    print(f"👋 {msg['author']} が離脱しました")

asyncio.run(participation_manager())
```

---

## メッセージの順序保証

`timestamp_usec` フィールドはマイクロ秒精度のUnix時間を文字列で提供します。
外部アプリケーションでメッセージを正しい順序でソートするには、このフィールドを使用してください。

```python
# メッセージリストを時系列でソート
messages.sort(key=lambda m: int(m['timestamp_usec']))
```

---

## 重複メッセージの処理

ネットワーク状況によっては同じメッセージが複数回配信される可能性があります。
`id` フィールドを使用して重複を排除してください。

```python
seen_ids = set()

def process_message(msg):
    if msg['id'] in seen_ids:
        return  # 重複をスキップ
    seen_ids.add(msg['id'])

    # メッセージを処理
    handle_chat(msg)
```

---

## 注意事項

1. **ローカル接続のみ**: セキュリティ上の理由から、WebSocketサーバーは `127.0.0.1` でのみリッスンします。

2. **認証なし**: ローカルホスト接続のため認証は実装されていません。

3. **再接続**: 接続が切断された場合は、クライアント側で再接続ロジックを実装してください。

4. **メッセージ量**: ライブ配信中は大量のメッセージが配信される可能性があります。適切なバッファリングを実装してください。

5. **liscov起動**: WebSocket APIはliscovアプリケーション起動時に自動的に開始されます。liscovが起動していない場合は接続できません。
