import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
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
  MOCK_SERVER_URL,
  TEST_APP_NAME,
  TEST_KEYRING_SERVICE,
} from './utils/test-helpers';
import {
  type MockBouyomichan,
  startMockBouyomichan,
  stopMockBouyomichan,
  writeTestTtsConfig,
  waitForReceivedTexts,
} from './utils/tts-mock-bouyomichan';

/**
 * YouTube が同じ接続に同じメッセージを再送しても、読み上げは 1 回だけであることを
 * 実 Tauri + モック棒読みちゃんで検証する（02_chat.md「受信済み message_id の記憶」）。
 *
 * 再送は、モックサーバーに同じ id のメッセージを次のポーリング分として積み直して再現する。
 */

test.describe('TTS 再送メッセージ (実 Tauri + モック棒読みちゃん)', () => {
  test.setTimeout(120000);
  let browser: Browser;
  let mainPage: Page;
  let mockBouyomichan: MockBouyomichan;

  test.beforeAll(async () => {
    mockBouyomichan = await startMockBouyomichan();
    log.info(`Mock bouyomichan listening on 127.0.0.1:${mockBouyomichan.port}`);

    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();

    writeTestTtsConfig({ bouyomichanPort: mockBouyomichan.port });

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
    await stopMockBouyomichan(mockBouyomichan);
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test('同じ id のメッセージが再送されても 1 回だけ読み上げる', async () => {
    const resent = {
      id: 'resend_e2e_1',
      message_type: 'text',
      author: '@viewer-resend',
      content: '再送されるコメント',
      channel_id: 'UCviewer_resend_e2e',
    };

    await addMockMessage(resent);
    await connectToMockStream(mainPage);
    await waitForReceivedTexts(mockBouyomichan, 1, 30000);

    // 次のポーリングで同じ id が再び届く。続く別のコメントが読まれた時点で、再送分の処理も済んでいる
    await addMockMessage(resent);
    await addMockMessage({
      message_type: 'text',
      author: '@viewer-resend',
      content: '後続のコメント',
      channel_id: 'UCviewer_resend_e2e',
    });
    await waitForReceivedTexts(mockBouyomichan, 2, 30000);

    const texts = mockBouyomichan.receivedTexts;
    expect(texts.filter((t) => t.includes('再送されるコメント'))).toHaveLength(1);
    expect(texts[1]).toContain('後続のコメント');
  });
});
