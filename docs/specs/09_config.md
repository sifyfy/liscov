# 設定機能

## 目的（Why）

ユーザーのアプリケーション設定（フォントサイズ、テーマ、ストレージモード等）をTOMLファイルに永続化し、次回起動時に復元する。

## 振る舞い（What）

### 設定の読み込み

| 状況 | 結果 |
|------|------|
| config.toml が存在する | ファイルを読み込み、パース |
| config.toml が存在しない | デフォルト値を使用 |
| config.toml のパースに失敗 | warnログを出力し、デフォルト値を使用 |
| 未知のキーがファイルに含まれる | 無視（エラーにならない） |
| 既知のキーがファイルに含まれない | そのキーのみデフォルト値を使用 |

### 設定の保存

| 状況 | 結果 |
|------|------|
| `config_set_value` 呼び出し | メモリ上のConfigを更新し、config.tomlに書き込み |
| `config_save` 呼び出し | 受け取った Config でメモリ上の Config を置き換えて書き込む。ただし `raw_response` セクションは受け取った値を使わず、現在値を保持する |
| 書き込み失敗 | エラーログを出力。メモリ上の変更は維持（次回の書き込みで反映される可能性あり）。ファイルは前の内容のまま |
| 書き込みの途中でアプリや PC が止まった | 前の内容か新しい内容のどちらかが残る（一時ファイルに書いてから置き換える。FEATURE_SPECIFICATION.md「永続化ファイル一覧」） |
| 更新（`config_save`・`config_set_value`・`raw_response_update_config`・`auth_use_fallback_storage`）が同時に届いた | 1 つずつ順に「今の設定を読む → 変える → 置き換える → 書き込む」を行う。互いの変更を消さず、ファイルには最後に適用した内容が残る |
| ディレクトリが存在しない | 自動作成を試行。失敗時はエラーログ、保存スキップ |

## 制約・不変条件（Boundaries）

| 制約 | 理由 |
|------|------|
| 未知のキーはエラーにせず無視する | 将来のバージョンダウン時に設定ファイルが壊れないようにする |
| 存在しないキーはデフォルト値で補完する | 将来のバージョンアップ時にキーが追加されても既存設定が動作する |
| 設定ファイルパスは環境変数 `LISCOV_APP_NAME` で分離可能 | E2Eテストが本番設定を破壊することを防ぐ |
| `raw_response` セクションを変更するのは `raw_response_update_config` だけ | フロントエンドの `Config` 型はこのセクションを持たない。`config_save` で上書きを許すと、送られてこなかったセクションがデフォルト値に戻り、保存設定が黙って無効になる |

## 設定ファイル

### 保存先

| OS | パス |
|----|------|
| Windows | `%APPDATA%/liscov-tauri/config.toml` |
| macOS | `~/Library/Application Support/liscov-tauri/config.toml` |
| Linux | `~/.config/liscov-tauri/config.toml` |

