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
  // 2 件目は 1 件目でつないだ配信に続けて積む
  test.describe.configure({ mode: 'serial' });
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
      LISCOV_AUTH_URL: `${mockServerUrl()}/?auto_login=true`,
      LISCOV_SESSION_CHECK_URL: `${mockServerUrl()}/youtubei/v1/account/account_menu`,
      LISCOV_YOUTUBE_BASE_URL: mockServerUrl(),
    });

    const conn = await connectToApp();
    browser = conn.browser;
    mainPage = conn.page;

    await expect(mainPage.locator('nav button[aria-label="チャット"]')).toBeVisible({
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
      author: '@新人',
      content: '',
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

    // 文は 04_tts.md「スーパーチャット/メンバーシップの読み上げ例」。新規メンバーは YouTube の英文 (Welcome to ...) を読まない
    expect(mockBouyomichan.receivedTexts).toEqual([
      'スパチャ太郎さん、500円のスーパーチャット、スパチャ',
      '新人さん、メンバー加入',
      '通常ユーザーさん、普通のコメント',
    ]);
  });

  test('パーサーを通したメンバーシップ系・ステッカーの読み上げ (04_tts.md の例)', async () => {
    const before = mockBouyomichan.receivedTexts.length;
    await addMockMessage({
      message_type: 'membership_milestone',
      author: '@古参',
      content: 'いつも楽しい！',
      milestone_months: 6,
      channel_id: 'UCviewer_milestone',
      is_member: true,
    });
    await addMockMessage({
      message_type: 'membership_gift',
      author: '@太っ腹',
      content: '',
      gift_count: 5,
      channel_id: 'UCviewer_gift',
    });
    await addMockMessage({
      message_type: 'supersticker',
      author: '@ステッカー',
      content: '',
      amount: '¥200',
      channel_id: 'UCviewer_sticker',
    });

    await waitForReceivedTexts(mockBouyomichan, before + 3, 30000);

    // 優先度はステッカー (SuperChat) が先。メンバーシップ系どうしの順は問わない
    const spoken = mockBouyomichan.receivedTexts.slice(before);
    expect(spoken).toHaveLength(3);
    expect(spoken[0]).toBe('ステッカーさん、200円のスーパーステッカー');
    expect(spoken.slice(1)).toEqual(
      expect.arrayContaining(['古参さん、6ヶ月のメンバーシップ、いつも楽しい！', '太っ腹さん、5人へのメンバーシップギフト']),
    );
  });
});
