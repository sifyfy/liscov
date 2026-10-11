import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import {
  mockServerUrl,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockReaction,
  disconnectAndInitialize,
  openUrlInput,
  APP_READY,
} from './utils/test-helpers';

/**
 * E2E tests for ライブリアクション（02_chat.md「ライブリアクション」「状態の行のリアクション」）
 * モックの emojiFountainDataEntity → パース → DB → chat:reaction → 状態の行のリアクション
 * (主の配信の累計だけを、多い順に 3 つまで出す。勢い (/分) は出さない)
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

  async function connect(videoId: string, url = `${mockServerUrl()}/watch?v=${videoId}`) {
    const urlInput = (await openUrlInput(mainPage));
    await urlInput.fill(url);
    await mainPage.getByRole('button', { name: '接続', exact: true }).click();
    await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });
  }

  // 状態の行のリアクション (絵文字ごとの「🎉 4」)。多い順
  const statusReactions = () => mainPage.getByTestId('status-reactions');
  const reactionItems = () => statusReactions().locator('> span');
  function total(emoji: string) {
    return reactionItems().filter({ hasText: emoji });
  }

  // 仕様例: 🎉4 → ❤3 → 状態の行に `🎉 4  ❤ 3`（多い順）
  test('リアクションを絵文字別の累計で状態の行に出す', async () => {
    await connect('test_video_reactions_meter');
    // 1 件も無いうちは出さない
    await expect(statusReactions()).toHaveCount(0);

    await addMockReaction({ '🎉': 4 }, 2);
    await expect(total('🎉')).toHaveText('🎉 4', { timeout: 5000 });
    await addMockReaction({ '❤': 3 });
    await expect(total('❤')).toHaveText('❤ 3', { timeout: 5000 });

    await expect(reactionItems()).toHaveText(['🎉 4', '❤ 3']);

    // 切断して主の配信が無くなったら出さない
    await disconnectAndInitialize(mainPage);
    await expect(statusReactions()).toHaveCount(0);
  });

  // 仕様例: ❤430・🎉418・😄94・💯56 → 状態の行は多い 3 つ。4 つ目以降もヒントと読み上げ用の名前に全部出す
  test('4 種類以上は多い順に 3 つまで出し、全部をヒントと読み上げ用の名前に出す', async () => {
    await connect('test_video_reactions_top3');
    await addMockReaction({ '❤': 43, '🎉': 41, '😄': 9, '💯': 5 });
    await expect(reactionItems()).toHaveCount(3, { timeout: 5000 });

    await expect(reactionItems()).toHaveText(['❤ 43', '🎉 41', '😄 9']);
    const all = 'リアクション ❤ 43、🎉 41、😄 9、💯 5';
    await expect(statusReactions()).toHaveAttribute('title', all);
    await expect(statusReactions()).toHaveAttribute('aria-label', all);

    await disconnectAndInitialize(mainPage);
  });

  // 仕様: F5リロード → 累計を get_connection_reactions で読み直す
  test('F5 リロード後も状態の行のリアクションを復元する', async () => {
    await connect('test_video_reactions_reload');
    await addMockReaction({ '😄': 2, '💯': 1 });
    await expect(total('😄')).toHaveText('😄 2', { timeout: 5000 });

    await mainPage.reload();
    await expect(mainPage.locator(APP_READY)).toBeVisible({ timeout: 30000 });

    await expect(total('😄')).toHaveText('😄 2', { timeout: 10000 });
    await expect(total('💯')).toHaveText('💯 1');

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
    await connect(videoId, `${mockServerUrl()}/live/${videoId}`);
    await addMockReaction({ '❤': 5 });
    await expect(total('❤')).toHaveText('❤ 5', { timeout: 5000 });
    await disconnectAndInitialize(mainPage);

    await connect(videoId);
    await expect(total('❤')).toHaveText('❤ 5', { timeout: 10000 });

    await disconnectAndInitialize(mainPage);
  });
});
