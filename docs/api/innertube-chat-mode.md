# InnerTube API: チャットモード切替仕様

このドキュメントは、YouTube InnerTube APIにおけるライブチャットモード（トップチャット/すべてのチャット）の切替機能について、調査で判明した仕様をまとめたものです。

## 概要

YouTubeライブチャットには2つの表示モードがあります：

| モード | 説明 | chattype値 |
|--------|------|------------|
| TopChat (トップチャット) | フィルタリングされた重要なメッセージのみ表示 | `0x04` (4) |
| AllChat (すべてのチャット) | すべてのメッセージを表示 | `0x01` (1) |

## Continuation Tokenの種類

YouTubeは2種類のcontinuation tokenを使用しています：

### 1. Main Continuation Token（メイントークン）

- **用途**: InnerTube APIへのリクエストに使用
- **長さ**: 約180〜400文字（Base64エンコード後）
- **取得元**:
  - 初回: HTMLページの`ytInitialData`から抽出
  - 以降: API応答の`continuations`フィールドから取得
- **特徴**: API応答から取得したトークンにはchattype情報が含まれる

### 2. Reload Continuation Token（リロードトークン）

- **用途**: ページリロード/モード切替のナビゲーション用
- **長さ**: 約32文字（Base64エンコード後）
- **取得元**: HTMLページの`subMenuItems`配列から抽出
- **特徴**: InnerTube APIで直接使用すると400 Bad Requestエラー

## Token構造（Protocol Buffer形式）

### Reload Tokenのバイナリ構造

```
位置  バイト列        説明
----  -------------  --------------------------------
0-4   d2 87 cc c8 03  ヘッダー
5-7   12 1a 00        Field 2
8-9   30 01           Field 6
10-12 82 01 08        Field 16 (length-delimited, 長さ8)
13-14 08 04           Field 1 = chattype (04=TopChat, 01=AllChat)
15-17 18 00 20 00     その他フィールド
18-20 28 01 a8 01 01  末尾フィールド
```

**TopChat トークン例**:
```
Base64: 0ofMyAMSGgAwAYIBCAgEGAAgACgBqAEB
Bytes:  d2 87 cc c8 03 12 1a 00 30 01 82 01 08 08 04 18 00 20 00 28 01 a8 01 01
                                                  ^^
                                                  chattype = 4
```

**AllChat トークン例**:
```
Base64: 0ofMyAMSGgAwAYIBCAgBGAAgACgBqAEB
Bytes:  d2 87 cc c8 03 12 1a 00 30 01 82 01 08 08 01 18 00 20 00 28 01 a8 01 01
                                                  ^^
                                                  chattype = 1
```

### Main Tokenのバイナリ構造

初回ページ取得時のメイントークンは、リロードトークンとは異なる構造を持ちます：

```
位置  バイト列        説明
----  -------------  --------------------------------
0-4   d2 87 cc c8 03  ヘッダー（共通）
5-6   80 01           Field 16 (varint形式)
7+    ...             可変長データ
```

**重要**: 初回取得のメイントークンにはchattype情報が含まれていない場合があります。しかし、API応答から取得したメイントークンにはchattype情報が含まれています。

## モード切替の実装方法

### 方法1: バイナリ変換（推奨）

API応答から取得したメイントークンに対して、chattypeフィールドを直接書き換える方法です。

```rust
// Field 16パターンを検索: 0x82 0x01 + length + 0x08 + chattype
for i in 0..data.len() - 4 {
    if data[i] == 0x82 && data[i + 1] == 0x01 {
        let len = data[i + 2] as usize;
        if data[i + 3] == 0x08 {
            let chattype = data[i + 4];
            if chattype == 0x01 || chattype == 0x04 {
                data[i + 4] = new_chattype; // 書き換え
            }
        }
    }
}
```

### 方法2: ページ再取得（フォールバック）

リロードトークンを使用してlive_chatページを再取得し、新しいメイントークンを抽出する方法です。

```
URL形式: https://www.youtube.com/live_chat?continuation={RELOAD_TOKEN}
```

この方法では、指定したモード用の新しいメイントークンが取得できます。

## subMenuItemsの構造

HTMLページ内の`subMenuItems`からリロードトークンを抽出できます：

```json
"subMenuItems": [
  {
    "title": "トップチャット",
    "selected": true,
    "continuation": {
      "reloadContinuationData": {
        "continuation": "0ofMyAMSGgAwAYIBCAgEGAAgACgBqAEB"
      }
    }
  },
  {
    "title": "チャット",
    "selected": false,
    "continuation": {
      "reloadContinuationData": {
        "continuation": "0ofMyAMSGgAwAYIBCAgBGAAgACgBqAEB"
      }
    }
  }
]
```

**タイトルのバリエーション**:
- TopChat: "Top chat", "トップチャット", "トップのチャット", "上位のチャット"
- AllChat: "Live chat", "チャット", "すべてのチャット"

## API エンドポイント

### ライブチャットメッセージ取得

```
POST https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={API_KEY}

Request Body:
{
  "context": {
    "client": {
      "clientName": "WEB",
      "clientVersion": "2.20251222.04.00"
    }
  },
  "continuation": "{MAIN_CONTINUATION_TOKEN}"
}
```

**注意**: このエンドポイントにリロードトークンを送信すると、400 Bad Request（"Request contains an invalid argument"）エラーが返されます。

## 実装上の注意点

1. **初回接続時**: メイントークンにchattype情報がない可能性があるため、バイナリ変換が失敗することがあります

2. **API応答後**: API応答から取得したトークンにはchattype情報が含まれるため、バイナリ変換が成功します

3. **リロードトークンの直接使用**: APIでは使用不可。ページ取得URLのパラメータとしてのみ使用可能

4. **モード検出**: トークンのchattype値（0x04または0x01）を読み取ることで現在のモードを検出できます

## 関連ファイル

- `src/api/continuation_builder.rs`: バイナリ変換ロジック
- `src/api/innertube.rs`: InnerTubeクライアント実装
- `src/api/youtube.rs`: HTMLからのトークン抽出ロジック

## 参考情報

- Protocol Bufferのvarint/タグエンコーディング仕様
- YouTubeのsubMenuItems構造は言語設定によりタイトルが変わる
