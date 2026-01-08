# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## プロジェクト概要

**liscov** は YouTube Live Chat Monitor として設計された Rust + Dioxus 0.7 ベースのデスクトップアプリケーション。リアルタイムでYouTubeライブチャットを監視・分析し、収益トラッキング、視聴者エンゲージメント分析、TTS読み上げなどの機能を提供する。

### 技術スタック
- **GUI Framework**: Dioxus 0.7 (Desktop)
- **Runtime**: Tokio (非同期ランタイム)
- **Database**: SQLite (rusqlite)
- **HTTP Client**: reqwest
- **API**: YouTube InnerTube (非公式)
- **TTS**: 棒読みちゃん / VOICEVOX 対応
- **File Dialog**: rfd (Native file dialogs)

### プロジェクト状況
- **移行完了**: Slint → Dioxus への移行は完了
- **TTS機能**: 棒読みちゃん/VOICEVOX対応の読み上げ機能実装済み
- 現在はビルド・テストが正常に通る状態

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
│   └── export/         # CSV/Excel/JSON エクスポーター
├── api/               # YouTube InnerTube API統合
│   ├── auth/          # 認証関連
│   └── innertube/     # InnerTube API実装
├── bin/               # バイナリエントリーポイント
├── chat_management/   # チャット管理・フィルタリング
├── database/          # SQLiteデータベース操作
├── gui/               # Dioxus 0.7 GUI実装
│   ├── commands/      # チャットコマンド処理
│   ├── components/    # UIコンポーネント
│   ├── events/        # イベント定義・処理
│   ├── features/      # 機能モジュール
│   ├── hooks/         # カスタムフック
│   ├── plugins/       # プラグイン実装
│   │   └── tts_plugin/    # TTS読み上げプラグイン
│   │       ├── backends/  # 棒読みちゃん/VOICEVOX バックエンド
│   │       ├── config.rs  # TTS設定
│   │       ├── queue.rs   # 優先度キュー
│   │       └── launcher.rs # アプリ自動起動
│   ├── state/         # 状態管理
│   ├── styles/        # テーマ・CSS
│   ├── config_manager.rs   # 設定管理
│   ├── tts_manager.rs      # グローバルTTSマネージャー
│   ├── signal_manager.rs   # シグナル管理
│   └── state_management.rs # 状態管理コア
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
  - `tts_plugin/`: TTS読み上げ（棒読みちゃん/VOICEVOX）
  - `analytics_plugin.rs`: 分析機能
  - `message_filter_plugin.rs`: メッセージフィルタ
  - `notification_plugin.rs`: 通知機能
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
- `desktop`: デスクトップ機能
- `web`: Web機能（将来対応予定）
- `testing`: テスト専用機能

**注意**: デフォルトでは機能フラグは有効化されていません（`default = []`）。デスクトップビルド時は `dioxus` クレートの `desktop` 機能が依存関係で自動的に有効になります。

## 開発ガイドライン

### コーディングルール
- 新規ファイル作成より既存ファイル編集を優先
- 公開機能に対するテストコード原則作成
- 機能変更時のドキュメント更新必須
- クリーンコード維持

### コード品質の維持
- Dioxus 0.7 への移行は完了済み
- 既存のアーキテクチャパターンを踏襲すること
- シグナルベースの状態管理（`signal_manager.rs`）を活用

### パフォーマンス考慮事項
- メモリ効率: 循環バッファによるメッセージ管理
- UI最適化: Dioxus Signalsによる効率的な状態更新
- 接続管理: YouTube API継続トークン処理

## よくある作業

### ビルド確認
```bash
cargo check   # 構文チェック
cargo build   # ビルド
cargo test    # テスト実行
```

### 新機能追加
1. 適切なモジュール（`analytics/`, `gui/components/` 等）を選択
2. Dioxus 0.7の`use_signal`パターンに従う
3. エラーハンドリングで `LiscovError` を使用
4. テストコード追加

### アナリティクス機能拡張
- `analytics/` モジュールで収益分析機能
- `analytics/export/` でCSV/Excel/JSON エクスポート
- `database/models.rs` でデータモデル定義

### TTS機能拡張
- `gui/plugins/tts_plugin/`: TTS読み上げプラグイン
  - `backends/`: 棒読みちゃん/VOICEVOXバックエンド実装
  - `config.rs`: TTS設定構造体
  - `queue.rs`: 優先度付きキュー（SuperChat > Membership > Normal）
  - `launcher.rs`: アプリ自動起動/終了
- `gui/tts_manager.rs`: グローバルTTSマネージャー
- `gui/components/tts_settings.rs`: TTS設定UI

## 参考ドキュメント

- [アーキテクチャ詳細](docs/architecture/README.md)
- [Dioxus 0.7ドキュメント](https://dioxuslabs.com/learn/0.7/)
- [プロジェクト設定](Dioxus.toml)

