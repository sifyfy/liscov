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

  test.describe('Auto-Message Continuous Flow (実際のYouTubeシミュレーション)', () => {
    // This test uses auto-message generation to simulate real YouTube's continuous chat flow
    // Tests the exact scenario reported: connect -> pause -> resume -> no new messages

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

    test('should continue receiving auto-generated messages after pause/resume', async () => {
      // This test reproduces the real YouTube scenario:
      // - Messages flow continuously
      // - User pauses
      // - User resumes
      // - Messages should continue flowing

      // Step 1: Enable auto-message generation (10 messages per poll)
      await enableAutoMessages(10);

      // Step 2: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_auto_msg`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 3: Wait for initial messages to accumulate
      console.log('Waiting for auto-generated messages...');
      await mainPage.waitForTimeout(5000);

      // Get message count before pause
      const messageCountBefore = await mainPage.locator('[data-message-id]').count();
      console.log(`Messages before pause: ${messageCountBefore}`);
      expect(messageCountBefore).toBeGreaterThan(20);

      // Record the last message ID before pause
      const lastMessageBefore = await mainPage.locator('[data-message-id]').last().getAttribute('data-message-id');
      console.log(`Last message ID before pause: ${lastMessageBefore}`);

      // Step 4: Pause
      console.log('Pausing...');
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

      // Check auto-message status (should continue generating on server)
      const statusAfterPause = await getAutoMessageStatus();
      console.log(`Auto-message total after pause: ${statusAfterPause.total_generated}`);

      // Wait a bit (messages continue to be generated server-side)
      await mainPage.waitForTimeout(2000);

      // Step 5: Resume
      console.log('Resuming...');
      await mainPage.locator('button:has-text("再開")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });
      console.log('Resume completed');

      // Step 6: Wait for new messages to arrive
      console.log('Waiting for new messages after resume...');
      await mainPage.waitForTimeout(5000);

      // Get message count after resume
      const messageCountAfter = await mainPage.locator('[data-message-id]').count();
      console.log(`Messages after resume wait: ${messageCountAfter}`);

      // Get the last message ID after resume
      const lastMessageAfter = await mainPage.locator('[data-message-id]').last().getAttribute('data-message-id');
      console.log(`Last message ID after resume: ${lastMessageAfter}`);

      // CRITICAL CHECK: new messages should have arrived
      // The message count should have increased
      expect(messageCountAfter).toBeGreaterThan(messageCountBefore);

      // The last message ID should be different
      expect(lastMessageAfter).not.toBe(lastMessageBefore);

      // Check server auto-message status
      const statusAfterResume = await getAutoMessageStatus();
      console.log(`Auto-message total after resume: ${statusAfterResume.total_generated}`);
      expect(statusAfterResume.total_generated).toBeGreaterThan(statusAfterPause.total_generated);

      // Cleanup
      await disableAutoMessages();
      await disconnectAndInitialize(mainPage);
    });

    test('should not lose messages during rapid pause/resume with auto-generation', async () => {
      // Stress test: rapid pause/resume while messages are being generated

      // Step 1: Enable high-volume auto-message generation
      await enableAutoMessages(20);

      // Step 2: Connect
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_rapid_auto`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Wait for initial messages
      await mainPage.waitForTimeout(3000);
      const initialCount = await mainPage.locator('[data-message-id]').count();
      console.log(`Initial message count: ${initialCount}`);

      // Step 3: Perform 5 rapid pause/resume cycles
      for (let i = 0; i < 5; i++) {
        console.log(`Rapid cycle ${i + 1}/5`);

        // Pause
        await mainPage.locator('button:has-text("停止")').click();
        await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

        // Very short pause
        await mainPage.waitForTimeout(100);

        // Resume
        await mainPage.locator('button:has-text("再開")').click();
        await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

        // Very short wait
        await mainPage.waitForTimeout(100);
      }

      // Step 4: Wait for messages to stabilize
      await mainPage.waitForTimeout(3000);

      const finalCount = await mainPage.locator('[data-message-id]').count();
      console.log(`Final message count after 5 rapid cycles: ${finalCount}`);

      // Messages should have continued increasing
      expect(finalCount).toBeGreaterThan(initialCount);

      // Cleanup
      await disableAutoMessages();
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('High Volume Resume (UIフリーズ回避)', () => {
    // This test verifies the fix for the UI freeze bug that occurred when:
    // 1. Connected to a stream with high message volume (100+ messages)
    // 2. Paused the connection
    // 3. Resumed the connection
    // The bug caused the UI to freeze due to accumulated scrollToBottom() callbacks

    test('should not freeze UI when resuming with high message volume', async () => {
      // CRITICAL: This test must FAIL if UI freezes after resume
      // The key is to try UI interaction IMMEDIATELY after resume, not after waiting

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 2: Send many messages and WAIT for them to be rendered
      // This simulates having existing messages before pause (MAX_MESSAGES = 500)
      console.log('Sending 500 messages to fill message buffer...');
      const messagePromises = [];
      for (let i = 0; i < 500; i++) {
        // Mix different message types to simulate real YouTube
        const msgType = i % 20 === 0 ? 'superchat' : i % 10 === 0 ? 'membership' : 'text';
        messagePromises.push(
          addMockMessage({
            message_type: msgType,
            author: `User${i % 50}`,
            content: `Pre-pause message ${i} with some additional text to make it longer and more realistic like actual YouTube chat messages`,
            channel_id: `UC_user_${i % 50}`,
            is_member: i % 5 === 0,
            amount: msgType === 'superchat' ? '¥500' : undefined,
          })
        );
      }
      await Promise.all(messagePromises);

      // Wait for messages to be received AND rendered
      await mainPage.waitForTimeout(5000);

      // Verify messages are actually in the UI
      const messageCount = await mainPage.locator('[data-message-id]').count();
      console.log(`Messages in UI before pause: ${messageCount}`);
      expect(messageCount).toBeGreaterThan(100);

      // Step 3: Pause the connection
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

      // Step 4: Add more messages to queue (will be returned on resume)
      // 200 messages to simulate real YouTube's first poll after resume
      console.log('Adding 200 messages to queue for resume...');
      const resumeMessages = [];
      for (let i = 0; i < 200; i++) {
        const msgType = i % 15 === 0 ? 'superchat' : i % 8 === 0 ? 'membership' : 'text';
        resumeMessages.push(
          addMockMessage({
            message_type: msgType,
            author: `ResumeUser${i % 30}`,
            content: `Resume batch message ${i} - testing high volume scenario with realistic message length`,
            channel_id: `UC_resume_${i % 30}`,
            is_member: i % 4 === 0,
            amount: msgType === 'superchat' ? '¥1000' : undefined,
          })
        );
      }
      await Promise.all(resumeMessages);

      // Step 5: Resume and IMMEDIATELY try to interact
      console.log('Clicking resume and immediately trying tab switch...');
      const resumeButton = mainPage.locator('button:has-text("再開")');
      const settingsTab = mainPage.locator('button:has-text("Settings")');

      // Click resume
      await resumeButton.click();

      // IMMEDIATELY try to click settings tab (no waiting!)
      // If UI freezes, this click won't be processed
      const interactionStart = Date.now();

      // Try clicking multiple times to detect freeze
      // Use a Promise.race with a timeout to detect freeze
      const freezeTimeout = 3000; // 3 seconds max for UI response

      try {
        await Promise.race([
          (async () => {
            // Try clicking the settings tab
            await settingsTab.click({ timeout: freezeTimeout });
            // Verify we actually switched tabs by checking for settings content
            await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible({ timeout: 1000 });
          })(),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error('UI FREEZE DETECTED: Tab click not processed')), freezeTimeout)
          ),
        ]);
      } catch (error) {
        const elapsed = Date.now() - interactionStart;
        console.error(`UI freeze detected after ${elapsed}ms`);
        throw error;
      }

      const interactionDuration = Date.now() - interactionStart;
      console.log(`Tab switch completed in ${interactionDuration}ms`);

      // Strict threshold: should respond within 1 second
      expect(interactionDuration).toBeLessThan(1000);

      // Go back to chat tab
      await mainPage.locator('button:has-text("Chat")').click();
      await mainPage.waitForTimeout(500);

      // Cleanup
      await disconnectAndInitialize(mainPage);
    });

    test('should continue receiving messages after multiple pause/resume cycles with high volume', async () => {
      // Stress test: multiple pause/resume cycles with continuous message flow

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 2: Run 3 pause/resume cycles with message bursts
      for (let cycle = 0; cycle < 3; cycle++) {
        console.log(`Pause/Resume cycle ${cycle + 1}/3`);

        // Send 50 messages rapidly
        const messagePromises = [];
        for (let i = 0; i < 50; i++) {
          messagePromises.push(
            addMockMessage({
              message_type: 'text',
              author: `CycleUser${i % 5}`,
              content: `Cycle ${cycle} message ${i}`,
              channel_id: `UC_cycle_${i % 5}`,
            })
          );
        }
        await Promise.all(messagePromises);
        await mainPage.waitForTimeout(1000);

        // Pause
        await mainPage.locator('button:has-text("停止")').click();
        await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

        // Resume
        await mainPage.locator('button:has-text("再開")').click();
        await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

        // Verify UI is responsive (tab labels are English)
        await mainPage.locator('button:has-text("Settings")').click();
        await mainPage.waitForTimeout(300);
        await mainPage.locator('button:has-text("Chat")').click();
        await mainPage.waitForTimeout(300);
      }

      // Step 3: Final verification - new message should be received
      const finalContent = `FinalCheck_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'FinalCheckUser',
        content: finalContent,
        channel_id: 'UC_final_check',
      });

      await expect(mainPage.getByText(finalContent)).toBeVisible({ timeout: 10000 });
      console.log('Final message received after 3 pause/resume cycles!');

      // Cleanup
      await disconnectAndInitialize(mainPage);
    });

    test('should handle rapid pause/resume without UI freeze', async () => {
      // Edge case: very rapid pause/resume clicks

      // Step 1: Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 2: Send initial batch of messages
      const messagePromises = [];
      for (let i = 0; i < 100; i++) {
        messagePromises.push(
          addMockMessage({
            message_type: 'text',
            author: `RapidUser${i % 10}`,
            content: `Rapid test message ${i}`,
            channel_id: `UC_rapid_${i % 10}`,
          })
        );
      }
      await Promise.all(messagePromises);
      await mainPage.waitForTimeout(2000);

      // Step 3: Rapid pause/resume (5 times in quick succession)
      console.log('Performing rapid pause/resume cycles...');
      for (let i = 0; i < 5; i++) {
        await mainPage.locator('button:has-text("停止")').click();
        await mainPage.waitForTimeout(200);
        await mainPage.locator('button:has-text("再開")').click();
        await mainPage.waitForTimeout(200);
      }

      // Step 4: Wait for final state to stabilize
      // Should end in connected state (停止 button visible)
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      // Step 5: Verify UI is still responsive (tab labels are English)
      const tabClickStart = Date.now();
      await mainPage.locator('button:has-text("Settings")').click();
      await mainPage.waitForTimeout(300);
      await mainPage.locator('button:has-text("Chat")').click();
      const tabClickDuration = Date.now() - tabClickStart;
      console.log(`Tab switching after rapid cycles: ${tabClickDuration}ms`);
      expect(tabClickDuration).toBeLessThan(3000);

      // Step 6: Verify new messages are still received
      const rapidContent = `AfterRapid_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'RapidTestUser',
        content: rapidContent,
        channel_id: 'UC_rapid_final',
      });

      await expect(mainPage.getByText(rapidContent)).toBeVisible({ timeout: 10000 });
      console.log('Message received after rapid pause/resume cycles!');

      // Cleanup
      await disconnectAndInitialize(mainPage);
    });
  });
});
