/**
 * E2Eテスト用カスタムフィクスチャ
 *
 * テストごとにログバッファのライフサイクルを管理する。
 * - 成功テスト: バッファを破棄（コンソール出力なし）
 * - 失敗テスト: バッファを attachment としてレポーターに渡す
 * - @verbose タグ付きテスト: 常に attachment として渡す
 *
 * 出力はカスタムレポーター (buffered-log-reporter.ts) が担当する。
 * レポーター配列で list の後に配置することで、テスト結果行の後にログが出る。
 *
 * 前提: workers=1（シリアル実行）。共有リングバッファを使用するため
 * 複数ワーカーでの実行は未対応。
 */

import { test as base } from '@playwright/test';
import { drainBuffer, clearBuffer } from './logger';

/** attachment名。レポーターがこの名前で検索する */
// 先頭 _ で list レポーターの自動表示を抑制する
export const LOG_ATTACHMENT_NAME = '_buffered-log';

export const test = base.extend({
  // autoフィクスチャ: テスト結果に応じてバッファを制御
  _logBuffer: [
    async ({}, use, testInfo) => {
      // workers=1 前提の実行時ガード
      if (testInfo.parallelIndex > 0) {
        throw new Error(
          'E2Eテストのログバッファは workers=1 を前提としています。' +
            '複数ワーカーでの実行はサポートされていません。'
        );
      }

      await use(undefined);

      // テスト結果に応じてバッファを制御
      const shouldFlush =
        testInfo.status !== testInfo.expectedStatus ||
        testInfo.tags.includes('@verbose');

      if (shouldFlush) {
        const content = drainBuffer();
        if (content) {
          await testInfo.attach(LOG_ATTACHMENT_NAME, {
            body: Buffer.from(content, 'utf8'),
            contentType: 'text/plain',
          });
        }
      } else {
        clearBuffer();
      }
    },
    { auto: true },
  ],
});

export { expect } from '@playwright/test';
