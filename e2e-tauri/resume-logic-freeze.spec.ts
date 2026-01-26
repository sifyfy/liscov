import { test, expect, chromium, BrowserContext, Page, Browser } from '@playwright/test';
import { exec, execSync, spawn, ChildProcess } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

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

const CDP_URL = 'http://127.0.0.1:9222';
const MOCK_SERVER_URL = 'http://localhost:3456';
const PROJECT_DIR = process.cwd().replace(/[\\/]e2e-tauri$/, '');
const TEST_APP_NAME = 'liscov-test';
const TEST_KEYRING_SERVICE = 'liscov-test';

function getTestDataDirs(): string[] {
  const dirs: string[] = [];
  const configDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.config');
  if (configDir) {
    dirs.push(path.join(configDir, TEST_APP_NAME));
  }
  return dirs;
}

async function cleanupTestData(): Promise<void> {
  for (const dir of getTestDataDirs()) {
    if (fs.existsSync(dir)) {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  }
}

async function waitForCDP(timeout = 120000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`${CDP_URL}/json/version`);
      if (response.ok) {
        console.log(`CDP available after ${Date.now() - start}ms`);
        return;
      }
    } catch { }
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  throw new Error(`CDP not available after ${timeout}ms`);
}

async function connectToApp(): Promise<{ browser: Browser; context: BrowserContext; page: Page }> {
  const browser = await chromium.connectOverCDP(CDP_URL);
  const contexts = browser.contexts();
  if (contexts.length === 0) throw new Error('No browser contexts found');
  const context = contexts[0];
  const pages = context.pages();
  if (pages.length === 0) throw new Error('No pages found in context');
  return { browser, context, page: pages[0] };
}

async function killTauriApp(): Promise<void> {
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /IM liscov-tauri.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f liscov-tauri', { stdio: 'ignore' });
    }
  } catch { }
  await new Promise(resolve => setTimeout(resolve, 1000));
}

async function killMockServer(): Promise<void> {
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /IM mock_server.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f mock_server', { stdio: 'ignore' });
    }
  } catch { }
  await new Promise(resolve => setTimeout(resolve, 500));
}

let mockServerProcess: ChildProcess | null = null;

async function startMockServer(): Promise<void> {
  await killMockServer();
  const cargoPath = path.join(PROJECT_DIR, 'src-tauri', 'Cargo.toml');
  mockServerProcess = spawn('cargo', ['run', '--manifest-path', cargoPath, '--bin', 'mock_server'], {
    cwd: PROJECT_DIR,
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: true,
  });
  mockServerProcess.stdout?.on('data', (data) => {
    const msg = data.toString().trim();
    if (msg && !msg.includes('Compiling')) console.log(`[mock] ${msg}`);
  });
  mockServerProcess.stderr?.on('data', (data) => {
    const msg = data.toString().trim();
    if (msg && !msg.includes('Compiling') && !msg.includes('Finished') && !msg.includes('warning:')) {
      console.log(`[mock] ${msg}`);
    }
  });

  const timeout = 60000;
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`${MOCK_SERVER_URL}/status`);
      if (response.ok) {
        console.log(`Mock server ready after ${Date.now() - start}ms`);
        return;
      }
    } catch { }
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  throw new Error(`Mock server not ready after ${timeout}ms`);
}

// Start Tauri app pointing to real YouTube (no mock)
async function startTauriAppForRealYouTube(): Promise<void> {
  const env = {
    ...process.env,
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  };
  exec(`cd "${PROJECT_DIR}" && pnpm tauri dev`, { env });
  await waitForCDP();
}

