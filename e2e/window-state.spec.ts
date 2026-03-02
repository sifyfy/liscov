import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
import { exec } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  CDP_URL,
  PROJECT_DIR,
  TEST_APP_NAME,
  TEST_KEYRING_SERVICE,
  killTauriApp,
  cleanupTestData,
  cleanupTestCredentials,
  waitForCDP,
  connectToApp,
} from './utils/test-helpers';

/**
 * E2E tests for Window State Persistence based on 10_window_state.md specification.
 *
 * Tests verify:
 * - Window size is saved and restored after app restart
 * - Window position is saved and restored after app restart
 *
 * Run tests:
 *    pnpm exec playwright test --config e2e/playwright.config.ts window-state.spec.ts
 */

// Window state file uses the app identifier from tauri.conf.json
const WINDOW_STATE_APP_ID = 'com.liscov-tauri.app';

// Get test config directory based on platform
function getTestConfigDir(): string {
  const configDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.config');

  return path.join(configDir!, TEST_APP_NAME);
}

// Get window state file path (uses app identifier, not LISCOV_APP_NAME)
function getWindowStateFilePath(): string {
  const configDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.config');

  return path.join(configDir!, WINDOW_STATE_APP_ID, '.window-state.json');
}

// Clean up test data directories (extended to include window state file)
async function cleanupTestDataWithWindowState(): Promise<void> {
  await cleanupTestData();

  // Also clean up window state file
  const windowStateFile = getWindowStateFilePath();
  if (fs.existsSync(windowStateFile)) {
    log.debug(`Cleaning up window state file: ${windowStateFile}`);
    fs.unlinkSync(windowStateFile);
  }
}

// Helper to close Tauri app gracefully via window close
async function closeTauriAppGracefully(page: Page): Promise<void> {
  try {
    await page.evaluate(async () => {
      // @ts-ignore - Tauri internal global
      const invoke = window.__TAURI_INTERNALS__.invoke;
      await invoke('plugin:window|close', { label: 'main' });
    });
  } catch {
    // Window may already be closed
  }
  // Wait for app to fully exit and release port
  await new Promise(resolve => setTimeout(resolve, 2000));
}

// Helper to start Tauri app with test isolation
async function startTauriApp(): Promise<void> {
  const env = {
    ...process.env,
    // Test isolation: use separate namespace
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    // Mock server URLs (needed for app to start without errors)
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
    // Enable CDP for Playwright
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  };

  log.info(`Starting Tauri app with test namespace: ${TEST_APP_NAME}`);

  // Start app in background
  exec(`cd "${PROJECT_DIR}" && pnpm tauri dev`, { env });

  // Wait for CDP to be available
  await waitForCDP();
}

// Read window state file and return the stored values
function readWindowState(): { width?: number; height?: number; x?: number; y?: number } | null {
  const windowStateFile = getWindowStateFilePath();
  if (!fs.existsSync(windowStateFile)) {
    log.debug('Window state file does not exist');
    return null;
  }
  try {
    const content = fs.readFileSync(windowStateFile, 'utf-8');
    const state = JSON.parse(content);
    return state.main || null;
  } catch (e) {
    log.debug(`Failed to parse window state file: ${e}`);
    return null;
  }
}

// Get window bounds using Tauri invoke (via __TAURI_INTERNALS__)
async function getWindowBounds(page: Page): Promise<{ width: number; height: number; x: number; y: number }> {
  return await page.evaluate(async () => {
    // @ts-ignore - Tauri internal global
    const invoke = window.__TAURI_INTERNALS__.invoke;
    // Use Tauri's internal plugin command to get window position and size
    const position = await invoke('plugin:window|inner_position', { label: 'main' });
    const size = await invoke('plugin:window|inner_size', { label: 'main' });
    return {
      width: size.width,
      height: size.height,
      x: position.x,
      y: position.y,
    };
  });
}

// Set window bounds using Tauri invoke (via __TAURI_INTERNALS__)
async function setWindowBounds(
  page: Page,
  bounds: { width?: number; height?: number; x?: number; y?: number }
): Promise<void> {
  await page.evaluate(async (b) => {
    // @ts-ignore - Tauri internal global
    const invoke = window.__TAURI_INTERNALS__.invoke;
    if (b.width !== undefined && b.height !== undefined) {
      await invoke('plugin:window|set_size', {
        label: 'main',
        value: { Logical: { width: b.width, height: b.height } }
      });
    }
    if (b.x !== undefined && b.y !== undefined) {
      await invoke('plugin:window|set_position', {
        label: 'main',
        value: { Logical: { x: b.x, y: b.y } }
      });
    }
  }, bounds);
}

