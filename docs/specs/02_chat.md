# チャット接続・モニタリング機能

## 概要

YouTube Live配信に接続し、リアルタイムでチャットメッセージを取得・表示する。

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `connect_to_stream` | `url: String, chat_mode: Option<String>` | `ConnectionResult` | 配信に接続 |
| `disconnect_stream` | なし | `()` | 切断 |
| `get_chat_messages` | `limit: Option<usize>` | `Vec<GuiChatMessage>` | メッセージ取得（デフォルト100件） |
| `set_chat_mode` | `mode: String` | `bool` | チャットモード切り替え |

## データモデル

### GuiChatMessage

フロントエンドに送信されるチャットメッセージ。

```rust
pub struct GuiChatMessage {
    pub id: String,                           // メッセージID（YouTube上で一意）
    pub timestamp: String,                    // タイムスタンプ（RFC3339/ISO8601形式、UTC）
    pub timestamp_usec: String,               // マイクロ秒単位タイムスタンプ
    pub author: String,                       // チャットユーザー名
    pub author_icon_url: Option<String>,      // プロフィール画像URL
    pub channel_id: String,                   // ユーザーのチャンネルID
    pub content: String,                      // メッセージテキスト（絵文字は代替テキスト）
    pub runs: Vec<MessageRun>,                // テキストと絵文字の混在リスト
    pub message_type: String,                 // メッセージ種別
    pub amount: Option<String>,               // スーパーチャット/スーパーステッカーの金額
    pub is_member: bool,                      // メンバーシップ登録済み
    pub is_first_time_viewer: bool,           // 初見さん（配信者チャンネルでの初コメント）
    pub in_stream_comment_count: Option<u32>, // この配信（video_id単位）でのコメント回数
    pub metadata: Option<GuiMessageMetadata>, // メタデータ
}
```

### MessageRun

テキストと絵文字の混在表現。

```rust
#[serde(tag = "type")]
pub enum MessageRun {
    Text { content: String },
    Emoji { emoji_id: String, image_url: String, alt_text: String }
}
```

**例:** "こんにちは 😀 元気？" → `[Text("こんにちは "), Emoji(...), Text(" 元気？")]`

### MessageType

| 種別 | 説明 | 付加情報（metadata内） |
|------|------|----------------------|
| `text` | 通常のチャットメッセージ | なし |
| `superchat` | スーパーチャット | `amount`（金額文字列）、`superchat_colors` |
| `supersticker` | スーパーステッカー | `amount`（金額文字列）、`superchat_colors` |
| `membership` | メンバーシップ新規/更新 | `milestone_months`（マイルストーン月数、新規はNone） |
| `membership_gift` | メンバーシップギフト配布 | `gift_count`（ギフト数） |
| `system` | システムメッセージ | なし |

### GuiMessageMetadata

```rust
pub struct GuiMessageMetadata {
    pub amount: Option<String>,                // 金額
    pub milestone_months: Option<u32>,         // メンバーシップマイルストーン月数
    pub gift_count: Option<u32>,               // メンバーシップギフト数
    pub badges: Vec<String>,                   // バッジ識別子
    pub badge_info: Vec<BadgeInfo>,            // バッジ詳細
    pub is_moderator: bool,                    // モデレータ
    pub is_verified: bool,                     // 検証済みアカウント
    pub superchat_colors: Option<SuperChatColors>,
}

pub struct SuperChatColors {
    pub header_background: String,  // "#RRGGBB"
    pub header_text: String,
    pub body_background: String,
    pub body_text: String,
}

pub struct BadgeInfo {
    pub badge_type: String,        // "member", "moderator", "verified", etc.
    pub label: String,             // 表示ラベル
    pub tooltip: Option<String>,   // ツールチップテキスト
    pub image_url: Option<String>, // バッジ画像URL
}
```

### ChatMode

| モード | 説明 |
|-------|------|
| `TopChat` | 重要なメッセージのみ（YouTubeアルゴリズムで選別） |
| `AllChat` | すべてのメッセージ（時系列順） |

### ConnectionResult

```rust
pub struct ConnectionResult {
    pub success: bool,
    pub stream_title: Option<String>,
    pub broadcaster_channel_id: Option<String>,
    pub broadcaster_name: Option<String>,
    pub is_replay: bool,                       // アーカイブ再生判定
    pub error: Option<String>,
    pub session_id: Option<String>,            // DBセッションID
}
```

## InnerTube API

### エンドポイント

| 用途 | URL |
|------|-----|
| メッセージ取得 | `https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={api_key}` |
| アカウントメニュー（認証検証） | `https://www.youtube.com/youtubei/v1/account/account_menu` |

### リクエスト形式

```json
{
    "context": {
        "client": {
            "clientName": "WEB",
            "clientVersion": "{extracted_version}"
        }
    },
    "continuation": "{continuation_token}"
}
```

### Continuation Token

メッセージ取得の継続に使用するトークン。Protocol Buffer形式でエンコードされている。

| データタイプ | 優先度 | 用途 |
|-------------|-------|------|
| `invalidationContinuationData` | 1 | 新メッセージ検出時に即座に通知 |
| `timedContinuationData` | 2 | 指定時間後にポーリング |
| `reloadContinuationData` | 3 | フォールバック |

### チャットモード切り替え

**方式1: Binary Modification（高速）**

Continuation tokenのバイト列を直接修正する。

```
Field 16内のNested Field 1を探索
値: 0x04 = TopChat, 0x01 = AllChat
```

**方式2: Reload Token経由（確実）**