// Start Tauri app pointing to mock server
async function startTauriAppForMockServer(): Promise<void> {
  const env = {
    ...process.env,
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  };
  exec(`cd "${PROJECT_DIR}" && pnpm tauri dev`, { env });
  await waitForCDP();
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
  console.log(`JS execution test before tab click: ${JSON.stringify(jsTestBefore)}`);

  // Click Settings tab
  console.log('Clicking Settings tab...');
  await page.locator('button:has-text("Settings")').click();

  // Wait a moment for any potential state change
  await page.waitForTimeout(500);

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
  console.log(`JS execution test after tab click: ${JSON.stringify(jsTestAfter)}`);

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
    console.log(`Settings tab button state: ${JSON.stringify(activeTabButton)}`);

    return {
      works: false,
      details: `Tab click did not change content (Settings heading not visible after clicking Settings tab). Tab button state: ${JSON.stringify(activeTabButton)}`
    };
  }

  // Test 2: Click back to Chat tab
  await page.locator('button:has-text("Chat")').click();
  await page.waitForTimeout(500);

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

    console.log('Starting Tauri app for real YouTube...');
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
        console.log(`[CRITICAL] ${text}`);
      } else if (text.includes('[chat.svelte.ts]') || text.includes('resume') || text.includes('error') || text.includes('Error')) {
        console.log(`[CONSOLE] ${text}`);
      }
    });

    await mainPage.waitForLoadState('domcontentloaded');
    await mainPage.waitForTimeout(2000);
    console.log('Connected to Tauri app');
  });

  test.afterAll(async () => {
    await killTauriApp();
    await cleanupTestData();
  });

  test('should detect application logic freeze after pause/resume with real YouTube', async () => {
    // Step 1: Connect to YouTube stream
    console.log(`Connecting to: ${YOUTUBE_URL}`);
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
    await urlInput.fill(YOUTUBE_URL);
    await mainPage.locator('button:has-text("開始")').click();

    // Wait for connection
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 30000 });
    console.log('Connected to stream');

    // Step 2: Verify application logic works BEFORE pause
    console.log('Testing application logic BEFORE pause...');
    const beforePause = await testApplicationLogicWorks(mainPage);
    console.log(`Before pause: ${beforePause.details}`);
    expect(beforePause.works).toBe(true);

    // Step 3: Wait a bit to ensure stable connection
    console.log('Waiting 5 seconds for stable connection...');
    await mainPage.waitForTimeout(5000);

    // Step 4: Pause
    console.log('Pausing...');
    await mainPage.locator('button:has-text("停止")').click();
    await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });
    console.log('Paused');

    // Step 5: Wait while paused
    console.log('Waiting 3 seconds while paused...');
    await mainPage.waitForTimeout(3000);

    // Step 6: Resume
    console.log('Resuming...');
    await mainPage.locator('button:has-text("再開")').click();
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });
    console.log('Resume completed');

    // Step 7: Test application logic AFTER resume
    console.log('Testing application logic AFTER resume...');

    // Give a moment for any potential issue to manifest
    await mainPage.waitForTimeout(1000);

    const afterResume = await testApplicationLogicWorks(mainPage);
    console.log(`After resume: ${afterResume.details}`);

    if (!afterResume.works) {
      // Take screenshot for debugging
      await mainPage.screenshot({ path: 'logic-freeze-real-youtube.png' });
      throw new Error(`BUG DETECTED: Application logic frozen after resume. ${afterResume.details}`);
    }

    console.log('Test passed: Application logic works after resume');

    // Cleanup
    try {
      await mainPage.locator('button:has-text("停止")').click({ timeout: 2000 });
      await mainPage.locator('button:has-text("初期化")').click({ timeout: 2000 });
    } catch {
      console.log('Cleanup skipped');
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

    console.log('Starting mock server...');
    await startMockServer();

    console.log('Starting Tauri app for mock server...');
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
        console.log(`[CRITICAL] ${text}`);
      } else if (text.includes('[chat.svelte.ts]') || text.includes('resume') || text.includes('error') || text.includes('Error')) {
        console.log(`[CONSOLE] ${text}`);
      }
    });

    await mainPage.waitForLoadState('domcontentloaded');
    await mainPage.waitForTimeout(2000);
    console.log('Connected to Tauri app');
  });

  test.afterAll(async () => {
    await killTauriApp();
    if (mockServerProcess) {
      mockServerProcess.kill();
      mockServerProcess = null;
    }
    await killMockServer();
    await cleanupTestData();
  });

  test('should detect application logic freeze after pause/resume with mock server', async () => {
    // Enable auto-message generation to simulate real YouTube's continuous message flow
    // Real YouTube has ~15-20 messages per poll (1.5s interval) = ~10-15 msgs/sec
    console.log('Enabling auto-messages (20 per poll to match real YouTube)...');
    await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: true, messages_per_poll: 20 }),
    });

    // Step 1: Connect to mock stream
    console.log('Connecting to mock stream...');
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_logic_freeze`);
    await mainPage.locator('button:has-text("開始")').click();

    // Wait for connection
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });
    console.log('Connected to mock stream');

    // Step 2: Verify application logic works BEFORE pause
    console.log('Testing application logic BEFORE pause...');
    const beforePause = await testApplicationLogicWorks(mainPage);
    console.log(`Before pause: ${beforePause.details}`);
    expect(beforePause.works).toBe(true);

    // Step 3: Wait for messages to accumulate (like real YouTube)
    console.log('Waiting 5 seconds for messages...');
    await mainPage.waitForTimeout(5000);

    const msgCountBefore = await mainPage.locator('[data-message-id]').count();
    console.log(`Messages before pause: ${msgCountBefore}`);

    // Step 4: Pause
    console.log('Pausing...');
    await mainPage.locator('button:has-text("停止")').click();
    await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });
    console.log('Paused');

    // Step 5: Wait while paused (messages continue to be generated server-side)
    console.log('Waiting 3 seconds while paused...');
    await mainPage.waitForTimeout(3000);

    // Step 6: Resume
    console.log('Resuming...');
    await mainPage.locator('button:has-text("再開")').click();
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });
    console.log('Resume completed');

    // Step 7: Test application logic AFTER resume
    console.log('Testing application logic AFTER resume...');
    await mainPage.waitForTimeout(1000);

    const afterResume = await testApplicationLogicWorks(mainPage);
    console.log(`After resume: ${afterResume.details}`);

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

    console.log('Test passed: Application logic works after resume');

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
      console.log('Cleanup skipped');
    }
  });

  test('should detect application logic freeze with network delay simulation', async () => {
    // Simulate real YouTube network latency (500ms for /watch, 200ms for chat)
    console.log('Configuring network delay simulation...');
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
    console.log('Connecting to mock stream with delay...');
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_delay`);
    await mainPage.locator('button:has-text("開始")').click();

    // Wait for connection (may take longer due to delay)
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 15000 });
    console.log('Connected');

    // Step 2: Verify application logic works BEFORE pause
    const beforePause = await testApplicationLogicWorks(mainPage);
    console.log(`Before pause: ${beforePause.details}`);
    expect(beforePause.works).toBe(true);

    // Step 3: Wait for messages
    console.log('Waiting 5 seconds for messages...');
    await mainPage.waitForTimeout(5000);

    // Step 4: Pause
    console.log('Pausing...');
    await mainPage.locator('button:has-text("停止")').click();
    await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });
    console.log('Paused');

    // Step 5: Wait while paused
    await mainPage.waitForTimeout(3000);

    // Step 6: Resume (this is where the bug might occur with delay)
    console.log('Resuming with network delay...');
    await mainPage.locator('button:has-text("再開")').click();
    await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 15000 });
    console.log('Resume completed');

    // Step 7: Test application logic AFTER resume
    console.log('Testing application logic AFTER resume with delay...');
    await mainPage.waitForTimeout(1000);

    const afterResume = await testApplicationLogicWorks(mainPage);
    console.log(`After resume with delay: ${afterResume.details}`);

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

    console.log('Test passed: Application logic works after resume with delay');

    // Cleanup
    try {
      await mainPage.locator('button:has-text("停止")').click({ timeout: 2000 });
      await mainPage.locator('button:has-text("初期化")').click({ timeout: 2000 });
    } catch {
      console.log('Cleanup skipped');
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
      console.log(`Cycle ${i + 1}/3: Pause...`);
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

      await mainPage.waitForTimeout(1000);

      console.log(`Cycle ${i + 1}/3: Resume...`);
      await mainPage.locator('button:has-text("再開")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Test logic after each resume
      const result = await testApplicationLogicWorks(mainPage);
      console.log(`Cycle ${i + 1}/3: ${result.details}`);

      if (!result.works) {
        await mainPage.screenshot({ path: `logic-freeze-cycle-${i + 1}.png` });
        throw new Error(`BUG DETECTED at cycle ${i + 1}: ${result.details}`);
      }
    }

    console.log('All 3 cycles passed');

    // Cleanup
    try {
      await mainPage.locator('button:has-text("停止")').click({ timeout: 2000 });
      await mainPage.locator('button:has-text("初期化")').click({ timeout: 2000 });
    } catch {
      console.log('Cleanup skipped');
    }
  });
});
