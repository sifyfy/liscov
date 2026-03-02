# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

liscov-tauri プロジェクトの開発ガイド

## プロジェクト概要

**liscov-tauri** は YouTube Live Chat Monitor の Tauri + SvelteKit 版。元の liscov (Rust + Dioxus) から移行したバージョン。

### 技術スタック
- **Backend**: Tauri v2 + Rust 2024 Edition
- **Frontend**: SvelteKit + Tailwind CSS + Svelte 5 Runes
- **Database**: SQLite (rusqlite)
- **TTS**: 棒読みちゃん / VOICEVOX 対応

## 開発コマンド

### フロントエンド

```bash
pnpm dev          # 開発サーバー
pnpm build        # ビルド
pnpm check        # 型チェック (svelte-check)
```

### ユニットテスト (Vitest)

```bash
pnpm test         # ウォッチモード
pnpm test:run     # 単発実行
pnpm test:coverage # カバレッジ測定
```

テストファイル: `src/**/*.{test,spec}.ts`、環境: jsdom、Tauri APIはモック済み (`src/lib/test/setup.ts`)。

### Rust (バックエンド)

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

> **Note**: Git Bash環境でのlink.exe競合問題は `.cargo/config.toml` で明示的にリンカパスを指定することで解決済み。Developer PowerShellは不要。

### Tauri

```bash
pnpm tauri dev    # 開発モード（フロントエンド + バックエンド同時起動）
pnpm tauri build  # リリースビルド
```

### E2Eテスト

E2Eテストは `e2e-tauri/` ディレクトリにあります（Playwright + CDP + モックサーバー）。

```bash
# 全E2Eテストを実行（モックサーバー・アプリは自動起動・終了）
pnpm test:e2e

# 特定のテストファイルを実行
pnpm test:e2e:chat   # チャット表示
pnpm test:e2e:auth   # 認証フロー
pnpm test:e2e:ws     # WebSocket

# 個別テスト（ファイル指定）
pnpm exec playwright test --config e2e-tauri/playwright.config.ts e2e-tauri/viewer-management.spec.ts
```

> **Note**: モックサーバーはテスト実行時に自動的に起動・停止されます。手動起動は不要です。

**ログレベル制御**: 環境変数 `E2E_LOG_LEVEL` で制御（`debug`|`info`|`warn`|`error`|`silent`）。

**テスト分離**: E2Eテストは本番データと分離された専用の名前空間を使用します。

| 環境変数                   | デフォルト  | テスト時      | 用途                                    |
| -------------------------- | ----------- | ------------- | --------------------------------------- |
| `LISCOV_APP_NAME`          | `liscov`    | `liscov-test` | 設定・DB・ログのディレクトリ名          |
| `LISCOV_KEYRING_SERVICE`   | `liscov`    | `liscov-test` | Windows資格情報マネージャーのサービス名 |
| `LISCOV_AUTH_URL`          | YouTube URL | mock server   | 認証ウィンドウのURL                     |
| `LISCOV_SESSION_CHECK_URL` | YouTube API | mock server   | セッション検証エンドポイント            |

テスト実行時、`beforeAll`フックで以下が自動実行されます：
1. 既存のTauriアプリを終了
2. テストデータディレクトリ (`%APPDATA%/liscov-test`) を削除
3. テスト用認証情報を削除
4. テスト用名前空間でアプリを起動

### モックサーバー

E2Eテスト用のYouTube InnerTube APIモックサーバー。

```bash
# 基本起動
cargo run --manifest-path src-tauri/Cargo.toml --bin mock_server

# NDJSONファイルからリプレイ
cargo run --manifest-path src-tauri/Cargo.toml --bin mock_server -- -f replay.ndjson

# オプション
#   -p, --port <PORT>        ポート番号 (default: 3456)
#   -f, --file <FILE>        NDJSONリプレイファイル
#   -s, --speed <SPEED>      リプレイ速度 (default: 1.0)
#   -l, --loop               リプレイをループ
#   --generate <PATH>        サンプルNDJSON生成
```

**認証テスト用エンドポイント**:
- `POST /set_auth_state` - 認証状態を制御
- `GET /auth_status` - 現在の認証状態を取得
- `GET /status` - サーバー状態（`login_page_visits`含む）
- `POST /reset` - サーバー状態をリセット

## プロジェクト構造