HTMLから取得した`reload_token`を使用して新しいcontinuation tokenを取得する。

### 認証ヘッダー（メンバー限定配信用）

```
Authorization: SAPISIDHASH {hash}
Cookie: SID=...; HSID=...; SSID=...; APISID=...; SAPISID=...
X-Origin: https://www.youtube.com
Origin: https://www.youtube.com
```

## 接続処理フロー

```
1. ユーザーがURLを入力 → connect_to_stream呼び出し
        ↓
2. URLからビデオIDを抽出
        ↓
3. YouTubeページをフェッチ、以下を抽出:
   - API Key
   - Client Version
   - Continuation Token
   - 配信者情報（チャンネルID、名前）
   - チャットモード切替用トークン
        ↓
4. InnerTubeクライアントを初期化
        ↓
5. 同一配信への再接続か確認
   ├─ 再接続の場合 → DBから前回セッションのメッセージを復元
   └─ 新規接続の場合 → 新規セッションをDBに作成
        ↓
6. 視聴者カスタム情報をDBからプリロード
        ↓
7. 配信内コメント数カウンタを初期化
   ├─ 再接続の場合 → DBから該当video_idの全メッセージをchannel_idごとに集計して復元
   └─ 新規接続の場合 → 空のHashMapで初期化
        ↓
8. チャット監視タスクを起動（tokio::spawn）
        ↓
9. ConnectionResultをフロントエンドに返却
```

### メッセージ復元

同一配信（同一video_id）への再接続時、DBから前回取得したメッセージを復元する。

| 項目 | 仕様 |
|-----|------|
| 判定条件 | video_idが一致するセッションがDBに存在 |
| 復元対象 | 該当セッションの全メッセージ |
| 復元順序 | timestamp_usec昇順（古い順） |
| 重複排除 | message_idで重複チェック |

## チャット監視タスク

```
┌─ ループ（1.5秒ごと）─────────────────────────┐
│ 1. is_monitoringフラグを確認                  │
│ 2. fetch_messages_with_raw()でAPI呼び出し     │
│    └─ 新しいcontinuation tokenを取得          │
│ 3. 各メッセージを処理:                         │
│    ├─ 配信内コメント数カウンタ更新              │
│    ├─ DBに保存（save_message）                │
│    │   ├─ INSERT OR IGNORE (messages)         │
│    │   ├─ upsert_viewer_profile               │
│    │   └─ upsert_viewer_stream(video_id)      │
│    ├─ is_first_time_viewer(video_id)で初見判定 │
│    ├─ メモリバッファに追加                     │
│    ├─ GuiChatMessageに初見・回数を付与         │
│    └─ Tauriイベントを発行                     │
│ 4. sleep(1500ms)                              │
└───────────────────────────────────────────────┘
```

### 初見さん判定

配信者チャンネルにおいて初めてコメントした視聴者を判定する。

#### データ構造: viewer_streams テーブル

各視聴者がどの配信にコメントしたかを記録する。

```sql
CREATE TABLE viewer_streams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    viewer_profile_id INTEGER NOT NULL,     -- viewer_profiles.id
    video_id TEXT NOT NULL,                 -- 配信のvideo_id
    first_comment_at TEXT NOT NULL,         -- この配信での最初のコメント時刻
    last_comment_at TEXT NOT NULL,          -- この配信での最新のコメント時刻
    message_count INTEGER DEFAULT 1,        -- この配信でのコメント数
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (viewer_profile_id) REFERENCES viewer_profiles(id) ON DELETE CASCADE,
    UNIQUE(viewer_profile_id, video_id)
);
```

#### 判定方法

`viewer_streams` テーブルで、視聴者の最古の `video_id` が現在の配信の `video_id` と一致するかで判定する。

```
メッセージ受信時:
  1. save_message → upsert_viewer_profile (viewer_profile_id を取得)
                  → upsert_viewer_stream(viewer_profile_id, video_id)
  2. is_first_time_viewer(broadcaster_id, channel_id, video_id) で判定:
     oldest_video_id = SELECT video_id FROM viewer_streams vs
                       JOIN viewer_profiles vp ON vs.viewer_profile_id = vp.id
                       WHERE vp.broadcaster_channel_id = ? AND vp.channel_id = ?
                       ORDER BY vs.first_comment_at ASC LIMIT 1
     msg.is_first_time_viewer = (oldest_video_id == current_video_id)
```

#### 注意事項

| ポイント | 説明 |
|---------|------|
| 配信者スコープ | `viewer_profiles.broadcaster_channel_id` 経由で配信者スコープを維持 |
| 永続性 | `viewer_streams` テーブルに基づくためアプリ再起動後も正確 |
| 配信内一貫性 | DB問い合わせのため、再接続後も自動的に正しい判定を維持（メモリキャッシュ不要） |
| 判定タイミング | `save_message`（`upsert_viewer_stream` 含む）の**後**に判定する |
| システムメッセージ | `message_type == "system"` のメッセージは判定対象外 |
| 将来の拡張 | 視聴者の訪問配信一覧、前回来た配信の特定等に利用可能 |

### 配信内コメント数カウンタ

配信（video_id）単位での視聴者ごとのコメント回数を管理する、メモリ上のカウンタ。

#### データ構造

```rust
/// video_id単位の視聴者コメント数カウンタ
/// key: channel_id, value: コメント回数
type InStreamCommentCounter = HashMap<String, u32>;
```

#### ライフサイクル

