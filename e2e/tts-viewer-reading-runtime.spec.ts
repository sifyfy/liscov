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
  mockServerUrl,
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
 * 視聴者パネルで登録した読み仮名が TTS に使われることを、実 Tauri アプリ経由で確かめる
 * （04_tts.md「投稿者名の処理順序」「どの読み仮名を使うか」、06_viewer.md「カスタム情報の管理」）。
 *
 * カバー範囲:
 * - チャット画面の視聴者パネルで保存 → viewer_custom_info に入る
 * - 次に届いたコメントで、監視ループがこの接続の配信者のもとで読み仮名を引く → /Talk の文に使われる
 * - 読み仮名を空にして保存すると投稿者名の処理に戻る
 *
 * 複数接続での配信者の取り違えは、モックサーバーのメッセージキューが全接続で共通のため
 * ここでは扱わず、core/chat_runtime.rs の単体テストで確かめている。
 */

test.describe('TTS Viewer Reading Runtime (実 Tauri + モック棒読みちゃん)', () => {
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

    // add_honorific / strip_at_prefix / strip_handle_suffix はすべて true
    writeTestTtsConfig({ bouyomichanPort: mockBouyomichan.port });

    await startMockServer();
    await resetMockServer();

    await startTauriAppWithEnv({
      LISCOV_APP_NAME: TEST_APP_NAME,
      LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
      LISCOV_AUTH_URL: `${mockServerUrl()}/?auto_login=true`,
      LISCOV_SESSION_CHECK_URL: `${mockServerUrl()}/youtubei/v1/account/account_menu`,
      LISCOV_YOUTUBE_BASE_URL: mockServerUrl(),
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

  /** 視聴者パネルを開き、読み仮名を入れて保存する */
  async function saveReading(messageContent: string, reading: string): Promise<void> {
    await mainPage
      .locator('[data-message-id]')
      .filter({ hasText: messageContent })
      .first()
      .click();
    const input = mainPage.locator('#viewer-reading');
    await expect(input).toBeEnabled({ timeout: 5000 });
    await input.fill(reading);
    await mainPage.locator('button:has-text("保存")').click();
    await expect(mainPage.getByText('保存しました')).toBeVisible({ timeout: 5000 });
    await mainPage.locator('button:has-text("✕")').click();
  }

  test('読み仮名を保存すると次のコメントから読み仮名で読み、空にすると投稿者名に戻る', async () => {
    const author = '@田中-abc';
    const channelId = 'UCviewer_reading_e2e';

    await connectToMockStream(mainPage);

    // 読み仮名なし → 投稿者名を処理したもの
    await addMockMessage({ message_type: 'text', author, content: 'いちばんめ', channel_id: channelId });
    await waitForReceivedTexts(mockBouyomichan, 1, 30000);
    expect(mockBouyomichan.receivedTexts[0]).toBe('田中さん、いちばんめ');

    // spec: @田中-abc（読み仮名:たなか）, true, true, true → たなかさん
    await saveReading('いちばんめ', 'たなか');
    await addMockMessage({ message_type: 'text', author, content: 'にばんめ', channel_id: channelId });
    await waitForReceivedTexts(mockBouyomichan, 2, 30000);
    expect(mockBouyomichan.receivedTexts[1]).toBe('たなかさん、にばんめ');

    // spec: 読み仮名を空にして保存した → 投稿者名を処理したもの
    await saveReading('にばんめ', '');
    await addMockMessage({ message_type: 'text', author, content: 'さんばんめ', channel_id: channelId });
    await waitForReceivedTexts(mockBouyomichan, 3, 30000);
    expect(mockBouyomichan.receivedTexts[2]).toBe('田中さん、さんばんめ');
  });
});
