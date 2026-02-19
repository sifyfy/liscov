# 視聴者管理機能

## 概要

チャット参加者の情報を管理し、カスタム情報（読み仮名、メモ）を保存する。視聴者プロフィールは配信者ごとにスコープされ、同じ視聴者でも配信者ごとに異なるプロフィール（メッセージ数、貢献額等）とカスタム情報を持つ。

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `viewer_get_list` | `broadcaster_id, search_query?, limit?, offset?` | `Vec<ViewerWithCustomInfo>` | 視聴者リスト取得 |
| `viewer_get_custom_info` | `viewer_profile_id` | `Option<ViewerCustomInfo>` | カスタム情報取得（単一） |
| `viewer_upsert_custom_info` | `viewer_profile_id, reading?, notes?, custom_data?` | `()` | カスタム情報保存 |
| `viewer_get_profile` | `broadcaster_id, channel_id` | `Option<ViewerProfile>` | プロフィール取得 |
| `viewer_search` | `broadcaster_id, query, limit?` | `Vec<ViewerWithCustomInfo>` | 検索 |
| `viewer_delete` | `viewer_profile_id` | `bool` | 視聴者データ削除 |
| `broadcaster_get_list` | なし | `Vec<BroadcasterChannel>` | 配信者リスト取得 |
| `broadcaster_delete` | `broadcaster_id` | `(bool, u32)` | 配信者データ削除 |

## 永続化

| テーブル | 用途 |
|---------|------|
| `viewer_profiles` | 配信者ごとの視聴者プロフィール・統計 |
| `viewer_custom_info` | 視聴者のカスタム情報（読み仮名、メモ等） |
| `broadcaster_profiles` | 配信者情報 |

すべて `%APPDATA%/liscov-tauri/liscov.db` (SQLite) に保存。

## データモデル

### viewer_profiles テーブル

配信者ごとの視聴者プロフィール。同じ視聴者でも配信者ごとに異なる統計（メッセージ数、貢献額等）を持つ。

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
| `channel_id` | TEXT | 視聴者YouTubeチャンネルID |
| `display_name` | TEXT | 表示名 |
| `first_seen` | TEXT | 初見日時（RFC3339） |
| `last_seen` | TEXT | 最終確認日時（RFC3339） |
| `message_count` | INTEGER | メッセージ数 |
| `total_contribution` | REAL | 総貢献額（スーパーチャット等） |
| `membership_level` | TEXT | メンバーシップレベル |
| `tags` | TEXT | タグ（カンマ区切り） |

### viewer_custom_info テーブル

`viewer_profiles`の拡張情報として、読み仮名やメモを保存。`viewer_profile_id`で1:1対応。

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
    thumbnail_url TEXT
);
```

## Rust構造体

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
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
```

### ViewerCustomInfo

```rust
pub struct ViewerCustomInfo {
    pub viewer_profile_id: i64,
    pub reading: Option<String>,
    pub notes: Option<String>,
    pub custom_data: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
```

### ViewerWithCustomInfo（結合モデル）

```rust
pub struct ViewerWithCustomInfo {
    // viewer_profiles
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

    // viewer_custom_info
    pub reading: Option<String>,
    pub notes: Option<String>,
    pub custom_data: Option<String>,
}
```

### BroadcasterChannel

```rust
pub struct BroadcasterChannel {
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub handle: Option<String>,
    pub viewer_count: usize,
}
```

## 配信者スコープ

### 概要

同じ視聴者でも配信者ごとに異なるプロフィール（統計情報）とカスタム情報を持つ。

### 例

```
視聴者: UCxxx_common_viewer

配信者A (UCaaa) での情報:
- viewer_profiles:
  - message_count: 150
  - total_contribution: 5000.0
- viewer_custom_info:
  - reading: "たなかさん"
  - notes: "常連さん"

配信者B (UCbbb) での情報:
- viewer_profiles:
  - message_count: 10
  - total_contribution: 0.0
- viewer_custom_info:
  - reading: "タナカ"
  - notes: "初見"
```

### 実装

- `viewer_profiles` は `(broadcaster_channel_id, channel_id)` の複合キーで一意性を保証
- `viewer_custom_info` は `viewer_profile_id` で `viewer_profiles` と1:1対応
- 配信者削除時は関連する全ての `viewer_profiles` が削除され、CASCADE削除で `viewer_custom_info` も削除される

## カスタム情報フィールド

### reading（読み仮名）

| 項目 | 内容 |
|-----|------|
| 用途 | TTS読み上げ時の読み仮名 |
| 例 | 表示名「山田太郎」→ reading「やまだたろう」 |
| 適用 | TTS読み上げ時に投稿者名の代わりに使用 |

### notes（メモ）

| 項目 | 内容 |
|-----|------|
| 用途 | 視聴者に関する自由記述メモ |
| 例 | "よく質問する人", "リクエスト常連" |
| 検索 | 部分一致検索の対象 |

### custom_data（拡張データ）

