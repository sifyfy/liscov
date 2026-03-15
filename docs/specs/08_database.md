# データベース・セッション管理

## 目的（Why）

チャットセッション、メッセージ、視聴者情報をSQLiteに永続化し、アプリ再起動後もデータを保持する。セッション履歴の参照、視聴者統計の蓄積、エクスポート機能の基盤となる。

## 振る舞い（What）

### セッションライフサイクル

| イベント | 結果 |
|---------|------|
| 配信に接続 | UUID v4でセッションIDを生成し、sessionsテーブルにINSERT（end_time = NULL） |
| メッセージ受信 | messagesテーブルにINSERT + viewer_profilesをUPSERT |
| 配信から切断 | sessionsテーブルのend_timeを更新、統計（total_messages, total_revenue）を最終集計 |

### メッセージ重複排除

| 状況 | 結果 |
|------|------|
| 同一セッション内で同じmessage_idのメッセージ | INSERT OR IGNORE（重複を無視） |
| 異なるセッションで同じmessage_id | 別レコードとして保存（session_id + message_idの複合ユニーク） |

### マイグレーション

| 変更種別 | 方法 |
|---------|------|
| 新規カラム追加 | ALTER TABLE + DEFAULT値（既存データに影響なし） |
| 新規テーブル追加 | CREATE TABLE IF NOT EXISTS（既存DBに影響なし） |
| キー削除 | 未知のキーは無視（エラーにならない） |

## 制約・不変条件（Boundaries）

| 制約 | 理由 |
|------|------|
| メッセージの重複排除は `(session_id, message_id)` の複合ユニークインデックスで保証する | YouTubeのmessage_idはセッションをまたぐと一意性が保証されないため、session_idとの複合で管理 |
| 外部キー制約にCASCADE削除を使用する（sessions→messages, viewer_profiles→viewer_custom_info等） | 親レコード削除時に関連データが孤立することを防ぐ |
| セッションIDはUUID v4形式（36文字） | 衝突確率が実質ゼロであり、セッション間の独立性を保証する |
| DBファイルパスは環境変数 `LISCOV_APP_NAME` で分離可能 | E2Eテストが本番DBを破壊することを防ぐ |

## 永続化

| ファイル | パス |
|---------|------|
| liscov.db | `%APPDATA%/liscov-tauri/liscov.db` |

