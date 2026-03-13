import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  TEST_APP_NAME,
  TEST_KEYRING_SERVICE,
  cleanupTestData,
  killTauriApp,
  killMockServer,
  startMockServer,
  startTauriAppWithEnv,
  connectToApp,
  disconnectAndInitialize,
} from './utils/test-helpers';

/**
 * E2Eテスト: 切断/再接続後のアプリケーションロジックフリーズ検出
 *
 * 元のバグ症状:
 * - 再開（resume）後に全アプリケーションロジックが停止
 * - DOMインタラクション（スクロール、ドロップダウン）は動作する
 * - ボタンのクリックハンドラが発火しない
 * - タブ切り替えが動作しない
 *
 * 多接続リファクタリング後: 停止/再開 → 切断/再接続に変更
 */

// 実YouTube向け（モックなし）でTauriアプリを起動する
async function startTauriAppForRealYouTube(): Promise<void> {
  await startTauriAppWithEnv({
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
  });
}

// モックサーバー向けでTauriアプリを起動する
async function startTauriAppForMockServer(): Promise<void> {
  await startTauriAppWithEnv({
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
  });
}

/**
 * アプリケーションロジックが動作しているかテスト
 * タブ切り替えで検証（Settings → Chat → Settings）
 */
async function testApplicationLogicWorks(page: Page): Promise<{ works: boolean; details: string }> {
  const settingsHeading = page.getByRole('heading', { name: 'YouTube認証' });

  // JS実行テスト
  const jsTestBefore = await page.evaluate(() => {
    return { works: true, timestamp: Date.now() };
  }).catch((e) => ({ works: false, error: String(e) }));
  log.debug(`JS execution test before tab click: ${JSON.stringify(jsTestBefore)}`);

  // Settingsタブをクリック
  log.debug('Clicking Settings tab...');
  await page.locator('button:has-text("Settings")').click();

  // Settings内容が表示されているか確認
  const settingsVisibleAfter = await settingsHeading.isVisible().catch(() => false);

  if (!settingsVisibleAfter) {
    const activeTabButton = await page.locator('button:has-text("Settings")').evaluate((el) => {
      const style = window.getComputedStyle(el);
      return {
        background: style.background.substring(0, 100),
        fontWeight: style.fontWeight
      };
    }).catch(() => ({ error: 'Failed to get button style' }));
    log.debug(`Settings tab button state: ${JSON.stringify(activeTabButton)}`);

    return {
      works: false,
      details: `Tab click did not change content. Tab button state: ${JSON.stringify(activeTabButton)}`
    };
  }

  // Chatタブに戻る
  await page.locator('button:has-text("Chat")').click();

  const settingsVisibleAfterChat = await settingsHeading.isVisible().catch(() => false);
  if (settingsVisibleAfterChat) {
    return {
      works: false,
      details: 'Tab click back to Chat did not change content (Settings still visible)'
    };
  }

  return { works: true, details: 'Application logic is working' };
}

/**
 * 接続を切断する（個別切断ボタン使用、メッセージはクリアしない）
 */
async function disconnectStream(page: Page): Promise<void> {
  const disconnectBtn = page.locator('.connection-item .disconnect-btn').first();
  if (await disconnectBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await disconnectBtn.click();
    await expect(page.locator('.connection-item')).toHaveCount(0, { timeout: 10000 });
  }
}

/**
 * ストリームに接続する
 */
async function connectToStream(page: Page, url: string, timeout = 30000): Promise<void> {
  const urlInput = page.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
  await urlInput.fill(url);
  await page.locator('button:has-text("開始")').click();
  await expect(page.locator('.connection-item').first()).toBeVisible({ timeout });
}

