# liscov-tauri 機能仕様書

## 機能一覧

| No. | 機能名 | 概要 | 仕様書 |
|-----|-------|------|--------|
| 1 | 認証機能 | YouTube アカウント認証を管理し、メンバー限定配信へのアクセスを可能にする | [01_auth.md](specs/01_auth.md) |
| 2 | チャット接続・モニタリング機能 | YouTube Live配信に接続し、リアルタイムでチャットメッセージを取得・表示する | [02_chat.md](specs/02_chat.md) |
| 3 | WebSocket API機能 | 外部アプリケーションにリアルタイムチャットデータを提供する | [03_websocket.md](specs/03_websocket.md) |
| 4 | TTS（読み上げ）機能 | チャットメッセージを音声で読み上げる。棒読みちゃん / VOICEVOX対応 | [04_tts.md](specs/04_tts.md) |
| 5 | 生レスポンス保存機能 | YouTube InnerTube APIの生レスポンスをNDJSON形式で保存する | [05_raw_response.md](specs/05_raw_response.md) |
| 6 | 視聴者管理機能 | チャット参加者の情報を管理し、カスタム情報を保存する | [06_viewer.md](specs/06_viewer.md) |
| 7 | 収益分析・エクスポート機能 | SuperChat、メンバーシップ等の収益を分析し、エクスポートする | [07_revenue.md](specs/07_revenue.md) |
| 8 | データベース・セッション管理 | チャットセッション、メッセージ、視聴者情報をSQLiteに永続化する | [08_database.md](specs/08_database.md) |
| 9 | 設定機能 | アプリケーション設定の永続化と管理を行う | [09_config.md](specs/09_config.md) |
| 10 | ウィンドウ状態管理 | ウィンドウサイズと位置を永続化し、次回起動時に復元する | [10_window_state.md](specs/10_window_state.md) |

## 永続化ファイル一覧

| ファイル | パス | 形式 | 用途 |
|---------|------|------|------|
| config.toml | `%APPDATA%/liscov-tauri/config.toml` | TOML | アプリケーション設定 |
| credentials.toml | `%APPDATA%/liscov-tauri/credentials.toml` | TOML | YouTube認証情報（fallbackモード時） |
| tts_config.toml | `%APPDATA%/liscov-tauri/tts_config.toml` | TOML | TTS設定 |
| liscov.db | `%APPDATA%/liscov-tauri/liscov.db` | SQLite | セッション・メッセージ・視聴者情報 |
| raw_responses.ndjson | ユーザー指定パス（デフォルト: `raw_responses.ndjson`） | NDJSON | 生APIレスポンス（任意） |

## テスト要件

各機能について以下をテストすること：

1. **コマンド呼び出し**: 正しいパラメータでTauriコマンドが呼ばれる
2. **レスポンス反映**: コマンドの戻り値がUIに正しく反映される
3. **永続化**: 設定がファイルに保存され、再読込で復元される
4. **エラーハンドリング**: エラー時にユーザーに適切なフィードバック
