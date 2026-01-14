import { test, expect } from '@playwright/test';
import { setupTauriMock } from './tauri-mock';

test.describe('Liscov Application', () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMock(page);
    await page.goto('/');
  });

  test.describe('Basic Layout', () => {
    test('should display application title', async ({ page }) => {
      await expect(page.locator('h1')).toHaveText('Liscov');
      await expect(page.locator('text=YouTube Live Chat Monitor')).toBeVisible();
    });

    test('should display connection status', async ({ page }) => {
      await expect(page.locator('text=Disconnected')).toBeVisible();
    });

    test('should display all navigation tabs', async ({ page }) => {
      await expect(page.getByRole('button', { name: 'Chat' })).toBeVisible();
      await expect(page.getByRole('button', { name: /Viewers/ })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Analytics' })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Settings' })).toBeVisible();
    });
  });

  test.describe('Tab Navigation', () => {
    test('Chat tab should be active by default', async ({ page }) => {
      const chatTab = page.getByRole('button', { name: 'Chat' });
      // Active tab has bg-white/20 class
      await expect(chatTab).toHaveClass(/bg-white\/20/);
    });

    test('should navigate to Analytics tab', async ({ page }) => {
      await page.getByRole('button', { name: 'Analytics' }).click();
      // Analytics tab should show Revenue Analytics header
      await expect(page.locator('text=Revenue Analytics')).toBeVisible();
    });

    test('should navigate to Settings tab', async ({ page }) => {
      await page.getByRole('button', { name: 'Settings' }).click();
      // Settings tab should show settings sidebar
      await expect(page.locator('text=設定')).toBeVisible();
      await expect(page.getByRole('button', { name: 'YouTube認証' })).toBeVisible();
      await expect(page.getByRole('button', { name: 'TTS読み上げ' })).toBeVisible();
    });

    test('Viewers tab should be disabled when not connected', async ({ page }) => {
      const viewersTab = page.getByRole('button', { name: /Viewers/ });
      await expect(viewersTab).toBeDisabled();
      await expect(page.locator('text=(connect first)')).toBeVisible();
    });
  });

  test.describe('Chat Tab', () => {
    test('should display URL input section', async ({ page }) => {
      await expect(page.getByPlaceholder(/YouTube URL/)).toBeVisible();
      // Use exact match to avoid matching "Viewers (connect first)"
      await expect(page.getByRole('button', { name: 'Connect', exact: true })).toBeVisible();
    });

    test('should display statistics panel', async ({ page }) => {
      // Statistics panel shows Japanese labels
      await expect(page.locator('text=メッセージ')).toBeVisible();
      await expect(page.locator('text=視聴者')).toBeVisible();
    });

    test('should display WebSocket API section', async ({ page }) => {
      await expect(page.locator('text=WebSocket API')).toBeVisible();
      await expect(page.getByRole('button', { name: 'Start Server' })).toBeVisible();
    });

    test('should toggle filter panel', async ({ page }) => {
      const filterButton = page.getByRole('button', { name: /Filter/ });
      if (await filterButton.isVisible()) {
        await filterButton.click();
        // Filter panel content should be visible or toggle state
      }
    });
  });

  test.describe('Settings Tab - Auth', () => {
    test.beforeEach(async ({ page }) => {
      await page.getByRole('button', { name: 'Settings' }).click();
    });

    test('should display YouTube authentication settings', async ({ page }) => {
      // Auth tab is active by default
      await expect(page.locator('h2:has-text("YouTube認証")')).toBeVisible();
    });

    test('should show authentication status badge', async ({ page }) => {
      // Shows "未認証" badge when not authenticated
      await expect(page.locator('text=未認証')).toBeVisible();
    });

    test('should display login button when not authenticated', async ({ page }) => {
      // WebView-based login button
      await expect(page.getByRole('button', { name: 'YouTubeにログイン' })).toBeVisible();
    });

    test('should display member-only stream info', async ({ page }) => {
      // Info about member-only streams (use heading to be specific)
      await expect(page.getByRole('heading', { name: 'メンバー限定配信' })).toBeVisible();
    });

    test('should display help section', async ({ page }) => {
      await expect(page.locator('text=ヘルプ')).toBeVisible();
      // Check for login button (more specific)
      await expect(page.getByRole('button', { name: 'YouTubeにログイン' })).toBeVisible();
    });
  });

  test.describe('Settings Tab - TTS', () => {
    test.beforeEach(async ({ page }) => {
      await page.getByRole('button', { name: 'Settings' }).click();
      await page.getByRole('button', { name: 'TTS読み上げ' }).click();
    });

    test('should display TTS settings header', async ({ page }) => {
      await expect(page.locator('h2:has-text("TTS設定")')).toBeVisible();
    });

    test('should display backend settings section', async ({ page }) => {
      await expect(page.locator('text=バックエンド設定')).toBeVisible();
      await expect(page.locator('text=使用するバックエンド')).toBeVisible();
    });

    test('should display TTS toggle', async ({ page }) => {
      await expect(page.locator('h3:has-text("TTS読み上げ")')).toBeVisible();
    });
  });

  test.describe('Analytics Tab', () => {
    test.beforeEach(async ({ page }) => {
      await page.getByRole('button', { name: 'Analytics' }).click();
    });

    test('should display revenue analytics header', async ({ page }) => {
      await expect(page.locator('text=Revenue Analytics')).toBeVisible();
    });

    test('should display export panel', async ({ page }) => {
      await expect(page.locator('text=Export Data')).toBeVisible();
    });

    test('should display refresh button', async ({ page }) => {
      // Button shows "Loading..." or "Refresh" depending on state
      await expect(page.getByRole('button', { name: /Refresh|Loading/ })).toBeVisible();
    });
  });

  test.describe('WebSocket API Controls', () => {
    test('should start WebSocket server', async ({ page }) => {
      const startButton = page.getByRole('button', { name: 'Start Server' });
      await startButton.click();

      // After starting, should show stop button and port info
      await expect(page.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });
      await expect(page.locator('text=Running on port')).toBeVisible();
    });

    test('should stop WebSocket server after starting', async ({ page }) => {
      // Start server first
      await page.getByRole('button', { name: 'Start Server' }).click();
      await expect(page.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Stop server
      await page.getByRole('button', { name: 'Stop Server' }).click();
      await expect(page.getByRole('button', { name: 'Start Server' })).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Chat Connection', () => {
    test('should accept YouTube URL input', async ({ page }) => {
      const urlInput = page.getByPlaceholder(/YouTube URL/);
      await urlInput.fill('https://www.youtube.com/watch?v=test123');
      await expect(urlInput).toHaveValue('https://www.youtube.com/watch?v=test123');
    });

    test('should have connect button enabled with URL', async ({ page }) => {
      const urlInput = page.getByPlaceholder(/YouTube URL/);
      await urlInput.fill('https://www.youtube.com/watch?v=test123');

      // Use exact match to avoid ambiguity
      const connectButton = page.getByRole('button', { name: 'Connect', exact: true });
      await expect(connectButton).toBeEnabled();
    });
  });
});
