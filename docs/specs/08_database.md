# データベース・セッション管理

## 概要

チャットセッション、メッセージ、視聴者情報をSQLiteに永続化する。

## 永続化

| ファイル | パス |
|---------|------|
| liscov.db | `%APPDATA%/liscov/liscov.db` |

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
    total_messages INTEGER DEFAULT 0,
    super_chat_count INTEGER DEFAULT 0,
    membership_count INTEGER DEFAULT 0,
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
| `total_messages` | INTEGER | 合計メッセージ数 |
| `super_chat_count` | INTEGER | SuperChat件数 |
| `membership_count` | INTEGER | メンバーシップ獲得数 |

### messages テーブル

```sql
CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    author TEXT NOT NULL,
    channel_id TEXT,
    content TEXT,
    message_type TEXT NOT NULL,
    amount REAL,
    metadata TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_messages_session_timestamp ON messages(session_id, timestamp);
CREATE INDEX idx_messages_channel_id ON messages(channel_id);
CREATE INDEX idx_messages_type ON messages(message_type);
```

| カラム | 型 | 説明 |
|-------|-----|------|
| `id` | INTEGER | メッセージID（自動増分） |
| `session_id` | TEXT | セッションID（外部キー） |
| `timestamp` | TEXT | メッセージタイムスタンプ |
| `author` | TEXT | 投稿者名 |
| `channel_id` | TEXT | 投稿者チャンネルID |
| `content` | TEXT | メッセージ本文 |
| `message_type` | TEXT | メッセージタイプ |
| `amount` | REAL | SuperChat金額（通常はNULL） |
| `metadata` | TEXT | JSON形式のメタデータ |

### viewer_profiles テーブル

詳細は[視聴者管理機能](06_viewer.md)を参照。

```sql
CREATE TABLE viewer_profiles (
    channel_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    first_seen TEXT,
    last_seen TEXT,
    message_count INTEGER DEFAULT 0,
    total_contribution REAL DEFAULT 0.0,
    membership_level TEXT,
    tags TEXT,
    behavior_stats TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### viewer_custom_info テーブル

詳細は[視聴者管理機能](06_viewer.md)を参照。

```sql
CREATE TABLE viewer_custom_info (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    broadcaster_channel_id TEXT NOT NULL,
    viewer_channel_id TEXT NOT NULL,
    reading TEXT,
    notes TEXT,
    custom_data TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(broadcaster_channel_id, viewer_channel_id)
);

CREATE INDEX idx_viewer_custom_info_lookup
    ON viewer_custom_info(broadcaster_channel_id, viewer_channel_id);
CREATE INDEX idx_viewer_custom_info_broadcaster
    ON viewer_custom_info(broadcaster_channel_id);
```

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
    pub total_messages: i64,
    pub super_chat_count: i64,
    pub membership_count: i64,
}
```

### StoredMessage

```rust
pub struct StoredMessage {
    pub id: i64,
    pub session_id: String,
    pub timestamp: String,
    pub author: String,
    pub channel_id: Option<String>,
    pub content: Option<String>,
    pub message_type: String,
    pub amount: Option<f64>,
    pub metadata: Option<String>,
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
    total_messages: number;
    super_chat_count: number;
    membership_count: number;
}

interface StoredMessage {
    id: number;
    session_id: string;
    timestamp: string;
    author: string;
    channel_id: string | null;
    content: string | null;
    message_type: string;
    amount: number | null;
    metadata: string | null;
}
```

## インデックス一覧

| インデックス | 対象 | 用途 |
|------------|------|------|
| `idx_messages_session_timestamp` | messages(session_id, timestamp) | セッション別メッセージ検索 |
| `idx_messages_channel_id` | messages(channel_id) | 投稿者別メッセージ検索 |
| `idx_messages_type` | messages(message_type) | タイプ別メッセージ検索 |
| `idx_hourly_stats_session` | hourly_stats(session_id) | セッション別統計検索 |
| `idx_contributor_stats_session` | contributor_stats(session_id) | セッション別貢献者検索 |
| `idx_viewer_custom_info_lookup` | viewer_custom_info(...) | 視聴者情報検索 |
| `idx_viewer_custom_info_broadcaster` | viewer_custom_info(...) | 配信者別視聴者検索 |

## トリガー一覧

| トリガー | 対象 | 動作 |
|---------|------|------|
| `update_sessions_timestamp` | sessions | UPDATE時にupdated_atを更新 |
| `update_viewer_profiles_timestamp` | viewer_profiles | UPDATE時にupdated_atを更新 |
| `update_contributor_stats_timestamp` | contributor_stats | UPDATE時にupdated_atを更新 |
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

セッション削除時、関連する全データが自動削除される。