| イベント | 動作 |
|---------|------|
| 新規接続 | 空のHashMapを作成 |
| 再接続（同一video_id） | DBから該当セッションのメッセージを集計し、channel_idごとのカウントで初期化 |
| メッセージ受信 | カウンタをインクリメントし、現在値を `in_stream_comment_count` に設定 |
| 切断 | カウンタを破棄（DBにメッセージが保存されているため復元可能） |

#### 再接続時の復元SQL

```sql
SELECT m.channel_id, COUNT(*) as count
FROM messages m
JOIN sessions s ON m.session_id = s.id
WHERE s.stream_url LIKE '%{video_id}%'
GROUP BY m.channel_id
```

#### パフォーマンス

| 操作 | 計算量 |
|------|--------|
| カウント参照 | O(1) - HashMap lookup |
| カウント更新 | O(1) - HashMap insert/update |
| 初期化（再接続時） | O(N) - Nはユニーク視聴者数 |

### 設定値

| 項目 | 値 |
|-----|-----|
| ポーリング間隔 | 1,500ms |
| メモリバッファ上限（Backend） | 1,000件 |
| デフォルトAPI Key | `AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8` |

## Tauriイベント

| イベント名 | ペイロード | 説明 |
|-----------|-----------|------|
| `chat:connection` | `ConnectionResult` | 接続状態変更 |
| `chat:message` | `GuiChatMessage` | 新着メッセージ |

## フロントエンド

### コンポーネント

| コンポーネント | 役割 |
|--------------|------|
| `InputSection.svelte` | URL入力、接続/停止ボタン、統計情報表示 |
| `ChatDisplay.svelte` | メッセージ一覧表示 |
| `FilterPanel.svelte` | メッセージフィルタリング、表示制御 |

### Store状態（chat.svelte.ts）

#### 接続・メッセージ関連

| 状態 | 型 | 説明 |
|-----|-----|------|
| `messages` | `ChatMessage[]` | メッセージ配列 |
| `filteredMessages` | `ChatMessage[]` | フィルタ済みメッセージ（derived） |
| `displayedMessages` | `ChatMessage[]` | displayLimit適用後の表示用メッセージ（derived） |
| `connectionState` | `ConnectionState` | 接続状態（idle/connecting/connected/paused/error） |
| `isConnected` | `boolean` | 接続状態（derived: connectionState === 'connected'） |
| `isConnecting` | `boolean` | 接続処理中（derived: connectionState === 'connecting'） |
| `isPaused` | `boolean` | 一時停止中（derived: connectionState === 'paused'） |
| `streamTitle` | `string \| null` | 配信タイトル |
| `broadcasterName` | `string \| null` | 配信者名 |
| `broadcasterChannelId` | `string \| null` | 配信者チャンネルID |
| `isReplay` | `boolean` | アーカイブ再生判定 |
| `chatMode` | `ChatMode` | TopChat / AllChat |
| `error` | `string \| null` | エラーメッセージ |

#### 表示設定関連

| 状態 | 型 | 説明 |
|-----|-----|------|
| `messageFontSize` | `number` | フォントサイズ（10〜24px、デフォルト13px） |
| `showTimestamps` | `boolean` | タイムスタンプ表示 |
| `autoScroll` | `boolean` | 自動スクロール有効 |
| `displayLimit` | `number \| null` | 表示件数制限（null=無制限） |
| `scrollToLatestTrigger` | `number` | スクロールトリガー（インクリメントで発火） |

#### 視聴者情報パネル関連

| 状態 | 型 | 説明 |
|-----|-----|------|
| `selectedViewer` | `SelectedViewer \| null` | 選択中の視聴者 |
| `showViewerInfoPanel` | `boolean` | 視聴者情報パネル表示 |
| `viewerCustomInfoCache` | `Map<string, ViewerCustomInfo>` | 視聴者カスタム情報キャッシュ |

### フィルタ機能

```typescript
interface ChatFilter {
    showText: boolean;        // 通常チャット表示
    showSuperchat: boolean;   // スーパーチャット/ステッカー表示
    showMembership: boolean;  // メンバーシップ関連表示
    searchQuery: string;      // 検索クエリ（著者/コンテンツ）
}
```

### ユーザー操作

| 操作 | 動作 |
|-----|------|
| URL入力 + 「開始」クリック | `connect_to_stream`呼び出し |
| 「停止」クリック | `disconnect_stream`呼び出し、一時停止状態へ |
| 「再開」クリック | 一時停止状態から再接続 |
| 「初期化」クリック | 接続状態をクリア、アイドル状態へ |
| チャットモード切り替え | `set_chat_mode`呼び出し |
| フィルタ選択 | ローカルでフィルタリング |
| フォントサイズ変更 | ローカル設定を更新 |

### 接続状態遷移

```
                開始
Idle ─────────────→ Connecting ─────→ Connected
  ↑                                      │
  │              初期化                   │ 停止
  │         ←───────────────           ↓
  └───────────── Paused ←──────────────┘
                   │ 再開
                   └──────→ Connecting → Connected
```

| 状態 | 説明 | UI表示 |
|-----|------|-------|
| `idle` | 未接続、初期状態 | URL入力欄 + 「開始」ボタン |
| `connecting` | 接続処理中 | 「接続中...」表示 |
| `connected` | 接続済み、メッセージ受信中 | 配信タイトル + 「停止」ボタン |
| `paused` | 一時停止中、メッセージ保持 | 「再開」「初期化」ボタン |
| `error` | エラー発生 | エラーメッセージ表示 |

