import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  disconnectAndInitialize,
  navigateToTab,
} from './utils/test-helpers';

/**
 * E2E tests for Analytics tab rendering bug.
 *
 * Bug: When a contributor only sends SuperStickers (no SuperChats),
 * their `highest_tier` is `null` in the backend response.
 * RevenueDashboard.svelte accesses `tierConfig[null]` which is `undefined`,
 * causing a TypeError that breaks Svelte 5's reactive rendering pipeline.
 * This makes tab switching impossible while buttons inside the Analytics tab
 * still work (they use native DOM events, not Svelte reactivity).
 *
 * Run:
 *   pnpm exec playwright test --config e2e-tauri/playwright.config.ts e2e-tauri/analytics-tab-freeze.spec.ts
 */

// Helper to wait for app to be fully loaded
async function waitForAppReady(page: Page): Promise<void> {
  // Wait for the tab navigation to be visible (indicates SvelteKit app has rendered)
  await expect(page.locator('nav button:has-text("Chat")')).toBeVisible({ timeout: 30000 });
}

// NOTE: 共有版 connectToMockStream と異なり、waitForAppReady + 明示的 visibility check が必要
// analytics テストはタブ切替後の再接続が多く、UI安定化を待つ追加ステップが必須
async function connectToMockStream(page: Page): Promise<void> {
  await waitForAppReady(page);
  const urlInput = page.locator('input[placeholder*="youtube.com"]');
  await expect(urlInput).toBeVisible({ timeout: 10000 });
  await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
  await page.locator('button:has-text("開始")').click();
  await expect(page.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });
}

test.describe('Analytics Tab Freeze Bug', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000);

    log.info('Setting up test environment for Analytics tab freeze tests...');
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
    await resetMockServer();
  });

  test('should render analytics without crash when contributor has only SuperStickers (highest_tier=null)', async () => {
    // 1. Connect to mock stream
    await connectToMockStream(mainPage);

    // 2. Send SuperSticker-only messages from a contributor
    //    This creates a contributor with highest_tier = None/null in the backend
    await addMockMessage({
      message_type: 'supersticker',
      author: 'StickerOnlyUser',
      content: '',
      amount: '¥500',
      channel_id: 'UC_sticker_only_001',
    });
    await addMockMessage({
      message_type: 'supersticker',
      author: 'StickerOnlyUser',
      content: '',
      amount: '¥1,000',
      channel_id: 'UC_sticker_only_001',
    });

    // Also add a normal SuperChat for comparison
    await addMockMessage({
      message_type: 'superchat',
      author: 'NormalDonator',
      content: 'Great stream!',
      amount: '¥5,000',
      tier: 'red',
      channel_id: 'UC_normal_donor_001',
    });

    // 4. Navigate to Analytics tab
    await navigateToTab(mainPage, 'Analytics');

    // 5. Wait for analytics to load (the Refresh button should appear after loading)
    //    If the bug exists, this will crash: tierConfig[null].bgColor → TypeError
    await expect(mainPage.locator('button:has-text("Refresh")')).toBeVisible({ timeout: 15000 });

    // 6. Verify analytics data is displayed (stat cards should show counts)
    await expect(mainPage.locator('text=Total Paid Messages')).toBeVisible();
    await expect(mainPage.locator('text=Super Stickers')).toBeVisible();

    // 7. Disconnect
    await navigateToTab(mainPage, 'Chat');
    await disconnectAndInitialize(mainPage);
  });

  test('should allow tab switching after analytics renders with sticker-only contributors', async () => {
    // 1. Connect and send sticker-only messages
    await connectToMockStream(mainPage);

    await addMockMessage({
      message_type: 'supersticker',
      author: 'StickerFan',
      content: '',
      amount: '¥2,000',
      channel_id: 'UC_sticker_fan_001',
    });
    await addMockMessage({
      message_type: 'supersticker',
      author: 'StickerFan',
      content: '',
      amount: '¥3,000',
      channel_id: 'UC_sticker_fan_001',
    });

    // 2. Open Analytics tab
    await navigateToTab(mainPage, 'Analytics');

    // 3. Wait for analytics data to load
    //    With the bug, Loading... stays forever because Svelte rendering crashes
    await expect(mainPage.locator('button:has-text("Refresh")')).toBeVisible({ timeout: 15000 });

    // 4. Switch to Chat tab — this is the critical test
    //    With the bug, Svelte's reactivity is broken and tab switching fails
    await navigateToTab(mainPage, 'Chat');

    // 5. Verify Chat tab content is visible (URL input or connected state)
    const chatContent = mainPage.locator('button:has-text("停止"), input[placeholder*="youtube.com"]');
    await expect(chatContent.first()).toBeVisible({ timeout: 5000 });

    // 6. Switch to Settings tab to confirm navigation works
    await navigateToTab(mainPage, 'Settings');
    await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible({ timeout: 5000 });

    // 7. Switch back to Analytics
    await navigateToTab(mainPage, 'Analytics');
    await expect(mainPage.locator('text=Revenue Analytics')).toBeVisible({ timeout: 5000 });

    // 8. Cleanup
    await navigateToTab(mainPage, 'Chat');
    await disconnectAndInitialize(mainPage);
  });

  test('should display top contributors correctly even when highest_tier is null', async () => {
    // 1. Connect and create a scenario with mixed contributors
    await connectToMockStream(mainPage);

    // Sticker-only contributor (will have highest_tier=null)
    for (let i = 0; i < 3; i++) {
      await addMockMessage({
        message_type: 'supersticker',
        author: 'StickerKing',
        content: '',
        amount: '¥1,000',
        channel_id: 'UC_sticker_king',
      });
    }

    // Mixed contributor (has both stickers and superchat → will have a tier)
    await addMockMessage({
      message_type: 'supersticker',
      author: 'MixedUser',
      content: '',
      amount: '¥500',
      channel_id: 'UC_mixed_user',
    });
    await addMockMessage({
      message_type: 'superchat',
      author: 'MixedUser',
      content: 'Also a superchat!',
      amount: '¥10,000',
      tier: 'yellow',
      channel_id: 'UC_mixed_user',
    });

    // 2. Navigate to Analytics tab
    await navigateToTab(mainPage, 'Analytics');

    // 3. Wait for data to load
    await expect(mainPage.locator('button:has-text("Refresh")')).toBeVisible({ timeout: 15000 });

    // 4. Verify Top Contributors section renders without crashing
    //    StickerKing (3 contributions) and MixedUser should be in the DOM
    //    Even with highest_tier=null, this should render without crashing
    //    Note: Use textContent check instead of toBeVisible because the Analytics tab
    //    has an overflow-y-auto scroll container and Playwright considers items below
    //    the scroll viewport as "hidden" even though they are rendered in the DOM
    const analyticsContainer = mainPage.locator('.overflow-y-auto').filter({ has: mainPage.locator('text=Revenue Analytics') });
    await expect(analyticsContainer).toContainText('Top Contributors', { timeout: 5000 });
    await expect(analyticsContainer).toContainText('StickerKing', { timeout: 5000 });
    await expect(analyticsContainer).toContainText('MixedUser', { timeout: 5000 });

    // 5. Cleanup
    await navigateToTab(mainPage, 'Chat');
    await disconnectAndInitialize(mainPage);
  });
});
