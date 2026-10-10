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
| `build/` (`pnpm build`) | テスト中に `127.0.0.1` の空きポートで配信され、WebView2 の `localhost:5173` (Tauri の `devUrl`) がそこへ付け替えられる (`utils/static-frontend-server.ts`。下の「並列実行と分離」) |
| `target/debug/liscov-tauri.exe` | debug ビルドはフロントエンドを埋め込まず、上の `devUrl` を読む |
| `target/debug/mock-server.exe` | InnerTube API のモック |

フロントエンドだけ変えたときに exe を作り直す (`lib.rs` を touch する等) 必要は無い。
E2E は 5173 を listen しないので、`pnpm dev` / `pnpm tauri dev` を動かしたままでも走らせられる。

ビルドを挟まずにテストだけ回したいとき (spec だけ直して繰り返すとき) は、Playwright を直接呼ぶ。
**直前のビルドより後にソースを変えていれば、古い成果物でテストすることになる**ので注意する。

```bash
pnpm exec playwright test --config e2e/playwright.config.ts e2e/viewer-management.spec.ts
```

## 並列実行と分離

複数のワークツリーの E2E を同時に走らせても干渉しない。本番の liscov や `pnpm dev` を動かしたままでもよい。
干渉の恐れがあるときは、黙って続けずにエラーで止まる。経緯と検討した選択肢は
[ADR-005](../docs/decisions/005_e2e_worktree_isolation.md) にある。

### ワークツリーごとの名前空間

`liscov-test-<ワークツリー ID>` をテスト用の名前に使う。ワークツリー ID はプロジェクトのパスの SHA-256 の先頭 8 桁 (16 進の小文字)。
Windows ではパスの大文字・小文字を区別しない。

| プロジェクトのパス | ワークツリー ID |
|--------------------|-----------------|
| `C:\Users\cat\dev\liscov` と `c:\users\cat\dev\liscov` | 同じ |
| `C:\Users\cat\dev\liscov` と `C:\Users\cat\dev\liscov\.claude\worktrees\foo` | 違う |
| 同じパスで何度実行しても | 同じ (実行ごとには変わらない) |

この名前を次のものに使う (`<名前>` = `liscov-test-<ワークツリー ID>`)。

| もの | 場所・値 |
|------|----------|
| アプリのデータ (`LISCOV_APP_NAME`) | `%APPDATA%\<名前>` |
| 資格情報 (`LISCOV_KEYRING_SERVICE`) | Windows 資格情報マネージャーの `youtube_credentials.<名前>` |
| WebView2 のデータ (`WEBVIEW2_USER_DATA_FOLDER`) | `%LOCALAPPDATA%\<名前>\EBWebView` |
| ウィンドウ状態 | `%APPDATA%\com.liscov-tauri.app\.window-state.<名前>.json` |

### ポート

固定ポートは使わない。どれも OS に空きポートを選ばせ、**自分が起動したものから**実際のポートを読む。

| 相手 | 起動のしかた | ポートの読み方 |
|------|--------------|----------------|
| CDP | `--remote-debugging-port=0` | 起動前に消しておいた `<WebView2 のデータ>\EBWebView\DevToolsActivePort` が書かれるのを待って読む |
| モックサーバー | `mock-server.exe --port 0` | 標準出力の `Mock server on http://127.0.0.1:<port>` を読む |
| フロントエンド | 静的サーバーを `127.0.0.1:0` で listen | listen した結果から読む |
| アプリの WebSocket サーバー | アプリに `LISCOV_WEBSOCKET_PORT=0` を渡す | 画面のステータス表示 `WS:<port>(<接続数>)` から読む |

WebSocket サーバーは、環境変数を渡さないと本番と同じ 8765〜8774 を使う。本番の liscov が止まっていると E2E のアプリが 8765 を取り、
本番向けに 8765 へつなぎに来る OBS のブラウザソースのオーバーレイがつながってしまう (接続数のテストが落ち、テストのコメントが配信画面に流れうる)。
そのため `startTauriAppWithEnv()` で必ず渡す ([03_websocket.md](../docs/specs/03_websocket.md)「開始ポートの上書き」)。

`DevToolsActivePort` の中身は 1 行目がポート、2 行目がブラウザのエンドポイントのパス。

```text
56032
/devtools/browser/793d0980-164f-4b16-8e77-89cc5677dce5
```

このとき CDP には `ws://127.0.0.1:56032/devtools/browser/793d0980-164f-4b16-8e77-89cc5677dce5` で接続する。
1 行目が数字でない、または 1〜65535 の範囲外、2 行目が `/devtools/browser/` で始まらないときは、まだ書き終わっていないものとして待ち続ける。

フロントエンドは、Tauri の `devUrl` (`src-tauri/tauri.conf.json`) を変えずに接続先だけを付け替える。
静的サーバーのポートが 56031 なら、WebView2 には次の引数を渡す。ページの URL は `http://localhost:5173/` のまま。

```text
--remote-debugging-port=0 --host-resolver-rules="MAP localhost:5173 127.0.0.1:56031"
```