**一時停止状態の特徴:**
- メッセージは保持される（クリアされない）
- 配信情報は保持される
- 「再開」で同じ配信に再接続可能
- 「初期化」でアイドル状態に戻る

## メッセージ表示仕様

配信者が視聴者対応を判断するために必要な情報を、視覚的優先度を考慮して表示する。

### メッセージレイアウト

各メッセージは2行構成で表示する。

```
┌──────────────────────────────────────────────────────────────────────┐
│ [時刻] [アイコン] [著者名] [読み仮名] [バッジ群] [初見] [コメント数] │ ← 第1行：メタデータ行
│         [メッセージ本文（テキスト＋絵文字）]                          │ ← 第2行：本文行
└──────────────────────────────────────────────────────────────────────┘
```

#### 第1行：メタデータ行の構成要素

| 要素 | サイズ | 色 | 備考 |
|------|--------|-----|------|
| タイムスタンプ | 10px固定 | `var(--text-muted)` | HH:MM:SS形式（ローカルタイムゾーン）、表示/非表示切り替え可 |
| アイコン | 20×20px | - | 円形、object-fit: cover |
| 著者名 | フォント設定値 | メンバー: `var(--member-accent)`、非メンバー: `var(--accent)` | font-weight: 600、最大200px |
| 読み仮名 | 11px固定 | `var(--text-muted)` | 登録時のみ表示 |
| バッジ画像 | 16×16px | - | 複数表示可 |
| 初見バッジ | フォント設定値 | 下記参照 | `is_first_time_viewer`がtrueの場合のみ表示。配信内コメント回数に応じて表示が変化 |
| 配信内コメント回数 | フォント設定値 | 下記参照 | 初見バッジとは独立して表示。`#1` のみ目立つ色 |

#### 第2行：本文行

- 左インデント: 8px（アイコン分）
- フォントサイズ: ユーザー設定値（10〜24px）
- 行間: 1.4

### メッセージタイプ別表示スタイル

#### 通常チャット（text）

| 項目 | 値 |
|-----|-----|
| 背景 | `var(--bg-surface-2)` |
| 左枠線 | 4px solid `var(--accent)` |

**メンバーの場合:**

| 項目 | 値 |
|-----|-----|
| 背景 | `var(--member-subtle)` |
| 左枠線 | 4px solid `var(--member-accent)` |

#### スーパーチャット（superchat）

**ヘッダー行:**
```
[💰] Super Chat [金額バッジ]
```

| 項目 | 値 |
|-----|-----|
| 背景 | グラデーション（YouTube API提供色） |
| 左枠線 | 4px solid（ヘッダー背景色） |
| 金額バッジ | padding: 4px 12px、border-radius: 16px |

**配色（YouTubeから取得）:**

| プロパティ | 用途 |
|-----------|------|
| `header_background` | ヘッダー背景、左枠線 |
| `header_text` | ヘッダーテキスト（通常は白） |
| `body_background` | 本文背景 |
| `body_text` | 本文テキスト |

**背景色:**

SuperChatの色情報がある場合はYouTube APIから取得した色を使用。色情報がない場合は `var(--bg-surface-2)` をフォールバックとして使用。

#### スーパーステッカー（supersticker）

**ヘッダー行:**
```
[🎨] Super Sticker [金額バッジ]
```

| 項目 | デフォルト値 |
|-----|-------------|
| 背景 | YouTube API提供色、なければ `var(--bg-surface-2)` |
| 左枠線 | 4px solid（ヘッダー背景色） |

#### メンバーシップ（membership）

**新規メンバー加入（milestone_months: None）:**

```
[🎉] メンバー加入
```

| 項目 | 値 |
|-----|-----|
| 背景 | `var(--member-subtle)` |
| 左枠線 | 4px solid `var(--member-accent)` |

**マイルストーン達成（milestone_months: Some(n)）:**

```
[🏆] マイルストーン [{n}ヶ月バッジ]
```

| 項目 | 値 |
|-----|-----|
| 背景 | `var(--bg-surface-2)` |
| 左枠線 | 4px solid `var(--accent)` |

#### メンバーシップギフト（membership_gift）

```
[🎁] メンバーシップギフト [{gift_count}人バッジ]
```

| 項目 | 値 |
|-----|-----|
| 背景 | `var(--info-subtle)` |
| 左枠線 | 4px solid `var(--info)` |

#### システムメッセージ（system）

| 項目 | 値 |
|-----|-----|
| 背景 | `var(--info-subtle)` |
| 左枠線 | 4px solid `var(--info)` |
| テキスト色 | `var(--text-secondary)` |
| 著者 | "System"（アイコンなし） |

### バッジ表示

#### バッジの種類

| バッジ | 判定条件 | 表示 |
|-------|---------|------|
| メンバー | tooltip含む "メンバー" or "Member" | `var(--member-subtle)` 背景 + `var(--member-accent)` テキスト |
| モデレーター | tooltip含む "モデレーター" or "Moderator" | `var(--info-subtle)` 背景 + `var(--info)` テキスト |
| 認証済み | tooltip含む "認証" or "Verified" | `var(--bg-surface-3)` 背景 + `var(--text-secondary)` テキスト |

#### バッジ表示優先順位

1. YouTube API提供の画像バッジ（`badge_info[].image_url`）
2. テキストフォールバック

### メンバー識別

配信者がメンバーを一目で識別できるよう、複数の視覚的手がかりを提供する。

| 手がかり | 表示 |
|---------|------|
| 著者名の色 | メンバー: `var(--member-accent)`、非メンバー: `var(--accent)` |
| メッセージ背景 | メンバー: `var(--member-subtle)` |
| バッジ | メンバーバッジ画像または「Member」テキスト |
| 枠線 | メンバー: `var(--member-accent)` |

