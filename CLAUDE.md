# liscov-tauri

YouTube Live配信のチャットをリアルタイムでモニタリング・管理するTauriデスクトップアプリケーション。

## 技術スタック

- バックエンド: Rust + Tauri v2
- フロントエンド: Svelte 5 (Runes) + TypeScript + Vite
- データベース: SQLite (rusqlite)
- テスト: Vitest (フロントエンド) / cargo test (Rust) / Playwright (E2E)

## コマンドリファレンス

```bash
# 開発
pnpm tauri dev                              # フロントエンド + バックエンド同時起動
pnpm dev                                    # フロントエンドのみ

# テスト
pnpm test:run                               # フロントエンドユニットテスト (Vitest)
cargo test --manifest-path src-tauri/Cargo.toml  # Rustユニットテスト
pnpm test:e2e                               # E2E全テスト（ビルド→実行、自動起動）
pnpm exec playwright test --config e2e/playwright.config.ts e2e/<file>.spec.ts  # E2E個別

# チェック
pnpm check                                  # svelte-check
cargo check --manifest-path src-tauri/Cargo.toml
```

## ディレクトリ構成

- `src/` — フロントエンド（Svelte 5）
  - `src/lib/components/` — UIコンポーネント
  - `src/lib/stores/` — ストア (Svelte 5 Runes)
  - `src/lib/tauri/` — Tauriコマンドラッパー
- `src-tauri/` — バックエンド（Rust + Tauri）
  - `src-tauri/src/commands/` — Tauriコマンド
  - `src-tauri/src/state.rs` — AppState定義
- `crates/mock-server/` — E2Eテスト用モックサーバー
- `e2e/` — E2Eテスト（Playwright）
- `docs/` — ドキュメント
  - `docs/specs/` — 機能仕様書
  - `docs/decisions/` — ADR

## 必読ドキュメント

- [constitution.md](./constitution.md) — 開発ルール・規約。**必ず従うこと。**
- [docs/SPECIFICATION_GUIDE.md](./docs/SPECIFICATION_GUIDE.md) — 仕様書・ドキュメント運用ルール。
- [docs/specs/](./docs/specs/) — 機能仕様書。**実装前に該当する仕様を確認すること。**

### コマンド → 仕様書マッピング

| コマンド | 仕様書 |
| --- | --- |
| `commands/auth.rs`, `auth_window.rs` | `docs/specs/01_auth.md` |
| `commands/chat.rs` | `docs/specs/02_chat.md` |
| `commands/websocket.rs` | `docs/specs/03_websocket.md` |
| `commands/tts.rs` | `docs/specs/04_tts.md` |
| `commands/raw_response.rs` | `docs/specs/05_raw_response.md` |
| `commands/viewer.rs` | `docs/specs/06_viewer.md` |
| `commands/viewer.rs` | `docs/specs/06_viewer.md` |
| `commands/analytics.rs` | `docs/specs/07_revenue.md` |
| `commands/database.rs` | `docs/specs/08_database.md` |
| `commands/config.rs` | `docs/specs/09_config.md` |

### 新機能追加の流れ

1. `docs/specs/` に仕様書を作成（仕様駆動）
2. `src-tauri/src/commands/` にTauriコマンドを実装
3. `src-tauri/src/lib.rs` の `invoke_handler!` にコマンドを登録
4. `src/lib/tauri/` にTypeScriptラッパーを作成
5. `src/lib/stores/` にストアを作成 (Svelte 5 Runes)
6. `src/lib/components/` にUIコンポーネントを作成
7. `e2e/` にE2Eテストを追加

## 作業開始前チェックリスト

1. [constitution.md](./constitution.md) を読み、プロジェクトのルールを把握する
2. 該当する機能の仕様書 ([docs/specs/](./docs/specs/)) を確認する
3. 仕様書の「制約・不変条件」に従う。変更が必要な場合は実装せずに報告する

ルールの詳細は constitution.md、ドキュメントの書き方は [docs/SPECIFICATION_GUIDE.md](./docs/SPECIFICATION_GUIDE.md) を参照。
