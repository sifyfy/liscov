# E2E Tests

実際のTauriアプリケーションを使用したE2Eテストです。WebView2のCDP（Chrome DevTools Protocol）を通じてPlaywrightで操作します。

## 前提条件

- Tauri開発ビルドが可能な環境
- Playwrightがインストール済み

## テスト実行方法

各 spec が `setupTestEnvironment()` などでモックサーバー (`target/debug/mock-server.exe`) と
Tauri アプリ (`target/debug/liscov-tauri.exe`) を自分で起動する。アプリやモックを手で起動しておく必要はない。

```bash
pnpm test:e2e
```

`pnpm test:e2e` は成果物 (`build/`・`target/debug/*.exe`) を用意してから全件を流す。
spec を絞るときは `pnpm test:e2e:build` の後に playwright を直接呼ぶ (直接呼ぶとビルドは挟まらない)。

```bash
pnpm exec playwright test --config e2e/playwright.config.ts e2e/chat-basic.spec.ts
```

debug ビルドの exe はフロントエンドを埋め込まず、`src-tauri/tauri.conf.json` の `devUrl` (`http://localhost:5173`) を読む。
E2E ではヘルパー (`utils/static-frontend-server.ts`) がこのワークツリーの `build/` を 5173 で配信する。
そのため E2E 中は `pnpm dev` / `pnpm tauri dev` を止めておく (下の「ポート 5173 が使用中でエラーになる」を参照)。

## テスト内容

### auth-flow.spec.ts

認証ウィンドウのフロー全体をテスト:

1. **ログイン**: 認証ウィンドウを開き、モックサーバーでログイン
2. **認証状態表示**: ログイン後の状態が正しく表示されることを確認
3. **ログアウト**: ログアウト後の状態が正しく表示されることを確認

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

### ポート 5173 が使用中でエラーになる

`E2E のフロントエンド配信用のポート ...:5173 が使用中です` で止まる場合は、ほかのプロセスが 5173 を使っている。
多いのは `pnpm dev` / `pnpm tauri dev` の Vite と、別のワークツリーで走っている E2E。止めてから再実行する。

```powershell
Get-NetTCPConnection -LocalPort 5173 -State Listen | Select-Object LocalAddress, OwningProcess
```

以前は使用中のサーバーを黙って再利用していたため、このワークツリーの `build/` ではないフロントエンド
(Vite が配信するソースや、別のワークツリーの `build/`) を検証してしまうことがあった。
また Vite は `localhost` を `::1` で待ち受けるので、E2E 側の `127.0.0.1:5173` の listen は成功してしまう。
WebView2 は `localhost` を `::1` から試すため、エラーも出ないまま Vite のフロントが読まれていた。
いまは `127.0.0.1` と `::1` の両方で listen し、どちらかが使用中なら失敗させている。

なお 9222 (CDP) と 3456 (モックサーバー) も固定ポートで、テスト用のデータフォルダ (`%APPDATA%\liscov-test`) も共通。
そのため、複数のワークツリーの E2E を同時に走らせることはできない。
