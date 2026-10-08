import { test, expect } from './utils/fixtures';
import type { Browser, Page } from '@playwright/test';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  connectToMockStream,
  disconnectAndInitialize,
} from './utils/test-helpers';

/**
 * 流れたものの欄とお礼の一覧 (docs/specs/02_chat.md「配信中の画面」の「流れたもの」「配信の最後のお礼」)
 *
 * 仕様の例: ¥10,000 の赤スパのあとにコメントが 40 件来て流れの画面から外れた → 「流れたスパチャ等」の欄の先頭に出る
 */

// 視聴者は実行ごとに新しい channel_id にする (初見の判定はテスト用の DB に残るため)
const RUN = Date.now().toString(36);

test.describe('流れたものの欄とお礼の一覧', () => {
  let browser: Browser;
  let page: Page;

  test.beforeAll(async () => {
    test.setTimeout(120000);
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    page = connection.page;
    await page.setViewportSize({ width: 640, height: 1080 });
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test('流れて画面の外に出たスパチャと新規さんが欄に残り、お礼の一覧で時刻順に見られる', async () => {
    test.setTimeout(120000);
    await resetMockServer();
    await connectToMockStream(page);

    await addMockMessage({ message_type: 'text', author: 'NewViewer', content: 'はじめまして', channel_id: `UC_new_${RUN}` });
    await addMockMessage({
      message_type: 'superchat',
      author: 'RedDonor',
      content: '遅れてきました！',
      amount: '¥10,000',
      tier: 'red',
      channel_id: `UC_red_${RUN}`,
    });
    // 同じ視聴者の 40 件 (初回の 1 件だけが新規さんの初回)
    for (let i = 0; i < 40; i++) {
      await addMockMessage({ message_type: 'text', author: 'Filler', content: `コメント ${i}`, channel_id: `UC_filler_${RUN}` });
    }
    await expect(page.getByText('コメント 39')).toBeVisible({ timeout: 15000 });

    // 流れた: スパチャと、新規さんの初回コメントが、それぞれの欄に出る
    const flowedSupers = page.locator('section[aria-label="流れたスパチャ等"]');
    const flowedNewcomers = page.locator('section[aria-label="流れた新規さん"]');
    await expect(flowedSupers).toContainText('RedDonor', { timeout: 5000 });
    await expect(flowedSupers).toContainText('¥10,000');
    await expect(flowedSupers).toContainText('遅れてきました！');
    await expect(flowedNewcomers).toContainText('NewViewer');

    // 配信の最後のお礼: 一覧を開く (1 手)。流れたスパチャ等の欄は一覧に含まれるので消え、最新のコメントは見えたまま
    await page.getByRole('button', { name: /この配信のスパチャ等の一覧/ }).click();
    const thanks = page.locator('section[aria-label="この配信のスパチャ等"]');
    await expect(thanks).toContainText('RedDonor');
    await expect(thanks).toContainText('1 件・時刻順');
    await expect(flowedSupers).toHaveCount(0);
    await expect(flowedNewcomers).toContainText('NewViewer');
    await expect(page.getByText('コメント 39')).toBeVisible();

    await thanks.getByRole('button', { name: '閉じる' }).click();
    await expect(thanks).toHaveCount(0);
    await expect(flowedSupers).toContainText('RedDonor');

    // 手でスクロールして戻り、また見えたら欄から外れる。
    // (手でスクロールしたら自動スクロールを止めるのは段階 5.2c。今は表示の設定で止めてから戻る)
    await page.locator('label').filter({ hasText: '自動スクロール' }).locator('input[type="checkbox"]').uncheck();
    await page.getByText('コメント 39').hover();
    await page.mouse.wheel(0, -20000);
    await expect(page.getByText('遅れてきました！').first()).toBeVisible({ timeout: 5000 });
    await expect(flowedSupers).toHaveCount(0, { timeout: 5000 });
    await expect(flowedNewcomers).toHaveCount(0);

    await disconnectAndInitialize(page);
  });
});
