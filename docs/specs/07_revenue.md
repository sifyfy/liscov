# 収益分析・エクスポート機能

## 目的（Why）

配信者がSuperChat、メンバーシップ等の収益状況を配信中・配信後にリアルタイムで把握し、CSV/JSON形式でエクスポートして外部ツールで分析できるようにする。

## 振る舞い（What）

### Tier別集計

SuperChatの色情報（`headerBackgroundColor`）に基づいてtierを判定し、tier別に件数を集計する。**金額の数値計算は行わない。**

判定はヘッダー背景色（`#RRGGBB`）が下の表と完全に一致するかで行う（大文字小文字は区別しない）。どれにも一致しない色と、色が分からないもの（色を保存する前の過去セッション）は **段階不明**（`tier_unknown`）に数える。金額から段階を推定しない。

| Tier | 色 | ヘッダー背景色 | USD相当額の目安 | 実データの例 |
|------|----|------|----------------|------|
| Blue | 青 | `#1565C0` | $1-2 | ¥100 |
| Cyan | 水色 | `#00B8D4` | $2-5 | ¥200, ¥320 |
| Green | 緑 | `#00BFA5` | $5-10 | ¥500, ¥800, HK$25.00 |
| Yellow | 黄 | `#FFB300` | $10-20 | ¥1,000, ¥1,600 |
| Orange | オレンジ | `#E65100` | $20-50 | ¥2,222, ¥3,200 |
| Magenta | マゼンタ | `#C2185B` | $50-100 | ¥5,000, ¥9,999 |
| Red | 赤 | `#D00000` | $100-500 | （実データ未確認） |

実データの例は、2026-09 の配信の生レスポンスで確かめたもの。赤だけは実データが無く、YouTube の配色から決めている。

| 状況 | 結果 |
|------|------|
| ヘッダー背景色が `#00BFA5` の ¥500 | Green |
| ヘッダー背景色が表に無い色 | 段階不明 |
| 色を保存する前（マイグレーション 005 より前）の過去セッションのスーパーチャット | 段階不明 |
| ダッシュボード | 段階不明が 1 件以上あれば「段階不明: N件」も出す |

### エクスポート

| 操作 | 結果 |
|------|------|
| CSV形式でエクスポート | メタデータ（セッション情報）+ メッセージ一覧をCSV出力 |
| JSON形式でエクスポート | metadata + messages + statistics の構造化データを出力 |
| 多接続時にエクスポート | 全接続のメッセージを対象 |

### 上位貢献者

SuperChat件数でソートし、上位10人を表示。同一件数の場合は最高tierで比較。ギフトは含めない。

### ギフト集計

ジュエルで送るギフトは、SuperChat・メンバーシップとは別枠で集計する。既存の集計値には影響しない。

| 受信したギフト | gift_count | gifts_by_name | jewel_known_count | total_jewels |
|---------------|-----------|---------------|-------------------|--------------|
| なし | 0 | [] | 0 | 0 |
| `Hiding`(ジュエル数なし)・`Press F`(10)・`Press F`(10) | 3 | Press F 2件, Hiding 1件 | 2 | 20 |

- gifts_by_name は件数の多い順。同数ならギフト名の昇順
- ダッシュボードには「ジュエル数不明: {gift_count - jewel_known_count}件」も出し、合計が過小であることが分かるようにする
- エクスポート: `message_type` = `gift`、`amount_display` = `10 Jewels`（ジュエル数が無ければ空）。JSON の `statistics` に `gifts`（GiftStats）を足す

## 制約・不変条件（Boundaries）

| 制約 | 理由 |
|------|------|
| SuperChatの金額に対して数値計算（合算・比較）を行わない | 通貨が異なるため数値加算は不正確（¥500 + $5 ≠ 505）。為替レート取得は複雑さとコストを増す |
| 集計はYouTubeが返す色情報（tier）に基づく | YouTubeがtierを色で表現しており、同じ基準で通貨横断的に集計可能 |
| `amount` フィールドは表示用文字列（"¥500"等）としてのみ保持する | パース・計算を行わず、ユーザーへの表示とエクスポートにのみ使用 |
| ギフトはジュエル数が分かったものだけを合計し、分からないものを推測で埋めない | ギフトの価格は YouTube 側で変わりうる。不明件数を別に示せば合計の意味が保たれる |

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `get_revenue_analytics` | なし | `RevenueAnalytics` | 現在セッションの分析 |
| `get_session_analytics` | `session_id: String` | `RevenueAnalytics` | 過去セッションの分析 |
| `export_session_data` | `session_id, file_path, config` | `()` | セッションデータエクスポート |
| `export_current_messages` | `file_path, config` | `()` | 現在メッセージエクスポート（多接続時は全接続のメッセージを対象） |

## データモデル

### RevenueAnalytics

