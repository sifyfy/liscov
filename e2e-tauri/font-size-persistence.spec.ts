import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
import { exec } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
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
 * E2E tests for Font Size Persistence based on 09_config.md specification.
 *
 * Tests verify:
 * - Font size changes are saved to config.toml
 * - Font size is restored after app restart
 *
 * Run tests:
 *    pnpm exec playwright test --config e2e-tauri/playwright.config.ts font-size-persistence.spec.ts
 */

// Get test config directory based on platform
function getTestConfigDir(): string {
  const configDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.config');

  return path.join(configDir!, TEST_APP_NAME);
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

// Read config.toml and return message_font_size value
function readConfigFontSize(): number | null {
  const configPath = path.join(getTestConfigDir(), 'config.toml');
  if (!fs.existsSync(configPath)) {
    return null;
  }
  const content = fs.readFileSync(configPath, 'utf-8');
  const match = content.match(/message_font_size\s*=\s*(\d+)/);
  return match ? parseInt(match[1], 10) : null;
}

test.describe('Font Size Persistence', () => {
  // アプリ2回起動が必要なテストがあるためタイムアウトを延長
  test.setTimeout(180000);

  test.beforeAll(async () => {
    // Clean up before tests
    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test.afterAll(async () => {
    // Clean up after tests
    await killTauriApp();
  });

  test('文字サイズ変更が再起動後も維持される', async () => {
    // ============================================
    // Phase 1: 初回起動、文字サイズ変更
    // ============================================
    await startTauriApp();
    const { browser, page } = await connectToApp();

    // Svelteアプリのマウント完了を待つ
    await page.waitForLoadState('networkidle');
    await page.getByRole('heading', { name: 'Chat Monitor' }).waitFor({ state: 'visible', timeout: 30000 });

    // 初期フォントサイズを確認 (13px)
    const fontSizeDisplay = page.locator('.text-xs.text-center').filter({ hasText: /\d+px/ });
    await expect(fontSizeDisplay).toHaveText('13px');

    // A+ボタンを3回クリック (13→16px)
    const increaseButton = page.getByTitle('文字を大きく');
    for (let i = 0; i < 3; i++) {
      await increaseButton.click();
      await page.waitForTimeout(100); // クリック間隔
    }

    // 変更後のサイズを確認
    await expect(fontSizeDisplay).toHaveText('16px');

    // 設定が保存されるまで少し待機
    await page.waitForTimeout(500);

    // config.toml に保存されていることを確認
    const savedFontSize = readConfigFontSize();
    log.debug(`Saved font size: ${savedFontSize}`);
    expect(savedFontSize).toBe(16);

    await browser.close();
    await killTauriApp();

    // ============================================
    // Phase 2: 再起動、設定が維持されていることを確認
    // ============================================
    await startTauriApp();
    const { browser: browser2, page: page2 } = await connectToApp();

    // Svelteアプリのマウント完了を待つ
    await page2.waitForLoadState('networkidle');
    await page2.getByRole('heading', { name: 'Chat Monitor' }).waitFor({ state: 'visible', timeout: 30000 });

    // フォントサイズが維持されていることを確認
    const fontSizeDisplay2 = page2.locator('.text-xs.text-center').filter({ hasText: /\d+px/ });
    await expect(fontSizeDisplay2).toHaveText('16px');

    await browser2.close();
  });

  test('文字サイズの上限・下限が守られる', async () => {
    // Clean slate
    await killTauriApp();
    await cleanupTestData();

    await startTauriApp();
    const { browser, page } = await connectToApp();

    await page.waitForLoadState('networkidle');
    await page.getByRole('heading', { name: 'Chat Monitor' }).waitFor({ state: 'visible', timeout: 30000 });

    const fontSizeDisplay = page.locator('.text-xs.text-center').filter({ hasText: /\d+px/ });
    const increaseButton = page.getByTitle('文字を大きく');
    const decreaseButton = page.getByTitle('文字を小さく');

    // 上限テスト: 13px → 24px (11回クリック)
    for (let i = 0; i < 15; i++) {
      await increaseButton.click();
      await page.waitForTimeout(50);
    }
    await expect(fontSizeDisplay).toHaveText('24px'); // 上限は24px

    // 下限テスト: 24px → 10px (14回クリック)
    for (let i = 0; i < 20; i++) {
      await decreaseButton.click();
      await page.waitForTimeout(50);
    }
    await expect(fontSizeDisplay).toHaveText('10px'); // 下限は10px

    await browser.close();
  });
});