### 初見さんバッジ

配信者チャンネルで初めてコメントした視聴者（初見さん）を識別するためのバッジ。
初見さんは配信内で一貫して表示され、コメント回数に応じて表示スタイルが変化する。

| 条件 | 表示 | スタイル |
|------|------|---------|
| `is_first_time_viewer == true` かつ `in_stream_comment_count == 1` | `🎉初見さん` | `var(--success)` テキスト、`var(--success-subtle)` 背景、太字、border-radius: 4px、padding: 1px 6px |
| `is_first_time_viewer == true` かつ `in_stream_comment_count > 1` | `初見さん` | `var(--text-muted)` テキスト、背景なし |
| `is_first_time_viewer == false` | （非表示） | - |

**初見さんの判定基準:**

| 項目 | 仕様 |
|------|------|
| スコープ | 配信者チャンネル単位（`viewer_profiles` の `(broadcaster_channel_id, channel_id)` ペア） |
| 判定タイミング | 配信内でそのビューワーの最初のメッセージ受信時に `viewer_exists` でDBチェック。結果は `HashSet<String>` にキャッシュし、配信中は一貫して同じ判定を維持 |
| 永続性 | `viewer_profiles` テーブルに基づくため、アプリ再起動後も同一視聴者は初見扱いにならない |
| 配信者間の独立性 | 配信者Aで初見でも、配信者Bでは別途初見判定される |
| 再接続時の復元 | `in_stream_counts` のキー（DBから復元済み）に対して `viewer_exists` で再チェックし、`first_time_viewers` セットを復元する |
| 配信内一貫性 | 一度初見と判定されたビューワーは、その配信中は何回コメントしても `is_first_time_viewer == true` を維持する |

### 配信内コメント回数表示

同一配信（video_id単位）内での視聴者のコメント回数を表示する。

| 回数 | 表示 | スタイル |
|------|------|---------|
| 1回目 | `#1` | `var(--warning)` テキスト、太字 |
| 2回目以降 | `#N` | `var(--text-muted)` テキスト |
| 取得不可 | （非表示） | - |

**配信内コメント回数の仕様:**

| 項目 | 仕様 |
|------|------|
| スコープ | video_id単位（URLから抽出） |
| カウント対象 | 当該video_idのセッションに紐づく全メッセージ |
| 再接続時 | DBから復元した前回セッションのメッセージも含めてカウント |
| カウント方法 | メモリ上のHashMap<channel_id, u32>でO(1)管理 |

**初見バッジとの同時表示:**

初見さんバッジと配信内コメント回数は独立した要素として同時に表示できる。

| ケース | 表示例 |
|--------|--------|
| チャンネル初見 + 配信1回目 | `🎉初見さん` `#1` |
| チャンネル初見 + 配信2回目以降 | `初見さん` `#N` |
| チャンネル初見ではない + 配信1回目 | `#1` |
| チャンネル初見ではない + 配信3回目 | `#3` |

### 絵文字表示

| 項目 | 値 |
|-----|-----|
| サイズ | フォントサイズ + 4px |
| 配置 | インライン（vertical-align: middle） |
| 余白 | 左右2px |

### インタラクション

#### ホバー時

| 効果 | 値 |
|-----|-----|
| 移動 | translateY(-1px) |
| 影 | box-shadow: 0 4px 12px rgba(0,0,0,0.1) |
| 遷移 | all 0.2s ease |

#### クリック時

メッセージクリックで**視聴者情報パネル**（ViewerInfoPanel）を表示。詳細は「視聴者情報パネル」セクションを参照。

#### 選択状態

| 効果 | 値 |
|-----|-----|
| 枠線 | 2px solid `var(--accent)` |
| 影 | box-shadow: 0 0 8px rgba(56,189,248,0.4) |

### スクロール動作

#### 自動スクロール

自動スクロールはチェックボックス（`autoScrollEnabled`）のみで制御する。スクロール位置やユーザーの手動スクロールによる自動制御は行わない。

| 項目 | 値 |
|-----|-----|
| トリガー | 新着メッセージ到着時 |
| 実行条件 | `autoScrollEnabled` チェックボックスがON |
| デフォルト | ON |

#### コントロール

| UI要素 | 動作 |
|-------|------|
| 自動スクロール チェックボックス | ON/OFFで自動スクロールを制御 |
| 最新に戻る ボタン | チェックボックスをONにして最新までスクロール |

#### ボタン表示条件

「最新に戻る」ボタンは自動スクロールがOFFの時のみ表示される。

| 条件 | ボタン表示 |
|-----|----------|
| チェックボックスON | 非表示 |
| チェックボックスOFF | 表示 |

#### スクロール実行の信頼性

新着メッセージ到着時のスクロールは複数回試行して確実に最下部まで到達する。

| 試行 | タイミング | 目的 |
|-----|----------|------|
| 1回目 | 即座 | 基本的なスクロール |
| 2回目 | requestAnimationFrame後 | DOMレンダリング完了後 |
| 3回目 | 50ms後 | 画像等の非同期コンテンツ読み込み後 |

### 表示設定

#### フォントサイズ

| 項目 | 値 |
|-----|-----|
| 範囲 | 10〜24px |
| デフォルト | 13px |
| 調整 | A-/A+ ボタン（±1px） |
| 永続化 | `config.toml` に保存、次回起動時に復元 |