> **Note**: ディレクトリ名 `liscov-tauri` は環境変数 `LISCOV_APP_NAME` で変更可能（E2Eテスト用）。詳細は[認証機能仕様のE2Eテストセクション](01_auth.md#e2eテスト)を参照。

### ファイル形式

TOML形式で保存。

```toml
[storage]
mode = "secure"  # "secure" or "fallback"

[chat_display]
message_font_size = 13
show_timestamps = true
hide_panel_on_connect = true
chat_mode = "top"  # "top" or "all"
flowed_newcomers_height_ratio = 0.125
flowed_supporters_height_ratio = 0.125

[chat_fetch]
push_notification = true

[ui]
theme = "dark"  # "dark" or "light"

[raw_response]
enabled = false
file_path = "raw_responses.ndjson"
max_file_size_mb = 100
enable_rotation = true
max_backup_files = 5
```

## 設定項目

### storage セクション

認証情報の保存先に関する設定。詳細は[認証機能仕様](01_auth.md)を参照。

| キー | 型 | デフォルト | 説明 |
|-----|-----|----------|------|
| `mode` | string | `"secure"` | ストレージモード（`secure` / `fallback`） |

### chat_display セクション

チャット表示に関する設定。詳細は[チャット機能仕様](02_chat.md)を参照。

| キー | 型 | デフォルト | 範囲 | 説明 |
|-----|-----|----------|------|------|
| `message_font_size` | integer | `13` | 10〜24 | メッセージフォントサイズ（px） |
| `show_timestamps` | boolean | `true` | - | タイムスタンプ表示 |
| `hide_panel_on_connect` | boolean | `true` | - | 配信に接続したら操作パネルを隠す (02_chat.md「上の行と操作パネル」) |
| `chat_mode` | string | `"top"` | `top` / `all` | チャットモード（トップ / 全て）。最後に選んだモードを覚えておき、次の起動でもそのモードで接続する |
| `flowed_newcomers_height_ratio` | float | `0.125` | 0.05〜1/3 | 「流れた新規さん」の欄の高さの上限 (コメントの流れの部分の高さに対する割合)。欄の境目のドラッグで変わる (02_chat.md「流れたもの」) |
| `flowed_supporters_height_ratio` | float | `0.125` | 0.05〜1/2 | 「この配信のスパチャ等の一覧」の欄の高さの上限 (概要と詳細で共通)。同上。キーの名前は統合前の「流れたスパチャ等」の欄のときのまま |

`flowed_*_height_ratio` を `config_set_value` で範囲外 (または数でない値) にしようとするとエラーにする。ファイルに範囲外の値が書かれていても読み込みは失敗させず、画面が表示のときに範囲に収める。

`auto_scroll_enabled` は 2026-10 に外した (自動スクロールは止めたままにできなくなった。02_chat.md「自動スクロール」)。古い config.toml に残っていても、未知のキーとして無視する。

### chat_fetch セクション

チャットの取り方に関する設定。詳細は[チャット機能仕様](02_chat.md)を参照。

| キー | 型 | デフォルト | 説明 |
|-----|-----|----------|------|
| `push_notification` | boolean | `true` | YouTube のプッシュ通知を受けて取得を早める (02_chat.md「プッシュ通知」)。実験的な機能で、問題が起きたときに止めるためのもの。設定の画面の「チャット取得」のタブで切り替える。変更はすぐ効く (接続中の接続にも) |

### ui セクション

UIの表示に関する設定。

| キー | 型 | デフォルト | 説明 |
|-----|-----|----------|------|
| `theme` | string | `"dark"` | テーマ（`dark` / `light`） |

### raw_response セクション

生レスポンス保存の設定。キー・デフォルト値・振る舞いは[生レスポンス保存機能仕様](05_raw_response.md)を参照。変更は `raw_response_get_config` / `raw_response_update_config` で行い、`config_get_value` / `config_set_value` の対象外。

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `config_load` | なし | `Config` | 設定を読み込み |
| `config_save` | `Config` | `()` | 設定を保存 |
| `config_get_value` | `section: String, key: String` | `Option<Value>` | 個別値を取得 |
| `config_set_value` | `section: String, key: String, value: Value` | `()` | 個別値を設定・保存 |

## データモデル

```rust
pub struct Config {
    pub storage: StorageConfig,
    pub chat_display: ChatDisplayConfig,
    pub chat_fetch: ChatFetchConfig,
    pub ui: UiConfig,
    pub raw_response: SaveConfig,  // 05_raw_response.md
}

pub struct StorageConfig {
    pub mode: StorageMode,  // Secure or Fallback
}

pub struct ChatDisplayConfig {
    pub message_font_size: u32,
    pub show_timestamps: bool,
    pub hide_panel_on_connect: bool,
    pub chat_mode: ChatModeSetting,
    pub flowed_newcomers_height_ratio: f64,
    pub flowed_supporters_height_ratio: f64,
}

pub struct ChatFetchConfig {
    pub push_notification: bool,
}

pub enum ChatModeSetting {
    Top,  // "top"
    All,  // "all"
}

pub enum Theme {
    Dark,
    Light,
}

pub struct UiConfig {
    pub theme: Theme,
}
```

## 読み込み・保存フロー

### アプリ起動時

バックエンドは起動時に config.toml を読み込んでメモリ上の Config を初期化する（フロントエンドの `config_load` を待たない）。チャット監視など、フロントエンドを経由しない処理も起動直後から保存済みの設定で動くようにするため。

```
1. config.tomlの存在確認
   ├─ 存在する → ファイルを読み込み、パース
   └─ 存在しない → デフォルト値を使用
        ↓
2. パース成功 → Config構造体を返却
   パース失敗 → warnログ、デフォルト値を使用
```

### 設定変更時

```
1. config_set_value呼び出し
        ↓
2. メモリ上のConfigを更新
        ↓
3. config.tomlに書き込み
        ↓
4. 書き込み成功 → 完了
   書き込み失敗 → エラーログ、メモリ上の変更は維持
```

## エラーハンドリング

| エラー | 動作 |
|-------|------|
| ファイル読み込み失敗 | デフォルト値を使用、warnログ |
| パース失敗 | デフォルト値を使用、warnログ |
| 書き込み失敗 | エラーログ、処理継続 |
| ディレクトリ作成失敗 | エラーログ、保存スキップ |

## マイグレーション

### 新規キー追加時

存在しないキーはデフォルト値を使用。既存の設定は保持される。

### キー削除時

未知のキーは無視される（エラーにならない）。

## フロントエンド連携

### 設定の読み込み

```typescript
// アプリ起動時
const config = await invoke<Config>('config_load');
chatStore.setFontSize(config.chat_display.message_font_size);
chatStore.setShowTimestamps(config.chat_display.show_timestamps);
```

### 設定の保存

```typescript
// フォントサイズ変更時
async function setFontSize(size: number) {
    messageFontSize = size;
    await invoke('config_set_value', {
        section: 'chat_display',
        key: 'message_font_size',
        value: size
    });
}
```
