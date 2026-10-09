import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  disconnectAndInitialize,
  setStreamState,
  openUrlInput,
  openControlPanel,
  connectionChips,
} from './utils/test-helpers';

/**
 * E2E tests for Chat Display — message type rendering.
 * Covers: SuperChat and Special Messages, Additional Message Types,
 * Special Text Patterns, 初見さんバッジと配信内コメント回数
 */

test.describe('Chat Display — Message Types (02_chat.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000); // 4 minutes for setup (includes mock server build time)

    log.info('Setting up test environment for Chat Display tests...');
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
    log.info('Connected to Tauri app');
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test.beforeEach(async () => {
    // Reset mock server before each test
    await resetMockServer();
  });

  test.describe('SuperChat and Special Messages', () => {
    test('should display SuperChat with amount', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add SuperChat message
      await addMockMessage({
        message_type: 'superchat',
        author: 'SuperChatter',
        content: 'Thanks for the stream!',
        amount: '¥1,000',
        tier: 'green',
      });

      // Verify SuperChat is displayed with amount
      // 仕様: 主の配信のスパチャは優先順位 1 の面。見出しに名前と金額、本文は面の中
      const card = mainPage.locator('[data-message-id][data-priority="1"]').filter({ hasText: 'SuperChatter' });
      await expect(card).toBeVisible({ timeout: 5000 });
      await expect(card).toContainText('SuperChatter');
      await expect(card).toContainText('¥1,000');
      await expect(card.getByTestId('paid-body')).toHaveText('Thanks for the stream!');

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display membership message', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add membership message
      await addMockMessage({
        message_type: 'membership',
        author: 'NewMember',
        content: 'New member!',
      });

      // Verify membership message is displayed
      await expect(mainPage.getByText('NewMember').first()).toBeVisible({ timeout: 5000 });
      // "新規メンバー" label in header badge (Japanese for "New Member")
      await expect(mainPage.getByText('新規メンバー').first()).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display SuperChat with YouTube-specified tier colors', async () => {
      // This test verifies that SuperChat messages use the color scheme from YouTube API
      // Per 02_chat.md spec:
      // - superchat_colors contains: header_background, header_text, body_background, body_text
      // - 面の背景 = body_background、見出しの背景 = header_background (インライン style)

      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add SuperChat with 'blue' tier (0x1565C0 = rgb(21, 101, 192))
      await addMockMessage({
        message_type: 'superchat',
        author: 'BlueTierDonator',
        content: 'Blue tier superchat!',
        amount: '¥5,000',
        tier: 'blue',
      });

      // 仕様: 面の背景は body_background、見出しの背景は header_background (どちらもインライン style)
      const superchatMessage = mainPage.locator('[data-message-id][data-priority="1"]').filter({
        hasText: 'BlueTierDonator',
      });
      await expect(superchatMessage).toBeVisible({ timeout: 5000 });

      // Blue tier の header_background は 0x1565C0 = rgb(21, 101, 192)
      // インライン style は要素の style 属性として読める (ブラウザが rgb に正規化することもある)
      const headerStyle =
        (await superchatMessage.locator('div[style*="background"]').filter({ hasText: 'BlueTierDonator' }).last().getAttribute('style')) ?? '';
      log.debug(`SuperChat header style: "${headerStyle}"`);
      expect(headerStyle.toLowerCase()).toMatch(/1565c0|rgb\(21,\s*101,\s*192\)/);

      // 面 (カード) の背景は body_background。モックは header と body に同じティアの色を送るので同じ色になる。
      // 本文 (paid-body) の文字色は body_text (青のティアは白。モックも YouTube と同じく、明るいティアでは黒を送る)
      const cardStyle = (await superchatMessage.locator('div[style*="background"]').first().getAttribute('style')) ?? '';
      expect(cardStyle.toLowerCase()).toMatch(/1565c0|rgb\(21,\s*101,\s*192\)/);
      const bodyColor = await superchatMessage.getByTestId('paid-body').evaluate((el) => getComputedStyle(el).color);
      expect(bodyColor).toBe('rgb(255, 255, 255)');

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display different SuperChat tiers with their respective colors', async () => {
      // Test multiple SuperChat tiers to verify color differentiation

      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add red tier SuperChat (0xD00000 = rgb(208, 0, 0))
      await addMockMessage({
        message_type: 'superchat',
        author: 'RedTierDonator',
        content: 'Red tier superchat!',
        amount: '¥50,000',
        tier: 'red',
      });

      // 仕様: 見出しの背景に header_background (red は 0xD00000 = rgb(208, 0, 0))
      const redSuperchat = mainPage.locator('[data-message-id][data-priority="1"]').filter({
        hasText: 'RedTierDonator',
      });
      await expect(redSuperchat).toBeVisible({ timeout: 5000 });

      const redStyle =
        (await redSuperchat.locator('div[style*="background"]').filter({ hasText: 'RedTierDonator' }).last().getAttribute('style')) ?? '';
      log.debug(`Red tier SuperChat header style: "${redStyle}"`);
      expect(redStyle.toLowerCase()).toMatch(/d00000|rgb\(208,\s*0,\s*0\)/);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Additional Message Types', () => {
    test('should display SuperSticker with amount', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add SuperSticker message
      await addMockMessage({
        message_type: 'supersticker',
        author: 'StickerUser',
        content: '',
        amount: '¥1,500',
      });

      // Verify SuperSticker is displayed
      // 仕様: 主の配信のスーパーステッカーも優先順位 1 の面。見出しに名前と金額
      const sticker = mainPage.locator('[data-message-id][data-priority="1"]').filter({ hasText: 'StickerUser' });
      await expect(sticker).toBeVisible({ timeout: 5000 });
      await expect(sticker).toContainText('¥1,500');

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display membership milestone message', async () => {
      // Milestone months are extracted from badge tooltip (e.g., "Member (12 months)")

      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add membership milestone message
      await addMockMessage({
        message_type: 'membership_milestone',
        author: 'LoyalMember',
        content: '1年間ありがとうございます！',
        milestone_months: 12,
      });

      // Verify milestone message is displayed
      // Note: content is headerSubtext ("Welcome to Channel!"), user message is in separate field
      await expect(mainPage.locator('text=LoyalMember')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.locator('text=Welcome to Channel')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display membership gift message', async () => {
      // Gift messages use liveChatSponsorshipsGiftPurchaseAnnouncementRenderer

      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add membership gift message
      await addMockMessage({
        message_type: 'membership_gift',
        author: 'GenerousGifter',
        content: '',
        gift_count: 10,
      });

      // Verify gift message is displayed
      await expect(mainPage.locator('text=GenerousGifter')).toBeVisible({ timeout: 5000 });
      // Check for gift count in the message content (e.g., "Sent 10 Channel gift memberships")
      await expect(mainPage.locator('text=/\\b10\\b.*gift/i').first()).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('バッジ (02_chat.md「バッジの種類」)', () => {
    test('モデレーターのコメントにモデレーターのバッジが表示される', async () => {
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      await addMockMessage({
        message_type: 'text',
        author: 'ModUser',
        content: 'モデレーターのコメント',
        channel_id: 'UC_moderator_1',
        is_moderator: true,
      });
      await addMockMessage({
        message_type: 'text',
        author: 'PlainUser',
        content: 'ふつうのコメント',
        channel_id: 'UC_plain_1',
      });

      const modMessage = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=モデレーターのコメント'),
      }).first();
      await expect(modMessage.locator('[title="モデレーター"]')).toBeVisible({ timeout: 5000 });

      const plainMessage = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=ふつうのコメント'),
      }).first();
      await expect(plainMessage).toBeVisible({ timeout: 5000 });
      await expect(plainMessage.locator('[title="モデレーター"]')).toHaveCount(0);

      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Special Text Patterns', () => {
    test('should display stream title with hashtags correctly', async () => {
      // Set stream title with hashtags
      const fullTitle = '【雑談配信】今日もゲーム実況 #vtuber #ゲーム実況 #参加型';
      await setStreamState({ title: fullTitle });

      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=hashtag_title_test`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();

      // Wait for connection
      await expect(connectionChips(mainPage, fullTitle)).toBeVisible({ timeout: 10000 });

      // 配信タイトルは操作パネルの接続の一覧に全文が出る
      await openControlPanel(mainPage);
      const titleElement = mainPage.locator('[data-testid="stream-title"]').first();
      await expect(titleElement).toBeVisible();

      // Get the full text content of the title element (even if visually truncated)
      const titleTextContent = await titleElement.textContent();
      log.debug(`Title text content: "${titleTextContent}"`);

      // Verify the full title is present in the DOM (not truncated at hashtag)
      expect(titleTextContent).toContain('#vtuber');
      expect(titleTextContent).toContain('#ゲーム実況');
      expect(titleTextContent).toContain('#参加型');
      expect(titleTextContent?.trim()).toBe(fullTitle);

      // Disconnect
      await disconnectAndInitialize(mainPage);

      // Reset stream state
      await setStreamState({ title: '' });
    });

    test('should display stream title with multiple hashtags and special chars', async () => {
      // Set stream title with many hashtags and special characters
      const fullTitle = '🎮 Gaming Stream #game #stream #live #fun @everyone!';
      await setStreamState({ title: fullTitle });

      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=special_title_test`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();

      // Wait for connection
      await expect(connectionChips(mainPage, fullTitle)).toBeVisible({ timeout: 10000 });

      // 配信タイトルは操作パネルの接続の一覧に全文が出る
      await openControlPanel(mainPage);
      const titleElement = mainPage.locator('[data-testid="stream-title"]').first();
      await expect(titleElement).toBeVisible();
      const titleTextContent = await titleElement.textContent();
      log.debug(`Title text content: "${titleTextContent}"`);

      // Verify full title content (not truncated)
      expect(titleTextContent).toContain('#game');
      expect(titleTextContent).toContain('#stream');
      expect(titleTextContent).toContain('#live');
      expect(titleTextContent).toContain('#fun');
      expect(titleTextContent).toContain('@everyone!');
      expect(titleTextContent?.trim()).toBe(fullTitle);

      // Disconnect
      await disconnectAndInitialize(mainPage);

      // Reset stream state
      await setStreamState({ title: '' });
    });

    test('should display messages with hashtags correctly', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add message with hashtag
      await addMockMessage({
        message_type: 'text',
        author: 'HashtagUser',
        content: '今日の配信 #gaming #vtuber #ゲーム実況',
      });

      // Verify message with hashtag is displayed correctly
      await expect(mainPage.getByText('#gaming')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.getByText('#vtuber')).toBeVisible();
      await expect(mainPage.getByText('#ゲーム実況')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display messages with various special characters', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add message with special characters
      await addMockMessage({
        message_type: 'text',
        author: 'SpecialCharUser',
        content: 'Hello! <script>alert(1)</script> 💖 @mention [link](url) **bold**',
      });

      // Verify special characters are escaped and displayed correctly
      await expect(mainPage.getByText('<script>alert(1)</script>')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.getByText('💖')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display messages with URLs', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add message with URL
      await addMockMessage({
        message_type: 'text',
        author: 'URLUser',
        content: 'Check this out: https://example.com/path?param=value&foo=bar',
      });

      // Verify URL is displayed
      await expect(mainPage.getByText('https://example.com/path?param=value&foo=bar')).toBeVisible({ timeout: 5000 });

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display long messages without breaking layout', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add long message
      const longText = 'これは非常に長いメッセージです。'.repeat(10);
      await addMockMessage({
        message_type: 'text',
        author: 'LongMessageUser',
        content: longText,
      });

      // Verify long message is displayed (should break words)
      const messageEl = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${longText.slice(0, 30)}`)
      }).first();
      await expect(messageEl).toBeVisible({ timeout: 5000 });

      // Check that the message has break-words class
      const hasBreakWords = await messageEl.locator('p.break-words').count();
      expect(hasBreakWords).toBeGreaterThan(0);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('初見さんバッジと配信内コメント回数', () => {
    test('新規さんの初回コメントは優先順位 2 (塗りの印「初見」) で #1 が表示される', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_first_time`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add message from a new user (test DB is clean, so all users are first-time)
      await addMockMessage({
        message_type: 'text',
        author: 'NewUser',
        content: 'はじめてのコメントです',
        channel_id: 'UC_new_1',
        is_member: false,
      });

      // 仕様: 新規さんの初回 (is_first_time_viewer かつ #1) は優先順位 2、塗りの印「初見」
      const messageEl = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=はじめてのコメントです'),
      }).first();
      await expect(messageEl).toHaveAttribute('data-priority', '2', { timeout: 5000 });
      await expect(messageEl.getByText('初見', { exact: true })).toBeVisible();
      await expect(messageEl.getByText('#1')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('新規さんの2回目は優先順位 3 (枠の印「初見」) で #2 が表示される', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_repeat`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // 1回目のメッセージ
      await addMockMessage({
        message_type: 'text',
        author: 'RepeatUser',
        content: '1回目のコメント',
        channel_id: 'UC_repeat',
        is_member: false,
      });

      await expect(mainPage.locator('text=1回目のコメント')).toBeVisible({ timeout: 5000 });

      // 2回目のメッセージ
      await addMockMessage({
        message_type: 'text',
        author: 'RepeatUser',
        content: '2回目のコメント',
        channel_id: 'UC_repeat',
        is_member: false,
      });

      await expect(mainPage.locator('text=2回目のコメント')).toBeVisible({ timeout: 5000 });

      // 仕様: 新規さんの 2 回目以降は優先順位 3 (枠だけの印、背景なし)。#2 が続く
      const secondMessageEl = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=2回目のコメント'),
      }).first();
      await expect(secondMessageEl).toHaveAttribute('data-priority', '3');
      await expect(secondMessageEl.getByText('初見', { exact: true })).toBeVisible();
      await expect(secondMessageEl.getByText('#2')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('異なるユーザーはそれぞれ優先順位 2 の「初見」と #1 が表示される', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_multi_first`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // ユーザーAのメッセージ
      await addMockMessage({
        message_type: 'text',
        author: 'UserAlpha',
        content: 'UserAlphaのコメント',
        channel_id: 'UC_a',
        is_member: false,
      });

      // ユーザーBのメッセージ
      await addMockMessage({
        message_type: 'text',
        author: 'UserBeta',
        content: 'UserBetaのコメント',
        channel_id: 'UC_b',
        is_member: false,
      });

      // 両方に優先順位 2 の印「初見」と #1 が表示される
      const messageA = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=UserAlphaのコメント'),
      }).first();
      await expect(messageA).toBeVisible({ timeout: 5000 });
      await expect(messageA).toHaveAttribute('data-priority', '2');
      await expect(messageA.getByText('初見', { exact: true })).toBeVisible();
      await expect(messageA.getByText('#1')).toBeVisible();

      const messageB = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=UserBetaのコメント'),
      }).first();
      await expect(messageB).toBeVisible();
      await expect(messageB).toHaveAttribute('data-priority', '2');
      await expect(messageB.getByText('初見', { exact: true })).toBeVisible();
      await expect(messageB.getByText('#1')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  // spec: 02_chat.md「ギフト」— ジュエルで送るギフト（giftMessageViewModel、channelId 無し）
  test.describe('ギフト（ジュエル）', () => {
    async function connect(videoId: string) {
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=${videoId}`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });
    }

    function giftRow(author: string, giftName: string) {
      return mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${author} が ${giftName} を送りました`),
      }).first();
    }

    test('ジュエル数付きのギフトを名前とジュエル数付きで表示し、注記は出さない', async () => {
      await connect('test_video_gift_jewels');
      await addMockMessage({
        message_type: 'gift',
        author: '@gift-jewel-user',
        content: 'sent Press F for 10 Jewels',
      });

      const row = giftRow('@gift-jewel-user', 'Press F');
      await expect(row).toBeVisible({ timeout: 5000 });
      await expect(row.locator('[data-testid="gift-jewels"]')).toHaveText('10 ジュエル');
      await openControlPanel(mainPage);
      await expect(mainPage.locator('[data-testid="jewel-count-notice"]')).toHaveCount(0);

      await disconnectAndInitialize(mainPage);
    });

    test('ジュエル数の無いギフトを受けた接続に注記を出す', async () => {
      await connect('test_video_gift_no_jewels');
      await addMockMessage({
        message_type: 'gift',
        author: '@gift-plain-user',
        content: 'sent Hiding',
      });

      const row = giftRow('@gift-plain-user', 'Hiding');
      await expect(row).toBeVisible({ timeout: 5000 });
      await expect(row.locator('[data-testid="gift-jewels"]')).toHaveCount(0);
      await openControlPanel(mainPage);
      await expect(mainPage.locator('[data-testid="jewel-count-notice"]')).toBeVisible();

      await disconnectAndInitialize(mainPage);
    });

    test('同じ接続でコメント済みの視聴者のギフトは同一人物として数える', async () => {
      await connect('test_video_gift_resolved');
      await addMockMessage({
        message_type: 'text',
        author: '@gift-regular',
        content: 'ギフト前のコメント',
        channel_id: 'UC_gift_regular',
      });
      await expect(mainPage.locator('text=ギフト前のコメント')).toBeVisible({ timeout: 5000 });

      await addMockMessage({
        message_type: 'gift',
        author: '@gift-regular',
        content: 'sent Hiding',
      });
      const row = giftRow('@gift-regular', 'Hiding');
      await expect(row).toBeVisible({ timeout: 5000 });
      // handle から channel_id を特定できたので配信内コメント回数が続く
      await expect(row.getByText('#2')).toBeVisible();

      await disconnectAndInitialize(mainPage);
    });

    test('視聴者を特定できないギフトは回数を出さず、クリックしても視聴者情報を開かない', async () => {
      await connect('test_video_gift_unresolved');
      await addMockMessage({
        message_type: 'gift',
        author: '@gift-stranger',
        content: 'sent Hiding',
      });
      const row = giftRow('@gift-stranger', 'Hiding');
      await expect(row).toBeVisible({ timeout: 5000 });
      await expect(row.getByText(/^#\d+$/)).toHaveCount(0);
      await expect(row.getByText('初見')).toHaveCount(0);

      await row.click();
      await expect(mainPage.locator('h2:has-text("視聴者情報")')).toHaveCount(0);

      await disconnectAndInitialize(mainPage);
    });
  });
});