※設定の詳細は[設定機能仕様](09_config.md)を参照

**影響範囲:**
- メッセージ本文
- 著者名
- 絵文字サイズ（フォント+4px）

※タイムスタンプ（10px）、バッジ（9px）は固定

#### 表示切り替え

| 項目 | デフォルト |
|-----|----------|
| タイムスタンプ表示 | ON |
| 自動スクロール | ON |

### タイムゾーン変換

タイムスタンプはバックエンドでUTCとして保存され、フロントエンドでユーザーのローカルタイムゾーンに変換して表示される。

#### データフロー

```
YouTube API (timestampUsec: マイクロ秒)
    ↓ Backend: chrono::DateTime::from_timestamp()
UTC DateTime (RFC3339形式)
    ↓ Frontend: new Date() + toLocaleTimeString()
ローカルタイムゾーン (HH:MM:SS形式)
```

#### 変換仕様

| レイヤー | 形式 | 例（JST +09:00の場合） |
|---------|------|----------------------|
| YouTube API | マイクロ秒 | `1737020127000000` |
| Backend保存 | RFC3339 (UTC) | `2025-01-16T04:55:27+00:00` |
| Frontend表示 | HH:MM:SS (ローカル) | `13:55:27` |

#### 実装詳細

**Backend (Rust):**
```rust
fn format_timestamp(timestamp_usec: &str) -> String {
    let secs = usec / 1_000_000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0);
    datetime.to_rfc3339()  // UTC RFC3339形式
}
```

**Frontend (TypeScript):**
```typescript
const date = new Date(message.timestamp);  // RFC3339をパース
date.toLocaleTimeString('ja-JP', {         // ローカルタイムゾーンに変換
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
});
```

#### 注意事項

- データベースにはUTCで保存（国際標準に準拠）
- 表示はユーザーのシステム設定に基づくタイムゾーンで変換
- `timestamp_usec`フィールドは元のマイクロ秒精度を保持（必要に応じて使用可能）

#### 表示件数制限

| オプション | 説明 |
|-----------|------|
| 50件 | 軽量表示 |
| 100件 | - |
| 200件 | - |
| 500件 | - |
| 無制限 | デフォルト |

※上限を設定した場合、超過した古いメッセージはアーカイブに移動

#### 実装詳細

- `displayedMessages` = `filteredMessages.slice(-displayLimit)`（displayLimitがnullの場合はfilteredMessagesと同一）
- `filteredMessages` はフィルタのみ適用（ステータスバーの件数表示に使用）
- アーカイブ = `messages` 配列内に存在するが `displayedMessages` に含まれないメッセージ
- ViewerInfoPanelの過去コメントは `messages` 配列（全件）を参照するため、アーカイブ済みメッセージも表示可能

#### フロントエンドメッセージ上限

フロントエンド（`messages` 配列）に保持するメッセージの件数に上限はない。仮想スクロールにより、大量メッセージでもDOMにはビューポート近辺のみレンダリングされるため、パフォーマンスへの影響は限定的。

#### 仮想スクロール

チャットメッセージの表示にはライブラリ `virtua` による仮想スクロールを使用する。
DOMにはビューポート近辺のメッセージのみレンダリングされ、大量メッセージ（1万件以上）でもスムーズにスクロール可能。

### 配信者向け情報の優先度

配信者が一目で確認すべき情報を優先度順に配置。

| 優先度 | 情報 | 表示位置 |
|-------|------|---------|
| 最高 | メッセージテキスト | 第2行 |
| 最高 | 投稿者名 | 第1行左（色分け） |
| 高 | メンバー/非メンバー | 著者名色 + 背景色 + バッジ |
| 高 | スーパーチャット金額 | ヘッダー行バッジ + 配色 |
| 高 | 初見さん | 第1行（🎉NEWバッジ） |
| 中 | 投稿者アイコン | 第1行左 |
| 中 | 読み仮名 | 第1行（登録時） |
| 中 | バッジ（モデ、認証） | 第1行右 |
| 低 | 配信内コメント回数 | 第1行右 |
| 低 | タイムスタンプ | 第1行左端 |

## 視聴者情報パネル

配信者が視聴者を識別・管理するためのスライドインパネル。

### レイアウト

```
┌──────────────────────────────────┐
│ ▶ 視聴者情報                   ✕ │ ← ヘッダー
├──────────────────────────────────┤
│ [アイコン 56×56]                 │
│ 表示名 (読み仮名)                │
├──────────────────────────────────┤
│ Channel ID: UCxxx...             │
├──────────────────────────────────┤
│ 読み仮名（ふりがな）             │
│ [入力欄]                         │
│ ※視聴者名の横に括弧書きで表示    │
│ [保存] ✓ 保存しました            │
├──────────────────────────────────┤
│ 投稿されたコメント (N件/新着順)   │
│ ┌─────────────────────────────┐ │
│ │ 12:34:56                    │ │
│ │ コメント内容...              │ │
│ │ [💰 ¥500] [メンバーバッジ]   │ │
│ └─────────────────────────────┘ │
│  (最大高300px, スクロール可)     │
└──────────────────────────────────┘
```

### パネルスタイル

| 項目 | 値 |
|-----|-----|
| 幅 | 320px |
| 位置 | 右側固定 |
| 背景 | `var(--bg-surface-1)` |
| ヘッダー背景 | `var(--bg-surface-2)` |
| 入力欄背景 | `var(--bg-surface-3)` |
| アニメーション | slideIn 0.25s ease-out（右から左へ） |
| z-index | 50 |
| シャドウ | `-4px 0 12px rgba(0,0,0,0.3)` |

