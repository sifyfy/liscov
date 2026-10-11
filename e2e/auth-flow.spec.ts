import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  mockServerUrl,
  setupTestEnvironment,
  teardownTestEnvironment,
  killTauriApp,
  startTauriApp,
  connectToApp,
  resetMockServer,
  cleanupTestData,
  cleanupTestCredentials,
  navigateToTab,
  openControlPanel,
} from './utils/test-helpers';

/**
 * E2E tests for Authentication feature based on 01_auth.md specification.
 *
 * Tests verify the UI behavior specified in the frontend operation table:
 * - Initial unauthenticated state display
 * - Login button click -> WebView opens -> Login completes -> Logout button appears
 * - AuthIndicator transitions from "ログイン中: 検証中" to "ログイン中" after session validation
 * - Credential persistence across app restart
 * - Logout button click -> Credentials deleted -> Login button reappears
 *
 * Run tests (mock server and app will be started automatically):
 *    pnpm exec playwright test --config e2e/playwright.config.ts auth-flow.spec.ts
 */

test.describe('Authentication Feature (01_auth.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000); // 4 minutes for setup (includes mock server build time)

    // Set up test environment (mock server, Tauri app, etc.)
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    // Set auth state to unauthenticated so app starts in logged out state
    await fetch(`${mockServerUrl()}/set_auth_state`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_valid: false }),
    });
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  // AuthIndicator は操作パネル (チャットのタブ) の中にある。開いて返す
  async function openAuthIndicator() {
    await navigateToTab(mainPage, 'Chat');
    await openControlPanel(mainPage);
    return mainPage.getByRole('region', { name: '操作パネル' }).getByTestId('auth-indicator');
  }

  test.beforeEach(async () => {
    // Ensure we're on the Settings page for auth tests
    await mainPage.getByRole('button', { name: '設定', exact: true }).click();
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
      // Spec: "| 未認証 | 控えめ (fg-3) | 「未ログイン」 |"
      const authIndicator = await openAuthIndicator();
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
      // Spec: "| 認証済み（有効） | success | 「ログイン中」 |"
      // ログイン → セッションの検証 → 表示の更新までを通す。文字全体で比べる (「ログイン中: 検証中」と区別する)
      const authIndicator = await openAuthIndicator();
      await expect(authIndicator).toHaveText('ログイン中', { timeout: 15000 });
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
      await mainPage.getByRole('button', { name: '設定', exact: true }).click();
      await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible();

      // Should be authenticated after restart (credentials loaded from secure storage)
      const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible({ timeout: 10000 });

      // Login button should NOT be visible
      const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
      await expect(loginButton).not.toBeVisible();

      // 検証を終えると「ログイン中」(「ログイン中: 検証中」のままではない)
      const authIndicator = await openAuthIndicator();
      await expect(authIndicator).toHaveText('ログイン中', { timeout: 15000 });
    });
  });

  // spec: 01_auth.md「AuthIndicator.svelte」・02_chat.md「状態 (異常のときだけ目立つ)」
  test.describe('セッション切れ', () => {
    test('起動時の検証でセッション切れ → 操作パネルを閉じていても状態の行に出し、押すと設定のログインを開く', async () => {
      // 保存したログイン情報のまま、YouTube がセッションを無効と返すようにして起動し直す
      await fetch(`${mockServerUrl()}/set_auth_state`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ session_valid: false }),
      });
      await browser.close();
      await killTauriApp();
      await startTauriApp();
      const connection = await connectToApp();
      browser = connection.browser;
      context = connection.context;
      mainPage = connection.page;

      // 起動したらチャットのタブで、操作パネルは閉じている
      await expect(mainPage.getByRole('region', { name: '操作パネル' })).toHaveCount(0);
      const alert = mainPage.getByTestId('status-alert');
      await expect(alert).toHaveText('ログイン: セッション切れ', { timeout: 15000 });

      await alert.click();
      await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible();

      // 操作パネルの中では全文を出す
      const authIndicator = await openAuthIndicator();
      await expect(authIndicator).toHaveText('ログイン: セッション切れ。再ログインが必要です');
    });
  });

  test.describe('Logout Flow', () => {
    test('should logout and show login button again', async () => {
      // Spec: "「ログアウト」ボタンクリック → 認証情報削除、認証状態が「未認証」に変更"

      // Accept the confirmation dialog
      mainPage.on('dialog', (dialog) => dialog.accept());

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
      // Spec: "| 未認証 | 控えめ (fg-3) | 「未ログイン」 |"
      const authIndicator = await openAuthIndicator();
      await expect(authIndicator).toContainText('未ログイン');
    });

    test('should require re-authentication after logout (cookies cleared)', async () => {
      // This test verifies that WebView cookies are cleared after logout
      // by confirming the mock server receives a fresh login page visit

      // Reset mock server state to track new login page visits
      await fetch(`${mockServerUrl()}/reset`, { method: 'POST' });

      // Verify login_page_visits is 0 after reset
      const initialStatus = await fetch(`${mockServerUrl()}/status`);
      const initialData = (await initialStatus.json()) as { login_page_visits: number };
      expect(initialData.login_page_visits).toBe(0);

      // Click login button again after logout
      const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
      await expect(loginButton).toBeVisible();
      await loginButton.click();

      // Wait for auth window to complete
      const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible({ timeout: 15000 });

      // Verify indicator shows authenticated state
      const authIndicator = await openAuthIndicator();
      await expect(authIndicator).toHaveText('ログイン中', { timeout: 15000 });

      // Verify mock server received a fresh login page visit
      // If cookies were NOT cleared, the auth window might have auto-detected
      // existing SAPISID without visiting the login page at all.
      // login_page_visits > 0 proves the auth window visited the login page,
      // which means cookies were properly cleared.
      const statusResponse = await fetch(`${mockServerUrl()}/status`);
      const status = (await statusResponse.json()) as { login_page_visits: number };
      log.debug('Mock server status after re-login', status);

      // Critical assertion: login page must have been visited
      expect(status.login_page_visits).toBeGreaterThan(0);
    });
  });
});
