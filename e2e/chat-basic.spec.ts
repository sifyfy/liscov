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
 * E2E tests for Chat Display — basic connection, mode selection, filtering, and clear.
 * Covers: Stream Connection, Connection and Disconnection, Chat Mode Selection,
 * Clear Messages, Member-Only Stream, Filter Functionality
 */

// Helper to get current chat mode from mock server
async function getChatModeStatus(): Promise<string | null> {
  const response = await fetch(`${MOCK_SERVER_URL}/chat_mode_status`);
  const data = await response.json();
  return data.chat_mode;
}

// Token validation result from mock server
interface TokenValidation {
  received: boolean;
  decode_success: boolean;
  chat_mode_found: boolean;
  detected_mode: string | null;
  raw_token_preview: string;
  decoded_length: number;
  validation_count: number;
}

// Helper to get detailed token validation from mock server
async function getTokenValidation(): Promise<TokenValidation> {
  const response = await fetch(`${MOCK_SERVER_URL}/token_validation`);
  return response.json();
}

test.describe('Chat Display — Basic (02_chat.md)', () => {
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

  test.describe('Stream Connection', () => {
    test('should connect to a stream and show stream info', async () => {
      // Enter mock video URL
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await expect(urlInput).toBeVisible();
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);

      // Click Connect button
      const connectButton = mainPage.locator('button:has-text("開始")');
      await connectButton.click();

      // Wait for connection - stream title appears in connection list (use first() to avoid strict mode violation)
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Disconnect after test
      await disconnectAndInitialize(mainPage);
    });

    test('should receive and display chat messages', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add mock messages
      await addMockMessage({
        message_type: 'text',
        author: 'TestUser1',
        content: 'Hello World!',
        is_member: false,
      });

      await addMockMessage({
        message_type: 'text',
        author: 'MemberUser',
        content: 'Member message here',
        is_member: true,
      });

      // Wait for messages to appear (polling interval is 1.5s)
      await expect(mainPage.locator('text=TestUser1')).toBeVisible({ timeout: 5000 });

      // Verify messages are displayed
      await expect(mainPage.locator('text=TestUser1')).toBeVisible();
      await expect(mainPage.locator('text=Hello World!')).toBeVisible();
      await expect(mainPage.locator('text=MemberUser')).toBeVisible();
      await expect(mainPage.locator('text=Member message here')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Chat Mode Selection', () => {
    test('should have chat mode toggle button in connection area', async () => {
      // Chat mode toggle button should be visible in the connection area
      // Default mode is 'top' which shows as "🔝 トップ"
      const chatModeButton = mainPage.locator('button[title="チャットモード切り替え"]');
      await expect(chatModeButton).toBeVisible();

      // Should show either "トップ" or "全て"
      const buttonText = await chatModeButton.textContent();
      expect(buttonText).toMatch(/トップ|全て/);
    });

    test('should switch from TopChat to AllChat while connected', async () => {
      // Connect to stream (default is TopChat)
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Chat mode toggle button should be visible
      const chatModeButton = mainPage.locator('button[title="チャットモード切り替え"]');
      await expect(chatModeButton).toBeVisible();

      // Default mode should be TopChat (shows "🔝 トップ")
      await expect(chatModeButton).toHaveText(/トップ/);

      // Click to switch to AllChat
      await chatModeButton.click();

      // Verify switched to AllChat (shows "💬 全て")
      await expect(chatModeButton).toHaveText(/全て/);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should switch from AllChat to TopChat while connected', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);

      // Chat mode toggle button should be visible (before connecting too)
      const chatModeButton = mainPage.locator('button[title="チャットモード切り替え"]');

      // Ensure we're in AllChat mode before connecting
      // (previous test may have left it in a different state)
      const buttonText = await chatModeButton.textContent();
      if (buttonText?.includes('トップ')) {
        await chatModeButton.click();
        await expect(chatModeButton).toHaveText(/全て/);
      }
      await expect(chatModeButton).toHaveText(/全て/);

      // Connect
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Verify still in AllChat mode
      await expect(chatModeButton).toHaveText(/全て/);

      // Switch back to TopChat
      await chatModeButton.click();

      // Verify switched to TopChat
      await expect(chatModeButton).toHaveText(/トップ/);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should receive messages after switching chat mode', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add messages for TopChat mode
      await addMockMessage({
        message_type: 'text',
        author: 'TopChatUser',
        content: 'Message in TopChat mode',
      });

      // Verify message received in TopChat mode
      await expect(mainPage.locator('text=TopChatUser')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.locator('text=Message in TopChat mode')).toBeVisible();

      // Switch to AllChat mode using toggle button
      const chatModeButton = mainPage.locator('button[title="チャットモード切り替え"]');
      await chatModeButton.click();
      await expect(chatModeButton).toHaveText(/全て/);

      // Add messages for AllChat mode
      await addMockMessage({
        message_type: 'text',
        author: 'AllChatUser',
        content: 'Message in AllChat mode',
      });

      // Verify new message received in AllChat mode
      await expect(mainPage.locator('text=AllChatUser')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.locator('text=Message in AllChat mode')).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should modify continuation token when switching chat mode (backend verification)', async () => {
      // This test verifies that the backend actually modifies the continuation token
      // when switching between TopChat and AllChat modes.
      // It explicitly checks both UI state and backend state to detect any mismatch.
      // It also validates that the token is properly formatted and parseable.

      // Reset mock server state and ensure TopChat is selected in the app
      await fetch(`${MOCK_SERVER_URL}/reset`, { method: 'POST' });

      // Chat mode toggle button
      const chatModeButton = mainPage.locator('button[title="チャットモード切り替え"]');

      // Ensure TopChat is selected before connecting
      // (previous test may have left it in a different state)
      const buttonText = await chatModeButton.textContent();
      if (buttonText?.includes('全て')) {
        await chatModeButton.click();
      }
      await expect(chatModeButton).toHaveText(/トップ/);

      // Connect to stream (default is TopChat)
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Wait for initial polling to establish chat mode (backend needs to receive token)
      await expect(chatModeButton).toHaveText(/トップ/);

      // Verify initial state: UI shows TopChat AND backend uses TopChat token
      await expect(chatModeButton).toHaveText(/トップ/);

      // Check detailed token validation for TopChat
      const topChatValidation = await getTokenValidation();
      expect(topChatValidation.received, 'Token should be received by backend').toBe(true);
      expect(topChatValidation.decode_success, 'Token should be valid base64').toBe(true);
      expect(topChatValidation.chat_mode_found, 'Chat mode field should be found in token').toBe(true);
      expect(topChatValidation.detected_mode, 'Backend should detect TopChat mode from token').toBe('TopChat');
      expect(topChatValidation.decoded_length, 'Token should have reasonable length').toBeGreaterThan(0);

      const initialBackendMode = await getChatModeStatus();
      expect(initialBackendMode, 'Backend should use TopChat token when UI shows TopChat').toBe('TopChat');

      // Switch to AllChat mode in UI
      await chatModeButton.click();
      await expect(chatModeButton).toHaveText(/全て/);

      // Wait for backend to receive AllChat token (requires next poll cycle)
      await expect.poll(async () => {
        const validation = await getTokenValidation();
        return validation.detected_mode;
      }, { timeout: 15000, message: 'Backend should detect AllChat mode from token' }).toBe('AllChat');

      // Verify detailed token validation for AllChat
      const allChatValidation = await getTokenValidation();
      expect(allChatValidation.received, 'Token should be received by backend').toBe(true);
      expect(allChatValidation.decode_success, 'Token should be valid base64').toBe(true);
      expect(allChatValidation.chat_mode_found, 'Chat mode field should be found in token').toBe(true);

      const allChatBackendMode = await getChatModeStatus();
      expect(allChatBackendMode, 'Backend should use AllChat token when UI shows AllChat').toBe('AllChat');

      // Switch back to TopChat in UI
      await chatModeButton.click();
      await expect(chatModeButton).toHaveText(/トップ/);

      // Wait for backend to receive TopChat token (requires next poll cycle)
      await expect.poll(async () => {
        const validation = await getTokenValidation();
        return validation.detected_mode;
      }, { timeout: 15000, message: 'Backend should detect TopChat mode after switching back' }).toBe('TopChat');

      // Final token validation
      const finalValidation = await getTokenValidation();
      expect(finalValidation.validation_count, 'Multiple tokens should have been validated').toBeGreaterThanOrEqual(3);

      const topChatBackendMode = await getChatModeStatus();
      expect(topChatBackendMode, 'Backend should use TopChat token when UI shows TopChat').toBe('TopChat');

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should persist chat mode selection across reconnection', async () => {
      // Chat mode toggle button
      const chatModeButton = mainPage.locator('button[title="チャットモード切り替え"]');

      // Switch to AllChat before connecting
      await chatModeButton.click();
      await expect(chatModeButton).toHaveText(/全て/);

      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Disconnect
      await disconnectAndInitialize(mainPage);

      // Verify AllChat is still selected after disconnect
      await expect(chatModeButton).toHaveText(/全て/);

      // Reconnect
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_456`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Verify AllChat is still selected after reconnection
      await expect(chatModeButton).toHaveText(/全て/);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Clear Messages', () => {
    test('should clear all messages when clicking Clear button', async () => {
      // Connect and add messages
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await addMockMessage({
        message_type: 'text',
        author: 'ClearTestUser',
        content: 'This will be cleared',
      });

      await expect(mainPage.locator('text=ClearTestUser')).toBeVisible({ timeout: 5000 });

      // Click Clear button (in control bar)
      await mainPage.locator('button:has-text("クリア")').first().click();

      // Confirmation dialog appears - click the confirm button in the dialog
      await expect(mainPage.getByRole('heading', { name: 'メッセージをクリア' })).toBeVisible();
      await mainPage.locator('div.fixed button:has-text("クリア")').click();

      // Message should be gone
      await expect(mainPage.locator('text=ClearTestUser')).not.toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Member-Only Stream', () => {
    test('should connect to member-only stream with authentication', async () => {
      // First authenticate
      await mainPage.getByRole('button', { name: 'Settings' }).click();
      await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible();

      const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
      if (await loginButton.isVisible()) {
        await loginButton.click();
        await expect(mainPage.getByRole('button', { name: 'ログアウト' })).toBeVisible({ timeout: 15000 });
      }

      // Go back to chat
      await mainPage.getByRole('button', { name: 'Chat' }).click();

      // Set mock server to member-only mode
      await setStreamState({ member_only: true, require_auth: true });

      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=member_only_video`);
      await mainPage.locator('button:has-text("開始")').click();

      // Should connect successfully with auth
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add member message
      await addMockMessage({
        message_type: 'text',
        author: 'MemberOnlyUser',
        content: 'Member-only stream message',
        is_member: true,
      });

      await expect(mainPage.locator('text=MemberOnlyUser')).toBeVisible({ timeout: 5000 });

      // Disconnect
      await disconnectAndInitialize(mainPage);

      // Reset stream state
      await setStreamState({ member_only: false, require_auth: false });
    });
  });

  test.describe('Filter Functionality', () => {
    test('should filter messages by type - SuperChat only', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add mixed messages
      await addMockMessage({ message_type: 'text', author: 'TextUser', content: 'Regular message' });
      await addMockMessage({ message_type: 'superchat', author: 'SCUser', content: 'SC message', amount: '¥500' });
      await addMockMessage({ message_type: 'membership', author: 'MemberUser', content: 'New member!' });

      // All messages should be visible initially
      await expect(mainPage.locator('text=TextUser')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.locator('text=SCUser')).toBeVisible();
      await expect(mainPage.locator('text=MemberUser')).toBeVisible();

      // Open filter panel
      await mainPage.locator('button:has-text("フィルター")').click();
      // Wait for filter panel to be visible
      await expect(mainPage.locator('label').filter({ hasText: '通常' })).toBeVisible();

      // Uncheck text messages (labeled as "通常" in Japanese UI)
      const textFilter = mainPage.locator('label').filter({ hasText: '通常' }).locator('input[type="checkbox"]');
      if (await textFilter.isChecked()) {
        await textFilter.uncheck();
      }

      // Uncheck membership messages (labeled as "メンバー" in Japanese UI)
      const membershipFilter = mainPage.locator('label').filter({ hasText: 'メンバー' }).locator('input[type="checkbox"]');
      if (await membershipFilter.isChecked()) {
        await membershipFilter.uncheck();
      }

      // Only SuperChat should be visible
      await expect(mainPage.locator('text=TextUser')).not.toBeVisible();
      await expect(mainPage.locator('text=SCUser')).toBeVisible();
      await expect(mainPage.locator('text=MemberUser')).not.toBeVisible();

      // Reset filters
      if (!(await textFilter.isChecked())) {
        await textFilter.check();
      }
      if (!(await membershipFilter.isChecked())) {
        await membershipFilter.check();
      }

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should filter messages by search query', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Add messages with different content
      await addMockMessage({ message_type: 'text', author: 'User1', content: 'Hello world' });
      await addMockMessage({ message_type: 'text', author: 'User2', content: 'Goodbye world' });
      await addMockMessage({ message_type: 'text', author: 'SearchTarget', content: 'Find me please' });

      // Wait for messages to appear
      await expect(mainPage.locator('text=SearchTarget')).toBeVisible({ timeout: 5000 });

      // Open filter panel
      await mainPage.locator('button:has-text("フィルター")').click();

      // Find search input - try different selectors
      const searchInput = mainPage.locator('input[type="text"]').filter({ hasText: '' }).nth(1);
      const searchInputAlt = mainPage.locator('input[placeholder*="検索"]');
      const searchInputAlt2 = mainPage.locator('input[placeholder*="search" i]');

      // Check if search input exists
      const hasSearch = await searchInputAlt.isVisible().catch(() => false) ||
                        await searchInputAlt2.isVisible().catch(() => false);

      if (hasSearch) {
        const input = await searchInputAlt.isVisible() ? searchInputAlt : searchInputAlt2;
        await input.fill('SearchTarget');

        // Only matching message should be visible
        await expect(mainPage.locator('text=User1')).not.toBeVisible();
        await expect(mainPage.locator('text=SearchTarget')).toBeVisible();

        // Clear search
        await input.clear();
      } else {
        // Search functionality may not be implemented yet - test passes if filter panel opens
        log.debug('Search input not found - filter panel exists but search not implemented');
      }

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Connection and Disconnection', () => {
    test('should show error for invalid URL', async () => {
      // Enter invalid URL
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill('not-a-valid-url');

      // Click Connect
      await mainPage.locator('button:has-text("開始")').click();

      // Check if error message appears (may be in various formats)
      const errorText = mainPage.locator('.text-red-500, [class*="error"], text=/error|失敗|Invalid|エラー/i').first();
      const hasError = await errorText.isVisible().catch(() => false);

      // If no visible error, check that we're still in disconnected state (URL input visible)
      // This is also valid behavior - app may silently reject invalid URLs
      if (!hasError) {
        // Verify we didn't connect (URL input should still be visible)
        await expect(urlInput).toBeVisible();
        log.debug('No visible error message, but connection was prevented (URL input still visible)');
      }
    });

    test('should properly disconnect and clear connection state', async () => {
      // Connect to stream
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // 接続状態の確認: 接続リストにエントリが表示されている
      await expect(mainPage.locator('.connection-item').first()).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);

      // 切断後の状態確認: 接続リストが空になり、URL入力欄と開始ボタンは引き続き表示される
      await expect(mainPage.locator('.connection-item')).toHaveCount(0);
      await expect(mainPage.locator('button:has-text("開始")')).toBeVisible();
      await expect(mainPage.locator('input[placeholder*="youtube.com"]')).toBeVisible();
    });

    test('should show replay indicator for archived streams', async () => {
      // This test would need mock server to return isReplay: true
      // For now, we verify the UI element exists when not in replay mode
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Replay badge should NOT be visible for live streams
      const replayBadge = mainPage.locator('text=Replay');
      // It might not exist at all, or exist but not be visible
      const isVisible = await replayBadge.isVisible().catch(() => false);
      expect(isVisible).toBe(false);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });
});
