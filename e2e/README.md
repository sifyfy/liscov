# E2E Tests

実際のTauriアプリケーションを使用したE2Eテストです。WebView2のCDP（Chrome DevTools Protocol）を通じてPlaywrightで操作します。

## 前提条件

- Tauri開発ビルドが可能な環境
- Playwrightがインストール済み

## テスト実行方法

### 1. モックサーバーを起動

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin mock_server
```

### 2. Tauriアプリを起動（デバッグポート有効）

**PowerShell:**
```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"
$env:LISCOV_AUTH_URL="http://localhost:3456/?auto_login=true"
pnpm tauri dev
```

**コマンドプロンプト:**
```cmd
set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
set LISCOV_AUTH_URL=http://localhost:3456/?auto_login=true
pnpm tauri dev
```

**Git Bash:**
```bash
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222" \
LISCOV_AUTH_URL="http://localhost:3456/?auto_login=true" \
pnpm tauri dev
```

### 3. E2Eテストを実行

```bash
pnpm exec playwright test --config e2e/playwright.config.ts
```

## テスト内容

### auth-flow.spec.ts

認証ウィンドウのフロー全体をテスト:

1. **ログイン**: 認証ウィンドウを開き、モックサーバーでログイン
2. **認証状態表示**: ログイン後の状態が正しく表示されることを確認
3. **ログアウト**: ログアウト後の状態が正しく表示されることを確認

## デザインの撮影

テストではなく、デザインの診断・批評に使うスクリーンショットを撮る。主要な画面を、データの状態
(未接続・空・通常・長い名前・スパチャ連続・大量・視聴者情報) × テーマ (ダーク/ライト) × 幅 (360/640/1024px) で撮る。

```bash
pnpm design:capture
```

- 起動・接続は通常の E2E と同じ (テスト用のアプリ名 `liscov-test`・モックサーバー)。prebuilt の成果物を使うので、
  frontend を変えたら先に `pnpm test:e2e:build`
- 出力は `.tmp/design-capture/latest/` (git に入れない)。フォルダ名は `LISCOV_DESIGN_CAPTURE_LABEL` で変えられる
- 各状態・テーマで、axe によるコントラストの検査 (WCAG 2 AA) も行い、`<名前>-<テーマ>.axe.json` に残す。
  `incomplete` (背景がグラデーションなどで判定できなかったもの) は合格ではない
- 撮影の本体は `design-capture.capture.ts`、流すデータは `utils/design-capture-data.ts` (名前・本文はすべて架空)。
  ファイル名が `*.spec.ts` ではないので、通常の E2E では動かない。設定は `playwright.capture.config.ts`

## 環境変数

| 変数名 | 説明 |
|--------|------|
| `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` | WebView2に渡す追加引数。CDPを有効にするために `--remote-debugging-port=9222` を指定 |
| `LISCOV_AUTH_URL` | 認証ウィンドウの初期URL。テスト時はモックサーバーを指定 |

## トラブルシューティング

### "No browser contexts found" エラー

Tauriアプリが `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 環境変数を設定して起動されていることを確認してください。

### 接続タイムアウト

1. Tauriアプリが完全に起動していることを確認
2. ポート9222が使用可能であることを確認（`netstat -an | findstr 9222`）
3. ファイアウォールがローカル接続をブロックしていないことを確認