| 項目 | 内容 |
|-----|------|
| 用途 | 将来の拡張用カスタムデータ |
| 形式 | JSON文字列 |
| 例 | `{"favorite_games":["APEX"],"language":"ja"}` |

## 視聴者情報の取得元

### YouTubeデータ

チャットメッセージから以下の情報を取得：

| 取得元フィールド | 保存先 |
|----------------|-------|
| `authorName.simpleText` | `display_name` |
| `authorExternalChannelId` | `channel_id` |
| `authorPhoto.thumbnails[0].url` | （未使用） |
| `authorBadges` | メンバーシップ判定 |
| `purchaseAmountText` | `total_contribution` に加算 |

### 自動更新

メッセージ受信時に `viewer_profiles` が自動更新される：

- `message_count` をインクリメント
- `last_seen` を更新
- スーパーチャット時は `total_contribution` に加算

## 検索機能

### 検索対象

1. `display_name` - 視聴者の表示名
2. `reading` - 読み仮名
3. `notes` - メモ

### 検索方式

- **部分一致**: `LIKE "%{検索文字}%"`
- **大文字小文字**: 区別なし
- **日本語**: 完全対応（UTF-8）

### SQL

```sql
SELECT vp.id, vp.broadcaster_channel_id, vp.channel_id, vp.display_name,
       vp.first_seen, vp.last_seen, vp.message_count, vp.total_contribution,
       vp.membership_level, vp.tags,
       vci.reading, vci.notes, vci.custom_data
FROM viewer_profiles vp
LEFT JOIN viewer_custom_info vci ON vp.id = vci.viewer_profile_id
WHERE vp.broadcaster_channel_id = ?1
  AND (vp.display_name LIKE ?2 OR vci.reading LIKE ?2 OR vci.notes LIKE ?2)
ORDER BY vp.message_count DESC
LIMIT ?3 OFFSET ?4
```

## 削除機能

### 視聴者データ削除

- `viewer_profiles` レコードを削除
- `viewer_custom_info` は FK CASCADE により自動削除

### 配信者データ削除

- その配信者に関連するすべての `viewer_profiles` を削除
- `viewer_custom_info` は FK CASCADE により自動削除
- `broadcaster_profiles` も削除
- 戻り値: `(broadcaster_deleted: bool, viewers_deleted: u32)`

## フロントエンド

### コンポーネント構成

```
ViewerManagement.svelte
├── BroadcasterSelector.svelte    # 配信者選択ドロップダウン
├── ViewerList.svelte             # 視聴者一覧
│   ├── 検索ボックス
│   └── ページネーション
├── ViewerEditModal.svelte        # 編集モーダル
└── DeleteConfirmDialog.svelte    # 削除確認
```

### ViewerManagement.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| 配信者選択 | `viewer_get_list`呼び出し、リスト更新 |
| 検索クエリ入力 | デバウンス後に`viewer_get_list`呼び出し |

### ViewerList.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| 視聴者行クリック | 編集モーダルを開く |
| ページ変更 | `viewer_get_list`をoffset変更で呼び出し |
| 更新ボタン | リストを再取得 |

### ViewerEditModal.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| 読み仮名入力 | フォーム状態を更新 |
| メモ入力 | フォーム状態を更新 |
| タグ入力 | カンマ区切りで入力 |
| 「保存」クリック | `viewer_upsert_custom_info`呼び出し、モーダルを閉じる |
| 「削除」クリック | 削除確認ダイアログを表示 |

### 編集可能フィールド

| フィールド | 入力形式 | 保存先 |
|-----------|---------|-------|
| 読み仮名 | テキスト | `viewer_custom_info.reading` |
| メモ | テキストエリア | `viewer_custom_info.notes` |
| タグ | カンマ区切りテキスト | `viewer_profiles.tags` |
| メンバーシップレベル | テキスト | `viewer_profiles.membership_level` |

### 表示項目

| 項目 | 説明 |
|-----|------|
| 表示名 | YouTubeの表示名 |
| 読み仮名 | カスタム読み仮名 |
| 初見日時 | 初めてコメントした日時 |
| 最終確認日時 | 最後にコメントした日時 |
| メッセージ数 | 総コメント数 |
| 総貢献額 | スーパーチャット等の合計 |
| タグ | 設定されたタグ |
| メモ | カスタムメモ |

## ページネーション

- 1ページあたり: 50件
- ソート順: `last_seen DESC`（最近アクティブな順）

## TypeScript型定義

```typescript
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

interface ViewerWithCustomInfo extends ViewerProfile {
    reading: string | null;
    notes: string | null;
    custom_data: string | null;
}

interface BroadcasterChannel {
    channel_id: string;
    channel_name: string | null;
    handle: string | null;
    viewer_count: number;
}
```

## TTS連携

読み仮名が設定されている視聴者のメッセージは、TTS読み上げ時にカスタム読み仮名を使用。

詳細は[TTS機能仕様](04_tts.md)を参照。
