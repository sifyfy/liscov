# 視聴者管理機能

## 目的（Why）

配信者が視聴者を識別・管理し、パーソナライズされた対応を行うための機能。読み仮名の設定によるTTS読み上げの改善、メモによる視聴者情報の記録、統計情報（メッセージ数、貢献額）の把握を提供する。

## 振る舞い（What）

### 配信者スコープ

同じ視聴者でも配信者ごとに独立したプロフィールとカスタム情報を持つ。

| 配信者 | 視聴者 | メッセージ数 | 読み仮名 |
|-------|-------|------------|---------|
| 配信者A (UCaaa) | 視聴者X (UCxxx) | 150 | "たなか" |
| 配信者B (UCbbb) | 視聴者X (UCxxx) | 10 | "タナカ" |

複数の配信に同時に接続しているとき、チャット画面の視聴者パネルはクリックしたコメントが届いた接続の配信者のもとで視聴者を引き、保存する。上の例で配信者Bの接続に届いた X のコメントを開くと「タナカ」が出て、保存すると B のもとに書く（02_chat.md「視聴者情報パネル」の「どの配信者のもとで開くか」）。

### カスタム情報の管理

| 操作 | 結果 |
|------|------|
| 読み仮名を入力して保存 | DBにUpsert。次に届くコメントから、TTS読み上げで投稿者名の代わりにカスタム読み仮名を使用（「さん」は TTS 設定どおり付く。例: 読み仮名「たなか」→「たなかさん」） |
| 読み仮名を空にして保存 | Noneとして保存。TTS読み上げは投稿者名を使用 |
| 視聴者を削除 | viewer_profiles + viewer_custom_info（CASCADE）が削除される |
| 配信者を削除 | その配信者に関連する全viewer_profiles + viewer_custom_info + broadcaster_profilesが削除される |

### 自動更新

| イベント | 結果 |
|---------|------|
| メッセージ受信 | viewer_profilesの `message_count` インクリメント、`last_seen` 更新 |
| SuperChat受信 | `total_contribution` に金額を加算 |
| 視聴者を特定できたギフト受信 | 通常のメッセージと同じく `message_count` インクリメント、`last_seen`・`display_name` 更新 |
| 視聴者を特定できないギフト受信 | 更新しない（プロフィールを作らない） |

ギフトには channel_id が付かないため、handle と `display_name` の一致で視聴者を引く（規則は 02_chat.md「ギフト」）。`display_name` はメッセージのたびに最新の handle に更新されるので、この照合に使える。

### 視聴者のタブ (配信の後・準備)

配信の後や準備のときに、視聴者の読み仮名・メモ・タグを続けて整理する画面 (ブリーフ §1。手は空いている)。
置き方の試作は `src/routes/design-proto/viewers` の C「2 行の一覧」(2026-10-09 に選定)。編集は押した行の下に広げる (チャットの視聴者情報パネルと同じ作法)。

| 状況 | 結果 |
|------|------|
| タブを開いた | 配信者は主の配信 (最初の接続) の配信者。接続していないか、その配信者の記録がまだなければ記録のある最初の配信者。視聴者を最終確認の新しい順に 50 人ずつ出す |
| 1 人の行 | 1 行目に名前 (読み仮名)・タグ、2 行目に最終確認の日付・コメント数・貢献額 (0 なら出さない)・メモ。長い名前・メモも省略せず折り返す (540 幅でも横にスクロールしない) |
| 「読み仮名なしだけ」をオンにした | 読み仮名が登録されていない (空を含む) 人だけを出す。探す欄と組み合わせられる。最初のページに戻る |
| 探す欄に打つ | 打ち終えて 300ms で、名前・読み仮名・メモの部分一致で絞る (下の「検索機能」)。最初のページに戻る |
| 行を押す (1 手) | その行のすぐ下に編集を広げ、読み仮名の欄にフォーカスする。欄は 読み仮名・タグ・メモ。ボタンは 保存・やめる・この視聴者を消す… |
| 読み仮名・タグ・メモの欄で Enter・「保存」を押す | 保存して閉じ、行に反映する。チャットの画面にある同じ視聴者 (同じ配信者のもと) の行にもすぐ読み仮名とメモを出す。日本語入力の変換を確定する Enter では保存しない |
| Esc・「やめる」 | 保存せずに閉じる |
| 開いている間に別の行を押す | 前の編集を閉じ (保存していない内容は捨てる)、押した行の下に開く |
| タグ | 読点 (、) かカンマ (,) で区切る。前後の空白と空のタグは除く |
| 「この視聴者を消す…」 | 編集の中に確認を出す: 「取り消せません。{名前} の記録 (読み仮名・メモ・タグ・コメント数など) をすべて消します。次にコメントしたら新しい視聴者として記録します」。フォーカスは「やめる」。「消す」で消して一覧から外す |
| 「この配信者の記録を消す…」 | 確認のダイアログ: 「取り消せません。{配信者名} のもとの視聴者 N 人の記録 (読み仮名・メモ・タグ・コメント数など) をすべて消します」。フォーカスは「やめる」。消したら、記録のある最初の配信者を選ぶ |
| ページ | 「前の 50 人」「次の 50 人」と「1〜50 人目」。次が無ければ「次の 50 人」は押せない |
| 読み込み中 | 「読み込み中…」 |
| 記録が無い・見つからない | 配信者に視聴者がいなければ「まだ視聴者の記録がありません」、絞り込んで 0 人なら「見つかりません」 |
| 記録のある配信者がいない | 「まだ記録がありません。配信に接続してコメントが届くと記録されます」 |

