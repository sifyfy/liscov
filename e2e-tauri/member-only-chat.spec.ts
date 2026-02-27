import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  addMockMessage,
  resetMockServer,
} from './utils/test-helpers';

/**
 * E2E tests for member-only stream chat access.
 *
 * Verifies that authenticated users can connect to member-only streams
 * and receive chat messages. This is a regression test for the bug where
 * connect_to_stream() did not pass auth cookies to initialize(),
 * causing connection failure on member-only streams.
 *
 * Run:
 *   pnpm exec playwright test --config e2e-tauri/playwright.config.ts e2e-tauri/member-only-chat.spec.ts
 */

// Helper to set stream state
async function setStreamState(state: {
  member_only?: boolean;
  require_auth?: boolean;
  title?: string;
}): Promise<void> {
  const response = await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(state),
  });
  if (!response.ok) {
    throw new Error(`Failed to set stream state: ${response.status}`);
  }
}

// Helper to set auth state on mock server
async function setAuthState(state: { session_valid?: boolean }): Promise<void> {
  const response = await fetch(`${MOCK_SERVER_URL}/set_auth_state`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(state),
  });
  if (!response.ok) {
    throw new Error(`Failed to set auth state: ${response.status}`);
  }
}

// Helper to fully disconnect and return to idle state
async function disconnectAndInitialize(page: Page): Promise<void> {
  const stopButton = page.locator('button:has-text("停止")');
  if (await stopButton.isVisible({ timeout: 1000 }).catch(() => false)) {
    await stopButton.click();
    await page.locator('button:has-text("初期化")').click();
    await expect(
      page.locator('input[placeholder*="youtube.com"]'),
    ).toBeVisible({ timeout: 5000 });
  }
}

test.describe('Member-Only Stream Chat Access', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000);

    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    // Step 1: ログインして認証状態にする
    await setAuthState({ session_valid: true });

    // Settings タブへ移動してログイン
    await mainPage.getByRole('button', { name: 'Settings' }).click();
    await expect(
      mainPage.getByRole('heading', { name: 'YouTube認証' }),
    ).toBeVisible();

    const loginButton = mainPage.getByRole('button', {
      name: 'YouTubeにログイン',
    });
    await expect(loginButton).toBeVisible();
    await loginButton.click();

    // ログイン完了を待つ
    const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
    await expect(logoutButton).toBeVisible({ timeout: 15000 });

    // 認証済み状態を確認
    const authIndicator = mainPage.getByTestId('auth-indicator');
    await expect(authIndicator).toContainText('認証済み', { timeout: 15000 });

    log.info('Login completed, ready for member-only stream tests');
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test.beforeEach(async () => {
    await resetMockServer();
    // 認証状態を再設定（resetで消えるため）
    await setAuthState({ session_valid: true });
  });

  test('should connect to member-only stream and receive messages when authenticated', async () => {
    // メンバー限定配信に設定
    await setStreamState({
      member_only: true,
      require_auth: true,
      title: 'メンバー限定配信テスト',
    });

    // Chat タブへ移動
    await mainPage.getByRole('button', { name: 'Chat' }).click();

    // ストリームURLを入力して接続
    const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
    await expect(urlInput).toBeVisible();
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=member_only_stream`);
    await mainPage.locator('button:has-text("開始")').click();

    // 接続成功: ストリームタイトルが表示される
    await expect(
      mainPage.getByText('メンバー限定配信テスト').first(),
    ).toBeVisible({ timeout: 10000 });

    // メッセージを追加
    await addMockMessage({
      message_type: 'text',
      author: 'MemberViewer',
      content: 'メンバー限定コメント',
      channel_id: 'UC_member_viewer',
      is_member: true,
    });

    // メッセージが表示されることを確認（ポーリング間隔1.5s）
    await expect(mainPage.locator('text=MemberViewer')).toBeVisible({
      timeout: 5000,
    });
    await expect(
      mainPage.locator('text=メンバー限定コメント'),
    ).toBeVisible();

    await disconnectAndInitialize(mainPage);
  });

  test('should still connect to normal (non-member-only) stream when authenticated', async () => {
    // 通常配信（require_auth=false）が認証済みセッションでも正常動作することを確認
    await setStreamState({
      member_only: false,
      require_auth: false,
      title: '通常配信テスト',
    });

    // Chat タブへ移動
    await mainPage.getByRole('button', { name: 'Chat' }).click();

    // ストリームURLを入力して接続
    const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
    await expect(urlInput).toBeVisible();
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=normal_stream`);
    await mainPage.locator('button:has-text("開始")').click();

    // 接続成功
    await expect(
      mainPage.getByText('通常配信テスト').first(),
    ).toBeVisible({ timeout: 10000 });

    // メッセージが受信できることを確認
    await addMockMessage({
      message_type: 'text',
      author: 'NormalViewer',
      content: '通常コメント',
      channel_id: 'UC_normal_viewer',
      is_member: false,
    });

    await expect(mainPage.locator('text=NormalViewer')).toBeVisible({
      timeout: 5000,
    });

    await disconnectAndInitialize(mainPage);
  });
});
