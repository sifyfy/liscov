import { test } from '@playwright/test';
import { setupTauriMock } from './tauri-mock';

test.describe('Screenshots', () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMock(page);
  });

  test('Chat tab', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(500);
    await page.screenshot({ path: '.temp/screenshots/01-chat-tab.png', fullPage: true });
  });

  test('Analytics tab', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Analytics' }).click();
    await page.waitForTimeout(500);
    await page.screenshot({ path: '.temp/screenshots/02-analytics-tab.png', fullPage: true });
  });

  test('Settings - Auth', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Settings' }).click();
    await page.waitForTimeout(500);
    await page.screenshot({ path: '.temp/screenshots/03-settings-auth.png', fullPage: true });
  });

  test('Settings - TTS', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Settings' }).click();
    await page.getByRole('button', { name: 'TTS読み上げ' }).click();
    await page.waitForTimeout(500);
    await page.screenshot({ path: '.temp/screenshots/04-settings-tts.png', fullPage: true });
  });
});