## 制約・不変条件（Boundaries）

| 制約 | 理由 |
|------|------|
| `viewer_profiles` は `(broadcaster_channel_id, channel_id)` の複合ユニーク制約 | 配信者ごとに視聴者プロフィールを分離する。同じ視聴者でも配信者が異なれば別レコード |
| `viewer_custom_info` は `viewer_profile_id` で `viewer_profiles` と1:1対応（FK CASCADE） | プロフィール削除時にカスタム情報も確実に削除する |
| 検索は `display_name`, `reading`, `notes` の3フィールドに対する部分一致（LIKE） | 視聴者を表示名・読み仮名・メモのいずれからでも見つけられるようにする |

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `viewer_get_list` | `broadcaster_id, search_query?, without_reading?, limit?, offset?` | `Vec<ViewerWithCustomInfo>` | 視聴者リスト取得。`without_reading` が true なら読み仮名の無い (空を含む) 人だけ |
| `viewer_get_custom_info` | `viewer_profile_id` | `Option<ViewerCustomInfo>` | カスタム情報取得（単一） |
| `viewer_upsert_custom_info` | `viewer_profile_id, reading?, notes?, custom_data?` | `()` | カスタム情報保存 |
| `viewer_get_profile` | `broadcaster_id, channel_id` | `Option<ViewerProfile>` | プロフィール取得 |
| `viewer_search` | `broadcaster_id, query, limit?` | `Vec<ViewerWithCustomInfo>` | 検索 |
| `viewer_delete` | `viewer_profile_id` | `bool` | 視聴者データ削除 |
| `broadcaster_get_list` | なし | `Vec<BroadcasterChannel>` | 配信者リスト取得 |
| `broadcaster_delete` | `broadcaster_id` | `(bool, u32)` | 配信者データ削除 |
| `get_top_contributors` | `session_id, limit?` | `Vec<GuiContributorStats>` | セッション内の貢献額上位の視聴者を取得 |

> **Note**: タグも含めて一括更新する場合は [08_database.md](08_database.md) の `viewer_update_info` を使用する。

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
  - reading: "たなか"
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
- チャット画面の視聴者パネルの配信者は `chatStore.broadcasterChannelIdOf(message)`（メッセージの `connection_id` から接続一覧の `broadcasterChannelId` を引く）で決める。`chatStore.broadcasterChannelId`（最初の接続）は使わない

## カスタム情報フィールド

### reading（読み仮名）

| 項目 | 内容 |
|-----|------|
| 用途 | TTS読み上げ時の読み仮名 |
| 例 | 表示名「山田太郎」→ reading「やまだたろう」 |
| 適用 | TTS読み上げ時に投稿者名の代わりに使用。「さん」は TTS の `add_honorific` で付くので、読み仮名には名前だけを書く。使うのはメッセージが届いた接続の配信者のもとでの読み仮名（04_tts.md「読み上げテキスト生成」） |

### notes（メモ）

| 項目 | 内容 |
|-----|------|
| 用途 | 視聴者に関する自由記述メモ。チャットのコメントの本文の下に毎回出す (02_chat.md「1 行の形」)。読み上げには使わない |
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
- **記号**: `%`・`_`・`\` は文字そのものとして探す（LIKE のワイルドカードにしない）

| 検索文字 | 見つかる | 見つからない |
|---------|---------|-------------|
| `a_b` | `a_b`、`xa_bx` | `axb` |
| `50%` | `50%off` | `500` |
| `\` | `a\b` | `ab` |

### SQL

`database::get_viewers_for_broadcaster` が `ViewerFilter { search, without_reading }` で絞る (`viewer_get_list`・`viewer_search` の両方が使う)。
探す文字が無ければ ?2 を NULL にして名前などで絞らない。

```sql
SELECT vp.id, vp.broadcaster_channel_id, vp.channel_id, vp.display_name,
       vp.first_seen, vp.last_seen, vp.message_count, vp.total_contribution,
       vp.membership_level, vp.tags,
       vci.reading, vci.notes, vci.custom_data
