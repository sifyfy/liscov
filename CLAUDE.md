# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

## アーキテクチャナビゲーション

### 新機能追加の流れ

1. `docs/specs/` に仕様書を作成（仕様駆動）
2. `src-tauri/src/commands/` にTauriコマンドを実装
3. `src-tauri/src/lib.rs` の `invoke_handler!` にコマンドを登録
4. `src/lib/tauri/` にTypeScriptラッパーを作成
5. `src/lib/stores/` にストアを作成 (Svelte 5 Runes)
6. `src/lib/components/` にUIコンポーネントを作成
7. `e2e/` にE2Eテストを追加

### コマンド → 仕様書マッピング

各コマンドモジュールには対応する仕様書がある。実装前に必ず仕様書を確認すること。

| コマンド | 仕様書 |
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

## 非自明な規約

- **Svelte 5 Runes**: `$state`, `$derived`, `$effect` を使用（旧Svelte storeパターンではない）
- **CSS変数テーマ**: `app.css` でダーク/ライトテーマをCSS変数で定義（`--bg-base`, `--bg-surface-1`〜`3`, `--accent`, `--text-primary` 等）。`data-theme` 属性で切替
- **AppState**: `Arc<RwLock<T>>` パターンで並行アクセス。`state.rs` で定義
- **Tauri Events**: `chat:message`, `chat:connection` でバックエンド→フロントエンド通知
- **Cargo workspace**: `src-tauri/`（メインアプリ）+ `crates/mock-server/`（E2Eモックサーバー）
- **E2Eテスト分離**: 環境変数 `LISCOV_APP_NAME=liscov-test` 等で本番データと分離。詳細はREADMEのE2Eテストセクション参照
