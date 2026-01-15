# TTS（読み上げ）機能

## 概要

チャットメッセージを音声で読み上げる。棒読みちゃん / VOICEVOX対応。

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `tts_get_config` | なし | `TtsConfigDto` | 設定取得 |
| `tts_update_config` | `config: TtsConfigDto` | `()` | 設定更新 |
| `tts_speak_direct` | `text: String` | `()` | 直接読み上げ（テスト用） |
| `tts_test_connection` | `backend: Option<String>` | `bool` | 接続テスト |
| `tts_start` | なし | `()` | キュー処理開始 |
| `tts_stop` | なし | `()` | キュー処理停止 |
| `tts_clear_queue` | なし | `()` | キュークリア |
| `tts_get_status` | なし | `TtsStatus` | 状態取得 |

### TtsStatus

```rust
pub struct TtsStatus {
    pub is_processing: bool,
    pub queue_size: u32,
}
```

## 永続化

| ファイル | パス | 形式 |
|---------|------|------|
| tts_config.toml | `%APPDATA%/liscov/tts_config.toml` | TOML |

> **Note**: ディレクトリ名 `liscov` は環境変数 `LISCOV_APP_NAME` で変更可能（E2Eテスト用）。詳細は[認証機能仕様のE2Eテストセクション](01_auth.md#e2eテスト)を参照。

## 設定ファイル形式

```toml
enabled = false
backend = "none"  # "none" | "bouyomichan" | "voicevox"
read_author_name = true
add_honorific = true
strip_at_prefix = true
strip_handle_suffix = true
read_superchat_amount = true
max_text_length = 200
queue_size_limit = 50

[bouyomichan]
host = "localhost"
port = 50080
voice = 0
volume = -1
speed = -1
tone = -1

[voicevox]
host = "localhost"
port = 50021
speaker_id = 1
volume_scale = 1.0
speed_scale = 1.0
pitch_scale = 0.0
intonation_scale = 1.0
```

## 設定項目詳細

### 基本設定

| キー | 型 | デフォルト | 説明 |
|-----|-----|----------|------|
| `enabled` | bool | `false` | TTS有効/無効 |
| `backend` | string | `"none"` | 使用バックエンド |
| `read_author_name` | bool | `true` | 投稿者名を読み上げる |
| `add_honorific` | bool | `true` | 投稿者名に「さん」を付ける |
| `strip_at_prefix` | bool | `true` | 先頭の`@`を除去 |
| `strip_handle_suffix` | bool | `true` | 末尾の`-xxx`サフィックスを除去 |
| `read_superchat_amount` | bool | `true` | スーパーチャット金額を読み上げる |
| `max_text_length` | u32 | `200` | 最大読み上げ文字数 |
| `queue_size_limit` | u32 | `50` | キューサイズ上限 |

### 棒読みちゃん設定

| キー | 型 | デフォルト | 範囲 | 説明 |
|-----|-----|----------|------|------|
| `host` | string | `"localhost"` | - | ホスト名 |
| `port` | u16 | `50080` | - | ポート番号 |
| `voice` | i32 | `0` | 0〜 | 声質ID（0=デフォルト） |
| `volume` | i32 | `-1` | -1〜 | 音量（-1=デフォルト） |
| `speed` | i32 | `-1` | -1〜 | 話速（-1=デフォルト） |
| `tone` | i32 | `-1` | -1〜 | 音高（-1=デフォルト） |

### VOICEVOX設定

| キー | 型 | デフォルト | 範囲 | 説明 |
|-----|-----|----------|------|------|
| `host` | string | `"localhost"` | - | ホスト名 |
| `port` | u16 | `50021` | - | ポート番号 |
| `speaker_id` | i32 | `1` | 0〜 | 話者ID（1=四国めたん） |
| `volume_scale` | f32 | `1.0` | 0.0〜2.0 | 音量倍率 |
| `speed_scale` | f32 | `1.0` | 0.5〜2.0 | 話速倍率 |
| `pitch_scale` | f32 | `0.0` | -0.15〜0.15 | 音高倍率 |
| `intonation_scale` | f32 | `1.0` | 0.0〜2.0 | 抑揚倍率 |

## 読み上げテキスト生成

### フォーマット

```
[投稿者名]、[スーパーチャット情報]、[メッセージ本文]
```

各要素は「、」で結合される。

### 投稿者名の処理フロー

```
1. 視聴者カスタム読み仮名をチェック
   ├─ あり → カスタム読み仮名を使用
   └─ なし → 投稿者名を処理
        ↓
2. strip_at_prefix=true → 先頭の @ を除去
        ↓
3. strip_handle_suffix=true → 末尾の -xxx を除去
        ↓
4. add_honorific=true → 「さん」を付与
```

### 投稿者名の例

| 元の名前 | strip_at_prefix | strip_handle_suffix | add_honorific | 結果 |
|---------|-----------------|---------------------|---------------|------|
| `@田中-abc` | true | true | true | `田中さん` |
| `@田中-abc` | false | true | true | `@田中さん` |
| `田中みな子` | - | - | true | `田中みな子さん` |
| `UCxxx（読み仮名:たなか）` | - | - | true | `たなかさん` |

### スーパーチャット/メンバーシップの読み上げ

| メッセージタイプ | 読み上げ形式 |
|----------------|-------------|
| SuperChat | `{amount}のスーパーチャット` |
| SuperSticker | `{amount}のスーパーステッカー` |
| Membership（新規） | `メンバー加入` |
| Membership（マイルストーン） | `{months}ヶ月のメンバーシップ` |
| MembershipGift | `{gift_count}人へのメンバーシップギフト` |

### テキストサニタイズ

1. URLを除去（`https?://\S+`）
2. 連続空白を1つに圧縮
3. `max_text_length`で切り詰め

### 読み上げ例

**入力:**
- 投稿者: `@山田太郎-xyz`
- メッセージタイプ: SuperChat
- 金額: `¥500`
- 本文: `こんにちは！`

**出力:**
```
山田太郎さん、500円のスーパーチャット、こんにちは
```

## キュー処理

### キュー構造

```rust
pub struct TtsMessage {
    pub text: String,
    pub priority: TtsPriority,
}

pub enum TtsPriority {
    Normal = 0,        // 通常メッセージ
    Membership = 1,    // メンバーシップ関連
    SuperChat = 2,     // スーパーチャット（最高優先度）
}
```

### 処理フロー

```
1. チャットメッセージ受信
        ↓
2. 読み上げテキスト生成
        ↓
3. キューに追加
   ├─ キュー空き → 追加成功
   └─ キュー満杯 → 破棄（ログ出力）
        ↓
4. バックグラウンドタスクが順次処理
        ↓
5. バックエンドに送信
        ↓
6. 読み上げ完了待機
        ↓
7. 次のメッセージを処理
```

### キューサイズ制限

- デフォルト: 50メッセージ
- 満杯時: 新規メッセージは破棄
- 処理順: FIFO（先入れ先出し）

## 棒読みちゃん連携

### 通信プロトコル

- **プロトコル**: HTTP GET
- **タイムアウト**: 5秒

### エンドポイント

#### 読み上げ

```
GET http://{host}:{port}/Talk?text={text}&voice={voice}&volume={volume}&speed={speed}&tone={tone}
```

| パラメータ | 説明 |
|-----------|------|
| `text` | 読み上げテキスト（URLエンコード） |
| `voice` | 声質ID |
| `volume` | 音量 |
| `speed` | 話速 |
| `tone` | 音高 |

#### 接続テスト

```
GET http://{host}:{port}/Talk?text=
```

空テキストで接続確認（読み上げは発生しない）。

### 声質ID一覧（参考）

| ID | 声質 |
|----|-----|
| 0 | デフォルト |
| 1 | 女性1 |
| 2 | 女性2 |
| 3 | 男性1 |
| 4 | 男性2 |
| 5 | 中性 |
| 6 | ロボット |
| 7 | 機械1 |
| 8 | 機械2 |

## VOICEVOX連携

### 通信プロトコル

- **プロトコル**: HTTP POST
- **タイムアウト**: 30秒

### 2段階音声合成

#### Step 1: audio_query

```
POST http://{host}:{port}/audio_query?speaker={speaker_id}&text={text}
```

テキストから音声クエリを生成。

#### Step 2: synthesis

```
POST http://{host}:{port}/synthesis?speaker={speaker_id}
Content-Type: application/json

{
  "accent_phrases": [...],
  "volumeScale": 1.0,
  "speedScale": 1.0,
  "pitchScale": 0.0,
  "intonationScale": 1.0,
  ...
}
```

音声クエリからWAVデータを生成。

### 音声再生

生成されたWAVデータはアプリ内で再生（rodioライブラリ使用）。

#### 接続テスト

```
GET http://{host}:{port}/version
```

バージョン情報取得で接続確認。

### 話者ID一覧（参考）

| ID | 話者 |
|----|-----|
| 0 | 四国めたん（あまあま） |
| 1 | 四国めたん（ノーマル） |
| 2 | 四国めたん（セクシー） |
| 3 | ずんだもん（ノーマル） |
| ... | ... |

## 視聴者カスタム読み仮名

### 概要

視聴者ごとにカスタム読み仮名を設定可能。設定されている場合、投稿者名の代わりにカスタム読み仮名を使用。

### データモデル

詳細は[視聴者管理機能](06_viewer.md)を参照。

```rust
pub struct ViewerCustomInfo {
    pub broadcaster_channel_id: String,
    pub viewer_channel_id: String,
    pub reading: Option<String>,  // カスタム読み仮名
    // ...
}
```

### 適用フロー

```
1. メッセージ受信
        ↓
2. viewer_channel_idでViewerCustomInfoを検索
   ├─ reading あり → カスタム読み仮名を使用
   └─ reading なし → 投稿者名を処理
```

### キャッシング

- 配信者ごとにViewerCustomInfoをメモリにキャッシュ
- 起動時にDBから全件ロード
- UI編集時にリアルタイム同期

## エラーハンドリング

### エラー種別

| エラー | 動作 |
|-------|------|
| 接続失敗 | エラーログ出力、メッセージ破棄 |
| タイムアウト | エラーログ出力、メッセージ破棄 |
| キュー満杯 | 警告ログ出力、メッセージ破棄 |
| 音声合成失敗（VOICEVOX） | エラーログ出力、メッセージ破棄 |
| 音声再生失敗（VOICEVOX） | エラーログ出力、次のメッセージへ |

### エラー時の継続性

- 1メッセージの失敗は他のメッセージに影響しない
- キュー処理は継続される

## フロントエンド

### TtsSettings.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| TTS有効トグル | `tts_update_config`呼び出し、設定が即座に保存される |
| バックエンド変更 | 300msデバウンス後に`tts_update_config`呼び出し |
| 「接続テスト」クリック | `tts_test_connection`呼び出し、結果表示 |
| テスト文入力 + 「読み上げ」クリック | `tts_speak_direct`呼び出し、ボタンにスピナー表示、読み上げ実行 |
| 設定変更（ホスト、ポート等） | 300msデバウンス後に自動保存（保存ボタンなし） |

### 設定UI構成

```
TTS設定
├─ 有効/無効トグル
├─ バックエンド選択（なし / 棒読みちゃん / VOICEVOX）
├─ 共通設定
│   ├─ 投稿者名を読む
│   ├─ 「さん」を付ける
│   ├─ @を除去
│   ├─ ハンドルサフィックスを除去
│   ├─ スーパーチャット金額を読む
│   ├─ 最大文字数
│   └─ キューサイズ上限
├─ バックエンド固有設定
│   ├─ [棒読みちゃん] ホスト、ポート、声質、音量、速度、音高
│   └─ [VOICEVOX] ホスト、ポート、話者、音量、速度、音高、抑揚
├─ 接続テストボタン
└─ テスト読み上げ（テキスト入力 + 読み上げボタン）
```

## データモデル

### TtsConfig（Rust）

```rust
pub struct TtsConfig {
    pub enabled: bool,
    pub backend: TtsBackend,
    pub read_author_name: bool,
    pub add_honorific: bool,
    pub strip_at_prefix: bool,
    pub strip_handle_suffix: bool,
    pub read_superchat_amount: bool,
    pub max_text_length: u32,
    pub queue_size_limit: u32,
    pub bouyomichan: BouyomichanConfig,
    pub voicevox: VoicevoxConfig,
}

pub enum TtsBackend {
    None,
    Bouyomichan,
    Voicevox,
}

pub struct BouyomichanConfig {
    pub host: String,
    pub port: u16,
    pub voice: i32,
    pub volume: i32,
    pub speed: i32,
    pub tone: i32,
}

pub struct VoicevoxConfig {
    pub host: String,
    pub port: u16,
    pub speaker_id: i32,
    pub volume_scale: f32,
    pub speed_scale: f32,
    pub pitch_scale: f32,
    pub intonation_scale: f32,
}
```

### TtsConfigDto（TypeScript）

```typescript
interface TtsConfigDto {
    enabled: boolean;
    backend: 'none' | 'bouyomichan' | 'voicevox';
    read_author_name: boolean;
    add_honorific: boolean;
    strip_at_prefix: boolean;
    strip_handle_suffix: boolean;
    read_superchat_amount: boolean;
    max_text_length: number;
    queue_size_limit: number;
    bouyomichan: BouyomichanConfig;
    voicevox: VoicevoxConfig;
}
```
