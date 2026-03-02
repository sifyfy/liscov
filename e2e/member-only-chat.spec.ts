import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  addMockMessage,
  resetMockServer,
  disconnectAndInitialize,
} from './utils/test-helpers';

/**
 * E2E tests for member-only stream chat access.
 *
 * Regression tests for:
 * 1. connect_to_stream() not passing auth cookies to initialize()
 * 2. Cookie domain scoping (YouTube vs Google domain cookies)
 * 3. raw_cookie_string storage and usage for full cookie auth
 * 4. /next API fallback when watch page doesn't return chat data
 *
 * Run:
 *   pnpm exec playwright test --config e2e/playwright.config.ts e2e/member-only-chat.spec.ts
 */

// Helper to set stream state
async function setStreamState(state: {
  member_only?: boolean;
  require_auth?: boolean;
  watch_force_no_chat?: boolean;
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

  test('should connect via /next API fallback when watch page returns no chat', async () => {
    // watch_force_no_chat: watchページがliveChatRendererを返さないケースをシミュレート
    // このとき /youtubei/v1/next APIフォールバックで接続が成功すべき
    await setStreamState({
      member_only: true,
      require_auth: true,
      watch_force_no_chat: true,
      title: 'APIフォールバックテスト',
    });

    // Chat タブへ移動
    await mainPage.getByRole('button', { name: 'Chat' }).click();

    // ストリームURLを入力して接続
    const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
    await expect(urlInput).toBeVisible();
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=fallback_stream`);
    await mainPage.locator('button:has-text("開始")').click();

    // 接続成功: /next APIフォールバック経由でストリームタイトルが表示される
    await expect(
      mainPage.getByText('APIフォールバックテスト').first(),
    ).toBeVisible({ timeout: 15000 });

    // メッセージが受信できることを確認
    await addMockMessage({
      message_type: 'text',
      author: 'FallbackViewer',
      content: 'フォールバック経由コメント',
      channel_id: 'UC_fallback_viewer',
      is_member: true,
    });

    await expect(mainPage.locator('text=FallbackViewer')).toBeVisible({
      timeout: 5000,
    });
    await expect(
      mainPage.locator('text=フォールバック経由コメント'),
    ).toBeVisible();

    await disconnectAndInitialize(mainPage);
  });

  test('should preserve all cookies through full pipeline (G1: completeness)', async () => {
    // G1: Cookie Pipeline Completeness — ブラウザが持つ全CookieがAPIリクエストに到達する
    // YouTubeが新Cookieを追加しても、パイプラインが全Cookieを保持すれば自動的に含まれる

    // メンバー限定配信に設定（認証Cookie必須）
    await setStreamState({
      member_only: true,
      require_auth: true,
      title: 'Cookie完全性テスト',
    });

    // Chat タブへ移動
    await mainPage.getByRole('button', { name: 'Chat' }).click();

    // ストリームURLを入力して接続
    const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
    await expect(urlInput).toBeVisible();
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=cookie_pipeline_test`);
    await mainPage.locator('button:has-text("開始")').click();

    // 接続成功を待つ
    await expect(
      mainPage.getByText('Cookie完全性テスト').first(),
    ).toBeVisible({ timeout: 10000 });

    // モックサーバーから実際に受信したCookieヘッダーを取得
    const cookieResponse = await fetch(`${MOCK_SERVER_URL}/last_request_cookies`);
    const lastCookies: { watch: string | null; next: string | null } = await cookieResponse.json();

    // /watch リクエストでCookieが送信されていることを確認
    expect(lastCookies.watch).not.toBeNull();
    const watchCookies = lastCookies.watch!;

    // 全8Cookieがパイプラインを通過していることを検証
    // 5基本Cookie
    expect(watchCookies).toContain('SID=');
    expect(watchCookies).toContain('HSID=');
    expect(watchCookies).toContain('SSID=');
    expect(watchCookies).toContain('APISID=');
    expect(watchCookies).toContain('SAPISID=');
    // 追加Cookie（member-only配信で必須）
    // NOTE: 本番では __Secure-1PSID だが、__Secure-* プレフィックスはHTTPS必須のため
    // HTTPモックでは SecurePSID で同等のパイプライン完全性を検証する
    expect(watchCookies).toContain('SecurePSID=');
    expect(watchCookies).toContain('YSC=');
    expect(watchCookies).toContain('VISITOR_INFO1_LIVE=');

    log.info(`Cookie pipeline verified: ${watchCookies.split(';').length} cookies received`);

    await disconnectAndInitialize(mainPage);
  });

  test('should fail to connect to member-only stream without authentication', async () => {
    // ログアウトしてから接続試行
    await mainPage.getByRole('button', { name: 'Settings' }).click();
    const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
    await expect(logoutButton).toBeVisible({ timeout: 5000 });
    await logoutButton.click();

    // ログアウト完了を待つ
    const loginButton = mainPage.getByRole('button', {
      name: 'YouTubeにログイン',
    });
    await expect(loginButton).toBeVisible({ timeout: 10000 });

    // メンバー限定配信に設定
    await setStreamState({
      member_only: true,
      require_auth: true,
      title: '未認証テスト配信',
    });

    // Chat タブへ移動して接続試行
    await mainPage.getByRole('button', { name: 'Chat' }).click();
    const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
    await expect(urlInput).toBeVisible();
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=no_auth_stream`);
    await mainPage.locator('button:has-text("開始")').click();

    // 接続失敗: エラー表示を確認
    // continuation tokenが取得できないためエラーになる
    await expect(
      mainPage.locator('text=Failed to get continuation token'),
    ).toBeVisible({ timeout: 15000 });

    await disconnectAndInitialize(mainPage);
  });
});