### 構成要素

| 要素 | サイズ | 色 | 備考 |
|------|--------|-----|------|
| ヘッダータイトル | 20px | `var(--text-primary)` | "視聴者情報" |
| 閉じるボタン | 16px | `var(--text-secondary)` | "✕" |
| アイコン | 56×56px | - | 円形、なければSVGユーザーアイコン |
| 視聴者名 | 18px | `var(--text-primary)` | font-weight: 600 |
| 読み仮名表示 | 16px | `var(--accent)` | 括弧内 |
| チャンネルID | 13px | `var(--text-muted)` | word-break適用 |
| ラベル | 16px | `var(--text-primary)` | - |
| 入力欄 | - | `var(--text-primary)` | padding: 12px 14px |
| 保存ボタン | 16px | `var(--text-inverse)` | 背景: `var(--accent)` |
| 保存メッセージ | 15px | `var(--success)` | "保存しました" |
| コメント数 | 16px | `var(--text-primary)` | "(N件/新着順)" |

### 読み仮名機能

配信者がTTS読み上げ時の発音を改善するための機能。

| 項目 | 仕様 |
|-----|------|
| 入力形式 | テキスト |
| プレースホルダー | "例: やまだ たろう" |
| 保存タイミング | 保存ボタン押下時 |
| 表示位置 | メッセージの著者名の横に括弧書き |
| 空文字の扱い | Noneとして保存（読み仮名なし） |

**保存フロー:**
```
1. 読み仮名を入力
2. 保存ボタンクリック
3. ViewerCustomInfoを作成
4. DBにUpsert（存在すれば更新、なければ新規作成）
5. "保存しました"メッセージ表示（3秒後に自動消去）
```

### 過去コメント表示

当該視聴者のコメントを新着順で表示。

#### データソース

| 優先度 | ソース | 説明 |
|-------|-------|------|
| 1 | メモリバッファ | 現在表示中のメッセージ |
| 2 | アーカイブ | 表示件数制限で移動したメッセージ |
| 3 | DB | 同一配信への再接続時、前回取得したメッセージを復元 |

**対象範囲:**
- 現在のプロセスで接続した配信のコメント
- 再接続時はDBから前回セッションのコメントも復元

**表示要素:**

| 要素 | サイズ | 色 |
|------|--------|-----|
| タイムスタンプ | 13px | `#aaa` |
| コメント内容 | 15px | `#fff` |

**メッセージタイプバッジ:**

| タイプ | アイコン | 背景色 | テキスト色 |
|-------|---------|--------|----------|
| SuperChat | 💰 + 金額 | `bg-yellow-400/20` | `text-yellow-400` |
| SuperSticker | 🎨 + 金額 | `bg-purple-400/20` | `text-purple-400` |
| Membership（新規） | ⭐ 新規メンバー | `var(--success-subtle)` | `var(--success)` |
| Membership（マイルストーン） | 🎉 + Xヶ月継続 | `var(--info-subtle)` | `var(--info)` |
| MembershipGift | 🎁 + X件ギフト | `bg-pink-400/20` | `text-pink-400` |
| System | ℹ️ システム | `var(--bg-surface-3)` | `var(--text-muted)` |

**スクロール:**
- 最大高: 300px
- overflow-y: auto
- コメント間ギャップ: 8px

**コメントクリック時の挙動:**

過去コメントをクリックすると、メインチャットの該当メッセージまでスクロールしてハイライト表示する。

| 処理 | 詳細 |
|-----|------|
| 自動スクロール無効化 | クリック時に自動スクロールをOFFにする |
| スクロール実行 | `scrollIntoView({ behavior: 'smooth', block: 'center' })` |
| ハイライト表示 | 該当メッセージを選択状態にする |

**スクロール仕様:**

| 項目 | 値 |
|-----|-----|
| 特定方法 | `data-message-id` 属性でDOM要素を特定 |
| アニメーション | smooth（スムーズスクロール） |
| 配置位置 | center（画面中央） |

**ハイライト仕様:**

| 項目 | 値 |
|-----|-----|
| 枠線 | 2px solid `var(--accent)` |
| グロー | box-shadow: 0 0 8px rgba(56,189,248,0.4) |
| 持続時間 | 永続（別のメッセージをクリックするまで） |

**処理フロー:**
```
1. 過去コメントをクリック
2. 自動スクロールを無効化
3. SelectedViewerを更新（クリックしたメッセージで）
4. JavaScriptでメインチャットをスクロール
5. 該当メッセージをハイライト表示
```

**エッジケース:**

| ケース | 動作 |
|-------|------|
| メッセージがアーカイブ済み | スクロール失敗（DOM要素が存在しない） |
| メッセージがフィルターで非表示 | スクロール失敗（DOM要素が存在しない） |
| メッセージが表示範囲内 | 正常にスクロール＆ハイライト |

### データモデル

#### SelectedViewer

パネルに表示する視聴者情報。

```rust
pub struct SelectedViewer {
    pub broadcaster_channel_id: String,    // 配信者ID
    pub viewer_channel_id: String,         // 視聴者ID
    pub display_name: String,              // 表示名
    pub message: GuiChatMessage,           // クリックされたメッセージ
    pub custom_info: Option<ViewerCustomInfo>,
}
```

#### ViewerCustomInfo

視聴者のカスタム情報（配信者ごとに管理）。

