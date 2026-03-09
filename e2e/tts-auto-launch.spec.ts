import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import { log } from './utils/logger';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  ensureSvelteHydrated,
  startTauriApp,
  connectToApp,
  killTauriApp,
  cleanupTestData,
  cleanupTestCredentials,
} from './utils/test-helpers';

// Helper to wait for SvelteKit app to fully render (not just HTML load)
async function waitForAppReady(page: Page): Promise<void> {
  await ensureSvelteHydrated(page);
}

/**
 * E2E tests for TTS VOICEVOX auto-launch feature based on 04_tts.md specification.
 *
 * Prerequisites:
 * - VOICEVOX must be installed at %LOCALAPPDATA%\Programs\VOICEVOX\VOICEVOX.exe
 *
 * Tests verify:
 * - Auto-detect executable path via "検出" button
 * - Manual launch/stop via buttons
 * - Auto-launch on app restart
 * - Auto-close on app exit
 * - Duplicate launch prevention
 * - Error display when launch fails
 *
 * Run tests:
 *    pnpm exec playwright test --config e2e/playwright.config.ts tts-auto-launch.spec.ts
 */

// Get VOICEVOX standard installation path
function getVoicevoxPath(): string | null {
  const localAppData = process.env.LOCALAPPDATA;
  if (!localAppData) return null;
  const voicevoxPath = path.join(localAppData, 'Programs', 'VOICEVOX', 'VOICEVOX.exe');
  return fs.existsSync(voicevoxPath) ? voicevoxPath : null;
}

// Check if VOICEVOX process is running
function isVoicevoxRunning(): boolean {
  try {
    const result = execSync('tasklist /FI "IMAGENAME eq VOICEVOX.exe" /NH', { encoding: 'utf8' });
    return result.includes('VOICEVOX.exe');
  } catch {
    return false;
  }
}

// Kill all VOICEVOX processes
function killVoicevox(): void {
  try {
    execSync('taskkill /F /IM VOICEVOX.exe 2>nul', { stdio: 'ignore' });
  } catch {
    // Process may not exist, which is fine
  }
}

// Helper to gracefully close Tauri app (triggers ExitRequested event)
async function closeTauriAppGracefully(): Promise<void> {
  try {
    if (process.platform === 'win32') {
      // Send WM_CLOSE message instead of force kill
      execSync('taskkill /IM liscov-tauri.exe 2>nul', { stdio: 'ignore' });
      // Wait for graceful shutdown
      await new Promise(resolve => setTimeout(resolve, 3000));
      // Force kill if still running
      execSync('taskkill /F /IM liscov-tauri.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -TERM -f liscov-tauri', { stdio: 'ignore' });
      await new Promise(resolve => setTimeout(resolve, 3000));
      execSync('pkill -KILL -f liscov-tauri', { stdio: 'ignore' });
    }
  } catch {
    // Process may not exist
  }
  // Wait for port to be released
  await new Promise(resolve => setTimeout(resolve, 1000));
}

// Navigate to TTS settings tab
async function navigateToTtsSettings(page: Page): Promise<void> {
  // Click Settings tab in main navigation
  await page.getByRole('button', { name: 'Settings' }).click();
  // Wait for settings sidebar button to appear (condition-based instead of fixed timeout)
  await expect(page.getByRole('button', { name: 'TTS読み上げ' })).toBeVisible({ timeout: 5000 });
  // Click TTS tab in the settings sidebar (it's a button, not a link)
  await page.getByRole('button', { name: 'TTS読み上げ' }).click();
  // Wait for TTS settings to be visible
  await expect(page.getByRole('heading', { name: 'TTS設定' })).toBeVisible({ timeout: 5000 });
}

// Select VOICEVOX backend from dropdown
async function selectVoicevoxBackend(page: Page): Promise<void> {
  const select = page.locator('#backend');
  await select.selectOption('voicevox');
  // Wait for VOICEVOX settings section to appear
  await expect(page.getByText('VOICEVOX設定')).toBeVisible({ timeout: 5000 });
}

// Wait for VOICEVOX process to start (with timeout)
async function waitForVoicevoxToStart(timeoutMs = 30000): Promise<boolean> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (isVoicevoxRunning()) {
      return true;
    }
    await new Promise(resolve => setTimeout(resolve, 300)); // Reduced from 1000ms for faster detection
  }
  return false;
}

// Wait for VOICEVOX process to stop (with timeout)
async function waitForVoicevoxToStop(timeoutMs = 10000): Promise<boolean> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (!isVoicevoxRunning()) {
      return true;
    }
    await new Promise(resolve => setTimeout(resolve, 300)); // Reduced from 500ms for faster detection
  }
  return false;
}

