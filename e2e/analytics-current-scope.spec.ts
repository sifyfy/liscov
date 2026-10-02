import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  connectToMockStream,
  disconnectAndInitialize,
  navigateToTab,
} from './utils/test-helpers';

/**
 * 「現在」の分析の対象（07_revenue.md「集計の対象」）
 * この起動で接続したセッションすべてを DB から数える。切断した配信も含む。
 */
test.describe('分析の対象 (07_revenue.md)', () => {
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

  async function receiveSuperChats(authors: string[]): Promise<void> {
    for (const author of authors) {
      await addMockMessage({
        message_type: 'superchat',
        author,
        content: 'がんばって',
        amount: '¥500',
        tier: 'green',
        channel_id: `UC_${author}`,
      });
    }
    // 画面に出たら DB には保存済み（保存してから画面に送る。08_database.md「書き込み」）
    for (const author of authors) {
      await expect(mainPage.getByText(author).first()).toBeVisible({ timeout: 10000 });
    }
  }

  // 仕様: 配信 A に接続 → 切断 → 配信 B に接続 → A と B の合計
  test('切断した配信も含めて、この起動で接続した配信を合わせて数える', async () => {
    await connectToMockStream(mainPage, 'test_video_scope_a');
    await receiveSuperChats(['ScopeDonorA1', 'ScopeDonorA2']);
    await disconnectAndInitialize(mainPage);

    await connectToMockStream(mainPage, 'test_video_scope_b');
    await receiveSuperChats(['ScopeDonorB1']);

    await navigateToTab(mainPage, 'Analytics');
    await mainPage.locator('button:has-text("Refresh")').click();
    // 「Super Chats」のラベルのすぐ後ろが件数
    const superChats = mainPage.locator('p:text-is("Super Chats") + p');
    await expect(superChats).toHaveText('3', { timeout: 10000 });

    await navigateToTab(mainPage, 'Chat');
    await disconnectAndInitialize(mainPage);
  });
});
