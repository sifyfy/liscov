import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  startTauriApp,
  connectToApp,
  killTauriApp,
  cleanupTestData,
  cleanupTestCredentials,
} from './utils/test-helpers';

// Helper to wait for SvelteKit app to fully render (not just HTML load)
async function waitForAppReady(page: Page): Promise<void> {
  await expect(page.locator('nav button:has-text("Chat")')).toBeVisible({ timeout: 30000 });
}

// Navigate to TTS settings tab
async function navigateToTtsSettings(page: Page): Promise<void> {
  await page.getByRole('button', { name: 'Settings' }).click();
  await expect(page.getByRole('button', { name: 'TTS読み上げ' })).toBeVisible({ timeout: 5000 });
  await page.getByRole('button', { name: 'TTS読み上げ' }).click();
  await expect(page.getByRole('heading', { name: 'TTS設定' })).toBeVisible({ timeout: 5000 });
}

/**
 * E2E tests for TTS first comment settings UI.
 *
 * Tests verify:
 * - AC-7: 初回コメントプレフィックスのトグル・入力欄・プレースホルダ表示
 * - AC-9: 初回コメントのみ読み上げの独立トグル表示
 *
 * Run tests:
 *    pnpm exec playwright test --config e2e/playwright.config.ts tts-first-comment.spec.ts
 */

test.describe('TTS First Comment Settings', () => {
  let browser: Browser;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000);

    log.info('Starting Tauri app for TTS first comment tests...');
    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();
    await startTauriApp();

    const connection = await connectToApp();
    browser = connection.browser;

    mainPage = connection.page;

    await waitForAppReady(mainPage);
    log.info('Connected to Tauri app');
  });

  test.afterAll(async () => {
    if (browser) {
      await browser.close();
    }
    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test.beforeEach(async () => {
    let needsReconnect = false;
    try {
      await mainPage.evaluate(() => document.readyState);
      await mainPage.waitForLoadState('load', { timeout: 5000 });
    } catch {
      needsReconnect = true;
    }

    if (needsReconnect) {
      log.info('Page connection lost, attempting to reconnect...');
      const connection = await connectToApp();
      browser = connection.browser;
  
      mainPage = connection.page;
    }

    await waitForAppReady(mainPage);
    await navigateToTtsSettings(mainPage);
  });

  test('AC-7: should display first comment prefix toggle and input with placeholder', async () => {
    // プレフィックストグルが存在する
    const prefixToggle = mainPage.locator('[data-testid="first-comment-prefix-toggle"]');
    await expect(prefixToggle).toBeVisible({ timeout: 5000 });

    // デフォルトはOFF
    await expect(prefixToggle).toHaveAttribute('aria-pressed', 'false');

    // トグルをONにする
    await prefixToggle.click();
    await expect(prefixToggle).toHaveAttribute('aria-pressed', 'true');

    // 入力欄が表示される
    const prefixInput = mainPage.locator('[data-testid="first-comment-prefix-input"]');
    await expect(prefixInput).toBeVisible({ timeout: 5000 });

    // プレースホルダが「1回目のコメント。」
    await expect(prefixInput).toHaveAttribute('placeholder', '1回目のコメント。');

    // トグルをOFFに戻す
    await prefixToggle.click();
    await expect(prefixToggle).toHaveAttribute('aria-pressed', 'false');

    // 入力欄が非表示になる
    await expect(prefixInput).not.toBeVisible();
  });

  test('AC-9: should display first comment only toggle independently', async () => {
    // 初回コメントのみ読み上げトグルが存在する
    const firstCommentOnlyToggle = mainPage.locator('[data-testid="first-comment-only-toggle"]');
    await expect(firstCommentOnlyToggle).toBeVisible({ timeout: 5000 });

    // デフォルトはOFF
    await expect(firstCommentOnlyToggle).toHaveAttribute('aria-pressed', 'false');

    // プレフィックストグルとは独立して操作可能
    const prefixToggle = mainPage.locator('[data-testid="first-comment-prefix-toggle"]');
    await expect(prefixToggle).toBeVisible();

    // 初回コメントのみトグルをON
    await firstCommentOnlyToggle.click();
    await expect(firstCommentOnlyToggle).toHaveAttribute('aria-pressed', 'true');

    // プレフィックストグルはOFFのまま
    await expect(prefixToggle).toHaveAttribute('aria-pressed', 'false');

    // 元に戻す
    await firstCommentOnlyToggle.click();
    await expect(firstCommentOnlyToggle).toHaveAttribute('aria-pressed', 'false');
  });

  test('settings round-trip: toggles and prefix text persist after reload', async () => {
    // プレフィックストグルをONにする
    const prefixToggle = mainPage.locator('[data-testid="first-comment-prefix-toggle"]');
    await prefixToggle.click();
    await expect(prefixToggle).toHaveAttribute('aria-pressed', 'true');

    // カスタムプレフィックスを入力
    const prefixInput = mainPage.locator('[data-testid="first-comment-prefix-input"]');
    await expect(prefixInput).toBeVisible({ timeout: 5000 });
    await prefixInput.fill('初コメ！');

    // 初回コメントのみ読み上げトグルをONにする
    const firstCommentOnlyToggle = mainPage.locator('[data-testid="first-comment-only-toggle"]');
    await firstCommentOnlyToggle.click();
    await expect(firstCommentOnlyToggle).toHaveAttribute('aria-pressed', 'true');

    // デバウンス保存を待つ
    await mainPage.waitForTimeout(500);

    // ページリロード
    await mainPage.reload();
    await waitForAppReady(mainPage);
    await navigateToTtsSettings(mainPage);

    // 設定が永続化されていることを確認
    const prefixToggleAfter = mainPage.locator('[data-testid="first-comment-prefix-toggle"]');
    await expect(prefixToggleAfter).toHaveAttribute('aria-pressed', 'true');

    const prefixInputAfter = mainPage.locator('[data-testid="first-comment-prefix-input"]');
    await expect(prefixInputAfter).toBeVisible({ timeout: 5000 });
    await expect(prefixInputAfter).toHaveValue('初コメ！');

    const firstCommentOnlyAfter = mainPage.locator('[data-testid="first-comment-only-toggle"]');
    await expect(firstCommentOnlyAfter).toHaveAttribute('aria-pressed', 'true');

    // テスト後のクリーンアップ: 設定を戻す
    // 注: prefixInput はトグルが ON のときのみ表示されるため、必ず toggle OFF より
    // 前に fill('') でクリアする。順序を逆にすると非表示要素への fill で失敗する。
    await prefixInputAfter.fill('');
    await prefixToggleAfter.click();
    await firstCommentOnlyAfter.click();
    await mainPage.waitForTimeout(500);
  });
});
