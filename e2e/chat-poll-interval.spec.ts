import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  setStreamState,
  connectToMockStream,
  disconnectAndInitialize,
  addMockMessage,
  chatRow,
  mockChatRequestCount,
} from './utils/test-helpers';

/**
 * チャットの取得の間隔（02_chat.md「取得の間隔」）
 * 待ちの計算は chat_runtime.rs の単体テストで仕様の例を確かめる。ここでは本番の監視ループが
 * 空振りで間隔を延ばし、応答の timeoutMs で止め、新しいメッセージで戻すことを実 Tauri で確かめる。
 */
test.describe('チャットの取得の間隔 (02_chat.md)', () => {
  let browser: Browser;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000);
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    mainPage = connection.page;
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test.beforeEach(async () => {
    await resetMockServer();
  });

  test.afterEach(async () => {
    await resetMockServer();
    await disconnectAndInitialize(mainPage);
  });

  // 仕様: timeoutMs 3000 の応答で空振りが続く → 1.5 → 2.5 → 3 → 3 …
  // 間隔が 3 秒で止まっていれば、9 秒間の取得は 3 回前後（1.5 秒固定なら 6 回、上限が効かなければ 1〜2 回）
  test('コメントが途切れると取得の間隔が延び、応答の timeoutMs で止まる', async () => {
    test.setTimeout(60000);
    await setStreamState({ chat_timeout_ms: 3000 });
    await connectToMockStream(mainPage, 'test_video_poll_ceiling');

    // 空振りが 3 回を超え、待ちが上限に届くまで待つ
    await mainPage.waitForTimeout(12000);
    const before = await mockChatRequestCount();
    await mainPage.waitForTimeout(9000);
    const polls = (await mockChatRequestCount()) - before;

    expect(polls).toBeGreaterThanOrEqual(2);
    expect(polls).toBeLessThanOrEqual(4);
  });

  // 仕様: 空振りが続いたあと新しいメッセージが届く → 次の待ちは 1.5秒
  // 届いた直後に積んだメッセージは、次の取得（1.5 秒後）で出る。戻らなければ上限の 5 秒かかる
  test('新しいメッセージが届いたら間隔が 1.5 秒に戻る', async () => {
    test.setTimeout(60000);
    await setStreamState({ chat_timeout_ms: 5000 });
    await connectToMockStream(mainPage, 'test_video_poll_reset');
    await mainPage.waitForTimeout(15000);

    await addMockMessage({ message_type: 'text', author: 'PollUser', content: '途切れた後の最初のコメント' });
    await expect(chatRow(mainPage, '途切れた後の最初のコメント')).toBeVisible({ timeout: 8000 });

    const started = Date.now();
    await addMockMessage({ message_type: 'text', author: 'PollUser', content: '続けて届くコメント' });
    await expect(chatRow(mainPage, '続けて届くコメント')).toBeVisible({ timeout: 8000 });
    expect(Date.now() - started).toBeLessThan(3000);
  });
});