### 繋がった相手の確認

静的サーバーは起動ごとに ID (UUID) を作り、`GET /__liscov_e2e_server_id` にその ID をテキストで返す。
ほかのパスは今までどおり `build/` のファイルを返し、無ければ `index.html` を返す。

アプリに接続した直後 (`connectToApp()`)、ページから `/__liscov_e2e_server_id` を fetch する。

| fetch の結果 | 動作 |
|--------------|------|
| 静的サーバーの ID と一致 | 続ける |
| 一致しない (例: `pnpm dev` の Vite が `index.html` を返した) | 止める: `フロントエンドが E2E の静的サーバーから読み込まれていません` |
| fetch が失敗した (例: 付け替えが効かず 5173 に誰もいない) | 同じエラーで止める |

### 同じワークツリーでの二重実行

同じワークツリーの E2E を 2 つ同時に走らせると名前空間が同じになり、互いのアプリを止め合う。
そこで、名前空間に触る操作 (アプリ・モックサーバーの停止と起動、テストデータ・資格情報の削除) の前に、
`.tmp/e2e.lock` に自分 (Playwright のワーカー) のプロセス ID を書く。停止もロックの後にするのは、
exe のパスで止める処理が、同じワークツリーの別の実行のアプリまで止めてしまうため。
`setupTestEnvironment()` を通らない spec もあるので、それぞれの関数の中で取る。ワーカーのプロセスが終わるときに消す。

| ロックファイルの状態 | 動作 |
|----------------------|------|
| 無い | 作って続ける |
| 自分のプロセス ID が書かれている | 続ける (同じ実行の中で何度起動してもよい) |
| 生きている別のプロセスの ID が書かれている | 止める: `このワークツリーでは別の E2E (PID <番号>) が実行中です` |
| もう居ないプロセスの ID が書かれている (前の実行が落ちて消せなかった) | 上書きして続ける |

### 残るもの

ワークツリーを消しても、`%LOCALAPPDATA%\liscov-test-<ID>` (WebView2 のデータ) とウィンドウ状態のファイルは残る。
ADR-005 より前の共通の名前空間 (`liscov-test`) のデータも残っている。E2E が動いていないときに消してよい。

```powershell
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\liscov-test*", "$env:APPDATA\liscov-test*", "$env:APPDATA\com.liscov-tauri.app\.window-state.liscov-test*.json"
```

`liscov-test*` は本番 (`liscov-tauri`、`com.liscov-tauri.app\.window-state.json`) に一致しない。

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
| `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` | WebView2 に渡す追加引数。CDP を空きポートで開く `--remote-debugging-port=0` と、フロントエンドを付け替える `--host-resolver-rules` |
| `WEBVIEW2_USER_DATA_FOLDER` | WebView2 のデータフォルダ。ワークツリーごとに分ける |
| `LISCOV_APP_NAME` / `LISCOV_KEYRING_SERVICE` | テスト用の名前 `liscov-test-<ワークツリー ID>` |
| `LISCOV_AUTH_URL` / `LISCOV_SESSION_CHECK_URL` / `LISCOV_YOUTUBE_BASE_URL` | モックサーバーの URL (起動ごとにポートが変わる)。モックを使わない spec でも、`startTauriApp()` はモックが起動していなければ起動して、その URL を渡す |

## トラブルシューティング

### "No browser contexts found" エラー

`setupTestEnvironment()` が Tauri アプリに `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` を渡せているか、
アプリが起動直後に落ちていないか (テスト失敗時に添付されるログ) を確認する。

### CDP の接続でタイムアウトする

1. アプリが起動直後に落ちていないか (テスト失敗時に添付されるログ)
2. 同じ WebView2 のデータフォルダを使うアプリが既に動いていないか。動いていると WebView2 はそのブラウザプロセスを共有し、
   追加引数が効かないので `DevToolsActivePort` が書かれない。前の実行の残りなら、このワークツリーの
   `target\debug\liscov-tauri.exe` から起動したプロセスを止める

### 変更したはずのフロントエンドが反映されない

`pnpm test:e2e*` ではなく Playwright を直接呼んでいないか (ビルドが挟まらない)。

### `フロントエンドが E2E の静的サーバーから読み込まれていません` で止まる

WebView2 が `--host-resolver-rules` に従わず、本物の `localhost:5173` を読んでいる。
WebView2 の更新で引数の扱いが変わった可能性がある。ADR-005 の選択肢 C (フロントを exe に埋め込む) を検討する。

以前 (PR #10 より前) は、5173 を使っているほかのサーバーを黙って再利用していたため、
このワークツリーの `build/` ではないフロントエンド (Vite が配信するソースや、別のワークツリーの `build/`) を検証してしまうことがあった。
この確認はその再発を防ぐためのもの。

### `このワークツリーでは別の E2E (PID ...) が実行中です` で止まる

同じワークツリーで E2E が既に動いている。終わるのを待つか、別のワークツリーで走らせる。
そのプロセスが E2E でないなら (PID の再利用)、`.tmp/e2e.lock` を消して再実行する。
