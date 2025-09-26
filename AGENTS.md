# Repository Guidelines

本ドキュメントは liscov リポジトリの貢献ルールなのだ。短く要点だけを示すのだ。

## Project Structure & Module Organization
- `src/`: コア実装。`api/`(InnerTube/DB), `gui/`(Dioxus Desktop), `analytics/`, `io/`。
- `src/bin/`: 実行バイナリ。`liscov`, `generate_test_data`, `run_benchmarks`。
- `tests/`: 統合・長時間テスト。補助データは `tests/data/`。
- `docs/architecture/`: 設計メモ。変更時は該当図書を更新。
- `dist/`: Windows ビルド成果物と WebView2 ランタイム。
- `Dioxus.toml`, `env.example`: 開発時の設定テンプレート。

## Build, Test, and Development Commands
- ビルド: `cargo build` / `cargo build --release`。
- 実行(GUI): `cargo run --bin liscov -- --log-level debug` など。
- テスト: `cargo test`（必要に応じ `--features debug-full`）。
- 品質チェック: `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all`。
- Dioxus CLI がある場合のみ: `dx serve`, `dx bundle`。

## Coding Style & Naming Conventions
- Rust 2021。`cargo fmt` 準拠（4スペース・100桁目安）。
- 命名: モジュール/関数は `snake_case`、型は `UpperCamelCase`、定数は `SCREAMING_SNAKE_CASE`。
- 例外処理: `Result` と `thiserror/anyhow` を使用。GUI経路での `unwrap()/expect()` は避ける。

## Testing Guidelines
- 標準テスト + `tokio-test`。外部ネットワークに依存しないこと。
- 命名: `tests/*_tests.rs`。長時間系はファイル名/テスト名に `long_running` を含める。
- カバレッジ任意: `cargo llvm-cov` 推奨（導入済み環境のみ）。

## Commit & Pull Request Guidelines
- Conventional Commits 準拠: `feat|fix|refactor|docs|test|chore: ...`。
- 1 PR = 1 目的。説明・再現手順・スクリーンショット（GUI変更）は必須。
- 関連 Issue をリンク。テストと `clippy/fmt` を通してから提出。

## Security & Configuration Tips
- 秘密情報はコミット禁止。`env.example` を `.env` にコピーして編集。
- ログ出力は CLI/環境変数（例: `LISCOV_LOG_DIR`）で指定可能。
- Windows での GUI 実行は WebView2 ランタイムが前提（`dist/` 同梱）。

## Agent-Specific Instructions
- 外部アドバイザー(GPT-5-Pro)と連携する場合は `.gpt-5-pro-communication-channel/{連番}_{caption}.md` を作成し、プロダクトオーナーへ必ず通知するのだ。

