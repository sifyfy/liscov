# ADR-004: E2Eテストのログバッファリング

## ステータス

承認

## コンテキスト

E2Eテスト（155件）の実行時、成功テストでも `log.info()` が約104箇所から出力され、テスト結果のサマリーが視認できなくなっていた。CLIの出力バッファ制限によりサマリーが切り捨てられる問題もあった。

### 要件

1. テスト名の表示（何をテストしているか）
2. 失敗時のみ経緯ログを表示
3. サマリー（全件/成功/失敗/失敗リスト）
4. 例外的に特定テストだけログを強制出力する手段

## 決定

TestLoggerにリングバッファモードを導入し、Playwrightのautoフィクスチャ + カスタムレポーターでテスト結果に応じた出力制御を行う。

### 設計

1. **リングバッファ**: 全ログをメモリ内のリングバッファ（上限200行）に蓄積し、直接コンソールに出力しない
2. **テスト結果連動**: autoフィクスチャがテスト終了時に判定
   - 失敗 (`status !== expectedStatus`) → バッファを `testInfo.attach()` でレポーターに渡す
   - 成功 → バッファを破棄（無音）
   - `@verbose` タグ → 常にレポーターに渡す
3. **カスタムレポーター**: `buffered-log-reporter.ts` が `onTestEnd` でattachment内容を出力。`list` レポーターの後に配置することで、`✓`/`✘` 行の後にログが表示される
4. **attachment名**: `_buffered-log`（先頭 `_` で `list` レポーターの自動表示を抑制）
5. **全レベルバッファ**: `debug`/`info`/`warn`/`error` すべてバッファ対象
6. **安全策**:
   - `process.on('exit')` で最終フラッシュ（`fs.writeSync` で同期書き込み）
   - `workers=1` の実行時ガード（`parallelIndex > 0` で throw）
   - リングバッファのoverflow表示（`... N lines truncated ...`）

### ファイル構成

- `e2e/utils/logger.ts` - リングバッファ付きTestLogger + `drainBuffer()` / `clearBuffer()`
- `e2e/utils/fixtures.ts` - Playwrightカスタムフィクスチャ（autoフィクスチャでattach）
- `e2e/utils/buffered-log-reporter.ts` - カスタムレポーター（attachment内容を出力）
- `e2e/playwright.config.ts` - レポーター設定 `[['list'], ['./utils/buffered-log-reporter.ts']]`
- `e2e/*.spec.ts` - インポートを `./utils/fixtures` に変更

### 出力例

```
  ✓  1 › 成功するテスト (6ms)              ← ログなし
  ✘  2 › 失敗するテスト (5ms)              ← 結果行の後にログ
    [INFO] Step 1: データを準備
    [WARN] Warning: レスポンスが遅い
    [ERROR] Error: 想定外のステータスコード
  ✓  3 › @verbose タグ付きテスト (8ms)     ← 成功でもログ
    [INFO] 調査中の情報
```

## 理由

### 検討した選択肢

| 選択肢 | メリット | デメリット |
|-------|---------|-----------|
| A: リングバッファ + フィクスチャ + カスタムレポーター（採用） | 出力順序が保証される。失敗時のみ経緯表示 | カスタムレポーター追加が必要 |
| B: フィクスチャ内で直接stdout出力 | レポーター不要でシンプル | ログが結果行の上に表示され、前テストのログに見える |
| C: デフォルトログレベルを `warn` に変更 | 1行変更で即効 | 失敗時の経緯が見えなくなる |
| D: Playwrightの `onStdOut` で制御 | 標準API | `onStdOut` はフィルタでなく通知のみ。抑制不可 |

### 採用理由

- A案はPlaywrightのレポーター配列が設定順に `onTestEnd` を呼ぶ仕様を利用して、`list` の後にカスタムレポーターがログを出力する。これにより `✓`/`✘` 行の後にログが確実に表示される
- B案の順序問題はユーザー体験に直結するため不採用
- C案は失敗テストのデバッグ情報を失う
- D案はPlaywrightの制約上実現不可

## 影響

- 全16 specファイルのインポートを `@playwright/test` → `./utils/fixtures` に変更
- `console.log()` 直接呼び出し11箇所を `log.debug()` に統一
- `playwright.config.ts` のレポーター設定を変更

### 既知の制限（v2で対応）

- `beforeAll`/`afterAll` のログはテスト単位フィクスチャの対象外（`beforeAll`失敗時はPlaywright自体がエラー報告する）
- suite共有の `beforeAll` 不具合が後続テストで顕在化した場合、セットアップログが失われる可能性
- `workers=1` 前提（複数ワーカー未対応）

## 参考

- Playwright Reporter API: レポーター配列は設定順に `onTestEnd` が呼ばれる
- Playwright attachment: 名前が `_` で始まる attachment は `list` レポーターが自動表示しない
- 155テスト全件パス確認済み（2026-03-12）