```rust
pub struct RevenueAnalytics {
    pub super_chat_count: usize,
    pub super_chat_by_tier: SuperChatTierStats,
    pub super_sticker_count: usize,
    pub membership_gains: usize,
    pub hourly_stats: Vec<HourlyStats>,
    pub top_contributors: Vec<ContributorInfo>,
    pub gifts: GiftStats,
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `super_chat_count` | usize | SuperChat総件数 |
| `super_chat_by_tier` | SuperChatTierStats | tier別SuperChat件数 |
| `super_sticker_count` | usize | SuperSticker総件数 |
| `membership_gains` | usize | メンバーシップ獲得数 |
| `hourly_stats` | Vec | 時間別統計データ（現在は常に空。将来実装予定） |
| `top_contributors` | Vec | 上位貢献者（件数ベース、`get_revenue_analytics`のみで集計） |
| `gifts` | GiftStats | ギフト集計（`get_revenue_analytics`・`get_session_analytics` の両方で集計） |

### GiftStats

```rust
pub struct GiftStats {
    pub gift_count: usize,                 // ギフトの件数
    pub gifts_by_name: Vec<GiftNameCount>, // 名前別件数（件数降順、同数は名前昇順）
    pub jewel_known_count: usize,          // ジュエル数が分かったギフトの件数
    pub total_jewels: u64,                 // その合計
}

