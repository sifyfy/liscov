import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  connectToMockStream,
  disconnectAndInitialize,
  openViewerPanel,
} from './utils/test-helpers';

/**
 * 同じ配信への再接続（02_chat.md「メッセージ受信」「制約: 複合キー connection_id:message_id」）
 *
 * YouTube は新しい接続にも直近のコメントを同じ message_id で返す。別の接続のメッセージなので
 * 新着として並び、同じ id が画面に 2 つあっても表示・操作が止まらないこと。
 */
test.describe('同じ配信への再接続 (02_chat.md)', () => {
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
    await disconnectAndInitialize(mainPage);
  });

  const comment = (text: string) => mainPage.locator('[data-message-id]', { hasText: text });

  test('再接続で同じ id のコメントが届いても両方表示され、その後も受信と視聴者パネルが動く', async () => {
    const backlog = {
      message_type: 'text',
      author: '@viewer-backlog',
      channel_id: 'UC_viewer_backlog',
      content: '再接続前のコメント',
      id: 'backlog-message-1',
    };
    await connectToMockStream(mainPage, 'test_video_reconnect_same');
    await addMockMessage(backlog);
    await expect(comment('再接続前のコメント')).toHaveCount(1, { timeout: 10000 });

    // 切断してもメッセージは残る。同じ配信に再接続すると、直近のコメントが同じ id で届く
    await mainPage.locator('[data-testid="connection-chip"] button').first().click();
    await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0);
    await connectToMockStream(mainPage, 'test_video_reconnect_same');
    await addMockMessage(backlog);
    await expect(comment('再接続前のコメント')).toHaveCount(2, { timeout: 10000 });

    // 画面が止まっていなければ、後から届いたコメントも出る
    await addMockMessage({ ...backlog, id: 'after-reconnect-1', content: '再接続後のコメント' });
    await expect(comment('再接続後のコメント')).toHaveCount(1, { timeout: 10000 });

    // 視聴者パネルの「この人のコメント」にも同じ id のコメントが 2 つ並ぶ
    const panel = await openViewerPanel(mainPage, '再接続後のコメント');
    await panel.getByRole('button', { name: /この人のコメント 3 件/ }).click();
    await expect(panel.getByTestId('viewer-history-item')).toHaveCount(3);
    await panel.getByRole('button', { name: '閉じる' }).click();
  });
});
