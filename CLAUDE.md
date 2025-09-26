# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

**liscov** は YouTube Live Chat Monitor として設計された Rust + Dioxus 0.6.3 ベースのデスクトップアプリケーション。リアルタイムでYouTubeライブチャットを監視・分析し、収益トラッキング、視聴者エンゲージメント分析、Q&A検出などの機能を提供する。

### 技術スタック
- **GUI Framework**: Dioxus 0.6.3 (Desktop)
- **Runtime**: Tokio (非同期ランタイム)
- **Database**: SQLite (rusqlite)
- **HTTP Client**: reqwest
- **API**: YouTube InnerTube (非公式)

### プロジェクト状況
- **移行中**: Slint → Dioxus 0.6.3 (Phase 0-1完了、Phase 2進行中)
- **⚠️ 注意**: 現在 `src/gui/components/chat_display.rs:1487` に構文エラーがあり、コンパイルできない状態

## 開発コマンド

### ビルドと実行
```bash
# 基本的なビルド
cargo build

# 構文チェック（推奨：エラー確認用）
cargo check

# メインアプリケーション実行
cargo run --bin liscov

# リリースビルド
cargo build --release

# デスクトップバンドル作成（Dioxus CLI使用）
dx bundle
```

### Dioxus CLIコマンド
```bash
# 開発サーバー起動（ホットリロード対応）
dx serve

# Dioxus CLI経由のビルド
dx build

# プロジェクト構文チェック
dx check

# RSXフォーマット
dx fmt
```

### テストとデバッグ
```bash
# テスト実行
cargo test

# デバッグ機能付きテスト
cargo test --features debug-full

# テストデータ生成
cargo run --bin generate_test_data

# ベンチマーク実行
cargo run --bin run_benchmarks
```

### 開発ツール
```bash
# tokio-console（プロファイリング用）
cargo run --features debug-tokio

# 詳細ログ出力
cargo run --bin liscov -- --log-level debug
```

## アーキテクチャ概要

### モジュール構成
```
src/
├── analytics/          # 収益分析・エクスポート機能
├── api/               # YouTube InnerTube API統合
├── chat_management/   # チャット管理・フィルタリング
├── database/          # SQLiteデータベース操作
├── gui/               # Dioxus 0.6.3 GUI実装
│   ├── components/    # UIコンポーネント
│   ├── hooks/         # カスタムフック
│   ├── plugins/       # プラグインシステム
│   └── state/         # 状態管理
└── io/                # ファイルI/O・NDJSON処理
```

### 重要な概念

#### イベント駆動アーキテクチャ
- 非同期メッセージ処理でリアルタイム性を実現
- Tokioベースの並行処理
- 階層化エラー処理システム（`LiscovError`）

#### プラグインシステム
- `gui/plugin_system.rs`: 拡張可能なプラグインアーキテクチャ
- `gui/plugins/`: 個別プラグイン実装
- 動的機能拡張が可能

#### YouTube API統合
- `api/innertube/`: YouTube InnerTube API（非公式）
- 継続的接続管理・エラー回復
- レート制限対応

#### データ管理
- **Database**: SQLite（`database/models.rs`）でメッセージ・セッション管理
- **Export**: CSV/Excel/JSON形式での分析データエクスポート
- **Configuration**: TOML設定ファイルによる動的設定管理

## 重要な設定ファイル

### 環境変数設定（env.example → .env）
```bash
# 開発用設定例
LISCOV_DEBUG_LEVEL=debug
LISCOV_DEV_MODE=true
TOKIO_CONSOLE=1           # tokio-consoleデバッグ用
LISCOV_API_DEBUG=true
```

### feature フラグ
- `debug-tokio`: tokio-console統合
- `debug-full`: 全デバッグ機能有効
- `desktop`: デスクトップ機能（デフォルト）
- `testing`: テスト専用機能

## 開発ガイドライン

### Cursorルール（重要）
- 新規ファイル作成より既存ファイル編集を優先
- 公開機能に対するテストコード原則作成
- 機能変更時のドキュメント更新必須
- クリーンコード維持

### 移行作業における注意点
- Slint → Dioxus移行中のため、一部コードに古い実装が残存
- `chat_display.rs` 等で構文エラーが発生中
- 新機能実装前に既存エラーの修正を推奨

### パフォーマンス考慮事項
- メモリ効率: 循環バッファによるメッセージ管理
- UI最適化: Dioxus Signalsによる効率的な状態更新
- 接続管理: YouTube API継続トークン処理

## よくある作業

### コンパイルエラー修正
1. `cargo check` でエラー箇所特定
2. 主に `src/gui/components/chat_display.rs` を確認
3. 括弧・セミコロンの不整合を修正

### 新機能追加
1. 適切なモジュール（`analytics/`, `gui/components/` 等）を選択
2. Dioxus 0.6.3の`use_signal`パターンに従う
3. エラーハンドリングで `LiscovError` を使用
4. テストコード追加

### アナリティクス機能拡張
- `analytics/` モジュールで収益分析機能
- `analytics/export/` でCSV/Excel/JSON エクスポート
- `database/models.rs` でデータモデル定義

## 参考ドキュメント

- [アーキテクチャ詳細](docs/architecture/README.md)
- [Dioxus 0.6.3ドキュメント](https://dioxuslabs.com/learn/0.6/)
- [プロジェクト設定](Dioxus.toml)