import * as fs from 'fs';
import * as path from 'path';
import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import {
  startTauriAppWithEnv,
  killTauriApp,
  cleanupTestData,
  cleanupTestCredentials,
  startMockServer,
  killMockServer,
  resetMockServer,
  addMockMessage,
  connectToApp,
  connectToMockStream,
  getTestAppDataDir,
  MOCK_SERVER_URL,
  TEST_APP_NAME,
  TEST_KEYRING_SERVICE,
} from './utils/test-helpers';

/**
 * バックエンドのログがファイルに残ることを実 Tauri で検証する
 * （FEATURE_SPECIFICATION.md「バックエンドのログ」）。
 *
 * `tracing::` で書いたログが出力先に届くこと、出力先が LISCOV_APP_NAME に従うことを確かめる。
 */

test.describe('バックエンドのログ (実 Tauri)', () => {
  test.setTimeout(120000);
  let browser: Browser;
  let mainPage: Page;

  test.beforeAll(async () => {
    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();
    await startMockServer();
    await resetMockServer();

    await startTauriAppWithEnv({
      LISCOV_APP_NAME: TEST_APP_NAME,
      LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
      LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
      LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
      LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
    });

    const conn = await connectToApp();
    browser = conn.browser;
    mainPage = conn.page;
    await expect(mainPage.locator('nav button:has-text("Chat")')).toBeVisible({
      timeout: 30000,
    });
  });

  test.afterAll(async () => {
    if (browser) await browser.close();
    await killTauriApp();
    await killMockServer();
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test('tracing のログがテスト用データディレクトリの logs/liscov.log に書かれる', async () => {
    await addMockMessage({ message_type: 'text', author: '@viewer-log', content: 'ログ確認' });
    await connectToMockStream(mainPage);

    const logFile = path.join(getTestAppDataDir(), 'logs', 'liscov.log');
    // 監視タスクの開始ログ（chat_runtime.rs の tracing::info!）が書かれるまで待つ
    await expect
      .poll(() => (fs.existsSync(logFile) ? fs.readFileSync(logFile, 'utf-8') : ''), {
        timeout: 15000,
      })
      .toContain('チャット監視タスク開始');
  });
});
