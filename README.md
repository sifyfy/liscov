# liscov

YouTube Live Chat Monitor - リアルタイムでYouTubeライブチャットを監視・分析するデスクトップアプリケーション

## 概要

**liscov** は Rust + Dioxus 0.7 で構築されたYouTubeライブチャット監視ツールです。配信者向けに、チャットのリアルタイム表示、収益トラッキング、TTS読み上げなどの機能を提供します。

## 主な機能

### ライブチャット監視
- リアルタイムメッセージ表示
- メッセージフィルタリング・検索
- 自動スクロール制御

### 収益分析
- Super Chat / Super Sticker 収益トラッキング
- メンバーシップ加入分析
- 時間別収益レポート

### TTS読み上げ
- 棒読みちゃん対応
- VOICEVOX対応
- 優先度キュー（SuperChat > Membership > 通常コメント）
- アプリ自動起動/終了機能
- 視聴者ごとの読み仮名設定

### データエクスポート
- CSV / Excel / JSON 形式対応
- カスタムフィルタリング

## 技術スタック

- **言語**: Rust 2021 Edition
- **GUI**: Dioxus 0.7 (Desktop)
- **非同期ランタイム**: Tokio
- **データベース**: SQLite (rusqlite)
- **API**: YouTube InnerTube (非公式)

## インストール

### 必要条件
- Rust 1.75+
- Windows 10/11 (WebView2 ランタイム)

### ビルド

```bash
# 開発ビルド
cargo build

# リリースビルド
cargo build --release

# 実行
cargo run --bin liscov
```

### Dioxus CLI を使用する場合

```bash
# Dioxus CLI インストール
cargo install dioxus-cli

# 開発サーバー（ホットリロード）
dx serve

# バンドル作成
dx bundle
```

## 使い方

1. アプリケーションを起動
2. YouTubeライブ配信のURLを入力
3. 「接続」ボタンをクリック
4. チャットがリアルタイムで表示される

### TTS設定

1. 「設定」タブを開く
2. 「TTS読み上げ」セクションで有効化
3. バックエンド（棒読みちゃん/VOICEVOX）を選択
4. 必要に応じて自動起動を有効化

## 設定ファイル

設定は以下の場所に保存されます:
- Windows: `%APPDATA%\sifyfy\liscov\` (`C:\Users\{ユーザー名}\AppData\Roaming\sifyfy\liscov\`)

## 開発

### プロジェクト構造

```
src/
├── analytics/       # 収益分析・エクスポート
├── api/            # YouTube InnerTube API
├── chat_management/ # チャット管理
├── database/       # SQLite操作
├── gui/            # Dioxus GUI
│   ├── components/ # UIコンポーネント
│   ├── hooks/      # カスタムフック
│   └── plugins/    # プラグイン（TTS等）
└── io/             # ファイルI/O
```

### テスト

```bash
cargo test
```

## ライセンス

MIT または Apache-2.0 のデュアルライセンス

## 謝辞

- [Dioxus](https://dioxuslabs.com/) - Rust用UIフレームワーク
- [棒読みちゃん](https://chi.usamimi.info/Program/Application/BouyomiChan/) - TTS読み上げソフト
- [VOICEVOX](https://voicevox.hiroshiba.jp/) - 無料のテキスト読み上げソフト