test.describe('Window State Persistence', () => {
  // アプリ2回起動が必要なテストがあるためタイムアウトを延長
  test.setTimeout(180000);

  test.beforeAll(async () => {
    // Clean up before tests
    await killTauriApp();
    await cleanupTestDataWithWindowState();
    await cleanupTestCredentials();
  });

  test.afterAll(async () => {
    // Clean up after tests
    await killTauriApp();
  });

  test('ウィンドウサイズが再起動後も維持される', async () => {
    // ============================================
    // Phase 1: 初回起動、ウィンドウサイズ変更
    // ============================================
    await startTauriApp();
    const { browser, page } = await connectToApp();

    // ページが読み込まれるのを待つ
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000); // UI安定化待機

    // 初期ウィンドウサイズを確認
    const initialBounds = await getWindowBounds(page);
    log.debug('Initial window bounds:', { initialBounds });

    // ウィンドウサイズを変更 (1000x700)
    const newWidth = 1000;
    const newHeight = 700;
    await setWindowBounds(page, { width: newWidth, height: newHeight });
    await page.waitForTimeout(500); // サイズ変更が反映されるまで待機

    // 変更後のサイズを確認
    const changedBounds = await getWindowBounds(page);
    log.debug('Changed window bounds:', { changedBounds });
    expect(changedBounds.width).toBe(newWidth);
    expect(changedBounds.height).toBe(newHeight);

    // Close app gracefully to trigger window state save
    await closeTauriAppGracefully(page);
    await browser.close();

    // 設定ファイルが保存されていることを確認
    const savedState = readWindowState();
    log.debug('Saved window state:', { savedState });
    expect(savedState).not.toBeNull();
    expect(savedState?.width).toBe(newWidth);
    expect(savedState?.height).toBe(newHeight);

    // ============================================
    // Phase 2: 再起動、サイズが維持されていることを確認
    // ============================================
    await startTauriApp();
    const { browser: browser2, page: page2 } = await connectToApp();

    // ページが読み込まれるのを待つ
    await page2.waitForLoadState('networkidle');
    await page2.waitForTimeout(2000); // UI安定化待機

    // ウィンドウサイズが維持されていることを確認
    const restoredBounds = await getWindowBounds(page2);
    log.debug('Restored window bounds:', { restoredBounds });
    expect(restoredBounds.width).toBe(newWidth);
    expect(restoredBounds.height).toBe(newHeight);

    await browser2.close();
  });

  test('ウィンドウ位置が再起動後も維持される', async () => {
    // Clean slate
    await killTauriApp();
    await cleanupTestDataWithWindowState();

    // ============================================
    // Phase 1: 初回起動、ウィンドウ位置変更
    // ============================================
    await startTauriApp();
    const { browser, page } = await connectToApp();

    // ページが読み込まれるのを待つ
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000); // UI安定化待機

    // ウィンドウ位置を変更 (x=150, y=150)
    const newX = 150;
    const newY = 150;
    await setWindowBounds(page, { x: newX, y: newY });
    await page.waitForTimeout(500); // 位置変更が反映されるまで待機

    // 変更後の位置を確認（ウィンドウ装飾の影響で数ピクセルずれる可能性あり）
    const changedBounds = await getWindowBounds(page);
    log.debug('Changed window bounds:', { changedBounds });
    expect(Math.abs(changedBounds.x - newX)).toBeLessThan(20);
    expect(Math.abs(changedBounds.y - newY)).toBeLessThan(50); // タイトルバー分のずれを許容

    // Close app gracefully to trigger window state save
    await closeTauriAppGracefully(page);
    await browser.close();

    // 設定ファイルが保存されていることを確認
    const savedState = readWindowState();
    log.debug('Saved window state:', { savedState });
    expect(savedState).not.toBeNull();
    // 保存された位置も許容範囲で確認
    expect(savedState?.x).toBeDefined();
    expect(savedState?.y).toBeDefined();

    // ============================================
    // Phase 2: 再起動、位置が維持されていることを確認
    // ============================================
    await startTauriApp();
    const { browser: browser2, page: page2 } = await connectToApp();

    // ページが読み込まれるのを待つ
    await page2.waitForLoadState('networkidle');
    await page2.waitForTimeout(2000); // UI安定化待機

    // ウィンドウ位置が維持されていることを確認（許容範囲あり）
    const restoredBounds = await getWindowBounds(page2);
    log.debug('Restored window bounds:', { restoredBounds });
    // 復元された位置が保存された位置と一致することを確認
    expect(Math.abs(restoredBounds.x - savedState!.x!)).toBeLessThan(20);
    expect(Math.abs(restoredBounds.y - savedState!.y!)).toBeLessThan(50);

    await browser2.close();
  });
});
