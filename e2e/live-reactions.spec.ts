import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockReaction,
  disconnectAndInitialize,
} from './utils/test-helpers';

/**
 * E2E tests for ライブリアクション（02_chat.md「ライブリアクション」「リアクションメーター」）
 * モックの emojiFountainDataEntity → パース → DB → chat:reaction → リアクションメーター
 */
test.describe('ライブリアクション (02_chat.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000);
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test.beforeEach(async () => {
    await resetMockServer();
  });

  async function connect(videoId: string, url = `${MOCK_SERVER_URL}/watch?v=${videoId}`) {
    const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
    await urlInput.fill(url);
    await mainPage.locator('button:has-text("開始")').click();
    await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });
  }

  function total(emoji: string) {
    return mainPage.locator(`[data-testid="reaction-total"][data-emoji="${emoji}"]`);
  }

  const meter = () => mainPage.locator('[data-testid="reaction-meter"]');
  const perMinute = () => mainPage.locator('[data-testid="reaction-per-minute"]');

  // 仕様例: 🎉4（2秒）→ ❤3 → 累計 ❤ 3・🎉 4（多い順）、勢い 7/分
  test('リアクションを絵文字別の累計と勢いで表示する', async () => {
    await connect('test_video_reactions_meter');
    await expect(meter()).toHaveCount(0);

    await addMockReaction({ '🎉': 4 }, 2);
    await expect(total('🎉')).toHaveText('🎉 4', { timeout: 5000 });
    await addMockReaction({ '❤': 3 });
    await expect(total('❤')).toHaveText('❤ 3', { timeout: 5000 });

    await expect(mainPage.locator('[data-testid="reaction-total"]')).toHaveText(['🎉 4', '❤ 3']);
    await expect(perMinute()).toHaveText('7/分');

    await disconnectAndInitialize(mainPage);
    await expect(meter()).toHaveCount(0);
  });

  // 仕様: F5リロード → 累計と直近60秒の件数を get_connection_reactions で読み直す
  test('F5 リロード後もメーターを復元する', async () => {
    await connect('test_video_reactions_reload');
    await addMockReaction({ '😄': 2, '💯': 1 });
    await expect(total('😄')).toHaveText('😄 2', { timeout: 5000 });

    await mainPage.reload();
    await expect(mainPage.locator('nav button:has-text("Chat")')).toBeVisible({ timeout: 30000 });

    await expect(total('😄')).toHaveText('😄 2', { timeout: 10000 });
    await expect(total('💯')).toHaveText('💯 1');
    await expect(perMinute()).toHaveText('3/分');

    await disconnectAndInitialize(mainPage);
  });

  // 仕様: 同じ配信に再接続 → 累計は DB の続きから
  test('同じ配信に再接続したら累計を続きから数える', async () => {
    await connect('test_video_reactions_reconnect');
    await addMockReaction({ '❤': 5 });
    await expect(total('❤')).toHaveText('❤ 5', { timeout: 5000 });
    await disconnectAndInitialize(mainPage);

    await connect('test_video_reactions_reconnect');
    await expect(total('❤')).toHaveText('❤ 5', { timeout: 10000 });
    await addMockReaction({ '❤': 2 });
    await expect(total('❤')).toHaveText('❤ 7', { timeout: 5000 });

    await disconnectAndInitialize(mainPage);
  });

  // 仕様: 同じ配信なら URL の書き方（/live/ と watch?v=）が違っても累計は続きから（08_database.md sessions.video_id）
  test('/live/ の URL で接続した配信に watch?v= で再接続しても累計を続きから数える', async () => {
    const videoId = 'test_video_reactions_live_url';
    await connect(videoId, `${MOCK_SERVER_URL}/live/${videoId}`);
    await addMockReaction({ '❤': 5 });
    await expect(total('❤')).toHaveText('❤ 5', { timeout: 5000 });
    await disconnectAndInitialize(mainPage);

    await connect(videoId);
    await expect(total('❤')).toHaveText('❤ 5', { timeout: 10000 });

    await disconnectAndInitialize(mainPage);
  });
});
