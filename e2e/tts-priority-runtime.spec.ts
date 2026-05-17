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
 * 実 Tauri アプリ経由で TTS キューの優先度順序を end-to-end 検証する。
 *
 * 仕様 (04_tts.md): SuperChat > Membership > Normal
 *
 * 投入順 Normal → Membership → SuperChat に対し、TtsManager.enqueue が
 * priority 順にキューを並び替えた結果として、speak は SuperChat →
 * Membership → Normal の順に発火することを確認する。
 *
 * メッセージは別ユーザー (異なる channel_id) から送り、初回コメント機能の
 * 影響を意図的に避ける (各 channel_id で count=1 となるが、
 * first_comment_prefix_enabled=false としているため変化なし)。
 */

test.describe('TTS Priority Order Runtime (実 Tauri + モック棒読みちゃん)', () => {
  test.setTimeout(120000);
  let browser: Browser;
  let mainPage: Page;
  let mockBouyomichan: MockBouyomichan;

  test.beforeAll(async () => {
    log.info('Starting mock bouyomichan server (priority order)...');
    mockBouyomichan = await startMockBouyomichan();
    log.info(`Mock bouyomichan listening on 127.0.0.1:${mockBouyomichan.port}`);

    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();

    writeTestTtsConfig({
      bouyomichanPort: mockBouyomichan.port,
      firstCommentPrefixEnabled: false,
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

  test('優先度順序 (実 Tauri): SuperChat > Membership > Normal の順で speak される', async () => {
    // 投入順は Normal → Membership → SuperChat
    // 1 回のポーリング (1.5s 間隔) で 3 件まとめて取得 → TtsManager.enqueue で
    // priority 順に再配置 → speak は SuperChat → Membership → Normal の順
    await addMockMessage({
      message_type: 'text',
      author: '@通常ユーザー',
      content: '普通のコメント',
      channel_id: 'UCviewer_normal',
    });
    await addMockMessage({
      message_type: 'membership',
      author: '@メンバー',
      content: 'メンバー加入',
      channel_id: 'UCviewer_member',
      is_member: true,
    });
    await addMockMessage({
      message_type: 'superchat',
      author: '@スパチャ太郎',
      content: 'スパチャ',
      amount: '¥500',
      channel_id: 'UCviewer_superchat',
    });

    await connectToMockStream(mainPage);

    await waitForReceivedTexts(mockBouyomichan, 3, 30000);

    const spoken = mockBouyomichan.receivedTexts;
    expect(spoken).toHaveLength(3);

    // 1 番目: SuperChat — 著者名 "スパチャ太郎" + 金額 + 本文 を含む
    expect(spoken[0]).toContain('スパチャ太郎');
    expect(spoken[0]).toContain('スパチャ');

    // 2 番目: Membership — メンバー加入文言を含む (04_tts.md 「メンバー加入」)
    // 04_tts.md より新規メンバーは "メンバー加入" を含む読み上げ
    expect(spoken[1]).toContain('メンバー');

    // 3 番目: Normal — 通常コメント
    expect(spoken[2]).toContain('普通のコメント');

    // SuperChat と Membership が Normal より前であることを順序として明示確認
    const idxSuper = spoken.findIndex((t) => t.includes('スパチャ太郎'));
    const idxMember = spoken.findIndex((t) => t.includes('メンバー加入') || t === spoken[1]);
    const idxNormal = spoken.findIndex((t) => t.includes('普通のコメント'));
    expect(idxSuper).toBe(0);
    expect(idxNormal).toBe(2);
    expect(idxMember).toBeLessThan(idxNormal);
    expect(idxSuper).toBeLessThan(idxMember);
  });
});
