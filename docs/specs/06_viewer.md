# 視聴者管理機能

## 概要

チャット参加者の情報を管理し、カスタム情報（読み仮名、メモ）を保存する。視聴者情報は配信者ごとにスコープされ、同じ視聴者でも配信者ごとに異なるカスタム情報を持てる。

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `viewer_get_list` | `broadcaster_id, search_query?, limit?, offset?` | `Vec<ViewerWithCustomInfo>` | 視聴者リスト取得 |
| `viewer_upsert_custom_info` | `info: ViewerCustomInfo` | `i64` | カスタム情報保存 |
| `viewer_get_profile` | `channel_id: String` | `Option<ViewerProfile>` | プロフィール取得 |
| `viewer_search` | `broadcaster_id, query, limit?` | `Vec<ViewerWithCustomInfo>` | 検索 |
| `viewer_delete` | `broadcaster_id, viewer_id` | `bool` | 視聴者データ削除 |
| `broadcaster_get_list` | なし | `Vec<BroadcasterChannel>` | 配信者リスト取得 |
| `broadcaster_delete` | `broadcaster_id` | `(bool, u32)` | 配信者データ削除 |

## 永続化

| テーブル | 用途 |
|---------|------|
| `viewer_profiles` | 視聴者の基本情報・統計（配信者間で共有） |
| `viewer_custom_info` | 配信者ごとのカスタム情報 |
| `broadcaster_profiles` | 配信者情報 |

すべて `%APPDATA%/liscov/liscov.db` (SQLite) に保存。

## データモデル

### viewer_profiles テーブル

視聴者の基本情報と統計を管理。複数配信者間で共有。

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

| カラム | 型 | 説明 |
|-------|-----|------|
| `channel_id` | TEXT | YouTubeチャンネルID（主キー） |
| `display_name` | TEXT | 表示名 |
| `first_seen` | TEXT | 初見日時 |
| `last_seen` | TEXT | 最終確認日時 |
| `message_count` | INTEGER | メッセージ数 |
| `total_contribution` | REAL | 総貢献額（スーパーチャット等） |
| `membership_level` | TEXT | メンバーシップレベル |
| `tags` | TEXT | タグ（カンマ区切り） |
| `behavior_stats` | TEXT | 行動統計（JSON形式） |

### viewer_custom_info テーブル

配信者ごとのカスタム情報。同じ視聴者でも配信者ごとに異なる情報を持てる。

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

| カラム | 型 | 説明 |
|-------|-----|------|
| `broadcaster_channel_id` | TEXT | 配信者のチャンネルID |
| `viewer_channel_id` | TEXT | 視聴者のチャンネルID |
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
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub message_count: i64,
    pub total_contribution: f64,
    pub membership_level: Option<String>,
    pub tags: Vec<String>,
    pub behavior_stats: Option<String>,
}
```

### ViewerCustomInfo

```rust
pub struct ViewerCustomInfo {
    pub id: Option<i64>,
    pub broadcaster_channel_id: String,
    pub viewer_channel_id: String,
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
    pub channel_id: String,
    pub display_name: String,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
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

同じ視聴者が異なる配信者では異なるカスタム情報を持てる。

### 例

```
視聴者: UCxxx_common_viewer

配信者A (UCaaa) のカスタム情報:
- reading: "たなかさん"
- notes: "常連さん"

配信者B (UCbbb) のカスタム情報:
- reading: "タナカ"
- notes: "初見"
```

### 実装

- `viewer_custom_info` テーブルは `(broadcaster_channel_id, viewer_channel_id)` の複合キーで一意性を保証
- `viewer_profiles` は配信者間で共有（基本情報・統計）

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
SELECT vp.*, vci.reading, vci.notes, vci.custom_data
FROM viewer_profiles vp
LEFT JOIN viewer_custom_info vci
    ON vp.channel_id = vci.viewer_channel_id
    AND vci.broadcaster_channel_id = ?1
WHERE (vp.display_name LIKE ?2
       OR vci.reading LIKE ?2
       OR vci.notes LIKE ?2)
ORDER BY vp.last_seen DESC
LIMIT ?3 OFFSET ?4
```

## 削除機能

### 視聴者カスタム情報削除

- `viewer_custom_info` のみ削除
- `viewer_profiles` は残存（他の配信者で使用される可能性）

### 配信者データ削除

- その配信者に関連するすべての `viewer_custom_info` を削除
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
    channel_id: string;
    display_name: string;
    first_seen: string | null;
    last_seen: string | null;
    message_count: number;
    total_contribution: number;
    membership_level: string | null;
    tags: string[];
}

interface ViewerCustomInfo {
    broadcaster_channel_id: string;
    viewer_channel_id: string;
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
