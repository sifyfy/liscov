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
 * 実 Tauri アプリ経由で TTS 初回コメントプレフィックスを end-to-end 検証する (AC-1/AC-2)。
 *
 * カバー範囲:
 * - tts_config.toml の load → Bouyomichan backend 構成 → /Talk リクエスト発火 まで
 * - in_stream_comment_count が DB 経由で 1, 2 と振られる
 * - build_first_comment_prefix で 1 件目のテキストに "1回目のコメント。" が付加される
 */

test.describe('TTS First Comment Prefix Runtime (実 Tauri + モック棒読みちゃん)', () => {
  test.setTimeout(120000);
  let browser: Browser;
  let mainPage: Page;
  let mockBouyomichan: MockBouyomichan;

  test.beforeAll(async () => {
    log.info('Starting mock bouyomichan server...');
    mockBouyomichan = await startMockBouyomichan();
    log.info(`Mock bouyomichan listening on 127.0.0.1:${mockBouyomichan.port}`);

    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();

    writeTestTtsConfig({
      bouyomichanPort: mockBouyomichan.port,
      firstCommentPrefixEnabled: true,
      firstCommentPrefix: '',
      firstCommentOnly: false,
    });

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

  test('AC-1/AC-2 (実 Tauri): 初回コメントにプレフィックスが付き、2回目には付かない', async () => {
    const author = '@山田太郎-xyz';
    const channelId = 'UCviewer_first_comment_e2e';

    await addMockMessage({
      message_type: 'text',
      author,
      content: 'こんにちは',
      channel_id: channelId,
    });
    await addMockMessage({
      message_type: 'text',
      author,
      content: '二回目です',
      channel_id: channelId,
    });

    await connectToMockStream(mainPage);

    await waitForReceivedTexts(mockBouyomichan, 2, 30000);

    const [first, second] = mockBouyomichan.receivedTexts;

    // AC-1 (E2E): プレフィックスON + count=1 → デフォルト "1回目のコメント。" が付加
    expect(first).toMatch(/^1回目のコメント。/);
    expect(first).toContain('山田太郎'); // @除去 + -xyz 除去
    expect(first).toContain('こんにちは');

    // AC-2 (E2E): プレフィックスON + count=2 → プレフィックスなし
    expect(second).not.toContain('1回目のコメント。');
    expect(second).toContain('二回目です');
  });
});
