# liscov-tauri

YouTube Live Chat Monitor — Tauri + SvelteKit 版。元の [liscov](https://github.com/) (Rust + Dioxus) からの移行バージョン。

## 技術スタック

| レイヤー | 技術 |
| --- | --- |
| Backend | Tauri v2 + Rust 2024 Edition |
| Frontend | SvelteKit + Tailwind CSS + Svelte 5 Runes |
| Database | SQLite (rusqlite) |
| TTS | 棒読みちゃん / VOICEVOX |

## セットアップ

```bash
pnpm install
```

> **Note**: Git Bash環境でのlink.exe競合問題は `.cargo/config.toml` で明示的にリンカパスを指定することで解決済み。Developer PowerShellは不要。

## 開発コマンド

### Tauri (フロントエンド + バックエンド同時起動)

```bash
pnpm tauri dev    # 開発モード
pnpm tauri build  # リリースビルド
```

### フロントエンドのみ

```bash
pnpm dev          # Vite開発サーバー
pnpm build        # ビルド
pnpm check        # 型チェック (svelte-check)
```

### Rustバックエンドのみ

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

### ユニットテスト (Vitest)

```bash
pnpm test         # ウォッチモード
pnpm test:run     # 単発実行
pnpm test:coverage # カバレッジ測定
```

テストファイル: `src/**/*.{test,spec}.ts`、環境: jsdom、Tauri APIはモック済み (`src/lib/test/setup.ts`)。

### E2Eテスト (Playwright + CDP + モックサーバー)

```bash
pnpm test:e2e              # 全テスト（ビルド→実行）
pnpm test:e2e:chat         # チャット表示テスト
pnpm test:e2e:auth         # 認証フローテスト
pnpm test:e2e:ws           # WebSocketテスト

# 個別テスト
pnpm exec playwright test --config e2e/playwright.config.ts e2e/viewer-management.spec.ts
```

モックサーバー・アプリは自動起動・自動停止されます。

**ログレベル制御**: 環境変数 `E2E_LOG_LEVEL` で制御（`debug` | `info` | `warn` | `error` | `silent`）。

#### テスト分離

E2Eテストは本番データと分離された専用の名前空間を使用します。

| 環境変数 | デフォルト | テスト時 | 用途 |
| --- | --- | --- | --- |
| `LISCOV_APP_NAME` | `liscov` | `liscov-test` | 設定・DB・ログのディレクトリ名 |
| `LISCOV_KEYRING_SERVICE` | `liscov` | `liscov-test` | Windows資格情報マネージャーのサービス名 |
| `LISCOV_AUTH_URL` | YouTube URL | mock server | 認証ウィンドウのURL |
| `LISCOV_SESSION_CHECK_URL` | YouTube API | mock server | セッション検証エンドポイント |

テスト実行時の `beforeAll` フックで自動的に:
1. 既存のTauriアプリを終了
2. テストデータディレクトリ (`%APPDATA%/liscov-test`) を削除
3. テスト用認証情報を削除
4. テスト用名前空間でアプリを起動

### モックサーバー

E2Eテスト用のYouTube InnerTube APIモックサーバー（`crates/mock-server/`）。

```bash
# 基本起動
cargo run --manifest-path crates/mock-server/Cargo.toml

# NDJSONファイルからリプレイ
cargo run --manifest-path crates/mock-server/Cargo.toml -- -f replay.ndjson

# オプション
#   -p, --port <PORT>        ポート番号 (default: 3456)
#   -f, --file <FILE>        NDJSONリプレイファイル
#   -s, --speed <SPEED>      リプレイ速度 (default: 1.0)
#   -l, --loop               リプレイをループ
#   --generate <PATH>        サンプルNDJSON生成
```

認証テスト用エンドポイント:
- `POST /set_auth_state` — 認証状態を制御
- `GET /auth_status` — 現在の認証状態を取得
- `GET /status` — サーバー状態（`login_page_visits`含む）
- `POST /reset` — サーバー状態をリセット

## プロジェクト構造

```
liscov-tauri/
├── Cargo.toml                    # Cargo workspace
├── crates/
│   └── mock-server/              # E2Eテスト用モックサーバー（独立クレート）
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── commands/             # Tauri commands
│   │   ├── core/                 # コアモジュール (api/, models/, chat_runtime.rs)
│   │   ├── connection.rs         # StreamConnection定義（多接続管理）
│   │   ├── database/             # SQLiteデータベース操作
│   │   └── tts/                  # TTS (棒読みちゃん/VOICEVOX)
│   └── Cargo.toml
├── src/                          # SvelteKit Frontend
│   ├── lib/
│   │   ├── components/           # UIコンポーネント
│   │   ├── stores/               # Svelte stores (Svelte 5 Runes)
│   │   ├── tauri/                # Tauri API wrappers
│   │   └── types/                # TypeScript型定義
│   └── routes/
├── e2e/                          # E2Eテスト (Playwright + CDP)
└── docs/
    ├── specs/                    # 機能仕様書
    └── decisions/                # アーキテクチャ決定記録 (ADR)
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

- **Tauri Commands**: フロントエンドは `invoke()` でRust関数を呼び出す。コマンドは `src-tauri/src/lib.rs` の `invoke_handler!` マクロで登録。
- **Tauri Events**: バックエンドからフロントエンドへのリアルタイム通知（`chat:message`, `chat:connection`）。
- **AppState** (`state.rs`): グローバル状態管理。`Arc<RwLock<HashMap<u64, StreamConnection>>>` で複数接続を並行管理。各接続は `CancellationToken` でライフサイクルを制御。

### コマンドモジュールと仕様書の対応

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

### 元liscovとの対応

| 元liscov | liscov-tauri |
| --- | --- |
| `src/gui/components/` | `src/lib/components/` |
| `src/gui/models.rs` | `src-tauri/src/commands/chat.rs` + `src/lib/types/` |
| `src/database/` | `src-tauri/src/database/` |
| `src/api/` | `src-tauri/src/core/` |

## ドキュメント

- **機能仕様**: `docs/specs/` — 認証、チャット、WebSocket、TTS、収益分析等
- **ADR**: `docs/decisions/` — セキュアストレージ、認証インジケータ等の設計判断
- **仕様書ガイド**: `docs/SPECIFICATION_GUIDE.md`
