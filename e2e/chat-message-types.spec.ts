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
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add SuperChat message
      await addMockMessage({
        message_type: 'superchat',
        author: 'SuperChatter',
        content: 'Thanks for the stream!',
        amount: '¥1,000',
        tier: 'green',
      });

      // Verify SuperChat is displayed with amount
      await expect(mainPage.locator('text=SuperChatter')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.locator('text=Thanks for the stream!')).toBeVisible();
      await expect(mainPage.locator('text=¥1,000')).toBeVisible();
      // Check for Super Chat label (Japanese: スーパーチャット)
      await expect(mainPage.locator('text=スーパーチャット')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display membership message', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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
      // - These should be applied via inline styles for border-left-color and background gradient

      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add SuperChat with 'blue' tier (0x1565C0 = rgb(21, 101, 192))
      await addMockMessage({
        message_type: 'superchat',
        author: 'BlueTierDonator',
        content: 'Blue tier superchat!',
        amount: '¥5,000',
        tier: 'blue',
      });

      // Find the SuperChat message element
      const superchatMessage = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=BlueTierDonator')
      }).first();

      await expect(superchatMessage).toBeVisible({ timeout: 5000 });

      // Get the inline style attribute - should contain YouTube's blue tier color
      // Blue tier: 0x1565C0 = #1565C0 = rgb(21, 101, 192)
      const styleAttr = await superchatMessage.getAttribute('style');

      // The spec requires:
      // border-left-color: {header_background}
      // background: linear-gradient(135deg, {body_background}22 0%, {header_background}22 100%)
      //
      // If superchat_colors is properly parsed from YouTube API, it should contain:
      // - The tier color (blue = #1565C0) in border-left-color
      // - A gradient background using the tier color

      log.debug(`SuperChat style attribute: "${styleAttr}"`);

      // CRITICAL ASSERTION: Style must contain the YouTube-specified blue tier color
      // Either as hex (#1565c0) or rgb (rgb(21, 101, 192))
      // If this fails, it means superchat_colors is not being parsed from the API
      expect(styleAttr).toBeTruthy();
      const hasYouTubeColor = styleAttr?.includes('1565c0') ||
                              styleAttr?.includes('1565C0') ||
                              styleAttr?.match(/rgb\(21,\s*101,\s*192\)/);

      expect(hasYouTubeColor).toBeTruthy();

      // Additionally, should have a gradient background
      expect(styleAttr).toContain('linear-gradient');

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display different SuperChat tiers with their respective colors', async () => {
      // Test multiple SuperChat tiers to verify color differentiation

      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add red tier SuperChat (0xD00000 = rgb(208, 0, 0))
      await addMockMessage({
        message_type: 'superchat',
        author: 'RedTierDonator',
        content: 'Red tier superchat!',
        amount: '¥50,000',
        tier: 'red',
      });

      // Find the red tier SuperChat
      const redSuperchat = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=RedTierDonator')
      }).first();

      await expect(redSuperchat).toBeVisible({ timeout: 5000 });

      const redStyle = await redSuperchat.getAttribute('style');
      log.debug(`Red tier SuperChat style: "${redStyle}"`);

      // Red tier should have red color (0xD00000 = #D00000 = rgb(208, 0, 0))
      expect(redStyle).toBeTruthy();
      const hasRedColor = redStyle?.toLowerCase().includes('d00000') ||
                          redStyle?.match(/rgb\(208,\s*0,\s*0\)/);
      expect(hasRedColor).toBeTruthy();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Additional Message Types', () => {
    test('should display SuperSticker with amount', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add SuperSticker message
      await addMockMessage({
        message_type: 'supersticker',
        author: 'StickerUser',
        content: '',
        amount: '¥1,500',
      });

      // Verify SuperSticker is displayed
      await expect(mainPage.locator('text=StickerUser')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.locator('text=¥1,500')).toBeVisible();
      await expect(mainPage.locator('text=スーパーステッカー')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display membership milestone message', async () => {
      // Milestone months are extracted from badge tooltip (e.g., "Member (12 months)")

      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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

  test.describe('Special Text Patterns', () => {
    test('should display stream title with hashtags correctly', async () => {
      // Set stream title with hashtags
      const fullTitle = '【雑談配信】今日もゲーム実況 #vtuber #ゲーム実況 #参加型';
      await setStreamState({ title: fullTitle });

      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=hashtag_title_test`);
      await mainPage.locator('button:has-text("開始")').click();

      // Wait for connection
      await expect(mainPage.getByText('【雑談配信】').first()).toBeVisible({ timeout: 10000 });

      // Find the title element specifically (in InputSection - the div with truncate class)
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
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=special_title_test`);
      await mainPage.locator('button:has-text("開始")').click();

      // Wait for connection
      await expect(mainPage.getByText('🎮 Gaming Stream').first()).toBeVisible({ timeout: 10000 });

      // Find the title element specifically (in InputSection - the div with truncate class)
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
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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
    test('初見さんの最初のメッセージに🎉初見さんバッジと#1が表示される', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_first_time`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add message from a new user (test DB is clean, so all users are first-time)
      await addMockMessage({
        message_type: 'text',
        author: 'NewUser',
        content: 'はじめてのコメントです',
        channel_id: 'UC_new_1',
        is_member: false,
      });

      // Find the message element and verify 🎉初見さん badge and #1 are present
      const messageEl = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=はじめてのコメントです'),
      }).first();
      await expect(messageEl.getByText('🎉初見さん')).toBeVisible({ timeout: 5000 });
      await expect(messageEl.getByText('#1')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('初見さんの2回目のメッセージにはmutedな初見さんと#2が表示される', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_repeat`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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

      // 2回目: 🎉初見さん(目立つ)は無いが、初見さん(muted)と#2が表示される
      const secondMessageEl = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=2回目のコメント'),
      }).first();
      await expect(secondMessageEl.getByText('🎉初見さん')).not.toBeVisible();
      await expect(secondMessageEl.getByText('初見さん')).toBeVisible();
      await expect(secondMessageEl.getByText('#2')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('異なるユーザーはそれぞれ初見バッジが表示される', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_multi_first`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

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

      // 両方に🎉初見さんと#1が表示されることを確認
      const messageA = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=UserAlphaのコメント'),
      }).first();
      await expect(messageA).toBeVisible({ timeout: 5000 });
      await expect(messageA.getByText('🎉初見さん')).toBeVisible();
      await expect(messageA.getByText('#1')).toBeVisible();

      const messageB = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=UserBetaのコメント'),
      }).first();
      await expect(messageB).toBeVisible();
      await expect(messageB.getByText('🎉初見さん')).toBeVisible();
      await expect(messageB.getByText('#1')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });
});
