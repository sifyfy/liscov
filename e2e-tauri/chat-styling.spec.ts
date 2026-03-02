import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
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
 * E2E tests for Chat Display — visual styling and display settings.
 * Covers: Author Name Color Coding, Font Size and Display Settings,
 * Display Settings, Message Display Styling, Layout and Overflow
 */

test.describe('Chat Display — Styling (02_chat.md)', () => {
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

  test.describe('Author Name Color Coding', () => {
    test('should display member names in member-accent color and non-member names in accent color', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Use unique identifiers to avoid text matching issues
      const nonMemberName = 'BlueViewer123';
      const memberName = 'GreenViewer456';
      const nonMemberContent = 'Hello from non-subscriber';
      const memberContent = 'Hello from subscriber';

      // Add non-member message first
      await addMockMessage({
        message_type: 'text',
        author: nonMemberName,
        content: nonMemberContent,
        channel_id: 'UC_blue_viewer',
        is_member: false,
      });

      // Wait for first message
      await mainPage.waitForTimeout(2000);
      await expect(mainPage.getByText(nonMemberName).first()).toBeVisible();

      // Check non-member color - should use var(--accent) CSS variable
      // Dark theme: #6fb8d9 = rgb(111, 184, 217)
      const nonMemberMessage = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${nonMemberContent}`)
      }).first();
      const nonMemberAuthor = nonMemberMessage.locator('span').filter({ hasText: nonMemberName }).first();
      const nonMemberColor = await nonMemberAuthor.evaluate(el => getComputedStyle(el).color);
      expect(nonMemberColor).toMatch(/rgb\(111,\s*184,\s*217\)/);

      // Add member message
      await addMockMessage({
        message_type: 'text',
        author: memberName,
        content: memberContent,
        channel_id: 'UC_green_viewer',
        is_member: true,
      });

      // Wait for second message with longer timeout
      await mainPage.waitForTimeout(3000);
      await expect(mainPage.getByText(memberName).first()).toBeVisible();

      // Check member color - should use var(--member-accent) CSS variable
      // Dark theme: #6ec98a = rgb(110, 201, 138)
      const memberMessage = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${memberContent}`)
      }).first();
      const memberAuthor = memberMessage.locator('span').filter({ hasText: memberName }).first();
      const memberColor = await memberAuthor.evaluate(el => getComputedStyle(el).color);

      expect(memberColor).toMatch(/rgb\(110,\s*201,\s*138\)/);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Font Size and Display Settings', () => {
    test('should increase font size when clicking A+ button', async () => {
      // Get initial font size
      const fontSizeDisplay = mainPage.locator('text=/\\d+px/').first();
      const initialSize = await fontSizeDisplay.textContent();
      const initialNum = parseInt(initialSize?.replace('px', '') || '13');

      // Click increase button
      const increaseButton = mainPage.locator('button[title="文字を大きく"]');
      await increaseButton.click();
      await mainPage.waitForTimeout(100);

      // Verify size increased
      const newSize = await fontSizeDisplay.textContent();
      const newNum = parseInt(newSize?.replace('px', '') || '13');
      expect(newNum).toBe(initialNum + 1);
    });

    test('should decrease font size when clicking A- button', async () => {
      // Get initial font size
      const fontSizeDisplay = mainPage.locator('text=/\\d+px/').first();
      const initialSize = await fontSizeDisplay.textContent();
      const initialNum = parseInt(initialSize?.replace('px', '') || '13');

      // Click decrease button
      const decreaseButton = mainPage.locator('button[title="文字を小さく"]');
      await decreaseButton.click();
      await mainPage.waitForTimeout(100);

      // Verify size decreased
      const newSize = await fontSizeDisplay.textContent();
      const newNum = parseInt(newSize?.replace('px', '') || '13');
      expect(newNum).toBe(initialNum - 1);
    });

    test('should toggle timestamp display', async () => {
      // Connect to stream and add message
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await addMockMessage({
        message_type: 'text',
        author: 'TimestampUser',
        content: 'Check timestamp',
      });

      await mainPage.waitForTimeout(3000);

      // Get timestamp toggle
      const timestampToggle = mainPage.locator('label:has-text("タイムスタンプ") input[type="checkbox"]');

      // If checked, timestamps should be visible
      const isChecked = await timestampToggle.isChecked();

      if (isChecked) {
        // Timestamp should be visible (HH:MM:SS format)
        const timestamp = mainPage.locator('text=/\\d{2}:\\d{2}:\\d{2}/').first();
        await expect(timestamp).toBeVisible();

        // Uncheck and verify timestamp is hidden
        await timestampToggle.uncheck();
        await mainPage.waitForTimeout(100);

        // Timestamp should not be visible
        await expect(timestamp).not.toBeVisible();
      }

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Display Settings', () => {
    test('should respect font size boundaries (10-24px)', async () => {
      // Get initial font size
      const fontSizeDisplay = mainPage.locator('text=/\\d+px/').first();
      const decreaseButton = mainPage.locator('button[title="文字を小さく"]');
      const increaseButton = mainPage.locator('button[title="文字を大きく"]');

      // Decrease to minimum
      for (let i = 0; i < 20; i++) {
        await decreaseButton.click();
        await mainPage.waitForTimeout(50);
      }

      // Should be at minimum (10px)
      let size = await fontSizeDisplay.textContent();
      expect(parseInt(size?.replace('px', '') || '0')).toBeGreaterThanOrEqual(10);

      // Increase to maximum
      for (let i = 0; i < 20; i++) {
        await increaseButton.click();
        await mainPage.waitForTimeout(50);
      }

      // Should be at maximum (24px)
      size = await fontSizeDisplay.textContent();
      expect(parseInt(size?.replace('px', '') || '0')).toBeLessThanOrEqual(24);

      // Reset to default (13px)
      for (let i = 0; i < 15; i++) {
        await decreaseButton.click();
        await mainPage.waitForTimeout(50);
      }
    });
  });

  test.describe('Message Display Styling', () => {
    test('should display member message with green styling', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add member message
      await addMockMessage({
        message_type: 'text',
        author: 'GreenBgMember',
        content: 'Member with green background',
        is_member: true,
      });

      await mainPage.waitForTimeout(3000);

      // Find the message element
      const memberMessage = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=GreenBgMember')
      }).first();

      await expect(memberMessage).toBeVisible();

      // Check for green styling - could be on border, background, or author name
      // Option 1: Check author name color (green for members)
      const authorSpan = memberMessage.locator('span').filter({ hasText: 'GreenBgMember' }).first();
      const authorColor = await authorSpan.evaluate(el => getComputedStyle(el).color);

      // Should use var(--member-accent): dark theme #6ec98a = rgb(110, 201, 138)
      expect(authorColor).toMatch(/rgb\(110,\s*201,\s*138\)/);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display author icon', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add message
      await addMockMessage({
        message_type: 'text',
        author: 'IconUser',
        content: 'User with icon',
      });

      await mainPage.waitForTimeout(3000);

      // Find message and check for icon (img element)
      const message = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=IconUser')
      }).first();

      const icon = message.locator('img').first();
      await expect(icon).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Layout and Overflow', () => {
    test('should not overflow window width with long stream title', async () => {
      // Set a very long stream title via mock server
      const longTitle = 'This is an extremely long stream title that should be truncated with ellipsis and not cause the application window to overflow beyond the viewport width - Testing overflow handling for Japanese YouTube Live streams with very long titles 超長いタイトルのテスト日本語も含めて配信タイトル';

      await setStreamState({ title: longTitle });

      // Connect to stream with long title
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();

      // Wait for connection and title display
      await mainPage.waitForTimeout(3000);

      // Find the title element (look for <div> containing part of the long title)
      // The title should be in the InputSection component
      const titleElement = mainPage.locator('[data-testid="stream-title"]').first();

      await expect(titleElement).toBeVisible({ timeout: 5000 });

      // CRITICAL: Check that the title element has proper overflow handling
      // Without these styles, long titles will cause horizontal overflow
      const styles = await titleElement.evaluate(el => {
        const style = window.getComputedStyle(el);
        return {
          overflow: style.overflow,
          textOverflow: style.textOverflow,
          whiteSpace: style.whiteSpace,
          // Also check if the text is actually being clipped
          scrollWidth: el.scrollWidth,
          clientWidth: el.clientWidth,
        };
      });

      // The title MUST have truncation styles to prevent overflow
      // Tailwind's `truncate` class applies: overflow-hidden, text-overflow: ellipsis, white-space: nowrap
      const hasTruncation = styles.textOverflow === 'ellipsis' && styles.overflow === 'hidden';

      // Assert: Title must have ellipsis truncation applied
      expect(hasTruncation).toBeTruthy();

      // Additional check: If text is being clipped, scrollWidth should be >= clientWidth
      // and the visible text should be shorter than the full title
      if (styles.scrollWidth > styles.clientWidth) {
        // Text is being truncated (scrollWidth > clientWidth means content overflows)
        // This is expected behavior with truncation
        console.log(`Title truncated: scrollWidth=${styles.scrollWidth}, clientWidth=${styles.clientWidth}`);
      }

      // Disconnect
      await disconnectAndInitialize(mainPage);

      // Reset stream title
      await setStreamState({ title: '' });
    });
  });
});
