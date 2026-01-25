import { test, expect, chromium, BrowserContext, Page, Browser } from '@playwright/test';
import { exec, execSync, spawn, ChildProcess } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

const execAsync = promisify(exec);

/**
 * E2E tests for Authentication feature based on 01_auth.md specification.
 *
 * Tests verify the UI behavior specified in the frontend operation table:
 * - Initial unauthenticated state display
 * - Login button click -> WebView opens -> Login completes -> Logout button appears
 * - AuthIndicator transitions from "検証中" to "認証済み" after session validation
 * - Credential persistence across app restart
 * - Logout button click -> Credentials deleted -> Login button reappears
 *
 * Run tests (mock server and app will be started automatically):
 *    pnpm exec playwright test --config e2e-tauri/playwright.config.ts auth-flow.spec.ts
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

  // Config directory (Windows: %APPDATA%, macOS: ~/Library/Application Support, Linux: ~/.config)
  const configDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.config');

  if (configDir) {
    dirs.push(path.join(configDir, TEST_APP_NAME));
  }

  // Data directory (Windows: %APPDATA%, macOS: ~/Library/Application Support, Linux: ~/.local/share)
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
      // Delete credentials from Windows Credential Manager
      // The keyring crate stores credentials with target format: <user>.<service>
      execSync(`cmdkey /delete:youtube_credentials.${TEST_KEYRING_SERVICE} 2>nul`, { stdio: 'ignore' });
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
    // Mock server URLs
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
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

test.describe('Authentication Feature (01_auth.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000); // 4 minutes for setup (includes mock server build time)

    // Step 1: Kill any existing processes
    console.log('Killing any existing Tauri app...');
    await killTauriApp();

    // Step 2: Clean up test data and credentials for a fresh start
    console.log('Cleaning up test data and credentials...');
    await cleanupTestData();
    await cleanupTestCredentials();

    // Step 3: Start mock server
    await startMockServer();

    // Step 4: Reset mock server state and set to unauthenticated
    console.log('Resetting mock server state...');
    await resetMockServer();
    // Set auth state to unauthenticated so app starts in logged out state
    await fetch(`${MOCK_SERVER_URL}/set_auth_state`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_valid: false }),
    });

    // Step 5: Start Tauri app with test namespace
    console.log('Starting Tauri app with test namespace...');
    await startTauriApp();

    // Step 6: Connect to the running Tauri app
    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
    // Wait for page to be fully loaded and stable before accessing
    await mainPage.waitForLoadState('load');
    await mainPage.waitForTimeout(1000);
    console.log('Connected to Tauri app');
  });

  test.afterAll(async () => {
    // Clean up: close browser connection and kill Tauri app
    console.log('Cleaning up after tests...');
    if (browser) {
      await browser.close();
    }
    await killTauriApp();
    // Stop mock server
    await killMockServer();
    // Clean up test data
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test.beforeEach(async () => {
    // Ensure we're on the Settings page for auth tests
    await mainPage.getByRole('button', { name: 'Settings' }).click();
    // Wait for YouTube認証 section to be visible
    await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible();
  });

  test.describe('Initial State', () => {
    test('should show unauthenticated state on first launch', async () => {
      // Spec: "「YouTubeにログイン」ボタン should be visible when unauthenticated"
      const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
      await expect(loginButton).toBeVisible();
    });

    test('should display AuthIndicator with unauthenticated state', async () => {
      // Spec: "| 未認証 | グレー / 鍵アイコン（閉） | 「未ログイン」 |"
      const authIndicator = mainPage.getByTestId('auth-indicator');
      await expect(authIndicator).toContainText('未ログイン');
    });
  });

  test.describe('Login Flow', () => {
    test('should complete login via WebView and show logout button', async () => {
      // Spec: "「YouTubeにログイン」ボタンクリック → WebViewウィンドウが開き、YouTubeログイン画面が表示される"
      // Spec: "WebViewでログイン完了 → SAPISIDを検出し、認証情報をセキュアストレージに保存。ウィンドウ自動クローズ"

      // Click login button
      const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
      await expect(loginButton).toBeVisible();
      await loginButton.click();

      // Wait for auth window to complete login (mock server auto-completes)
      // After successful auth, logout button should appear
      const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible({ timeout: 15000 });

      // Login button should no longer be visible
      await expect(loginButton).not.toBeVisible();
    });

    test('should update AuthIndicator after login', async () => {
      // Spec: "| 認証済み（有効） | 緑 / 鍵アイコン（開） | 「ログイン中: 有効」 |"
      // After login, indicator should show "認証済み" (session validated successfully)
      // This verifies the full flow: login -> session validation -> indicator update
      const authIndicator = mainPage.getByTestId('auth-indicator');
      await expect(authIndicator).toContainText('認証済み', { timeout: 15000 });

      // "未ログイン" should not be visible in indicator
      await expect(authIndicator).not.toContainText('未ログイン');

      // "検証中" should not be visible (session validation should complete)
      await expect(authIndicator).not.toContainText('検証中');
    });
  });

  test.describe('Persistence', () => {
    test('should persist credentials and restore authenticated state after app restart', async () => {
      // Spec (アプリ起動時の動作):
      // "1. セキュアストレージから認証情報をロード"
      // "2. 認証情報が存在する場合、インジケーターを「検証中」に設定"
      // "4. 検証結果に応じてインジケーターを更新"

      // Ensure we're in authenticated state before restart
      const logoutButtonBefore = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButtonBefore).toBeVisible();

      // Close current connection
      await browser.close();

      // Kill and restart the Tauri app
      await killTauriApp();
      await startTauriApp();

      // Reconnect to the restarted app
      const connection = await connectToApp();
      browser = connection.browser;
      context = connection.context;
      mainPage = connection.page;

      // Navigate to Settings
      await mainPage.getByRole('button', { name: 'Settings' }).click();
      await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible();

      // Should be authenticated after restart (credentials loaded from secure storage)
      const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible({ timeout: 10000 });

      // Login button should NOT be visible
      const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
      await expect(loginButton).not.toBeVisible();

      // Indicator should show "認証済み" after session validation completes
      const authIndicator = mainPage.getByTestId('auth-indicator');
      await expect(authIndicator).toContainText('認証済み', { timeout: 15000 });

      // "検証中" should not be visible (session validation should complete)
      await expect(authIndicator).not.toContainText('検証中');
    });
  });

  test.describe('Logout Flow', () => {
    test('should logout and show login button again', async () => {
      // Spec: "「ログアウト」ボタンクリック → 認証情報削除、認証状態が「未認証」に変更"

      // Accept the confirmation dialog
      mainPage.on('dialog', dialog => dialog.accept());

      // Click logout button
      const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible();
      await logoutButton.click();

      // Wait for login button to reappear (indicates logout complete)
      const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
      await expect(loginButton).toBeVisible({ timeout: 10000 });

      // Logout button should no longer be visible
      await expect(logoutButton).not.toBeVisible();
    });

    test('should show unauthenticated AuthIndicator after logout', async () => {
      // Spec: "| 未認証 | グレー / 鍵アイコン（閉） | 「未ログイン」 |"
      const authIndicator = mainPage.getByTestId('auth-indicator');
      await expect(authIndicator).toContainText('未ログイン');
    });

    test('should require re-authentication after logout (cookies cleared)', async () => {
      // This test verifies that WebView cookies are cleared after logout
      // by confirming the mock server receives a fresh login page visit

      // Reset mock server state to track new login page visits
      await fetch('http://localhost:3456/reset', { method: 'POST' });

      // Verify login_page_visits is 0 after reset
      const initialStatus = await fetch('http://localhost:3456/status');
      const initialData = await initialStatus.json() as { login_page_visits: number };
      expect(initialData.login_page_visits).toBe(0);

      // Click login button again after logout
      const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
      await expect(loginButton).toBeVisible();
      await loginButton.click();

      // Wait for auth window to complete
      const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible({ timeout: 15000 });

      // Verify indicator shows authenticated state
      const authIndicator = mainPage.getByTestId('auth-indicator');
      await expect(authIndicator).toContainText('認証済み', { timeout: 15000 });

      // Verify mock server received a fresh login page visit
      // If cookies were NOT cleared, the auth window might have auto-detected
      // existing SAPISID without visiting the login page at all.
      // login_page_visits > 0 proves the auth window visited the login page,
      // which means cookies were properly cleared.
      const statusResponse = await fetch('http://localhost:3456/status');
      const status = await statusResponse.json() as { login_page_visits: number };
      console.log('Mock server status after re-login:', status);

      // Critical assertion: login page must have been visited
      expect(status.login_page_visits).toBeGreaterThan(0);
    });
  });
});