```
liscov-tauri/
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── bin/                  # スタンドアロンバイナリ
│   │   │   └── mock_server.rs    # E2Eテスト用モックサーバー
│   │   ├── commands/             # Tauri commands
│   │   ├── core/                 # コアモジュール (api/, models/)
│   │   ├── database/             # SQLiteデータベース操作
│   │   └── tts/                  # TTS (棒読みちゃん/VOICEVOX)
│   └── Cargo.toml
├── src/                          # SvelteKit Frontend
│   ├── lib/
│   │   ├── components/           # UIコンポーネント
│   │   ├── stores/               # Svelte stores
│   │   ├── tauri/                # Tauri API wrappers
│   │   └── types/                # TypeScript型定義
│   └── routes/
├── e2e-tauri/                    # Tauri E2Eテスト (Playwright + CDP)
│   ├── auth-flow.spec.ts         # 認証フローテスト
│   └── playwright.config.ts      # Playwright設定
├── docs/                         # ドキュメント
│   ├── specs/                    # 機能仕様書
│   └── decisions/                # アーキテクチャ決定記録 (ADR)
└── package.json
```

## アーキテクチャ

### Frontend-Backend 連携パターン

```
Frontend (Svelte)           Backend (Rust/Tauri)
─────────────────           ────────────────────
src/lib/tauri/*.ts    ──→   src-tauri/src/commands/*.rs
    ↓ invoke()                     ↓
src/lib/stores/*.svelte.ts   AppState (state.rs)
    ↓                              ↓
src/lib/components/         core/api/ (InnerTubeClient, WebSocket)
```

**Tauri Commands**: フロントエンドは `invoke()` でRust関数を呼び出す。コマンドは `src-tauri/src/lib.rs` の `invoke_handler!` マクロで登録。

| コマンドモジュール | 仕様書 |
| --- | --- |
| `commands/auth.rs`, `auth_window.rs` | `docs/specs/01_auth.md` |
| `commands/chat.rs` | `docs/specs/02_chat.md` |
| `commands/websocket.rs` | `docs/specs/03_websocket.md` |
| `commands/tts.rs` | `docs/specs/04_tts.md` |
| `commands/raw_response.rs` | `docs/specs/05_raw_response.md` |
| `commands/viewer.rs` | `docs/specs/06_viewer.md` |
| `commands/analytics.rs` | `docs/specs/07_revenue.md` |
| `commands/database.rs` | `docs/specs/08_database.md` |
| `commands/config.rs` | `docs/specs/09_config.md` |

**Tauri Events**: バックエンドからフロントエンドへのリアルタイム通知。
- `chat:message` - 新規チャットメッセージ
- `chat:connection` - 接続状態の変更

**AppState** (`state.rs`): シングルトン状態管理。`Arc<RwLock<T>>` で並行アクセス。

### 新機能追加の流れ

1. `docs/specs/` に仕様書を作成
2. `src-tauri/src/commands/` にTauriコマンドを実装
3. `src-tauri/src/lib.rs` にコマンドを登録
4. `src/lib/tauri/` にTypeScriptラッパーを作成
5. `src/lib/stores/` にストアを作成 (Svelte 5 runes)
6. `src/lib/components/` にUIコンポーネントを作成
7. `e2e-tauri/` にE2Eテストを追加

## 開発ガイドライン

### 基本ルール（重要）
- 新規ファイル作成より既存ファイル編集を優先
- 公開機能に対するテストコード原則作成
- 機能変更時のドキュメント更新必須
- クリーンコード維持

## ドキュメント

- **機能仕様**: `docs/specs/` - 認証、チャット、WebSocket、TTS、収益分析等
- **ADR**: `docs/decisions/` - セキュアストレージ、認証インジケータ等の設計判断

## 元liscovとの対応

| 元liscov              | liscov-tauri                                        |
| --------------------- | --------------------------------------------------- |
| `src/gui/components/` | `src/lib/components/`                               |
| `src/gui/models.rs`   | `src-tauri/src/commands/chat.rs` + `src/lib/types/` |
| `src/database/`       | `src-tauri/src/database/`                           |
| `src/api/`            | `src-tauri/src/core/`                               |

## 重要な注意事項

1. **Svelte 5 Runes**: `$state`, `$derived`, `$effect` を使用
2. **CSS変数テーマ**: `app.css` でダーク/ライトテーマをCSS変数で定義（`--bg-base`, `--bg-surface-1`〜`3`, `--accent`, `--text-primary` 等）。`data-theme` 属性で切替。
3. **Tauri Events**: フロントエンドへのリアルタイム通知に使用
