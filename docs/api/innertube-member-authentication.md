# InnerTube API: メンバー限定配信の認証方式

このドキュメントは、YouTube InnerTube APIを使用してメンバー限定ライブ配信のチャットを取得するための認証方式について調査した結果をまとめたものです。

## 概要

メンバー限定配信のチャットにアクセスするには、認証が必要です。InnerTubeで利用可能な認証方式は以下の2つがあります：

| 方式 | 説明 | 難易度 | 推奨度 |
|------|------|--------|--------|
| Cookie認証 | ブラウザからCookieを取得して使用 | 中 | ★★★ |
| OAuth2認証 | Google OAuth2フローで認証 | 高 | ★★☆ |

## 方式1: Cookie認証（推奨）

### 必要なCookie

YouTubeの認証に必要なCookieは以下の5つです：

| Cookie名 | 説明 |
|----------|------|
| `SID` | セッションID |
| `HSID` | HTTPセキュアセッションID |
| `SSID` | セキュアセッションID |
| `APISID` | API用セッションID |
| `SAPISID` | セキュアAPI用セッションID（SAPISIDHASH生成に使用） |

### SAPISIDHASH の生成

InnerTube APIの認証には `SAPISIDHASH` ヘッダーが必要です。これは以下の形式で生成します：

```
SAPISIDHASH = {timestamp}_{SHA1_HASH}
SHA1_HASH = SHA1("{timestamp} {SAPISID} {origin}")
```

#### 生成例（Rust擬似コード）

```rust
use sha1::{Sha1, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

fn generate_sapisidhash(sapisid: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let origin = "https://www.youtube.com";
    let input = format!("{} {} {}", timestamp, sapisid, origin);

    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    format!("{}_{}", timestamp, hash_hex)
}
```

#### Python実装例（参考）

```python
import time
import hashlib

def generate_sapisidhash(sapisid: str) -> str:
    timestamp = str(int(time.time()))
    origin = "https://www.youtube.com"

    hash_input = f"{timestamp} {sapisid} {origin}"
    hash_result = hashlib.sha1(hash_input.encode()).hexdigest()

    return f"{timestamp}_{hash_result}"
```

### APIリクエストへの適用

認証付きリクエストには以下のヘッダーを追加します：

```http
POST https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={API_KEY}

Headers:
  Authorization: SAPISIDHASH {sapisidhash}
  Cookie: SID=xxx; HSID=xxx; SSID=xxx; APISID=xxx; SAPISID=xxx
  X-Origin: https://www.youtube.com
  Origin: https://www.youtube.com
```

### Cookieの取得方法

1. **シークレット/プライベートウィンドウを開く**
   - 通常のウィンドウでは自動Cookie更新が発生するため

2. **YouTubeにログイン**
   - メンバーシップを持つアカウントでログイン

3. **DevToolsでCookieを取得**
   - F12 → Network タブ
   - youtube.com へのリクエストを選択
   - `Cookie` ヘッダーの値をコピー

4. **必要なCookieを抽出**
   - SID, HSID, SSID, APISID, SAPISID の5つを抽出

### 注意事項

- Cookieには有効期限があり、定期的な更新が必要
- セキュリティ上、Cookieは安全に保管する必要がある
- 自動更新の仕組みを実装する場合は複雑になる

## 方式2: OAuth2認証

### 概要

Google OAuth2を使用した認証方式です。YouTube Data API v3と組み合わせて使用します。

### 制限事項

> **重要**: 2024年時点で、OAuth2認証はInnerTube APIでは**TVクライアント**でのみ動作します。WEBクライアントではCookie認証が必要です。

### OAuth2フロー（TVクライアント用）

1. **デバイスコード取得**
   ```
   POST https://oauth2.googleapis.com/device/code

   client_id={tv_client_id}
   scope=https://www.googleapis.com/auth/youtube
   ```

2. **ユーザー認証**
   - 返却されたURLにアクセスし、コードを入力

3. **トークン取得**
   ```
   POST https://oauth2.googleapis.com/token

   client_id={tv_client_id}
   client_secret={tv_client_secret}
   device_code={device_code}
   grant_type=urn:ietf:params:oauth:grant_type:device_code
   ```

4. **APIリクエスト**
   ```http
   Authorization: Bearer {access_token}
   ```

### TVクライアントの制限

- TVクライアントはデスクトップアプリでは不自然
- 一部の機能が制限される可能性がある
- レート制限が厳しい場合がある

## 方式3: YouTube Data API v3（公式API）

### 概要

Googleの公式YouTube Data APIを使用する方式です。OAuth2認証が必須です。

### 必要なスコープ

```
https://www.googleapis.com/auth/youtube
https://www.googleapis.com/auth/youtube.readonly
https://www.googleapis.com/auth/youtube.force-ssl
```

### エンドポイント

```
GET https://www.googleapis.com/youtube/v3/liveChat/messages
  ?liveChatId={CHAT_ID}
  &part=snippet,authorDetails
  &maxResults=2000
```

### メリット

- 公式サポートあり
- 安定したAPI
- ドキュメントが充実

### デメリット

- API割り当て制限がある
- 認証フローが複雑
- `liveChatId` の取得に追加APIコールが必要

## 実装推奨事項

### liscovへの実装提案

1. **Cookie認証を優先実装**
   - InnerTube APIとの整合性が高い
   - 既存のコードベースへの変更が最小限

2. **設定ファイルでCookieを管理**
   ```toml
   [auth]
   enabled = true
   cookies_file = "~/.config/liscov/cookies.txt"
   ```

3. **SAPISSIDHASHの自動生成**
   - リクエストごとにタイムスタンプを更新

### 必要な変更箇所

| ファイル | 変更内容 |
|----------|----------|
| `src/api/innertube.rs` | 認証ヘッダー追加ロジック |
| `src/api/youtube.rs` | Cookie解析・SAPISIDHASH生成 |
| `src/config.rs` | 認証設定の追加 |
| `src/gui/` | Cookie入力UI（オプション） |

### セキュリティ考慮事項

1. **Cookieの安全な保存**
   - 平文での保存は避ける
   - OS のキーチェーン/資格情報マネージャーの利用を検討

2. **Cookie有効期限の管理**
   - 定期的な有効性チェック
   - 期限切れ時のユーザー通知

3. **ログへの出力禁止**
   - Cookie値をログに出力しない
   - SAPISSIDHASHも同様

## 参考資料

- [YouTube.js Authentication Guide](https://ytjs.dev/guide/authentication)
- [SAPISIDHASH Generation (GitHub Gist)](https://gist.github.com/eyecatchup/2d700122e24154fdc985b7071ec7764a)
- [YouTube Data API - LiveChatMessages](https://developers.google.com/youtube/v3/live/docs/liveChatMessages)
- [YouTube.js GitHub](https://github.com/LuanRT/YouTube.js)
- [innertube Python (GitHub)](https://github.com/tombulled/innertube)
- [pytchat (GitHub)](https://github.com/taizan-hokuto/pytchat)

## 調査メモ

### 確認が必要な事項

1. メンバー限定配信でのcontinuation token取得可否
2. 認証状態でのsubMenuItems構造の違い
3. メンバーバッジの表示条件

### 未調査事項

1. Cookie自動更新の仕組み
2. 2段階認証アカウントでの動作
3. 複数アカウント切り替え
