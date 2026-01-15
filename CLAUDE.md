# CLAUDE.md

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
pnpm check        # 型チェック
pnpm test:e2e     # E2Eテスト
```

### Rust (バックエンド)

```bash
# 通常のcargoコマンドで実行可能
cargo check --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

> **Note**: Git Bash環境でのlink.exe競合問題は `.cargo/config.toml` で明示的にリンカパスを指定することで解決済み。Developer PowerShellは不要。

### Tauri

```bash
pnpm tauri dev    # 開発モード（フロントエンド + バックエンド同時起動）
pnpm tauri build  # リリースビルド
```

### E2Eテスト

認証機能のE2Eテストは `e2e-tauri/` ディレクトリにあります。

```bash
# 1. モックサーバーを起動（別ターミナル）
cargo run --manifest-path src-tauri/Cargo.toml --bin mock_server

# 2. E2Eテストを実行（アプリは自動起動・終了）
pnpm exec playwright test --config e2e-tauri/playwright.config.ts
```

**テスト分離**: E2Eテストは本番データと分離された専用の名前空間を使用します。

| 環境変数 | デフォルト | テスト時 | 用途 |
|---------|-----------|---------|------|
| `LISCOV_APP_NAME` | `liscov` | `liscov-test` | 設定・DB・ログのディレクトリ名 |
| `LISCOV_KEYRING_SERVICE` | `liscov` | `liscov-test` | Windows資格情報マネージャーのサービス名 |
| `LISCOV_AUTH_URL` | YouTube URL | mock server | 認証ウィンドウのURL |
| `LISCOV_SESSION_CHECK_URL` | YouTube API | mock server | セッション検証エンドポイント |

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
│   │   ├── core/                 # コアモジュール・モデル
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

## ドキュメント

- **機能仕様**: `docs/specs/` - 認証、チャット、WebSocket、TTS、収益分析等
- **ADR**: `docs/decisions/` - セキュアストレージ、認証インジケータ等の設計判断

## 元liscovとの対応

| 元liscov | liscov-tauri |
|----------|--------------|
| `src/gui/components/` | `src/lib/components/` |
| `src/gui/models.rs` | `src-tauri/src/commands/chat.rs` + `src/lib/types/` |
| `src/database/` | `src-tauri/src/database/` |
| `src/api/` | `src-tauri/src/core/` |

## 重要な注意事項

1. **Svelte 5 Runes**: `$state`, `$derived`, `$effect` を使用
2. **CSS変数**: `app.css` でカラーテーマを定義（`--primary-start`, `--bg-main` 等）
3. **Tauri Events**: フロントエンドへのリアルタイム通知に使用
