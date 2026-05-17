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
  assertNoFurtherSpeak,
} from './utils/tts-mock-bouyomichan';

/**
 * 実 Tauri アプリ経由で `first_comment_only` の挙動を end-to-end 検証する (AC-4/AC-5)。
 *
 * - AC-4: ON + count=1 → speak される
 * - AC-5: ON + count=2 → speak されない (スキップ)
 *
 * このバグが残るとオン設定時に2回目以降も読み上げてしまい仕様違反。
 */

test.describe('TTS First Comment Only Runtime (実 Tauri + モック棒読みちゃん)', () => {
  test.setTimeout(120000);
  let browser: Browser;
  let mainPage: Page;
  let mockBouyomichan: MockBouyomichan;

  test.beforeAll(async () => {
    log.info('Starting mock bouyomichan server (first_comment_only=true)...');
    mockBouyomichan = await startMockBouyomichan();
    log.info(`Mock bouyomichan listening on 127.0.0.1:${mockBouyomichan.port}`);

    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();

    writeTestTtsConfig({
      bouyomichanPort: mockBouyomichan.port,
      firstCommentPrefixEnabled: false,
      firstCommentOnly: true,
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

  test('AC-4/AC-5 (実 Tauri): first_comment_only=ON で1回目のみ speak、2回目以降はスキップ', async () => {
    const author = '@視聴者A';
    const channelId = 'UCviewer_first_comment_only_e2e';

    // 同一ユーザーから 2 件投入
    await addMockMessage({
      message_type: 'text',
      author,
      content: '初コメ',
      channel_id: channelId,
    });
    await addMockMessage({
      message_type: 'text',
      author,
      content: '2回目',
      channel_id: channelId,
    });

    await connectToMockStream(mainPage);

    // AC-4: count=1 のメッセージは speak される
    await waitForReceivedTexts(mockBouyomichan, 1, 30000);
    expect(mockBouyomichan.receivedTexts[0]).toContain('初コメ');

    // AC-5: count=2 のメッセージはスキップされる
    // 1.5 秒の沈黙期間で「2 件目が来ない」ことを確認
    // (innertube ポーリングが 1.5s 間隔で、2 件目もすでにキューに乗っているため、
    //  本来読み上げが行われるなら 1〜2 秒以内に観測されるはず)
    await assertNoFurtherSpeak(mockBouyomichan, 1, 3000);

    expect(mockBouyomichan.receivedTexts).toHaveLength(1);
    expect(mockBouyomichan.receivedTexts[0]).not.toContain('2回目');
  });
});