FROM viewer_profiles vp
LEFT JOIN viewer_custom_info vci ON vp.id = vci.viewer_profile_id
WHERE vp.broadcaster_channel_id = ?1
  AND (?2 IS NULL OR vp.display_name LIKE ?2 ESCAPE '\' OR vci.reading LIKE ?2 ESCAPE '\' OR vci.notes LIKE ?2 ESCAPE '\')
  AND (?5 = 0 OR vci.reading IS NULL OR vci.reading = '')   -- 読み仮名なしだけ
ORDER BY vp.last_seen DESC
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
ViewerManagement.svelte           # 視聴者のタブ: 配信者の選択・消す、探す・読み仮名なしだけ・読み込み直す、ページ送り、空の状態
├── ViewerList.svelte             # 2 行の一覧。押した行の下に ViewerEditor を広げる (開くのは 1 人だけ)
│   └── ViewerEditor.svelte       # 読み仮名・タグ・メモの編集と「この視聴者を消す…」の確認
└── DeleteConfirmDialog.svelte    # 「この配信者の記録を消す…」の確認ダイアログ
```

状態は `viewerStore` (`src/lib/stores/viewer.svelte.ts`) が持つ。絞り込み (探す文字・読み仮名なしだけ) もストアにあるので、タブを開き直しても前の条件のまま。
タグの区切りと表示は `src/lib/utils/tags.ts` (`parseTags`・`formatTags`)、最終確認の日付は `formatDay` (今年なら「10/9」、それ以外は「2025/10/1」)。

### ViewerManagement.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| タブを開く | `broadcaster_get_list` → props の配信者 (主の配信) が一覧にあれば選び、なければ最初の配信者を選んで `viewer_get_list` |
| 配信者選択 | `viewer_get_list` を offset 0 で呼ぶ |
| 探す欄に打つ | 300ms 待って `viewerStore.setFilter({ search })` (最初のページから `viewer_get_list`) |
| 「読み仮名なしだけ」 | すぐ `viewerStore.setFilter({ withoutReading })` |
| 読み込み直す | 配信者の一覧と視聴者の一覧を読み直す (選んでいる配信者のまま) |
| 編集で保存 | `viewer_update_info` (読み仮名・メモは空なら null、custom_data は元の値のまま) → 一覧を読み直す → `chatStore.setViewerInfo` でチャットの行にも読み仮名とメモを出す |
| この視聴者を消す | `viewer_delete` → 一覧を読み直す |
| この配信者の記録を消す | `broadcaster_delete` → 配信者の一覧を読み直し、最初の配信者を選ぶ |
| 前の・次の 50 人 | `viewer_get_list` を offset を変えて呼ぶ。返ってきた人数が 50 未満なら次は無い |

### ViewerEditor.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| 開く | 読み仮名の欄にフォーカス。タグは読点でつないで出す |
| 欄で Enter (変換の確定を除く)・「保存」 | 呼び出し元の onSave。失敗したら閉じずに「保存に失敗しました」 |
| Esc・「やめる」 | 保存せずに閉じる (フォーカスは行に戻す)。消す確認を出している間の Esc は確認だけを閉じる |
| 「この視聴者を消す…」 | 編集の中に確認 (フォーカスは「やめる」)。「消す」で onDelete。失敗したら「消すのに失敗しました」 |

### 編集可能フィールド

| フィールド | 入力形式 | 保存先 |
|-----------|---------|-------|
| 読み仮名 | テキスト | `viewer_custom_info.reading` |
| メモ | 1 行のテキスト (Enter で保存) | `viewer_custom_info.notes` |
| タグ | 読点・カンマ区切りテキスト | `viewer_profiles.tags` |

`viewer_profiles.membership_level` は手で編集しない (どこからも書かれず使われていない列のため、2026-10-09 に編集できる項目から外した。メンバーかどうかの記録は別に検討する)。

### 表示項目

視聴者のタブの 1 人の行 (上の「視聴者のタブ」)。

| 項目 | 説明 |
|-----|------|
| 表示名 | YouTubeの表示名。長くても省略せず折り返す |
| 読み仮名 | 名前の後ろに「(よみ)」。無ければ出さない |
| タグ | 名前の行に札で出す |
| 最終確認 | 「最終 10/9」(最後にコメントした日) |
| コメント数 | 「コメント 1,284」 |
| 総貢献額 | スーパーチャット等の合計。0 なら出さない |
| メモ | 2 行目の最後。長くても折り返す |

初見日時はタブには出さない (DB には残る)。

## ページネーション

- 1ページあたり: 50件
- ソート順: `last_seen DESC`（最近アクティブな順）

## TypeScript型定義

Rust の型から ts-rs で `src/lib/types/generated/` に生成する（手で書かない。型を変えたら `cargo test --manifest-path src-tauri/Cargo.toml export_bindings` で生成し直してコミットする。`make typecheck` が古い生成物を検出する）。
TS ではフロントエンドの呼び名で re-export する（`GuiViewerProfile` → `ViewerProfile`、`GuiViewerWithInfo` → `ViewerWithCustomInfo`、`GuiBroadcasterChannel` → `BroadcasterChannel`）。
`ViewerCustomInfo` 単体の TS の型は持たない（読むときは `ViewerWithCustomInfo`）。

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