// Real YouTube test
test.describe('Real YouTube - Application Logic Freeze Detection', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  const YOUTUBE_URL = 'https://www.youtube.com/watch?v=jfKfPfyJRdk'; // lofi girl

  test.beforeAll(async () => {
    test.setTimeout(300000);

    await killTauriApp();
    await cleanupTestData();

    log.info('Starting Tauri app for real YouTube...');
    await startTauriAppForRealYouTube();

    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    mainPage.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('[GLOBAL_ERROR]') || text.includes('[UNHANDLED_REJECTION]')) {
        log.error(`[CRITICAL] ${text}`);
      } else if (text.includes('error') || text.includes('Error')) {
        log.debug(`[CONSOLE] ${text}`);
      }
    });

    await mainPage.waitForLoadState('domcontentloaded');
    await mainPage.waitForTimeout(2000);
    log.info('Connected to Tauri app');
  });

  test.afterAll(async () => {
    await killTauriApp();
    await cleanupTestData();
  });

  test('should detect application logic freeze after disconnect/reconnect with real YouTube', async () => {
    // Step 1: 接続
    log.info(`Connecting to: ${YOUTUBE_URL}`);
    await connectToStream(mainPage, YOUTUBE_URL, 30000);
    log.info('Connected to stream');

    // Step 2: 接続前のロジック確認
    log.info('Testing application logic BEFORE disconnect...');
    const beforeDisconnect = await testApplicationLogicWorks(mainPage);
    log.info(`Before disconnect: ${beforeDisconnect.details}`);
    expect(beforeDisconnect.works).toBe(true);

    // Step 3: 安定接続を待つ
    log.info('Waiting 5 seconds for stable connection...');
    await mainPage.waitForTimeout(5000);

    // Step 4: 切断
    log.info('Disconnecting...');
    await disconnectStream(mainPage);
    log.info('Disconnected');

    // Step 5: 切断中に待機
    log.info('Waiting 3 seconds while disconnected...');
    await mainPage.waitForTimeout(3000);

    // Step 6: 再接続
    log.info('Reconnecting...');
    await connectToStream(mainPage, YOUTUBE_URL, 30000);
    log.info('Reconnected');

    // Step 7: 再接続後のロジック確認
    log.info('Testing application logic AFTER reconnect...');
    await mainPage.waitForTimeout(1000);

    const afterReconnect = await testApplicationLogicWorks(mainPage);
    log.info(`After reconnect: ${afterReconnect.details}`);

    if (!afterReconnect.works) {
      await mainPage.screenshot({ path: 'logic-freeze-real-youtube.png' });
      throw new Error(`BUG DETECTED: Application logic frozen after reconnect. ${afterReconnect.details}`);
    }

    log.info('Test passed: Application logic works after reconnect');

    // クリーンアップ
    await disconnectAndInitialize(mainPage);
  });
});

