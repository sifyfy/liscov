# liscov E2E Tests

Playwright + CDP (Chrome DevTools Protocol) を使用した liscov デスクトップアプリの E2E テスト。

## 概要

このテストスイートは、Dioxus + WebView2 で構築された liscov デスクトップアプリを、
Playwright を使って E2E テストするためのものです。

WebView2 は CDP をサポートしているため、環境変数でリモートデバッグポートを指定することで、
Playwright から WebView2 内のコンテンツを操作できます。

## セットアップ

### 1. 依存関係のインストール

```bash
cd e2e-tests
npm install
```

### 2. アプリのビルド

```bash
# プロジェクトルートで実行
cargo build --release
```

## テストの実行

### 方法1: アプリを手動起動してテスト実行（推奨）

アプリを CDP 有効で起動してから、テストを実行します。

**Windows (PowerShell):**
```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9223"
.\target\release\liscov.exe
```

**Windows (Command Prompt):**
```cmd
set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223
target\release\liscov.exe
```

**Linux/Mac:**
```bash
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 ./target/release/liscov
```

別のターミナルでテストを実行:
```bash
cd e2e-tests
npm test
```

### 方法2: デバッグモードでテスト実行

Playwright の UI を使ってデバッグしながらテストを実行:
```bash
npm run test:debug
```

### 方法3: ヘッドモードでテスト実行

ブラウザウィンドウを表示してテストを実行:
```bash
npm run test:headed
```

## テストレポートの確認

テスト実行後、HTML レポートを確認できます:
```bash
npm run report
```

## テストファイル構成

```
e2e-tests/
├── tests/
│   ├── app-launcher.ts          # アプリ起動・接続ユーティリティ
│   └── viewer-management.spec.ts # 視聴者管理タブのテスト
├── playwright.config.ts          # Playwright 設定
├── package.json
├── tsconfig.json
└── README.md
```

## 環境変数

| 変数名 | デフォルト | 説明 |
|--------|-----------|------|
| `CDP_PORT` | `9223` | CDP 接続ポート |
| `DEBUG` | - | 設定するとアプリの stdout/stderr を表示 |

## トラブルシューティング

### "App is not running with CDP enabled" エラー

アプリが CDP 有効で起動していません。上記の方法でアプリを起動してください。

### "No browser contexts found" エラー

アプリが完全に起動する前にテストが接続しようとしています。
アプリが完全に起動してからテストを実行してください。

### テストがタイムアウトする

- アプリが応答しているか確認
- CDP ポートが正しいか確認: `http://localhost:9223/json/version`
- ファイアウォールが CDP ポートをブロックしていないか確認

## 新しいテストの追加

1. `tests/` ディレクトリに `*.spec.ts` ファイルを作成
2. `app-launcher.ts` の `connectToApp()` を使ってアプリに接続
3. Playwright の API を使ってテストを記述

例:
```typescript
import { test, expect } from '@playwright/test';
import { connectToApp, closeApp, AppContext } from './app-launcher';

let appContext: AppContext;

test.beforeAll(async () => {
  appContext = await connectToApp();
});

test.afterAll(async () => {
  if (appContext) {
    await closeApp(appContext);
  }
});

test('my test', async () => {
  const { page } = appContext;
  // ... テストコード
});
```

## 制限事項

- WebView2 固有の機能（ファイルダイアログなど）は CDP 経由でテストできません
- アプリは1つのインスタンスしか起動できないため、テストは並列実行できません
- CDP 接続にはアプリが起動している必要があります

## 参考資料

- [Playwright Documentation](https://playwright.dev/docs/intro)
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- [WebView2 CDP Support](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/remote-debugging)
