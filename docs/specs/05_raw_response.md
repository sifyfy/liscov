# 生レスポンス保存機能

## 概要

YouTube InnerTube APIの生レスポンスをNDJSON形式で保存する（デバッグ・分析用）。

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `raw_response_get_config` | なし | `SaveConfig` | 設定取得 |
| `raw_response_update_config` | `config: SaveConfig` | `()` | 設定更新 |
| `raw_response_resolve_path` | `file_path: String` | `String` | 相対パスを絶対パスに解決 |

## 永続化

### 設定ファイル

| ファイル | パス | 形式 |
|---------|------|------|
| config.toml | `%APPDATA%/liscov/config.toml` | TOML |

生レスポンス保存設定は `config.toml` 内に含まれる。

### 生レスポンスファイル

| ファイル | パス | 形式 |
|---------|------|------|
| raw_responses.ndjson | `%APPDATA%/liscov/raw_responses.ndjson` | NDJSON |

> **Note**: ディレクトリ名 `liscov` は環境変数 `LISCOV_APP_NAME` で変更可能（E2Eテスト用）。詳細は[認証機能仕様のE2Eテストセクション](01_auth.md#e2eテスト)を参照。

## 設定項目

```rust
pub struct SaveConfig {
    pub enabled: bool,
    pub file_path: String,
    pub max_file_size_mb: u64,
    pub enable_rotation: bool,
    pub max_backup_files: u32,
}
```

| キー | 型 | デフォルト | 説明 |
|-----|-----|----------|------|
| `enabled` | bool | `false` | 保存機能の有効/無効 |
| `file_path` | string | `"raw_responses.ndjson"` | 保存先ファイルパス |
| `max_file_size_mb` | u64 | `100` | ローテーション閾値（MB） |
| `enable_rotation` | bool | `true` | ファイルローテーション有効 |
| `max_backup_files` | u32 | `5` | 保持するバックアップ世代数 |

## NDJSON形式

### 概要

NDJSON（Newline Delimited JSON）は、1行に1つのJSONオブジェクトを記録する形式。

```
{"timestamp":1705141234,"response":{...}}
{"timestamp":1705141238,"response":{...}}
{"timestamp":1705141242,"response":{...}}
```

### 各行の構造

```json
{
  "timestamp": 1705141234,
  "response": {
    "continuationContents": {
      "liveChatContinuation": {
        "continuation": "...",
        "actions": [...],
        "continuations": [...]
      }
    }
  }
}
```

### フィールド説明

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `timestamp` | u64 | Unixタイムスタンプ（秒） |
| `response` | object | YouTube InnerTube APIの生レスポンス |

### response の内容

| フィールド | 説明 |
|-----------|------|
| `continuationContents.liveChatContinuation.continuation` | 次回リクエスト用の継続トークン |
| `continuationContents.liveChatContinuation.actions` | チャットアクション（メッセージ、削除等） |
| `continuationContents.liveChatContinuation.continuations` | その他継続データ |

## パス解決ロジック

### 解決ルール

| 入力パス | 出力パス |
|---------|---------|
| 絶対パス（例: `C:\data\responses.ndjson`） | そのまま |
| 相対パス（例: `raw_responses.ndjson`） | `%APPDATA%/liscov/raw_responses.ndjson` |

### パス検証

セキュリティのため、以下のパスは拒否される：

| 検証項目 | 例 |
|---------|-----|
| ディレクトリトラバーサル | `../`, `..\` |
| Null文字 | `\0` |
| Windows危険文字 | `< > " | ? *` |
| システムディレクトリ | `C:\Windows`, `C:\Program Files` |
| パス長超過 | 4096文字以上 |

## ファイルローテーション

### ローテーション条件

ファイルサイズが `max_file_size_mb` に達した時点で自動実行。

### ローテーションフロー

```
1. 書き込み前にファイルサイズをチェック
        ↓
2. サイズ >= max_file_size_mb
        ↓
3. 現在のファイルをリネーム
   raw_responses.ndjson → raw_responses_20250114_143025.ndjson
        ↓
4. 新しい空ファイルで書き込み継続
        ↓
5. 古いバックアップを削除（max_backup_files 超過分）
```

### バックアップ命名規則

```
{ファイル名}_{YYYYMMDD}_{HHMMSS}.{拡張子}
```

**例:**
- 元ファイル: `raw_responses.ndjson`
- ローテーション後: `raw_responses_20250114_143025.ndjson`

### バックアップ削除

- ファイル作成日時でソート（新しい順）
- `max_backup_files` を超えた古いファイルを削除
- デフォルト: 5世代保持

## 書き込み処理

### 書き込みタイミング

- YouTube APIからレスポンスを受信するたびに実行
- ポーリング間隔に依存（通常数秒ごと）

### 処理フロー

```
1. APIレスポンス受信
        ↓
2. enabled チェック
   ├─ false → スキップ
   └─ true → 続行
        ↓
3. ローテーションチェック（enable_rotation=true時）
        ↓
4. ResponseEntry作成（タイムスタンプ付与）
        ↓
5. JSONシリアライズ
        ↓
6. ファイルに追記（append mode）
        ↓
7. flush()で強制書き込み
```

### 同期性

- 実行: 非同期タスク内
- ファイルI/O: 同期書き込み
- flush(): 毎回実行（データ損失防止）

## エラーハンドリング

| エラー | 動作 |
|-------|------|
| ファイルオープン失敗 | 警告ログ、書き込みスキップ |
| JSONシリアライズ失敗 | エラーログ、書き込みスキップ |
| 書き込み失敗 | 警告ログ、次回リトライなし |
| ローテーション失敗 | エラーログ、書き込み継続 |
| パス検証失敗 | エラー返却、設定拒否 |

## フロントエンド

### RawResponseSettings.svelte

| ユーザー操作 | 期待動作 |
|-------------|---------|
| 有効トグル | `raw_response_update_config`呼び出し、保存が有効/無効になる |
| ファイルパス入力 | `raw_response_resolve_path`呼び出し、「実際の保存先」に解決されたパスを表示 |
| 「参照」ボタンクリック | ファイル保存ダイアログを開き、選択したパスをファイルパス入力に設定 |
| 最大ファイルサイズ変更 | `raw_response_update_config`呼び出し |
| ローテーション設定変更 | `raw_response_update_config`呼び出し |

### 設定UI構成

```
生レスポンス保存設定
├─ 有効/無効トグル
├─ ファイルパス
│   ├─ テキスト入力
│   ├─ 「参照」ボタン
│   └─ 解決後パス表示
├─ 最大ファイルサイズ（MB）
├─ ファイルローテーション
│   ├─ 有効/無効トグル
│   └─ 保持世代数
```

## データモデル

### SaveConfig（Rust）

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct SaveConfig {
    pub enabled: bool,
    pub file_path: String,
    pub max_file_size_mb: u64,
    pub enable_rotation: bool,
    pub max_backup_files: u32,
}

impl Default for SaveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: "raw_responses.ndjson".to_string(),
            max_file_size_mb: 100,
            enable_rotation: true,
            max_backup_files: 5,
        }
    }
}
```

### ResponseEntry（Rust）

```rust
#[derive(Serialize)]
pub struct ResponseEntry {
    pub timestamp: u64,
    pub response: GetLiveChatResponse,
}
```

### SaveConfig（TypeScript）

```typescript
interface SaveConfig {
    enabled: boolean;
    file_path: string;
    max_file_size_mb: number;
    enable_rotation: boolean;
    max_backup_files: number;
}
```

## 利用シーン

### デバッグ

- APIレスポンスの詳細調査
- パース失敗時の原因特定
- 新しいメッセージタイプの発見

### 分析

- 配信のチャット傾向分析
- メッセージ量の時系列分析
- スーパーチャットの統計

### 再生

- 保存したレスポンスからチャットを再現
- 過去配信のメッセージ復元
