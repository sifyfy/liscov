import { test, expect, chromium, BrowserContext, Page, Browser } from '@playwright/test';
import { exec, execSync, spawn, ChildProcess } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

/**
 * E2E tests for connection state transitions based on 02_chat.md specification.
 *
 * Tests verify:
 * - Pause preserves stream title and broadcaster name
 * - Paused state displays stream info correctly (not fallback "配信")
 * - Resume reconnects to the same stream
 * - Initialize clears all state and returns to idle
 */

const CDP_URL = 'http://127.0.0.1:9222';
const MOCK_SERVER_URL = 'http://localhost:3456';
const PROJECT_DIR = process.cwd().replace(/[\\/]e2e-tauri$/, '');

// Test isolation: use separate namespace for credentials and data
const TEST_APP_NAME = 'liscov-test';
const TEST_KEYRING_SERVICE = 'liscov-test';

// Get test data directories based on platform
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

  const dataDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.local', 'share');

  if (dataDir && dataDir !== configDir) {
    dirs.push(path.join(dataDir, TEST_APP_NAME));
  }

  return dirs;
}

// Clean up test data directories
async function cleanupTestData(): Promise<void> {
  const dirs = getTestDataDirs();
  for (const dir of dirs) {
    if (fs.existsSync(dir)) {
      console.log(`Cleaning up test data directory: ${dir}`);
      fs.rmSync(dir, { recursive: true, force: true });
    }
  }
}

// Clean up test keyring credentials (Windows Credential Manager)
async function cleanupTestCredentials(): Promise<void> {
  if (process.platform === 'win32') {
    try {
      execSync(`cmdkey /delete:${TEST_KEYRING_SERVICE}:youtube_credentials 2>nul`, { stdio: 'ignore' });
      console.log('Cleaned up test credentials from Windows Credential Manager');
    } catch {
      // Credential may not exist, which is fine
    }
  }
}

// Helper to wait for CDP to be available
async function waitForCDP(timeout = 120000): Promise<void> {
  const start = Date.now();
  console.log('Waiting for CDP to be available...');
  let lastError = '';
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`${CDP_URL}/json/version`);
      if (response.ok) {
        console.log(`CDP available after ${Date.now() - start}ms`);
        return;
      }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
    await new Promise(resolve => setTimeout(resolve, 1000));
  }
  throw new Error(`CDP not available after ${timeout}ms. Last error: ${lastError}`);
}

// Helper to connect to Tauri app
async function connectToApp(): Promise<{ browser: Browser; context: BrowserContext; page: Page }> {
  const browser = await chromium.connectOverCDP(CDP_URL);
  const contexts = browser.contexts();

  if (contexts.length === 0) {
    throw new Error('No browser contexts found');
  }

  const context = contexts[0];
  const pages = context.pages();

  if (pages.length === 0) {
    throw new Error('No pages found in context');
  }

  return { browser, context, page: pages[0] };
}

// Helper to kill Tauri app
async function killTauriApp(): Promise<void> {
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /IM liscov-tauri.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f liscov-tauri', { stdio: 'ignore' });
    }
  } catch {
    // Process may not exist
  }
  // Wait for port to be released
  await new Promise(resolve => setTimeout(resolve, 1000));
}

// Helper to start Tauri app with test isolation and mock server
async function startTauriApp(): Promise<void> {
  const env = {
    ...process.env,
    // Test isolation: use separate namespace
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    // Mock server URLs - point all YouTube APIs to mock server
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
    // Enable CDP for Playwright
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  };

  console.log(`Starting Tauri app with test namespace: ${TEST_APP_NAME}`);

  // Start app in background
  exec(`cd "${PROJECT_DIR}" && pnpm tauri dev`, { env });

  // Wait for CDP to be available
  await waitForCDP();
}

// Mock server process reference
let mockServerProcess: ChildProcess | null = null;

// Helper to kill mock server process
async function killMockServer(): Promise<void> {
  if (mockServerProcess) {
    console.log('Stopping mock server...');
    mockServerProcess.kill();
    mockServerProcess = null;
  }
  // Also kill any orphaned mock_server processes
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /IM mock_server.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f mock_server', { stdio: 'ignore' });
    }
  } catch {
    // Process may not exist
  }
  await new Promise(resolve => setTimeout(resolve, 500));
}

// Helper to start mock server
async function startMockServer(): Promise<void> {
  console.log('Starting mock server...');

  // Kill any existing mock server first
  await killMockServer();

  // Start mock server as a child process
  const cargoPath = path.join(PROJECT_DIR, 'src-tauri', 'Cargo.toml');
  mockServerProcess = spawn('cargo', ['run', '--manifest-path', cargoPath, '--bin', 'mock_server'], {
    cwd: PROJECT_DIR,
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: true,
  });

  // Log mock server output for debugging
  mockServerProcess.stdout?.on('data', (data) => {
    const msg = data.toString().trim();
    if (msg) console.log(`[mock_server] ${msg}`);
  });
  mockServerProcess.stderr?.on('data', (data) => {
    const msg = data.toString().trim();
    // Filter out cargo build warnings/info
    if (msg && !msg.includes('Compiling') && !msg.includes('Finished') && !msg.includes('warning:')) {
      console.log(`[mock_server] ${msg}`);
    }
  });

  // Wait for mock server to be ready
  const timeout = 60000;
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`${MOCK_SERVER_URL}/status`);
      if (response.ok) {
        console.log(`Mock server ready after ${Date.now() - start}ms`);
        return;
      }
    } catch {
      // Server not ready yet
    }
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  throw new Error(`Mock server not ready after ${timeout}ms`);
}

