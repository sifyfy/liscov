import { test, expect, chromium, BrowserContext, Page, Browser } from '@playwright/test';
import { exec, execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

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

const CDP_URL = 'http://127.0.0.1:9222';
const MOCK_SERVER_URL = 'http://localhost:3456';
const PROJECT_DIR = process.cwd().replace(/[\\/]e2e-tauri$/, '');

// Test isolation: use separate namespace for credentials and data
const TEST_APP_NAME = 'liscov-test';
const TEST_KEYRING_SERVICE = 'liscov-test';

// Get test config directory based on platform
function getTestConfigDir(): string {
  const configDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.config');

  return path.join(configDir!, TEST_APP_NAME);
}

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

  console.log(`Starting Tauri app with test namespace: ${TEST_APP_NAME}`);

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

    // ページが読み込まれるのを待つ
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000); // UI安定化待機

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
    expect(savedFontSize).toBe(16);

    await browser.close();
    await killTauriApp();

    // ============================================
    // Phase 2: 再起動、設定が維持されていることを確認
    // ============================================
    await startTauriApp();
    const { browser: browser2, page: page2 } = await connectToApp();

    // ページが読み込まれるのを待つ
    await page2.waitForLoadState('networkidle');
    await page2.waitForTimeout(2000); // UI安定化待機

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
    await page.waitForTimeout(2000);

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
