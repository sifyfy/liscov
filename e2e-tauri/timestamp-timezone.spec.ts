import { test, expect, chromium, BrowserContext, Page, Browser } from '@playwright/test';
import { exec, execSync, spawn, ChildProcess } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

const execAsync = promisify(exec);

/**
 * E2E tests for Timestamp Timezone Display
 *
 * Verifies that chat message timestamps are displayed in the user's local timezone,
 * not in UTC. This is a critical usability feature for users in different timezones.
 *
 * Test Strategy:
 * 1. Send a message with a known timestamp (via mock server)
 * 2. Verify the displayed time matches the local timezone conversion
 *
 * Run tests:
 *    pnpm exec playwright test --config e2e-tauri/playwright.config.ts timestamp-timezone.spec.ts
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

// Helper to add message with specific timestamp to mock server
async function addMockMessageWithTimestamp(message: {
  message_type: string;
  author: string;
  content: string;
  channel_id?: string;
  is_member?: boolean;
  timestamp_usec?: string;  // Optional: specific timestamp in microseconds
}): Promise<void> {
  await fetch(`${MOCK_SERVER_URL}/add_message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(message),
  });
}

test.describe('Timestamp Timezone Display', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000); // 4 minutes for setup

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
    console.log('Connected to Tauri app:', await mainPage.title());
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
  });

  test('should display message timestamp in local timezone, not UTC', async () => {
    // This test verifies that timestamps are displayed in the user's local timezone.
    //
    // The bug: Backend formats timestamp using chrono::DateTime::from_timestamp()
    // which creates UTC DateTime, then formats it as HH:MM:SS without timezone conversion.
    // This causes the displayed time to be in UTC instead of local timezone.
    //
    // Expected behavior: If a message arrives at 19:00 JST (10:00 UTC),
    // the displayed time should be "19:00:00", not "10:00:00".

    // Enable timestamp display
    const timestampToggle = mainPage.locator('label:has-text("時刻") input[type="checkbox"]');
    if (!(await timestampToggle.isChecked())) {
      await timestampToggle.check();
    }

    // Connect to stream
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
    await mainPage.locator('button:has-text("Connect")').click();
    await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

    // Record the current local time BEFORE adding the message
    const localTimeBeforeMs = Date.now();

    // Add a message (timestamp will be generated by mock server using current time)
    await addMockMessageWithTimestamp({
      message_type: 'text',
      author: 'TimezoneTestUser',
      content: 'Testing timezone display',
    });

    // Wait for message to appear
    await mainPage.waitForTimeout(3000);

    // Verify message is displayed
    await expect(mainPage.locator('text=TimezoneTestUser')).toBeVisible();
    await expect(mainPage.locator('text=Testing timezone display')).toBeVisible();

    // Get the displayed timestamp from the UI
    // The timestamp is in HH:MM:SS format next to the message
    const messageElement = mainPage.locator('[data-message-id]').filter({
      has: mainPage.locator('text=TimezoneTestUser')
    }).first();

    // Find the timestamp element (HH:MM:SS format)
    const timestampElement = messageElement.locator('span').filter({
      hasText: /^\d{2}:\d{2}:\d{2}$/
    }).first();

    await expect(timestampElement).toBeVisible();
    const displayedTime = await timestampElement.textContent();
    console.log(`Displayed timestamp: ${displayedTime}`);

    // Parse the displayed time
    const [displayedHours, displayedMinutes] = displayedTime!.split(':').map(Number);

    // Calculate expected local time (what the time should be in user's timezone)
    const localTime = new Date(localTimeBeforeMs);
    const expectedLocalHours = localTime.getHours();
    const expectedLocalMinutes = localTime.getMinutes();

    // Calculate UTC time (what the bug would show)
    const utcHours = localTime.getUTCHours();
    const utcMinutes = localTime.getUTCMinutes();

    console.log(`Expected local time: ${expectedLocalHours.toString().padStart(2, '0')}:${expectedLocalMinutes.toString().padStart(2, '0')}`);
    console.log(`UTC time (bug would show): ${utcHours.toString().padStart(2, '0')}:${utcMinutes.toString().padStart(2, '0')}`);
    console.log(`Timezone offset: ${localTime.getTimezoneOffset()} minutes (${-localTime.getTimezoneOffset() / 60} hours from UTC)`);

    // Allow ±2 minutes tolerance for test execution time
    const isWithinLocalTimeRange = (
      Math.abs(displayedHours - expectedLocalHours) <= 0 ||
      // Handle day boundary (23:59 -> 00:01)
      (expectedLocalHours === 23 && displayedHours === 0) ||
      (expectedLocalHours === 0 && displayedHours === 23)
    ) && Math.abs(displayedMinutes - expectedLocalMinutes) <= 2;

    // Check if displayed time matches UTC (the bug)
    const isMatchingUTC = (
      Math.abs(displayedHours - utcHours) <= 0 ||
      (utcHours === 23 && displayedHours === 0) ||
      (utcHours === 0 && displayedHours === 23)
    ) && Math.abs(displayedMinutes - utcMinutes) <= 2;

    // If timezone offset is 0 (UTC), skip this test as it can't detect the bug
    if (localTime.getTimezoneOffset() === 0) {
      console.log('Skipping timezone test: system timezone is UTC, cannot detect UTC vs local bug');
      // Disconnect and return
      await mainPage.locator('button:has-text("Disconnect")').click();
      return;
    }

    // The timestamp SHOULD match local time, NOT UTC
    // If it matches UTC but not local time, the bug is present
    if (isMatchingUTC && !isWithinLocalTimeRange) {
      // This is the bug we're testing for!
      throw new Error(
        `Timestamp is displayed in UTC instead of local timezone!\n` +
        `  Displayed: ${displayedTime}\n` +
        `  Expected (local): ${expectedLocalHours.toString().padStart(2, '0')}:${expectedLocalMinutes.toString().padStart(2, '0')}:xx\n` +
        `  Bug shows (UTC): ${utcHours.toString().padStart(2, '0')}:${utcMinutes.toString().padStart(2, '0')}:xx\n` +
        `  Timezone offset: ${-localTime.getTimezoneOffset() / 60} hours from UTC`
      );
    }

    // Assert that timestamp matches local time
    expect(isWithinLocalTimeRange).toBeTruthy();

    // Disconnect
    await mainPage.locator('button:has-text("Disconnect")').click();
  });

  test('should display consistent timestamps across multiple messages', async () => {
    // This test verifies that all messages use the same timezone (local)

    // Enable timestamp display
    const timestampToggle = mainPage.locator('label:has-text("時刻") input[type="checkbox"]');
    if (!(await timestampToggle.isChecked())) {
      await timestampToggle.check();
    }

    // Connect to stream
    const urlInput = mainPage.locator('input[placeholder*="YouTube URL"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
    await mainPage.locator('button:has-text("Connect")').click();
    await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

    // Add multiple messages with short delays
    for (let i = 1; i <= 3; i++) {
      await addMockMessageWithTimestamp({
        message_type: 'text',
        author: `ConsistencyUser${i}`,
        content: `Consistency test message ${i}`,
      });
      await mainPage.waitForTimeout(500);
    }

    // Wait for all messages
    await mainPage.waitForTimeout(3000);

    // Get all displayed timestamps
    const timestamps: string[] = [];
    for (let i = 1; i <= 3; i++) {
      const messageElement = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=ConsistencyUser${i}`)
      }).first();

      const timestampElement = messageElement.locator('span').filter({
        hasText: /^\d{2}:\d{2}:\d{2}$/
      }).first();

      if (await timestampElement.isVisible()) {
        const ts = await timestampElement.textContent();
        if (ts) timestamps.push(ts);
      }
    }

    console.log(`Collected timestamps: ${timestamps.join(', ')}`);

    // All timestamps should have the same hour (unless crossing hour boundary)
    // This verifies they're all using the same timezone
    if (timestamps.length >= 2) {
      const hours = timestamps.map(ts => parseInt(ts.split(':')[0]));
      const firstHour = hours[0];

      // Allow for hour boundary crossing (e.g., 23 -> 00)
      const allSameTimezone = hours.every(h =>
        h === firstHour ||
        Math.abs(h - firstHour) === 1 ||
        (firstHour === 23 && h === 0) ||
        (firstHour === 0 && h === 23)
      );

      expect(allSameTimezone).toBeTruthy();
    }

    // Disconnect
    await mainPage.locator('button:has-text("Disconnect")').click();
  });
});