// Helper to reset mock server state
async function resetMockServer(): Promise<void> {
  await fetch(`${MOCK_SERVER_URL}/reset`, { method: 'POST' });
}

// Helper to add message to mock server
async function addMockMessage(message: {
  message_type: string;
  author: string;
  content: string;
  channel_id?: string;
  is_member?: boolean;
}): Promise<void> {
  await fetch(`${MOCK_SERVER_URL}/add_message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(message),
  });
}

// Helper to fully disconnect (stop + initialize) and return to idle state
async function disconnectAndInitialize(page: Page): Promise<void> {
  // Click 停止 to pause
  const stopButton = page.locator('button:has-text("停止")');
  if (await stopButton.isVisible({ timeout: 1000 }).catch(() => false)) {
    await stopButton.click();
    // After clicking 停止, app goes to paused state with 再開 and 初期化 buttons
    // Click 初期化 to return to idle state
    await page.locator('button:has-text("初期化")').click();
    // Wait for UI to return to idle state (URL input visible)
    await expect(page.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]')).toBeVisible({ timeout: 5000 });
  }
}

test.describe('Connection State Transitions (02_chat.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000); // 5 minutes for setup

    // Step 1: Kill any existing processes
    console.log('Killing any existing Tauri app...');
    await killTauriApp();

    // Step 2: Clean up test data and credentials for a fresh start
    console.log('Cleaning up test data and credentials...');
    await cleanupTestData();
    await cleanupTestCredentials();

    // Step 3: Start mock server
    await startMockServer();

    // Step 4: Reset mock server state
    console.log('Resetting mock server state...');
    await resetMockServer();

    // Step 5: Start Tauri app with test namespace
    console.log('Starting Tauri app with test namespace...');
    await startTauriApp();

    // Step 6: Connect to the running Tauri app
    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    // Wait for page to be stable (avoid navigation context destruction)
    await mainPage.waitForLoadState('domcontentloaded');
    await mainPage.waitForTimeout(2000);

    console.log('Connected to Tauri app');
  });

  test.afterAll(async () => {
    // Kill the Tauri app
    await killTauriApp();
    // Stop mock server
    await killMockServer();
    // Clean up test data
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test.beforeEach(async () => {
    // Reset mock server before each test
    await resetMockServer();

    // Ensure clean app state before each test
    await disconnectAndInitialize(mainPage);
  });

  test.describe('Pause State (停止)', () => {
    test('should show stream title in paused state, not fallback text', async () => {
      // Spec: 一時停止中: {streamTitle || broadcasterName || '配信'}
      // Bug fix verification: stream title should be preserved, not show "配信"

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();

      // Step 2: Wait for connection and verify stream title is shown
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 3: Click 停止 to pause
      await mainPage.locator('button:has-text("停止")').click();

      // Step 4: Verify paused state UI shows stream title (not "配信")
      // Use specific class selector to avoid matching multiple divs
      const pausedInfo = mainPage.locator('.bg-yellow-50.border-yellow-300:has-text("一時停止中:")');
      await expect(pausedInfo).toBeVisible({ timeout: 5000 });

      // Verify "Mock Live" (stream title) or "Mock Streamer" (broadcaster name) is shown
      // Should NOT just show "配信" as fallback
      const pausedText = await pausedInfo.textContent();
      expect(pausedText).toContain('一時停止中:');

      // Check that it contains actual stream info, not just the fallback
      const hasStreamTitle = pausedText?.includes('Mock Live');
      const hasBroadcasterName = pausedText?.includes('Mock Streamer');
      const onlyFallback = pausedText?.match(/一時停止中:\s*配信\s*$/);

      expect(hasStreamTitle || hasBroadcasterName).toBe(true);
      expect(onlyFallback).toBeFalsy();

      // Step 5: Verify 再開 and 初期化 buttons are visible
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();
      await expect(mainPage.locator('button:has-text("初期化")')).toBeVisible();

      // Cleanup
      await mainPage.locator('button:has-text("初期化")').click();
    });

    test('should preserve messages when paused', async () => {
      // Spec: 一時停止状態はメッセージを保持する

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 2: Wait for some messages to arrive
      await mainPage.waitForTimeout(3000);

      // Step 3: Get message count before pause
      const messageCountBefore = await mainPage.locator('[data-message-id]').count();

      // Step 4: Click 停止 to pause
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      // Step 5: Verify messages are still visible
      const messageCountAfter = await mainPage.locator('[data-message-id]').count();
      expect(messageCountAfter).toBe(messageCountBefore);

      // Cleanup
      await mainPage.locator('button:has-text("初期化")').click();
    });

    test('should show chat mode toggle in paused state', async () => {
      // Spec: チャットモード切り替えは一時停止中も操作可能

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 2: Click 停止 to pause
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      // Step 3: Verify chat mode toggle is visible
      const chatModeButton = mainPage.locator('button:has-text("トップ"), button:has-text("全て")');
      await expect(chatModeButton).toBeVisible();

      // Cleanup
      await mainPage.locator('button:has-text("初期化")').click();
    });
  });

  test.describe('Resume (再開)', () => {
    test('should reconnect to the same stream when resumed', async () => {
      // Spec: 再開ボタンで同じURLに再接続

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 2: Click 停止 to pause
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      // Step 3: Click 再開 to resume
      await mainPage.locator('button:has-text("再開")').click();

      // Step 4: Verify connected state - 停止 button visible again
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 5: Verify stream info is still shown
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible();

      // Cleanup
      await disconnectAndInitialize(mainPage);
    });

    test('should not clear messages when resumed', async () => {
      // Spec: 再開時はメッセージをクリアしない

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 2: Wait for some messages
      await mainPage.waitForTimeout(3000);

      // Step 3: Get message count before pause
      const messageCountBefore = await mainPage.locator('[data-message-id]').count();

      // Step 4: Pause and resume
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();
      await mainPage.locator('button:has-text("再開")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 5: Verify messages are still present
      const messageCountAfter = await mainPage.locator('[data-message-id]').count();
      expect(messageCountAfter).toBeGreaterThanOrEqual(messageCountBefore);

      // Cleanup
      await disconnectAndInitialize(mainPage);
    });

    test('should receive new messages after resume', async () => {
      // Critical test: Verify that new messages are actually received after resume

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 2: Wait for initial connection to stabilize
      await mainPage.waitForTimeout(2000);

      // Step 3: Pause
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      // Step 4: Resume
      await mainPage.locator('button:has-text("再開")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 5: Wait for resume to complete
      await mainPage.waitForTimeout(2000);

      // Step 6: Add a unique message via mock server AFTER resume
      const uniqueContent = `ResumeTest_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'ResumeTestUser',
        content: uniqueContent,
        channel_id: 'UC_resume_test',
      });

      // Step 7: Wait for the message to be received and displayed
      await expect(mainPage.getByText(uniqueContent)).toBeVisible({ timeout: 10000 });

      // Cleanup
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Initialize (初期化)', () => {
    test('should clear all state and return to idle', async () => {
      // Spec: 初期化ボタンで全状態をクリア

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 2: Wait for some messages
      await mainPage.waitForTimeout(2000);

      // Step 3: Click 停止 to pause
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("初期化")')).toBeVisible();

      // Step 4: Click 初期化
      await mainPage.locator('button:has-text("初期化")').click();

      // Step 5: Verify idle state - URL input visible
      await expect(mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]')).toBeVisible({ timeout: 5000 });

      // Step 6: Verify 開始 button visible
      await expect(mainPage.locator('button:has-text("開始")')).toBeVisible();

      // Step 7: Verify messages are cleared
      const messageCount = await mainPage.locator('[data-message-id]').count();
      expect(messageCount).toBe(0);

      // Step 8: Verify stream info is cleared (no Mock Live text)
      await expect(mainPage.getByText('Mock Live')).not.toBeVisible();
    });

    test('should clear URL input after initialize', async () => {
      // Spec: 初期化後はURL入力欄が空

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 2: Pause and initialize
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("初期化")')).toBeVisible();
      await mainPage.locator('button:has-text("初期化")').click();

      // Step 3: Verify URL input is empty and ready for new connection
      const newUrlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await expect(newUrlInput).toBeVisible({ timeout: 5000 });

      // Verify the input is empty or has placeholder only
      const inputValue = await newUrlInput.inputValue();
      expect(inputValue).toBe('');
    });
  });

  test.describe('State Transitions', () => {
    test('should follow correct state machine: idle -> connecting -> connected -> paused -> idle', async () => {
      // Spec: 状態遷移: idle → connecting → connected → paused → idle

      // Step 1: Initial state is idle (URL input visible)
      await expect(mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]')).toBeVisible();

      // Step 2: Click 開始 - should go to connecting then connected
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();

      // Brief "接続中..." state
      // Note: This may be too fast to catch in UI

      // Connected state
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 3: Click 停止 - should go to paused
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();
      await expect(mainPage.locator('button:has-text("初期化")')).toBeVisible();

      // Step 4: Click 初期化 - should go back to idle
      await mainPage.locator('button:has-text("初期化")').click();
      await expect(mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]')).toBeVisible({ timeout: 5000 });
    });

    test('should follow correct state machine: paused -> connecting -> connected via resume', async () => {
      // Spec: 状態遷移: paused → connecting → connected (再開)

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 2: Pause
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      // Step 3: Resume - should go to connecting then connected
      await mainPage.locator('button:has-text("再開")').click();

      // Should return to connected state
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Cleanup
      await disconnectAndInitialize(mainPage);
    });
  });
});
