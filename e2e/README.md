# E2E Tests

実際のTauriアプリケーションを使用したE2Eテストです。WebView2のCDP（Chrome DevTools Protocol）を通じてPlaywrightで操作します。

## 前提条件

- Windows (WebView2)。Rust と pnpm でビルドできる環境
- Playwright がインストール済み

## テスト実行方法

```bash
pnpm test:e2e              # ビルド → 全テスト
pnpm test:e2e:chat         # ビルド → チャット表示のテストだけ
pnpm test:e2e:auth         # ビルド → 認証フローのテストだけ
pnpm test:e2e:ws           # ビルド → WebSocket のテストだけ
```

モックサーバー・静的フロントエンドサーバー・Tauri アプリは、テストが自動で起動・終了する
(`utils/test-helpers.ts` の `setupTestEnvironment()`)。手で `pnpm tauri dev` を起動する必要は無い。

### ビルドについて

`pnpm test:e2e:build` (`scripts/build-for-e2e.ts`) は、**毎回** `pnpm build` と `cargo build --workspace` を実行する。
成果物の有無や mtime では判定しない。以前は「存在すれば skip」だったため、変更後も古い成果物のまま
E2E が走り、壊した本番経路でもテストが通ってしまった。変更が無いときの上乗せは約 14 秒
(cargo はインクリメンタルで約 1 秒、`pnpm build` が約 13 秒)。

テストが使う成果物と配信のされ方:

| 成果物 | 使われ方 |
|--------|----------|
| `build/` (`pnpm build`) | テスト中に `127.0.0.1` と `::1` の 5173 (Tauri の `devUrl`) で配信される (`utils/static-frontend-server.ts`) |
| `target/debug/liscov-tauri.exe` | debug ビルドはフロントエンドを埋め込まず、上の `devUrl` を読む |
| `target/debug/mock-server.exe` | InnerTube API のモック |

フロントエンドだけ変えたときに exe を作り直す (`lib.rs` を touch する等) 必要は無い。
E2E 中は 5173 を使う `pnpm dev` / `pnpm tauri dev` を止めておく (下の「ポート 5173 が使用中でエラーになる」を参照)。

ビルドを挟まずにテストだけ回したいとき (spec だけ直して繰り返すとき) は、Playwright を直接呼ぶ。
**直前のビルドより後にソースを変えていれば、古い成果物でテストすることになる**ので注意する。

```bash
pnpm exec playwright test --config e2e/playwright.config.ts e2e/viewer-management.spec.ts
```

## テスト内容

### auth-flow.spec.ts

認証ウィンドウのフロー全体をテスト:

1. **ログイン**: 認証ウィンドウを開き、モックサーバーでログイン
2. **認証状態表示**: ログイン後の状態が正しく表示されることを確認
3. **ログアウト**: ログアウト後の状態が正しく表示されることを確認

## 環境変数

Tauri アプリに渡す環境変数。`setupTestEnvironment()` が設定するので、手で設定する必要は無い。

| 変数名 | 説明 |
|--------|------|
| `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` | WebView2に渡す追加引数。CDPを有効にするために `--remote-debugging-port=9222` を指定 |
| `LISCOV_AUTH_URL` | 認証ウィンドウの初期URL。テスト時はモックサーバーを指定 |

## トラブルシューティング

### "No browser contexts found" エラー

`setupTestEnvironment()` が Tauri アプリに `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` を渡せているか、
アプリが起動直後に落ちていないか (テスト失敗時に添付されるログ) を確認する。

### 接続タイムアウト

1. Tauriアプリが完全に起動していることを確認
2. ポート9222が使用可能であることを確認（`netstat -an | findstr 9222`）
3. ファイアウォールがローカル接続をブロックしていないことを確認

### 変更したはずのフロントエンドが反映されない

1. `pnpm test:e2e*` ではなく Playwright を直接呼んでいないか (ビルドが挟まらない)
2. 以前は、ポート 5173 を別のプロセスが使っていると既存のサーバーを黙って再利用していた。
   いまはエラーで止まる (次の「ポート 5173 が使用中でエラーになる」)

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