test.describe('TTS VOICEVOX Auto-Launch', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  // Check VOICEVOX installation before running any tests
  test.beforeAll(async () => {
    test.setTimeout(300000); // 5 minutes for setup

    const voicevoxPath = getVoicevoxPath();
    if (!voicevoxPath) {
      log.error('='.repeat(60));
      log.error('ERROR: VOICEVOXがインストールされていません');
      log.error('このテストを実行するにはVOICEVOXを以下にインストールしてください:');
      log.error('%LOCALAPPDATA%\\Programs\\VOICEVOX\\VOICEVOX.exe');
      log.error('Expected path:', { path: path.join(process.env.LOCALAPPDATA || '', 'Programs', 'VOICEVOX', 'VOICEVOX.exe') });
      log.error('='.repeat(60));
      test.skip();
      return;
    }

    log.info(`VOICEVOX found at: ${voicevoxPath}`);

    // Step 1: Kill any existing processes
    log.info('Killing any existing Tauri app and VOICEVOX...');
    await killTauriApp();
    killVoicevox();

    // Step 2: Clean up test data for a fresh start
    log.info('Cleaning up test data and credentials...');
    await cleanupTestData();
    await cleanupTestCredentials();

    // Step 3: Start Tauri app with test namespace
    log.info('Starting Tauri app with test namespace...');
    await startTauriApp();

    // Step 4: Connect to the running Tauri app
    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    // Wait for SvelteKit app to fully render
    await waitForAppReady(mainPage);
    log.info('Connected to Tauri app');
  });

  test.afterAll(async () => {
    log.info('Cleaning up after tests...');

    // Kill VOICEVOX if running
    killVoicevox();

    if (browser) {
      await browser.close();
    }
    await killTauriApp();

    // Clean up test data
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test.beforeEach(async () => {
    // ページ接続が有効か確認、無効なら再接続
    let needsReconnect = false;
    try {
      await mainPage.evaluate(() => document.readyState);
      // ページは存在するが、ロード状態を確認
      await mainPage.waitForLoadState('load', { timeout: 5000 });
    } catch {
      needsReconnect = true;
    }

    if (needsReconnect) {
      log.info('Page connection lost, attempting to reconnect...');
      try {
        const connection = await connectToApp();
        browser = connection.browser;
        context = connection.context;
        mainPage = connection.page;
        log.info('Reconnected to Tauri app');
      } catch (e) {
        log.error('Failed to reconnect:', { error: e });
        throw e;
      }
    }

    // Wait for SvelteKit app to fully render before interacting
    await waitForAppReady(mainPage);

    // Navigate to TTS settings
    // Note: We don't kill VOICEVOX externally here because it desynchronizes app state
    // Instead, tests should manage VOICEVOX state through the UI
    await navigateToTtsSettings(mainPage);
  });

  test('should verify VOICEVOX is installed', async () => {
    // This test verifies the installation check works
    // If we reach here, VOICEVOX IS installed (otherwise beforeAll would have skipped)
    const voicevoxPath = getVoicevoxPath();
    expect(voicevoxPath).not.toBeNull();
    log.info(`VOICEVOX is installed at: ${voicevoxPath}`);
  });

  test('should auto-detect VOICEVOX executable path via detect button', async () => {
    // Select VOICEVOX backend
    await selectVoicevoxBackend(mainPage);

    // Find the VOICEVOX exe path input (it's the only one visible when VOICEVOX is selected)
    const exePathInput = mainPage.locator('input[placeholder="自動検出または参照で指定"]');

    // Clear any existing path first
    await exePathInput.fill('');

    // Click the detect button
    await mainPage.getByRole('button', { name: '検出' }).click();

    // Wait for path to be auto-detected
    await expect(exePathInput).not.toHaveValue('', { timeout: 5000 });

    // Verify the path contains VOICEVOX
    const detectedPath = await exePathInput.inputValue();
    expect(detectedPath.toLowerCase()).toContain('voicevox');
    log.info(`Auto-detected path: ${detectedPath}`);
  });

  test('should launch VOICEVOX manually via button', async () => {
    // Select VOICEVOX backend
    await selectVoicevoxBackend(mainPage);

    // Ensure initial state shows "停止中" (stopped)
    await expect(mainPage.getByText('停止中')).toBeVisible();

    // Find and click the launch button
    const launchButton = mainPage.getByRole('button', { name: '起動' });
    await launchButton.click();

    // Wait for VOICEVOX process to start
    const started = await waitForVoicevoxToStart();
    expect(started).toBe(true);

    // Verify UI shows "起動中" (running)
    await expect(mainPage.getByText('起動中')).toBeVisible({ timeout: 5000 });

    // Verify button text changed to "停止" (stop)
    await expect(mainPage.getByRole('button', { name: '停止' })).toBeVisible();
  });

  test('should stop VOICEVOX manually via button', async () => {
    // Select VOICEVOX backend
    await selectVoicevoxBackend(mainPage);

    // First, ensure VOICEVOX is launched (via UI)
    const launchButton = mainPage.getByRole('button', { name: '起動' });
    const stopButton = mainPage.getByRole('button', { name: '停止' });

    // If "起動" button is visible, click it to launch
    if (await launchButton.isVisible({ timeout: 1000 }).catch(() => false)) {
      await launchButton.click();
      await waitForVoicevoxToStart();
      // Wait for UI to update to show "停止" button
      await expect(stopButton).toBeVisible({ timeout: 10000 });
    }

    // Now VOICEVOX should be running
    expect(isVoicevoxRunning()).toBe(true);

    // Click the stop button
    await expect(stopButton).toBeVisible({ timeout: 5000 });
    await stopButton.click();

    // Wait for VOICEVOX process to stop
    const stopped = await waitForVoicevoxToStop();
    expect(stopped).toBe(true);

    // Verify UI shows "停止中"
    await expect(mainPage.getByText('停止中')).toBeVisible({ timeout: 5000 });

    // Verify button text changed back to "起動"
    await expect(launchButton).toBeVisible();
  });

  test('should auto-launch VOICEVOX on app restart', async () => {
    // このテストはアプリ再起動を伴うため、タイムアウトを延長
    test.setTimeout(180000); // 3分

    // Step 1: Configure auto-launch
    await selectVoicevoxBackend(mainPage);

    // First, ensure VOICEVOX is stopped via UI if running
    const stopButton = mainPage.getByRole('button', { name: '停止' });
    if (await stopButton.isVisible({ timeout: 1000 }).catch(() => false)) {
      await stopButton.click();
      await waitForVoicevoxToStop();
    }

    // Find and enable auto-launch toggle using data-testid for reliability
    const autoLaunchToggle = mainPage.locator('[data-testid="voicevox-auto-launch-toggle"]');
    await expect(autoLaunchToggle).toBeVisible({ timeout: 5000 });
    const isPressed = await autoLaunchToggle.getAttribute('aria-pressed');
    if (isPressed !== 'true') {
      await autoLaunchToggle.click();
      // Wait for toggle state to update
      await expect(autoLaunchToggle).toHaveAttribute('aria-pressed', 'true', { timeout: 3000 });
    }

    // Kill any running VOICEVOX process (external kill is okay here since we're restarting app anyway)
    killVoicevox();
    await waitForVoicevoxToStop();

    // Step 2: Restart the app
    await browser.close();
    await killTauriApp();
    await startTauriApp();

    // Reconnect
    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    // Step 3: Verify VOICEVOX was auto-launched
    // Wait a bit for auto-launch to trigger (VOICEVOXは通常10-20秒で起動)
    const autoLaunched = await waitForVoicevoxToStart(45000);
    expect(autoLaunched).toBe(true);

    // ページ接続が有効か確認、無効なら再接続
    try {
      await mainPage.evaluate(() => document.readyState);
    } catch {
      log.info('Page connection lost after VOICEVOX check, reconnecting...');
      const reconnection = await connectToApp();
      browser = reconnection.browser;
      context = reconnection.context;
      mainPage = reconnection.page;
    }

    // Wait for SvelteKit app to fully render after restart
    await waitForAppReady(mainPage);

    // Also verify UI shows correct state
    await navigateToTtsSettings(mainPage);
    await selectVoicevoxBackend(mainPage);
    await expect(mainPage.getByText('起動中')).toBeVisible({ timeout: 10000 });
  });

  test('should auto-close VOICEVOX on app exit', async () => {
    // Step 1: Configure settings
    await selectVoicevoxBackend(mainPage);

    // Ensure auto-close toggle is enabled using data-testid for reliability
    const autoCloseToggle = mainPage.locator('[data-testid="voicevox-auto-close-toggle"]');
    await expect(autoCloseToggle).toBeVisible({ timeout: 5000 });
    const isPressed = await autoCloseToggle.getAttribute('aria-pressed');
    if (isPressed !== 'true') {
      await autoCloseToggle.click();
      // Wait for toggle state to update
      await expect(autoCloseToggle).toHaveAttribute('aria-pressed', 'true', { timeout: 3000 });
    }

    // Step 2: Launch VOICEVOX manually via UI
    const launchButton = mainPage.getByRole('button', { name: '起動' });
    const stopButton = mainPage.getByRole('button', { name: '停止' });

    // If already running, that's fine. If not, launch it.
    if (await launchButton.isVisible({ timeout: 1000 }).catch(() => false)) {
      await launchButton.click();
      await waitForVoicevoxToStart();
      await expect(stopButton).toBeVisible({ timeout: 10000 });
    }

    // Verify VOICEVOX is running
    expect(isVoicevoxRunning()).toBe(true);

    // Step 3: Exit the app gracefully (triggers ExitRequested event which triggers auto-close)
    await browser.close();
    await closeTauriAppGracefully();

    // Step 4: Verify VOICEVOX was auto-closed
    // Wait a bit for auto-close to take effect
    const stopped = await waitForVoicevoxToStop(15000);
    expect(stopped).toBe(true);

    // Restart app for next tests
    await startTauriApp();
    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
    await waitForAppReady(mainPage);
  });

  test('should prevent duplicate launch (button state)', async () => {
    // Select VOICEVOX backend
    await selectVoicevoxBackend(mainPage);

    // Launch VOICEVOX via UI
    const launchButton = mainPage.getByRole('button', { name: '起動' });
    const stopButton = mainPage.getByRole('button', { name: '停止' });

    // If not already running, launch it
    if (await launchButton.isVisible({ timeout: 1000 }).catch(() => false)) {
      await launchButton.click();
      await waitForVoicevoxToStart();
      await expect(stopButton).toBeVisible({ timeout: 10000 });
    }

    // Verify VOICEVOX is running and button shows "停止"
    expect(isVoicevoxRunning()).toBe(true);
    await expect(stopButton).toBeVisible();

    // The "起動" button should not be visible when already launched
    // This is the UI-level duplicate prevention
    await expect(launchButton).not.toBeVisible();

    // Count VOICEVOX processes - should be at least 1
    try {
      const result = execSync('tasklist /FI "IMAGENAME eq VOICEVOX.exe" /FO CSV /NH', { encoding: 'utf8' });
      const lines = result.trim().split('\n').filter(line => line.includes('VOICEVOX.exe'));
      log.debug(`VOICEVOX process count: ${lines.length}`);
      expect(lines.length).toBeGreaterThan(0);
    } catch {
      // If tasklist fails, the button state check is sufficient
    }
  });

  test('should show error when launch fails with invalid path', async () => {
    // Kill any VOICEVOX process externally first to ensure clean state
    killVoicevox();
    await waitForVoicevoxToStop();

    // Refresh the page to get updated state
    await mainPage.reload();
    await mainPage.waitForLoadState('load');

    // Navigate to TTS settings
    await navigateToTtsSettings(mainPage);

    // Select VOICEVOX backend
    await selectVoicevoxBackend(mainPage);

    // Verify launch button is visible (VOICEVOX should be stopped)
    const launchButton = mainPage.getByRole('button', { name: '起動' });
    const stopButton = mainPage.getByRole('button', { name: '停止' });

    // If still showing "停止", stop the backend first
    if (await stopButton.isVisible({ timeout: 1000 }).catch(() => false)) {
      await stopButton.click();
      await expect(launchButton).toBeVisible({ timeout: 5000 });
    }

    // Find the VOICEVOX exe path input (use .last() since Bouyomichan has the same placeholder)
    const exePathInput = mainPage.locator('input[placeholder="自動検出または参照で指定"]').last();

    // Set invalid path using pressSequentially to ensure proper Svelte state sync
    await exePathInput.click();
    await exePathInput.fill('');
    const invalidPath = 'C:\\invalid\\nonexistent\\VOICEVOX.exe';
    await exePathInput.pressSequentially(invalidPath, { delay: 10 });
    await exePathInput.blur();

    // Wait for debounced config save
    await mainPage.waitForTimeout(1000);

    // Verify the path was set
    expect(await exePathInput.inputValue()).toBe(invalidPath);

    // Click launch button to trigger the error
    await expect(launchButton).toBeVisible({ timeout: 5000 });
    await launchButton.click();

    // Wait for error to be set
    await mainPage.waitForTimeout(2000);

    // Scroll to bottom to see the error display
    await mainPage.evaluate(() => {
      const container = document.querySelector('.overflow-y-auto');
      if (container) {
        container.scrollTop = container.scrollHeight;
      }
    });

    // Verify error message is displayed (CSS variable-based theme classes)
    const errorDisplay = mainPage.locator('div.rounded-lg p.text-sm').filter({ hasText: /not found|見つかりません|失敗|executable/i });
    await expect(errorDisplay).toBeVisible({ timeout: 10000 });
  });
});
