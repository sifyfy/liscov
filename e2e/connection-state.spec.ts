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
  openUrlInput,
} from './utils/test-helpers';

/**
 * E2E tests for connection state transitions based on 02_chat.md specification.
 *
 * 多接続UIへの対応:
 * - 停止/再開/初期化の概念はなくなった
 * - 接続中でも「+」で操作パネルを開けば、新しい接続を追加できる (URL フォームは操作パネルの中)
 * - 切断は上の行の接続の札の×ボタンで行う
 * - 接続状態は上の行の接続の札 (`connection-chip`) の存在で確認する
 */

test.describe('Connection State Transitions (02_chat.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000);

    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    await mainPage.waitForLoadState('domcontentloaded');
    // Wait for SvelteKit app to render
    await expect(mainPage.locator('nav button[aria-label="チャット"]')).toBeVisible({ timeout: 15000 });
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  // 状態の行の件数 (「1,820 件」。VList は DOM を間引くので、要素数ではなくこの値で数える)
  async function statusCount(): Promise<number> {
    const text = await mainPage.getByText(/^[\d,]+ 件$/).textContent();
    return parseInt((text ?? '0').replace(/[^\d]/g, ''), 10);
  }

  test.beforeEach(async () => {
    await resetMockServer();
    await disconnectAndInitialize(mainPage);
  });

  test.describe('Connected State (接続中)', () => {
    test('接続するとストリームタイトルが接続リストに表示される', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();

      // 接続リストにエントリが追加される
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });

      // ストリームタイトルが表示される
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);
    });

    test('接続中はURL入力フォームが引き続き表示される', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // 多接続UIでは接続中でも、操作パネルを開けば (「+」) URLフォームが使える
      await expect(await openUrlInput(mainPage)).toBeVisible();
      await expect(mainPage.getByRole('button', { name: '接続', exact: true })).toBeVisible();

      await disconnectAndInitialize(mainPage);
    });

    test('接続中はメッセージが受信できる', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      const uniqueContent = `ConnectTest_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'ConnectTestUser',
        content: uniqueContent,
        channel_id: 'UC_connect_test',
      });

      await expect(mainPage.getByText(uniqueContent)).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Disconnect (切断)', () => {
    test('個別切断ボタンで接続が切断される', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });

      // 個別切断ボタンをクリック
      const disconnectBtn = mainPage.locator('[data-testid="connection-chip"] button').first();
      await expect(disconnectBtn).toBeVisible();
      await disconnectBtn.click();

      // 接続リストが空になる
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0, { timeout: 10000 });
    });

    test('切断後もURL入力フォームは表示されている', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);

      // 切断後もURLフォームは表示される (コメントを消したので流れの場所に出る)
      await expect(mainPage.locator('#connect-url')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.getByRole('button', { name: '接続', exact: true })).toBeVisible();
    });

    test('切断後にメッセージが保持される', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      const uniqueContent = `PersistMsg_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'PersistUser',
        content: uniqueContent,
        channel_id: 'UC_persist_test',
      });

      await expect(mainPage.getByText(uniqueContent)).toBeVisible({ timeout: 5000 });
      const messageCountBefore = await mainPage.locator('[data-message-id]').count();

      // 切断のみ（disconnectAndInitializeはメッセージもクリアするため使わない）
      const disconnectBtn = mainPage.locator('[data-testid="connection-chip"] button').first();
      await disconnectBtn.click();
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0, { timeout: 10000 });

      // メッセージは保持される（クリアされない）
      const messageCountAfter = await mainPage.locator('[data-message-id]').count();
      expect(messageCountAfter).toBe(messageCountBefore);

      // テスト後クリーンアップ（次のテストのために状態をリセット）
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Reconnect (再接続)', () => {
    test('切断後に同じURLで再接続できる', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);

      // 再接続
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);
    });

    test('再接続後にメッセージを受信できる', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);

      // 再接続
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });

      const uniqueContent = `ReconnectTest_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'ReconnectTestUser',
        content: uniqueContent,
        channel_id: 'UC_reconnect_test',
      });

      await expect(mainPage.getByText(uniqueContent)).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('State Transitions (状態遷移)', () => {
    test('アイドル状態: URLフォームと接続ボタンが表示される', async () => {
      // 初期状態: 接続なし
      await expect(mainPage.locator('#connect-url')).toBeVisible();
      await expect(mainPage.getByRole('button', { name: '接続', exact: true })).toBeVisible();
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0);
    });

    test('接続中 → 切断 → 再接続の状態遷移が正しく動作する', async () => {
      // アイドル状態
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0);

      // 接続
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();

      // 接続中状態
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(1, { timeout: 10000 });

      // 切断
      await disconnectAndInitialize(mainPage);

      // アイドル状態に戻る
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0, { timeout: 5000 });

      // 再接続
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();

      // 再び接続中状態
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(1, { timeout: 10000 });

      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Auto-Message Continuous Flow (実際のYouTubeシミュレーション)', () => {
    async function enableAutoMessages(messagesPerPoll: number = 10): Promise<void> {
      await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: true, messages_per_poll: messagesPerPoll }),
      });
    }

    async function disableAutoMessages(): Promise<void> {
      await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: false }),
      });
    }

    async function getAutoMessageStatus(): Promise<{ enabled: boolean; total_generated: number }> {
      const response = await fetch(`${MOCK_SERVER_URL}/auto_message_status`);
      return response.json();
    }

    test('自動生成メッセージが連続して受信される', async () => {
      await enableAutoMessages(10);

      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_auto_msg`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });

      log.debug('Waiting for auto-generated messages...');
      await mainPage.waitForTimeout(5000);

      const messageCountBefore = await statusCount();
      log.debug(`Messages after waiting (status bar): ${messageCountBefore}`);
      expect(messageCountBefore).toBeGreaterThanOrEqual(10);

      const statusAfterCheck = await getAutoMessageStatus();
      log.debug(`Auto-message total generated: ${statusAfterCheck.total_generated}`);
      expect(statusAfterCheck.total_generated).toBeGreaterThan(0);

      await disableAutoMessages();
      await disconnectAndInitialize(mainPage);
    });

    test('切断後に再接続すると新しいメッセージが受信できる', async () => {
      await enableAutoMessages(5);

      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_reconnect_auto`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });

      await mainPage.waitForTimeout(3000);

      const countBefore = await statusCount();
      log.debug(`Messages before disconnect: ${countBefore}`);

      // 切断（disconnectAndInitializeはメッセージもクリアする）
      await disconnectAndInitialize(mainPage);

      // 再接続
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_reconnect_auto`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });

      await mainPage.waitForTimeout(5000);

      // 新しいメッセージが届いている（disconnectAndInitializeでクリア済みなので0から再開）
      const countAfter = await statusCount();
      log.debug(`Messages after reconnect: ${countAfter}`);
      expect(countAfter).toBeGreaterThan(0);

      await disableAutoMessages();
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('High Volume (UIフリーズ回避)', () => {
    test('大量メッセージ受信中もUIがフリーズしない', async () => {
      const urlInput = await openUrlInput(mainPage);
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]').first()).toBeVisible({ timeout: 10000 });

      log.debug('Sending 500 messages...');
      const messagePromises = [];
      for (let i = 0; i < 500; i++) {
        const msgType = i % 20 === 0 ? 'superchat' : i % 10 === 0 ? 'membership' : 'text';
        messagePromises.push(
          addMockMessage({
            message_type: msgType,
            author: `User${i % 50}`,
            content: `Pre-cut message ${i} with some additional text to make it longer and more realistic like actual YouTube chat messages`,
            channel_id: `UC_user_${i % 50}`,
            is_member: i % 5 === 0,
            amount: msgType === 'superchat' ? '¥500' : undefined,
          })
        );
      }
      await Promise.all(messagePromises);

      await mainPage.waitForTimeout(8000);

      const messageCount = await statusCount();
      log.debug(`Messages in UI (status bar): ${messageCount}`);
      expect(messageCount).toBeGreaterThan(100);

      // タブ切替でUIフリーズを検出
      const settingsTab = mainPage.locator('nav button[aria-label="設定"]');
      const interactionStart = Date.now();
      const freezeTimeout = 3000;

      try {
        await Promise.race([
          (async () => {
            await settingsTab.click({ timeout: freezeTimeout });
            await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible({ timeout: 1000 });
          })(),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error('UI FREEZE DETECTED: Tab click not processed')), freezeTimeout)
          ),
        ]);
      } catch (error) {
        const elapsed = Date.now() - interactionStart;
        log.error(`UI freeze detected after ${elapsed}ms`);
        throw error;
      }

      const interactionDuration = Date.now() - interactionStart;
      log.debug(`Tab switch completed in ${interactionDuration}ms`);
      expect(interactionDuration).toBeLessThan(1000);

      await mainPage.locator('nav button[aria-label="チャット"]').click();
      await mainPage.waitForTimeout(500);

      await disconnectAndInitialize(mainPage);
    });
  });
});
