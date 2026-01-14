# 設定機能

## 概要

アプリケーション設定の永続化と管理を行う。

## 設定ファイル

### 保存先

| OS | パス |
|----|------|
| Windows | `%APPDATA%/liscov/config.toml` |
| macOS | `~/Library/Application Support/liscov/config.toml` |
| Linux | `~/.config/liscov/config.toml` |

### ファイル形式

TOML形式で保存。

```toml
[storage]
mode = "secure"  # "secure" or "fallback"

[chat_display]
message_font_size = 13
show_timestamps = true
auto_scroll_enabled = true
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
| `auto_scroll_enabled` | boolean | `true` | - | 自動スクロール有効 |

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
}

pub struct StorageConfig {
    pub mode: StorageMode,  // Secure or Fallback
}

pub struct ChatDisplayConfig {
    pub message_font_size: u32,
    pub show_timestamps: bool,
    pub auto_scroll_enabled: bool,
}
```

## 読み込み・保存フロー

### アプリ起動時

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