```rust
pub struct ViewerCustomInfo {
    pub id: Option<i64>,
    pub broadcaster_channel_id: String,    // 配信者ごとのスコープ
    pub viewer_channel_id: String,
    pub reading: Option<String>,           // 読み仮名
    pub notes: Option<String>,             // メモ（将来用）
    pub custom_data: Option<String>,       // JSON形式拡張データ（将来用）
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
```

### 永続化

#### テーブルスキーマ

```sql
CREATE TABLE IF NOT EXISTS viewer_custom_info (
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
```

#### 配信者ごとの分離

同一視聴者でも配信者ごとに異なる読み仮名を設定可能。

| 配信者 | 視聴者 | 読み仮名 |
|-------|-------|---------|
| 配信者A | 視聴者X | "たなか" |
| 配信者B | 視聴者X | "タナカさん" |

### バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `get_viewer_custom_info` | `broadcaster_id, viewer_id` | `Option<ViewerCustomInfo>` | 視聴者カスタム情報を取得 |
| `upsert_viewer_custom_info` | `ViewerCustomInfo` | `i64` | カスタム情報を保存（Upsert） |
| `get_all_viewer_custom_info` | `broadcaster_id` | `HashMap<viewer_id, ViewerCustomInfo>` | 配信者の全視聴者情報を取得 |
| `delete_viewer_custom_info` | `broadcaster_id, viewer_id` | `bool` | カスタム情報を削除 |

### キャッシュ

配信接続時に当該配信者の全視聴者カスタム情報をプリロード。

```
配信接続
  ↓
get_all_viewer_custom_info(broadcaster_id)
  ↓
HashMap<viewer_id, ViewerCustomInfo> をメモリに保持
  ↓
コメントクリック時にキャッシュから読み込み
  ↓
保存時にキャッシュも同期更新
```

### インタラクション

#### 開く

メッセージクリック → `show_viewer_info_panel.set(Some(viewer))`

#### 閉じる

- ヘッダーの「✕」ボタンクリック
- `show_viewer_info_panel.set(None)`

※モードレス（背後のチャット表示は操作可能）

### 利用シーン

| シーン | 操作 |
|-------|------|
| 読み仮名設定 | コメントクリック → 読み仮名入力 → 保存 |
| 常連確認 | コメントクリック → 過去コメント一覧を確認 |
| 投げ銭履歴確認 | コメントクリック → SuperChatバッジで金額確認 |

## 永続化

### セッション

| テーブル | 用途 |
|---------|------|
| `sessions` | 接続セッション情報 |
| `messages` | チャットメッセージ |

### セッションデータ

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    stream_url TEXT,
    stream_title TEXT,
    broadcaster_channel_id TEXT,
    broadcaster_name TEXT,
    started_at TEXT,
    ended_at TEXT,
    total_messages INTEGER,
    total_revenue REAL
);
```

### メッセージデータ

```sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,  -- YouTube message ID（重複排除）
    session_id TEXT,
    timestamp_usec TEXT,
    author TEXT,
    author_channel_id TEXT,
    content TEXT,
    message_type TEXT,
    amount TEXT,
    is_member INTEGER,
    raw_json TEXT
);
```

## エラーハンドリング

### 接続エラー

| エラー | 発生条件 | 動作 |
|-------|---------|------|
| Invalid URL | URLからビデオIDを抽出できない | 接続失敗を返却 |
| Page Fetch Failed | YouTubeページの取得に失敗 | 接続失敗を返却 |
| API Key Not Found | HTMLからAPI Keyを抽出できない | 接続失敗を返却 |
| Continuation Not Found | Continuation Tokenを抽出できない | 接続失敗を返却 |
| Member Only | メンバー限定配信で認証なし | 接続失敗を返却 |

### ポーリングエラー

| エラー | 動作 |
|-------|------|
| API応答エラー | warnログ、次のポーリングで再試行 |
| DB保存エラー | warnログ、メッセージ処理は継続 |
| ネットワークエラー | warnログ、次のポーリングで再試行 |

### HTMLパース失敗時のデバッグ

重要な情報が抽出できなかった場合、HTMLを一時ファイルに保存する。

```
%TEMP%/liscov_debug_html_{reason}.txt
```

## 運用ケース

### アーカイブ再生

| 項目 | 動作 |
|-----|------|
| 判定 | YouTubeページのメタデータから`is_replay`を検出 |
| 表示 | UIにアーカイブ再生中であることを表示 |
| 動作 | 通常のライブと同様にメッセージを取得（再生時のチャットリプレイ） |

### 接続中の認証情報変更

| 操作 | 動作 |
|-----|------|
| 接続中にログアウト | 既存接続は旧認証情報を保持、メンバー限定チャット取得は継続可能 |
| 接続中に再ログイン | 既存接続には影響なし。次回`connect_to_stream`から新認証を使用 |
| 切断後に再接続 | 最新の認証情報を読み込んで接続 |

### メンバーシップメッセージの判定

| パターン | 判定 |
|---------|------|
| 新規メンバー | `milestone_months: None` |
| マイルストーン（n か月） | `milestone_months: Some(n)` |
| ギフト配布 | `message_type: membership_gift` + `gift_count` |
| ギフト受け取り | 新規メンバーとして処理 |

#### マイルストーン月数の抽出パターン

```
日本語: "(\d+)\s*か月", "メンバー歴\s*(\d+)"
英語: "(\d+)\s*month", "member\s+for\s+(\d+)"
```

#### ギフト数の抽出パターン

```
日本語: "(\d+)\s*人に"
英語: "[Gg]ifted\s+(\d+)", "(\d+)\s+membership"
```
