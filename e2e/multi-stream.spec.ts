import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  disconnectAndInitialize,
} from './utils/test-helpers';

/**
 * E2E tests for Multi-Stream Connection.
 *
 * 複数の配信に同時接続し、メッセージを受信・表示できることを確認する。
 * 対応仕様: ConnectionList, 個別切断, 全切断, 配信元インジケーター
 */

/**
 * ストリームに接続し、接続リストにエントリが追加されるのを待つ
 */
async function connectStream(page: Page, videoId: string, expectedTitle: string): Promise<void> {
  const urlInput = page.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
  // 入力欄をクリアしてから新しいURLを入力
  await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=${videoId}`);
  await page.locator('button:has-text("開始")').click();
  // 接続リストに該当タイトルが表示されるまで待機
  await expect(page.getByText(expectedTitle).first()).toBeVisible({ timeout: 10000 });
}

test.describe('Multi-Stream Connection', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000);

    log.info('Setting up test environment for Multi-Stream tests...');
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
    log.info('Connected to Tauri app');
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test.beforeEach(async () => {
    await resetMockServer();
    await disconnectAndInitialize(mainPage);
  });

  test.describe('同時接続', () => {
    test('2つの配信に同時接続できる', async () => {
      // Stream 1に接続
      await connectStream(mainPage, 'test_video_123', 'Mock Live');

      // 接続リストに1件表示されている
      await expect(mainPage.locator('.connection-item')).toHaveCount(1);

      // Stream 2に接続（既存接続を切断しない）
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 接続リストに2件表示されている
      await expect(mainPage.locator('.connection-item')).toHaveCount(2);

      // 両方のストリーム情報が表示されている
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible();
      await expect(mainPage.getByText('Mock Live 2').first()).toBeVisible();

      log.debug('2つの同時接続が確認された');
    });

    test('接続リストに各配信の情報（タイトル）が表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 各接続アイテムにストリームタイトルが表示される
      const items = mainPage.locator('.connection-item');
      await expect(items).toHaveCount(2);

      // .stream-title または .broadcaster-name のいずれかに期待テキストが含まれる
      const allText = await mainPage.locator('.connection-list').textContent();
      log.debug(`Connection list text: "${allText}"`);
      expect(allText).toContain('Mock Live');
      expect(allText).toContain('Mock Live 2');
    });

    test('各接続アイテムに個別切断ボタン（×）が表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 各接続アイテムに切断ボタンがある
      const disconnectBtns = mainPage.locator('.connection-item .disconnect-btn');
      await expect(disconnectBtns).toHaveCount(2);
    });
  });

  test.describe('メッセージ表示', () => {
    test('両方のストリームのメッセージが表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // Stream 1へのメッセージを追加
      const msg1Content = `Stream1Msg_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'Stream1User',
        content: msg1Content,
        channel_id: 'UC_stream1_user',
      });

      // メッセージが表示される
      await expect(mainPage.locator(`text=${msg1Content}`)).toBeVisible({ timeout: 8000 });

      // Stream 2へのメッセージを追加
      const msg2Content = `Stream2Msg_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'Stream2User',
        content: msg2Content,
        channel_id: 'UC_stream2_user',
      });

      // 両方のメッセージが表示される
      await expect(mainPage.locator(`text=${msg1Content}`)).toBeVisible({ timeout: 8000 });
      await expect(mainPage.locator(`text=${msg2Content}`)).toBeVisible({ timeout: 8000 });

      log.debug('両ストリームのメッセージが表示された');
    });

    test('2接続時にメッセージの配信元インジケーターが表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 接続が2件の場合、メッセージに配信元インジケーターが表示される
      const msg1Content = `IndicatorTest_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'IndicatorUser',
        content: msg1Content,
        channel_id: 'UC_indicator_user',
      });

      await expect(mainPage.locator(`text=${msg1Content}`)).toBeVisible({ timeout: 8000 });

      // 配信元インジケーター（色ライン or ラベル）が表示される
      // 2接続時は .source-indicator が表示される
      const messageEl = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${msg1Content}`),
      }).first();
      await expect(messageEl).toBeVisible();

      // .source-indicator の存在を確認
      const hasIndicator = await messageEl.locator('.source-indicator').count() > 0;
      log.debug(`Source indicator found: ${hasIndicator}`);
      // インジケーターが表示されることを確認（2接続時は必須）
      expect(hasIndicator).toBe(true);

      log.debug('配信元インジケーターの表示が確認された');
    });
  });

  test.describe('個別切断', () => {
    test('1つ目の×ボタンをクリックすると1件だけ切断される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      await expect(mainPage.locator('.connection-item')).toHaveCount(2);

      // 1つ目の接続の×ボタンをクリック
      const firstDisconnectBtn = mainPage.locator('.connection-item .disconnect-btn').first();
      await firstDisconnectBtn.click();

      // 接続リストが1件になる
      await expect(mainPage.locator('.connection-item')).toHaveCount(1, { timeout: 10000 });

      log.debug('1件の個別切断が確認された');
    });

    test('個別切断後も残りの接続が正常に機能する', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 1つ目の接続を切断
      const firstDisconnectBtn = mainPage.locator('.connection-item .disconnect-btn').first();
      await firstDisconnectBtn.click();
      await expect(mainPage.locator('.connection-item')).toHaveCount(1, { timeout: 10000 });

      // 残りの接続でメッセージを受信できる
      const remainingMsg = `RemainingStream_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'RemainingUser',
        content: remainingMsg,
        channel_id: 'UC_remaining_user',
      });

      await expect(mainPage.locator(`text=${remainingMsg}`)).toBeVisible({ timeout: 8000 });

      log.debug('個別切断後の残接続が正常に動作している');
    });
  });

  test.describe('全切断', () => {
    test('全切断ボタンで全ての接続が切断される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      await expect(mainPage.locator('.connection-item')).toHaveCount(2);

      // 全切断ボタンは2件以上の接続時に表示される
      const disconnectAllBtn = mainPage.locator('button:has-text("全切断")');
      await expect(disconnectAllBtn).toBeVisible({ timeout: 5000 });

      await disconnectAllBtn.click();

      // 接続リストが空になる
      await expect(mainPage.locator('.connection-item')).toHaveCount(0, { timeout: 10000 });

      log.debug('全切断が確認された');
    });

    test('1件の接続時は全切断ボタンが表示されない', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');

      await expect(mainPage.locator('.connection-item')).toHaveCount(1);

      // 1件の場合は全切断ボタンは表示されない
      const disconnectAllBtn = mainPage.locator('button:has-text("全切断")');
      await expect(disconnectAllBtn).not.toBeVisible();
    });
  });

  test.describe('接続状態の管理', () => {
    test('全切断後にURL入力フォームが引き続き表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      const disconnectAllBtn = mainPage.locator('button:has-text("全切断")');
      await disconnectAllBtn.click();
      await expect(mainPage.locator('.connection-item')).toHaveCount(0, { timeout: 10000 });

      // 全切断後もURLフォームは表示される
      await expect(mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]')).toBeVisible();
      await expect(mainPage.locator('button:has-text("開始")')).toBeVisible();
    });

    test('全切断後に再度接続できる', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      const disconnectAllBtn = mainPage.locator('button:has-text("全切断")');
      await disconnectAllBtn.click();
      await expect(mainPage.locator('.connection-item')).toHaveCount(0, { timeout: 10000 });

      // 再接続
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await expect(mainPage.locator('.connection-item')).toHaveCount(1);
    });
  });
});
