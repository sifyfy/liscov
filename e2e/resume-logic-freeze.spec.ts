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
} from './utils/test-helpers';

/**
 * E2E test to detect the "application logic freeze" bug
 *
 * Bug symptoms:
 * - After resume, ALL application logic stops working
 * - DOM interactions (scroll, dropdown visual) still work
 * - But button click handlers don't fire
 * - Tab switching doesn't work (content doesn't change)
 * - Viewer info panel can't be opened
 *
 * This is DIFFERENT from high-rate UI freeze (which was fixed in fcfa476)
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
 * Test that application logic works after resume
 * Returns true if logic is working, false if frozen
 */
async function testApplicationLogicWorks(page: Page): Promise<{ works: boolean; details: string }> {
  // Test 1: Tab switching - click Settings tab and verify content changes
  const chatTabContent = page.locator('[data-message-id]').first();
  const settingsHeading = page.getByRole('heading', { name: 'YouTube認証' });

  // Verify we're on Chat tab (or at least Settings content is not visible)
  const settingsVisibleBefore = await settingsHeading.isVisible().catch(() => false);

  // Check JavaScript execution before clicking
  const jsTestBefore = await page.evaluate(() => {
    const testVar = Date.now();
    return { works: true, timestamp: testVar };
  }).catch((e) => ({ works: false, error: String(e) }));
  log.debug(`JS execution test before tab click: ${JSON.stringify(jsTestBefore)}`);

  // Click Settings tab
  log.debug('Clicking Settings tab...');
  await page.locator('button:has-text("Settings")').click();

  // Check if click handler was registered by testing JS execution
  const jsTestAfter = await page.evaluate(() => {
    const testVar = Date.now();
    // Try to read Svelte state through window
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const anyWindow = window as any;
    return {
      works: true,
      timestamp: testVar,
      hasPendingMicrotasks: typeof anyWindow.queueMicrotask === 'function'
    };
  }).catch((e) => ({ works: false, error: String(e) }));
  log.debug(`JS execution test after tab click: ${JSON.stringify(jsTestAfter)}`);

  // Check if Settings content is now visible
  const settingsVisibleAfter = await settingsHeading.isVisible().catch(() => false);

  if (!settingsVisibleAfter) {
    // Additional diagnostics
    const activeTabButton = await page.locator('button:has-text("Settings")').evaluate((el) => {
      const style = window.getComputedStyle(el);
      return {
        isActive: style.fontWeight === '700' || style.background.includes('rgba(255, 255, 255'),
        background: style.background.substring(0, 100),
        fontWeight: style.fontWeight
      };
    }).catch(() => ({ error: 'Failed to get button style' }));
    log.debug(`Settings tab button state: ${JSON.stringify(activeTabButton)}`);

    return {
      works: false,
      details: `Tab click did not change content (Settings heading not visible after clicking Settings tab). Tab button state: ${JSON.stringify(activeTabButton)}`
    };
  }

  // Test 2: Click back to Chat tab
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

// Real YouTube test
test.describe('Real YouTube - Application Logic Freeze Detection', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  // Use a low-activity stream to rule out high-rate issues
  const YOUTUBE_URL = 'https://www.youtube.com/watch?v=jfKfPfyJRdk'; // lofi girl - usually stable

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

    // Capture ALL console logs for debugging
    mainPage.on('console', (msg) => {
      const text = msg.text();
      // Always capture errors and specific debug logs
      if (text.includes('[GLOBAL_ERROR]') || text.includes('[UNHANDLED_REJECTION]')) {
        log.error(`[CRITICAL] ${text}`);
      } else if (text.includes('[chat.svelte.ts]') || text.includes('resume') || text.includes('error') || text.includes('Error')) {
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

  test('should detect application logic freeze after pause/resume with real YouTube', async () => {
    // Step 1: Connect to YouTube stream
    log.info(`Connecting to: ${YOUTUBE_URL}`);
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
    await urlInput.fill(YOUTUBE_URL);
    await mainPage.locator('button:has-text("開始")').click();

    // Wait for connection
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 30000 });
    log.info('Connected to stream');

    // Step 2: Verify application logic works BEFORE pause
    log.info('Testing application logic BEFORE pause...');
    const beforePause = await testApplicationLogicWorks(mainPage);
    log.info(`Before pause: ${beforePause.details}`);
    expect(beforePause.works).toBe(true);

    // Step 3: Wait a bit to ensure stable connection
    log.info('Waiting 5 seconds for stable connection...');
    await mainPage.waitForTimeout(5000);

    // Step 4: Pause
    log.info('Pausing...');
    await mainPage.locator('button:has-text("停止")').click();
    await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });
    log.info('Paused');

    // Step 5: Wait while paused
    log.info('Waiting 3 seconds while paused...');
    await mainPage.waitForTimeout(3000);

    // Step 6: Resume
    log.info('Resuming...');
    await mainPage.locator('button:has-text("再開")').click();
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });
    log.info('Resume completed');

    // Step 7: Test application logic AFTER resume
    log.info('Testing application logic AFTER resume...');

    // Give a moment for any potential issue to manifest
    await mainPage.waitForTimeout(1000);

    const afterResume = await testApplicationLogicWorks(mainPage);
    log.info(`After resume: ${afterResume.details}`);

    if (!afterResume.works) {
      // Take screenshot for debugging
      await mainPage.screenshot({ path: 'logic-freeze-real-youtube.png' });
      throw new Error(`BUG DETECTED: Application logic frozen after resume. ${afterResume.details}`);
    }

    log.info('Test passed: Application logic works after resume');

    // Cleanup
    try {
      await mainPage.locator('button:has-text("停止")').click({ timeout: 2000 });
      await mainPage.locator('button:has-text("初期化")').click({ timeout: 2000 });
    } catch {
      log.debug('Cleanup skipped');
    }
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

    // Capture ALL console logs for debugging
    mainPage.on('console', (msg) => {
      const text = msg.text();
      // Always capture errors and specific debug logs
      if (text.includes('[GLOBAL_ERROR]') || text.includes('[UNHANDLED_REJECTION]')) {
        log.error(`[CRITICAL] ${text}`);
      } else if (text.includes('[chat.svelte.ts]') || text.includes('resume') || text.includes('error') || text.includes('Error')) {
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

  test('should detect application logic freeze after pause/resume with mock server', async () => {
    // Enable auto-message generation to simulate real YouTube's continuous message flow
    // Real YouTube has ~15-20 messages per poll (1.5s interval) = ~10-15 msgs/sec
    log.info('Enabling auto-messages (20 per poll to match real YouTube)...');
    await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: true, messages_per_poll: 20 }),
    });

    // Step 1: Connect to mock stream
    log.info('Connecting to mock stream...');
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_logic_freeze`);
    await mainPage.locator('button:has-text("開始")').click();

    // Wait for connection
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });
    log.info('Connected to mock stream');

    // Step 2: Verify application logic works BEFORE pause
    log.info('Testing application logic BEFORE pause...');
    const beforePause = await testApplicationLogicWorks(mainPage);
    log.info(`Before pause: ${beforePause.details}`);
    expect(beforePause.works).toBe(true);

    // Step 3: Wait for messages to accumulate (like real YouTube)
    log.info('Waiting 5 seconds for messages...');
    await mainPage.waitForTimeout(5000);

    const msgCountBefore = await mainPage.locator('[data-message-id]').count();
    log.debug(`Messages before pause: ${msgCountBefore}`);

    // Step 4: Pause
    log.info('Pausing...');
    await mainPage.locator('button:has-text("停止")').click();
    await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });
    log.info('Paused');

    // Step 5: Wait while paused (messages continue to be generated server-side)
    log.info('Waiting 3 seconds while paused...');
    await mainPage.waitForTimeout(3000);

    // Step 6: Resume
    log.info('Resuming...');
    await mainPage.locator('button:has-text("再開")').click();
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });
    log.info('Resume completed');

    // Step 7: Test application logic AFTER resume
    log.info('Testing application logic AFTER resume...');
    await mainPage.waitForTimeout(1000);

    const afterResume = await testApplicationLogicWorks(mainPage);
    log.info(`After resume: ${afterResume.details}`);

    if (!afterResume.works) {
      await mainPage.screenshot({ path: 'logic-freeze-mock-server.png' });
      // Disable auto-messages before throwing
      await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: false }),
      });
      throw new Error(`BUG DETECTED: Application logic frozen after resume. ${afterResume.details}`);
    }

    log.info('Test passed: Application logic works after resume');

    // Disable auto-messages
    await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: false }),
    });

    // Cleanup
    try {
      await mainPage.locator('button:has-text("停止")').click({ timeout: 2000 });
      await mainPage.locator('button:has-text("初期化")').click({ timeout: 2000 });
    } catch {
      log.debug('Cleanup skipped');
    }
  });

  test('should detect application logic freeze with network delay simulation', async () => {
    // Simulate real YouTube network latency (500ms for /watch, 200ms for chat)
    log.info('Configuring network delay simulation...');
    await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        watch_delay_ms: 500,  // Simulate /watch page load time
        chat_delay_ms: 200    // Simulate chat API latency
      }),
    });

    // Enable auto-messages to simulate continuous message flow
    await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: true, messages_per_poll: 10 }),
    });

    // Step 1: Connect to mock stream
    log.info('Connecting to mock stream with delay...');
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_delay`);
    await mainPage.locator('button:has-text("開始")').click();

    // Wait for connection (may take longer due to delay)
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 15000 });
    log.info('Connected');

    // Step 2: Verify application logic works BEFORE pause
    const beforePause = await testApplicationLogicWorks(mainPage);
    log.info(`Before pause: ${beforePause.details}`);
    expect(beforePause.works).toBe(true);

    // Step 3: Wait for messages
    log.info('Waiting 5 seconds for messages...');
    await mainPage.waitForTimeout(5000);

    // Step 4: Pause
    log.info('Pausing...');
    await mainPage.locator('button:has-text("停止")').click();
    await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });
    log.info('Paused');

    // Step 5: Wait while paused
    await mainPage.waitForTimeout(3000);

    // Step 6: Resume (this is where the bug might occur with delay)
    log.info('Resuming with network delay...');
    await mainPage.locator('button:has-text("再開")').click();
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 15000 });
    log.info('Resume completed');

    // Step 7: Test application logic AFTER resume
    log.info('Testing application logic AFTER resume with delay...');
    await mainPage.waitForTimeout(1000);

    const afterResume = await testApplicationLogicWorks(mainPage);
    log.info(`After resume with delay: ${afterResume.details}`);

    // Clean up delays
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

    if (!afterResume.works) {
      await mainPage.screenshot({ path: 'logic-freeze-with-delay.png' });
      throw new Error(`BUG DETECTED: Application logic frozen after resume with delay. ${afterResume.details}`);
    }

    log.info('Test passed: Application logic works after resume with delay');

    // Cleanup
    try {
      await mainPage.locator('button:has-text("停止")').click({ timeout: 2000 });
      await mainPage.locator('button:has-text("初期化")').click({ timeout: 2000 });
    } catch {
      log.debug('Cleanup skipped');
    }
  });

  test('should detect logic freeze with multiple pause/resume cycles', async () => {
    // Connect
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_multi_cycle`);
    await mainPage.locator('button:has-text("開始")').click();
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

    // Perform 3 pause/resume cycles
    for (let i = 0; i < 3; i++) {
      log.info(`Cycle ${i + 1}/3: Pause...`);
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

      await mainPage.waitForTimeout(1000);

      log.info(`Cycle ${i + 1}/3: Resume...`);
      await mainPage.locator('button:has-text("再開")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Test logic after each resume
      const result = await testApplicationLogicWorks(mainPage);
      log.info(`Cycle ${i + 1}/3: ${result.details}`);

      if (!result.works) {
        await mainPage.screenshot({ path: `logic-freeze-cycle-${i + 1}.png` });
        throw new Error(`BUG DETECTED at cycle ${i + 1}: ${result.details}`);
      }
    }

    log.info('All 3 cycles passed');

    // Cleanup
    try {
      await mainPage.locator('button:has-text("停止")').click({ timeout: 2000 });
      await mainPage.locator('button:has-text("初期化")').click({ timeout: 2000 });
    } catch {
      log.debug('Cleanup skipped');
    }
  });
});
