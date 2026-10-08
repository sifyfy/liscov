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
| `build/` (`pnpm build`) | テスト中に `127.0.0.1:5173` (Tauri の `devUrl`) で配信される |
| `target/debug/liscov-tauri.exe` | debug ビルドはフロントエンドを埋め込まず、上の `devUrl` を読む |
| `target/debug/mock-server.exe` | InnerTube API のモック |

フロントエンドだけ変えたときに exe を作り直す (`lib.rs` を touch する等) 必要は無い。

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

## デザインの撮影

テストではなく、デザインの診断・批評に使うスクリーンショットを撮る。主要な画面を、データの状態
(未接続・空・通常・長い名前・スパチャ連続・大量・視聴者情報) × テーマ (ダーク/ライト) × 大きさ
(540×960・640×1080・960×1080、`docs/design/brief.md` §3 の基準) で撮る。デザインの試作 (`src/routes/design-proto`) も場面ごとに撮る。

```bash
pnpm design:capture
```

- 起動・接続は通常の E2E と同じ (テスト用のアプリ名 `liscov-test`・モックサーバー)。prebuilt の成果物を使うので、
  frontend を変えたら先に `pnpm test:e2e:build` (上の「ビルドについて」)
- 出力は `.tmp/design-capture/latest/` (git に入れない)。フォルダ名は `LISCOV_DESIGN_CAPTURE_LABEL` で変えられる。
  撮影を始めるときにそのフォルダを消すので、シェルの作業ディレクトリにしていると消せずに失敗する
- 各状態・テーマで、axe によるコントラストの検査 (WCAG 2 AA) も行い、`<名前>-<テーマ>.axe.json` に残す。
  `incomplete` (背景がグラデーションなどで判定できなかったもの) は合格ではない
- 撮影の本体は `design-capture.capture.ts`、流すデータは `utils/design-capture-data.ts` (名前・本文はすべて架空)。
  ファイル名が `*.spec.ts` ではないので、通常の E2E では動かない。設定は `playwright.capture.config.ts`

### 見た目の回帰テスト

撮影と違い、通常の E2E として動く。`chat-row-visual.spec.ts` が `/design-tokens` の「コメントの行」
(本物の `ChatMessage` を固定の架空データ `src/routes/design-proto/data.ts` の `ROW_SAMPLES` で並べたもの) を
幅 540 のダーク/ライトで撮り、`chat-row-visual.spec.ts-snapshots/` の基準と比べる。
見た目を意図して変えたら、撮影で確かめてから `--update-snapshots` で基準を更新する。

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
2. ポート 5173 を別のプロセス (`pnpm dev` の Vite や、別ワークツリーの E2E) が使っていないか。
   使用中だと静的サーバーは起動をあきらめて既存のサーバーを再利用するので、このワークツリーの `build/` は配信されない
   (`netstat -ano | findstr :5173`)