> **Note**: ディレクトリ名 `liscov-tauri` は環境変数 `LISCOV_APP_NAME` で変更可能（E2Eテスト用）。詳細は[認証機能仕様のE2Eテストセクション](01_auth.md#e2eテスト)を参照。

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `session_get_list` | `limit: Option<usize>` | `Vec<Session>` | セッション履歴取得 |
| `session_get_messages` | `session_id, limit?` | `Vec<StoredMessage>` | セッションのメッセージ取得 |
| `session_create` | `stream_url, stream_title?` | `String` | セッション作成 |
| `session_end` | `session_id` | `()` | セッション終了 |

## テーブル一覧

| テーブル | 用途 |
|---------|------|
| `sessions` | セッション情報 |
| `messages` | チャットメッセージ |
| `viewer_profiles` | 視聴者プロフィール |
| `viewer_custom_info` | 視聴者カスタム情報 |
| `broadcaster_profiles` | 配信者プロフィール |
| `hourly_stats` | 時間別統計 |
| `contributor_stats` | 貢献者統計 |

## スキーマ定義

### sessions テーブル

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    start_time TEXT NOT NULL,
    end_time TEXT,
    stream_url TEXT,
    stream_title TEXT,
    broadcaster_channel_id TEXT,
    broadcaster_name TEXT,
    total_messages INTEGER DEFAULT 0,
    total_revenue REAL DEFAULT 0.0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_sessions_timestamp
    AFTER UPDATE ON sessions
BEGIN
    UPDATE sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
```

| カラム | 型 | 説明 |
|-------|-----|------|
| `id` | TEXT | セッションID（UUID v4） |
| `start_time` | TEXT | 開始時刻（RFC3339） |
| `end_time` | TEXT | 終了時刻（NULL=進行中） |
| `stream_url` | TEXT | YouTube Live URL |
| `stream_title` | TEXT | 配信タイトル |
| `broadcaster_channel_id` | TEXT | 配信者チャンネルID |
| `broadcaster_name` | TEXT | 配信者名 |
| `total_messages` | INTEGER | 合計メッセージ数 |
| `total_revenue` | REAL | 合計収益（SuperChat等） |

### messages テーブル

```sql
CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    timestamp_usec TEXT NOT NULL,
    author TEXT NOT NULL,
    author_icon_url TEXT,
    channel_id TEXT NOT NULL,
    content TEXT NOT NULL,
    message_type TEXT NOT NULL,
    amount TEXT,
    is_member INTEGER DEFAULT 0,
    metadata TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_messages_session_timestamp ON messages(session_id, timestamp);
CREATE INDEX idx_messages_channel_id ON messages(channel_id);
CREATE INDEX idx_messages_type ON messages(message_type);
CREATE UNIQUE INDEX idx_messages_unique ON messages(session_id, message_id);
```

| カラム | 型 | 説明 |
|-------|-----|------|
| `id` | INTEGER | 内部ID（自動増分） |
| `session_id` | TEXT | セッションID（外部キー） |
| `message_id` | TEXT | YouTube内部メッセージID |
| `timestamp` | TEXT | メッセージタイムスタンプ（ISO8601） |
| `timestamp_usec` | TEXT | マイクロ秒タイムスタンプ |
| `author` | TEXT | 投稿者名 |
| `author_icon_url` | TEXT | 投稿者アイコンURL |
| `channel_id` | TEXT | 投稿者チャンネルID |
| `content` | TEXT | メッセージ本文 |
| `message_type` | TEXT | メッセージタイプ（text/superchat/supersticker/membership等） |
| `amount` | TEXT | SuperChat金額（通貨記号含む、例: "¥500"） |
| `is_member` | INTEGER | メンバーシップ加入者フラグ（0/1） |
| `metadata` | TEXT | JSON形式のメタデータ |

### viewer_profiles テーブル

詳細は[視聴者管理機能](06_viewer.md)を参照。

視聴者プロフィールは配信者ごとにスコープされる。同じ視聴者でも配信者ごとに異なる統計情報（メッセージ数、貢献額等）を持つ。

```sql
CREATE TABLE viewer_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    broadcaster_channel_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    message_count INTEGER DEFAULT 0,
    total_contribution REAL DEFAULT 0.0,
    membership_level TEXT,
    tags TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(broadcaster_channel_id, channel_id)
);

