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
  navigateToTab,
  mockSignalerStatus,
  sendMockPushNotification,
} from './utils/test-helpers';

/**
 * プッシュ通知（02_chat.md「プッシュ通知」、ADR-008）
 * 待ちの計算・受信の解析・つなぎ直しの待ちは Rust の単体テストで仕様の例を確かめる。
 * ここでは、本番の接続処理が通知のサーバーにつなぎ、届いた通知で待ちを打ち切って取りに行くこと、
 * 設定の画面の切り替えが接続中の接続にすぐ効くことを実 Tauri で確かめる。
 */
test.describe('プッシュ通知 (02_chat.md)', () => {
  let browser: Browser;
  let mainPage: Page;

  const pushCheckbox = (page: Page) => page.getByLabel('プッシュ通知でコメントを早く取る（実験的）');

  async function openChatFetchSettings(page: Page): Promise<void> {
    await navigateToTab(page, 'Settings');
    await page.getByRole('button', { name: 'チャット取得' }).click();
    await expect(page.getByRole('heading', { name: 'チャット取得' })).toBeVisible();
  }

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
    // 設定はテストをまたいで残るので、既定（有効）に戻す
    await openChatFetchSettings(mainPage);
    if (!(await pushCheckbox(mainPage).isChecked())) {
      await pushCheckbox(mainPage).check();
    }
    await navigateToTab(mainPage, 'Chat');
    await resetMockServer();
    await disconnectAndInitialize(mainPage);
  });

  // 仕様: 通知が届いたら待ちを打ち切って取りに行き、リクエストに最後の通知の発行時刻を付ける
  // 発行時刻は通知を受けた liscov しか知らないので、届いていればプッシュ通知の経路が働いている
  test('通知が届くと待ちを打ち切って取りに行き、発行時刻を付ける', async () => {
    test.setTimeout(90000);
    await setStreamState({ chat_timeout_ms: 10000 });
    await connectToMockStream(mainPage, 'test_video_push');
    await expect.poll(async () => (await mockSignalerStatus()).open_receivers, { timeout: 10000 }).toBe(1);

    // 空振りを重ねて、ポーリングの待ちが長い（5 秒以上）状態にする
    await mainPage.waitForTimeout(20000);
    expect((await mockSignalerStatus()).last_publish_at).toBeNull();

    await addMockMessage({ message_type: 'text', author: 'PushUser', content: '通知で取るコメント' });
    const started = Date.now();
    const publishAt = await sendMockPushNotification();

    await expect.poll(async () => (await mockSignalerStatus()).last_publish_at, { timeout: 3000 }).toBe(publishAt);
    await expect(chatRow(mainPage, '通知で取るコメント')).toBeVisible({ timeout: 3000 });
    // 前回の取得から 1.5 秒は空けるので最大 1.5 秒＋画面に出るまで。打ち切らなければ 5 秒以上かかる
    expect(Date.now() - started).toBeLessThan(3500);
  });

  // 仕様: 接続中に設定を無効にすると通知の受信をすぐ止め、有効にすると受け始める（再接続は不要）
  test('設定の画面で切り替えると、接続中の接続にすぐ効く', async () => {
    test.setTimeout(90000);
    await connectToMockStream(mainPage, 'test_video_push_toggle');
    await expect.poll(async () => (await mockSignalerStatus()).open_receivers, { timeout: 10000 }).toBe(1);
    const opened = (await mockSignalerStatus()).choose_server_count;

    await openChatFetchSettings(mainPage);
    await expect(pushCheckbox(mainPage)).toBeChecked();
    await pushCheckbox(mainPage).uncheck();
    await expect.poll(async () => (await mockSignalerStatus()).open_receivers, { timeout: 10000 }).toBe(0);

    // 無効の間は通知を受けないので、発行時刻も付かない（取得はポーリングで続く）
    await sendMockPushNotification();
    await mainPage.waitForTimeout(3000);
    expect((await mockSignalerStatus()).last_publish_at).toBeNull();

    await pushCheckbox(mainPage).check();
    await expect.poll(async () => (await mockSignalerStatus()).open_receivers, { timeout: 10000 }).toBe(1);
    expect((await mockSignalerStatus()).choose_server_count).toBeGreaterThan(opened);
  });
});