pub struct GiftNameCount {
    pub gift_name: String,
    pub gift_image_url: Option<String>,    // 最初に見たものの画像URL
    pub count: usize,
}
```

`get_session_analytics` は DB の `messages.metadata`（08_database.md）の `gift_name`・`gift_image_url`・`jewel_count` から集計する。

### SuperChatTierStats

YouTubeのSuperChat色（tier）別の件数。色はAPIレスポンスの `headerBackgroundColor` から判定。

```rust
pub struct SuperChatTierStats {
    pub tier_red: usize,      // 最高tier（USD $100-500相当）
    pub tier_magenta: usize,  // USD $50-100相当
    pub tier_orange: usize,   // USD $20-50相当
    pub tier_yellow: usize,   // USD $10-20相当
    pub tier_green: usize,    // USD $5-10相当
    pub tier_cyan: usize,     // USD $2-5相当
    pub tier_blue: usize,     // 最低tier（USD $1-2相当）
    pub tier_unknown: usize,  // 段階不明（色が表に無い・分からない）
}
```

### SuperChatTier

```rust
pub enum SuperChatTier {
    Blue,     // 最低
    Cyan,
    Green,
    Yellow,
    Orange,
    Magenta,
    Red,      // 最高
}
```

### HourlyStats

```rust
pub struct HourlyStats {
    pub hour: String,              // "2025-01-14T14:00:00Z"
    pub super_chat_count: usize,
    pub super_sticker_count: usize,
    pub membership_count: usize,
    pub message_count: usize,
}
```

### ContributorInfo

```rust
pub struct ContributorInfo {
    pub channel_id: String,
    pub display_name: String,
    pub super_chat_count: usize,
    pub highest_tier: Option<SuperChatTier>,
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `channel_id` | String | YouTubeチャンネルID |
| `display_name` | String | 表示名 |
| `super_chat_count` | usize | SuperChat件数 |
| `highest_tier` | Option | 最高tierの色 |

## Tier判定

### 色情報の取得

YouTubeのAPIレスポンスには色情報が含まれる：

```rust
pub struct LiveChatPaidMessageRenderer {
    pub header_background_color: u64,  // tierの判定に使用
    pub body_background_color: u64,
    // ...
}
```

### Tier判定ロジック

パーサがヘッダー背景色を `#RRGGBB` にし（`SuperChatColors.header_background`）、[Tier別集計](#tier別集計) の表と完全一致で判定する。一致しなければ段階不明。
過去セッションは DB の `messages.superchat_color`（08_database.md）から同じ判定をする。

### 設計理由

金額ベースの計算を行わない理由：
- 通貨が異なるため単純な数値加算は不正確（¥500 + $5 ≠ 505）
- 為替レート取得は複雑さとコストを増す
- YouTubeがtierを色で表現しているため、同じ基準で集計可能

## 集計処理

### SuperChat集計

```
1. メッセージ受信（type: SuperChat）
        ↓
2. header_background_color からtierを判定
        ↓
3. 該当tierのカウントをインクリメント
        ↓
4. super_chat_count をインクリメント
        ↓
5. 貢献者情報を更新
```

### メンバーシップカウント

- Membership メッセージ受信時に `membership_gains` をインクリメント
- 新規加入とマイルストーンの両方をカウント

### 上位貢献者の更新

- SuperChat件数でソート
- 同一件数の場合は最高tierで比較
- 上位10人を保持

## エクスポート機能

### 対応形式

| 形式 | 拡張子 | 説明 |
|-----|-------|------|
| CSV | `.csv` | カンマ区切りテキスト |
| JSON | `.json` | 構造化データ |

### ExportConfig

```rust
pub struct ExportConfig {
    pub format: String,                    // "csv" or "json"
    pub include_metadata: bool,
    pub include_system_messages: bool,     // 現在未使用（将来用）
    pub max_records: Option<usize>,
    pub sort_order: Option<String>,        // 現在未使用（将来用）
}
```

> **未実装フィールド**: `date_range`（日付範囲フィルタ）、`sort_order`（ソート順）、`include_system_messages`（システムメッセージ除外）は将来の実装予定。現在のエクスポートは全メッセージを時系列順で出力する。

### エクスポート対象データ

```rust
pub struct ExportableData {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub message_type: String,
    pub amount_display: Option<String>,  // 表示用金額文字列（"¥500"等）
    pub tier: Option<SuperChatTier>,     // SuperChatのtier（superchat 以外と段階不明は None）
    pub is_moderator: Option<bool>,      // None = 不明
    pub is_member: bool,
    pub is_verified: Option<bool>,       // None = 不明
    pub badges: Option<Vec<String>>,     // None = 不明
}
```

`is_moderator`・`is_verified`・`badges` は、バッジを読めない種類（membership_gift・gift・system）と、バッジを保存する前（マイグレーション 005 より前）の過去セッションの行では不明（None）にする（08_database.md）。不明を false や空配列にしない。

| 出力 | 不明のとき |
|------|-----------|
| CSV | 空欄 |
| JSON | `null` |

### CSV形式

**ヘッダー:**
```
id,timestamp,author,author_id,content,message_type,amount_display,tier,is_moderator,is_member,is_verified,badges
```

**メタデータセクション（オプション）:**
```
# Metadata
# Session ID,<session_id>
# Channel,<channel_name>
# Stream URL,<stream_url>
# Start Time,<start_time>
# End Time,<end_time>
# Total Messages,<count>
# Unique Viewers,<count>
# SuperChat Count,<count>
# Export Time,<export_time>
```

### JSON形式

```json
{
  "metadata": {
    "session_id": "...",
    "stream_title": "...",
    "channel_name": "...",
    "start_time": "2025-01-14T...",
    "end_time": "2025-01-14T...",
    "filters_applied": [...]
  },
  "messages": [...],
  "statistics": {
    "total_messages": 100,
    "unique_viewers": 50,
    "super_chat_count": 15,
    "super_chat_by_tier": {
      "red": 1,
      "magenta": 2,
      "orange": 3,
      "yellow": 4,
      "green": 3,
      "cyan": 1,
      "blue": 1
    },
    "message_type_distribution": {}
  }
}
```

## フロントエンド

### RevenueDashboard.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| 画面表示 | `get_revenue_analytics`呼び出し、統計表示 |
| 「更新」クリック | `get_revenue_analytics`呼び出し、統計更新 |

### 表示項目

```
統計ダッシュボード
├─ 概要
│   ├─ SuperChat総件数
│   ├─ SuperSticker総件数
│   └─ メンバーシップ獲得数
├─ ギフト
│   ├─ 件数・ジュエル合計・ジュエル数不明の件数
│   └─ 名前別件数（画像付き）
├─ SuperChat tier別内訳
│   ├─ 赤: X件
│   ├─ マゼンタ: X件
│   ├─ オレンジ: X件
│   ├─ 黄: X件
│   ├─ 緑: X件
│   ├─ 水色: X件
│   └─ 青: X件
├─ 時間別グラフ
└─ 上位貢献者リスト
```

### ExportPanel.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| フォーマット選択 | CSV/JSON を選択 |
| オプション設定 | メタデータ含有、日付範囲等を設定 |
| 「エクスポート」クリック | ファイルダイアログ表示、エクスポート実行 |

## TypeScript型定義

```typescript
interface RevenueAnalytics {
    super_chat_count: number;
    super_chat_by_tier: SuperChatTierStats;
    super_sticker_count: number;
    membership_gains: number;
    hourly_stats: HourlyStats[];
    top_contributors: ContributorInfo[];
    gifts: GiftStats;
}

interface GiftStats {
    gift_count: number;
    gifts_by_name: { gift_name: string; gift_image_url: string | null; count: number }[];
    jewel_known_count: number;
    total_jewels: number;
}

interface SuperChatTierStats {
    tier_red: number;
    tier_magenta: number;
    tier_orange: number;
    tier_yellow: number;
    tier_green: number;
    tier_cyan: number;
    tier_blue: number;
}

type SuperChatTier = 'blue' | 'cyan' | 'green' | 'yellow' | 'orange' | 'magenta' | 'red';

interface HourlyStats {
    hour: string;
    super_chat_count: number;
    super_sticker_count: number;
    membership_count: number;
    message_count: number;
}

interface ContributorInfo {
    channel_id: string;
    display_name: string;
    super_chat_count: number;
    highest_tier: SuperChatTier | null;
}

interface ExportConfig {
    format: string;
    include_metadata: boolean;
    include_system_messages: boolean;
    max_records: number | null;
    sort_order: string | null;
}
```

## 永続化

統計データは以下のテーブルに保存：

| テーブル | 用途 |
|---------|------|
| `hourly_stats` | 時間別統計データ |
| `contributor_stats` | 貢献者統計 |

詳細は[データベース仕様](08_database.md)を参照。