CREATE INDEX idx_viewer_profiles_broadcaster ON viewer_profiles(broadcaster_channel_id);
CREATE INDEX idx_viewer_profiles_message_count ON viewer_profiles(broadcaster_channel_id, message_count DESC);
CREATE INDEX idx_viewer_profiles_contribution ON viewer_profiles(broadcaster_channel_id, total_contribution DESC);
```

| カラム | 型 | 説明 |
|-------|-----|------|
| `id` | INTEGER | サロゲートキー（自動増分） |
| `broadcaster_channel_id` | TEXT | 配信者チャンネルID |
| `channel_id` | TEXT | 視聴者チャンネルID |
| `display_name` | TEXT | 表示名 |
| `first_seen` | TEXT | 初見日時（RFC3339） |
| `last_seen` | TEXT | 最終確認日時（RFC3339） |
| `message_count` | INTEGER | メッセージ数 |
| `total_contribution` | REAL | 総貢献額（SuperChat等） |
| `membership_level` | TEXT | メンバーシップレベル |
| `tags` | TEXT | タグ（カンマ区切り） |

### viewer_custom_info テーブル

詳細は[視聴者管理機能](06_viewer.md)を参照。

`viewer_profiles`の拡張情報として、読み仮名やメモを保存する。`viewer_profile_id`で1:1対応。

```sql
CREATE TABLE viewer_custom_info (
    viewer_profile_id INTEGER PRIMARY KEY,
    reading TEXT,
    notes TEXT,
    custom_data TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (viewer_profile_id) REFERENCES viewer_profiles(id) ON DELETE CASCADE
);
```

| カラム | 型 | 説明 |
|-------|-----|------|
| `viewer_profile_id` | INTEGER | viewer_profiles.id（主キー・外部キー） |
| `reading` | TEXT | 読み仮名（TTS用） |
| `notes` | TEXT | メモ |
| `custom_data` | TEXT | 拡張データ（JSON形式） |

### broadcaster_profiles テーブル

```sql
CREATE TABLE broadcaster_profiles (
    channel_id TEXT PRIMARY KEY,
    channel_name TEXT,
    handle TEXT,
    thumbnail_url TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### hourly_stats テーブル

> **Status**: 未実装。スキーマ定義・CRUD操作ともにデータベースモジュールに未追加。

```sql
CREATE TABLE hourly_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    hour_timestamp TEXT NOT NULL,
    super_chat_count INTEGER DEFAULT 0,
    super_sticker_count INTEGER DEFAULT 0,
    membership_count INTEGER DEFAULT 0,
    message_count INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, hour_timestamp)
);

CREATE INDEX idx_hourly_stats_session ON hourly_stats(session_id);
```

### contributor_stats テーブル

> **Status**: 未実装。スキーマ定義・CRUD操作ともにデータベースモジュールに未追加。

```sql
CREATE TABLE contributor_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    super_chat_count INTEGER DEFAULT 0,
    highest_tier TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, channel_id)
);

CREATE INDEX idx_contributor_stats_session ON contributor_stats(session_id);
```

## セッション管理

### セッションID形式

- **形式**: UUID v4（RFC 4122）
- **例**: `f47ac10b-58cc-4372-a567-0e02b2c3d479`
- **長さ**: 36文字（ハイフン含）

### セッション開始

```
1. ユーザーが「配信を監視」クリック
        ↓
2. session_create コマンド呼び出し
        ↓
3. UUID v4 でセッションID生成
        ↓
4. sessions テーブルに INSERT
   - start_time = 現在時刻（UTC）
   - end_time = NULL
        ↓
5. セッションID を返却
```

### セッション終了

```
1. ユーザーが「監視を停止」クリック
   または 配信終了を検出
        ↓
2. session_end コマンド呼び出し
        ↓
3. sessions テーブルを UPDATE
   - end_time = 現在時刻（UTC）
        ↓
4. 統計を最終更新
   - total_messages, total_revenue を集計
```

### メッセージ保存

```
1. チャットメッセージ受信
        ↓
2. messages テーブルに INSERT
        ↓
3. viewer_profiles を UPSERT
   - message_count をインクリメント
   - last_seen を更新
   - SuperChat時は total_contribution に加算
        ↓
4. SuperChat/Membership時
   - hourly_revenue を更新
   - contributor_stats を更新
```

## データモデル（Rust）

### Session

```rust
pub struct Session {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub stream_url: Option<String>,
    pub stream_title: Option<String>,
    pub broadcaster_channel_id: Option<String>,
    pub broadcaster_name: Option<String>,
    pub total_messages: i64,
    pub total_revenue: f64,
}
```

### StoredMessage

```rust
pub struct StoredMessage {
    pub id: i64,
    pub session_id: String,
    pub message_id: String,
    pub timestamp: String,
    pub timestamp_usec: String,
    pub author: String,
    pub author_icon_url: Option<String>,
    pub channel_id: String,
    pub content: String,
    pub message_type: String,
    pub amount: Option<String>,
    pub is_member: bool,
    pub metadata: Option<String>,
}
```

### ViewerProfile

```rust
pub struct ViewerProfile {
    pub id: i64,
    pub broadcaster_channel_id: String,
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub message_count: i64,
    pub total_contribution: f64,
    pub membership_level: Option<String>,
    pub tags: Vec<String>,
}
```

### ViewerCustomInfo

```rust
pub struct ViewerCustomInfo {
    pub viewer_profile_id: i64,
    pub reading: Option<String>,
    pub notes: Option<String>,
    pub custom_data: Option<String>,
}
```

## TypeScript型定義

```typescript
interface Session {
    id: string;
    start_time: string;
    end_time: string | null;
    stream_url: string | null;
    stream_title: string | null;
    broadcaster_channel_id: string | null;
    broadcaster_name: string | null;
    total_messages: number;
    total_revenue: number;
}

interface StoredMessage {
    id: number;
    session_id: string;
    message_id: string;
    timestamp: string;
    timestamp_usec: string;
    author: string;
    author_icon_url: string | null;
    channel_id: string;
    content: string;
    message_type: string;
    amount: string | null;
    is_member: boolean;
    metadata: string | null;
}

interface ViewerProfile {
    id: number;
    broadcaster_channel_id: string;
    channel_id: string;
    display_name: string;
    first_seen: string;
    last_seen: string;
    message_count: number;
    total_contribution: number;
    membership_level: string | null;
    tags: string[];
}

interface ViewerCustomInfo {
    viewer_profile_id: number;
    reading: string | null;
    notes: string | null;
    custom_data: string | null;
}
```

## インデックス一覧

| インデックス | 対象 | 用途 |
|------------|------|------|
| `idx_messages_session_timestamp` | messages(session_id, timestamp) | セッション別メッセージ検索 |
| `idx_messages_channel_id` | messages(channel_id) | 投稿者別メッセージ検索 |
| `idx_messages_type` | messages(message_type) | タイプ別メッセージ検索 |
| `idx_messages_unique` | messages(session_id, message_id) | 重複防止 |
| `idx_viewer_profiles_broadcaster` | viewer_profiles(broadcaster_channel_id) | 配信者別視聴者検索 |
| `idx_viewer_profiles_message_count` | viewer_profiles(broadcaster_channel_id, message_count DESC) | アクティブ順ソート |
| `idx_viewer_profiles_contribution` | viewer_profiles(broadcaster_channel_id, total_contribution DESC) | 貢献額順ソート |
| `idx_hourly_stats_session` | hourly_stats(session_id) | セッション別統計検索 |
| `idx_contributor_stats_session` | contributor_stats(session_id) | セッション別貢献者検索 |

## トリガー一覧

| トリガー | 対象 | 動作 |
|---------|------|------|
| `update_sessions_timestamp` | sessions | UPDATE時にupdated_atを更新 |
| `update_viewer_profiles_timestamp` | viewer_profiles | UPDATE時にupdated_atを更新 |
| `update_viewer_custom_info_timestamp` | viewer_custom_info | UPDATE時にupdated_atを更新 |
| `update_broadcaster_profiles_timestamp` | broadcaster_profiles | UPDATE時にupdated_atを更新 |

## マイグレーション

### 新規キー追加時

- ALTER TABLE で新カラムを追加
- DEFAULT値を設定して既存データに影響なし

### テーブル追加時

- CREATE TABLE IF NOT EXISTS で安全に追加
- 既存DBに影響なし

## フロントエンド

### SessionHistory.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| 画面表示 | `session_get_list`呼び出し、履歴表示 |
| セッションクリック | `session_get_messages`呼び出し、詳細表示 |

### 表示項目

```
セッション履歴
├─ セッション一覧
│   ├─ 配信タイトル
│   ├─ 開始・終了時刻
│   ├─ メッセージ数
│   ├─ SuperChat件数
│   └─ メンバーシップ獲得数
└─ セッション詳細（選択時）
    ├─ メッセージ一覧
    └─ 統計サマリー
```

## 外部キー制約

| 親テーブル | 子テーブル | ON DELETE |
|-----------|-----------|-----------|
| sessions | messages | CASCADE |
| sessions | hourly_stats | CASCADE |
| sessions | contributor_stats | CASCADE |
| viewer_profiles | viewer_custom_info | CASCADE |

セッション削除時、関連する全データが自動削除される。
視聴者プロフィール削除時、関連するカスタム情報も自動削除される。