// Mock Server test
test.describe('Mock Server - Application Logic Freeze Detection', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000);

    await killTauriApp();
    await killMockServer();
    await cleanupTestData();

    log.info('Starting mock server...');
    await startMockServer();

    log.info('Starting Tauri app for mock server...');
    await startTauriAppForMockServer();

    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    mainPage.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('[GLOBAL_ERROR]') || text.includes('[UNHANDLED_REJECTION]')) {
        log.error(`[CRITICAL] ${text}`);
      } else if (text.includes('error') || text.includes('Error')) {
        log.debug(`[CONSOLE] ${text}`);
      }
    });

    await mainPage.waitForLoadState('domcontentloaded');
    await mainPage.waitForTimeout(2000);
    log.info('Connected to Tauri app');
  });

  test.afterAll(async () => {
    await killTauriApp();
    await killMockServer();
    await cleanupTestData();
  });

  test('should detect application logic freeze after disconnect/reconnect with mock server', async () => {
    const streamUrl = `${MOCK_SERVER_URL}/watch?v=test_logic_freeze`;

    // 自動メッセージを有効化（実YouTube相当の流量）
    log.info('Enabling auto-messages (20 per poll to match real YouTube)...');
    await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: true, messages_per_poll: 20 }),
    });

    // Step 1: 接続
    log.info('Connecting to mock stream...');
    await connectToStream(mainPage, streamUrl);
    log.info('Connected to mock stream');

    // Step 2: 切断前のロジック確認
    log.info('Testing application logic BEFORE disconnect...');
    const beforeDisconnect = await testApplicationLogicWorks(mainPage);
    log.info(`Before disconnect: ${beforeDisconnect.details}`);
    expect(beforeDisconnect.works).toBe(true);

    // Step 3: メッセージ受信を待つ
    log.info('Waiting 5 seconds for messages...');
    await mainPage.waitForTimeout(5000);

    const msgCountBefore = await mainPage.locator('[data-message-id]').count();
    log.debug(`Messages before disconnect: ${msgCountBefore}`);

    // Step 4: 切断
    log.info('Disconnecting...');
    await disconnectStream(mainPage);
    log.info('Disconnected');

    // Step 5: 切断中に待機（サーバーサイドではメッセージ生成が継続）
    log.info('Waiting 3 seconds while disconnected...');
    await mainPage.waitForTimeout(3000);

    // Step 6: 再接続
    log.info('Reconnecting...');
    await connectToStream(mainPage, streamUrl);
    log.info('Reconnected');

    // Step 7: 再接続後のロジック確認
    log.info('Testing application logic AFTER reconnect...');
    await mainPage.waitForTimeout(1000);

    const afterReconnect = await testApplicationLogicWorks(mainPage);
    log.info(`After reconnect: ${afterReconnect.details}`);

    if (!afterReconnect.works) {
      await mainPage.screenshot({ path: 'logic-freeze-mock-server.png' });
      await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: false }),
      });
      throw new Error(`BUG DETECTED: Application logic frozen after reconnect. ${afterReconnect.details}`);
    }

    log.info('Test passed: Application logic works after reconnect');

    // クリーンアップ
    await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: false }),
    });
    await disconnectAndInitialize(mainPage);
  });

  test('should detect application logic freeze with network delay simulation', async () => {
    const streamUrl = `${MOCK_SERVER_URL}/watch?v=test_delay`;

    // ネットワーク遅延シミュレーション
    log.info('Configuring network delay simulation...');
    await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ watch_delay_ms: 500, chat_delay_ms: 200 }),
    });

    await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: true, messages_per_poll: 10 }),
    });

    // Step 1: 接続（遅延あり）
    log.info('Connecting to mock stream with delay...');
    await connectToStream(mainPage, streamUrl, 15000);
    log.info('Connected');

    // Step 2: 切断前のロジック確認
    const beforeDisconnect = await testApplicationLogicWorks(mainPage);
    log.info(`Before disconnect: ${beforeDisconnect.details}`);
    expect(beforeDisconnect.works).toBe(true);

    // Step 3: メッセージ受信を待つ
    log.info('Waiting 5 seconds for messages...');
    await mainPage.waitForTimeout(5000);

    // Step 4: 切断
    log.info('Disconnecting...');
    await disconnectStream(mainPage);
    log.info('Disconnected');

    // Step 5: 待機
    await mainPage.waitForTimeout(3000);

    // Step 6: 再接続（遅延あり）
    log.info('Reconnecting with network delay...');
    await connectToStream(mainPage, streamUrl, 15000);
    log.info('Reconnected');

    // Step 7: 再接続後のロジック確認
    log.info('Testing application logic AFTER reconnect with delay...');
    await mainPage.waitForTimeout(1000);

    const afterReconnect = await testApplicationLogicWorks(mainPage);
    log.info(`After reconnect with delay: ${afterReconnect.details}`);

    // 遅延設定をリセット
    await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ watch_delay_ms: 0, chat_delay_ms: 0 }),
    });
    await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: false }),
    });

    if (!afterReconnect.works) {
      await mainPage.screenshot({ path: 'logic-freeze-with-delay.png' });
      throw new Error(`BUG DETECTED: Application logic frozen after reconnect with delay. ${afterReconnect.details}`);
    }

    log.info('Test passed: Application logic works after reconnect with delay');
    await disconnectAndInitialize(mainPage);
  });

  test('should detect logic freeze with multiple disconnect/reconnect cycles', async () => {
    const streamUrl = `${MOCK_SERVER_URL}/watch?v=test_multi_cycle`;

    // 接続
    await connectToStream(mainPage, streamUrl);

    // 3回の切断/再接続サイクル
    for (let i = 0; i < 3; i++) {
      log.info(`Cycle ${i + 1}/3: Disconnecting...`);
      await disconnectStream(mainPage);

      await mainPage.waitForTimeout(1000);

      log.info(`Cycle ${i + 1}/3: Reconnecting...`);
      await connectToStream(mainPage, streamUrl);

      // 再接続後のロジック確認
      const result = await testApplicationLogicWorks(mainPage);
      log.info(`Cycle ${i + 1}/3: ${result.details}`);

      if (!result.works) {
        await mainPage.screenshot({ path: `logic-freeze-cycle-${i + 1}.png` });
        throw new Error(`BUG DETECTED at cycle ${i + 1}: ${result.details}`);
      }
    }

    log.info('All 3 cycles passed');
    await disconnectAndInitialize(mainPage);
  });
});
