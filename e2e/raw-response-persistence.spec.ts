import { test, expect } from './utils/fixtures';
import type { Browser, Page } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  connectToMockStream,
  navigateToTab,
  restartApp,
  getTestAppDataDir,
} from './utils/test-helpers';

/**
 * E2E tests for raw response save settings based on 05_raw_response.md.
 *
 * Tests verify:
 * - 設定を変更すると config.toml の [raw_response] に保存される
 * - 接続中に有効化すると、再接続なしで次のレスポンスから保存される
 * - 相対パスの保存先はアプリデータディレクトリ（「実際の保存先」表示と一致）
 * - 再起動後も有効のまま
 *
 * Run tests:
 *    pnpm exec playwright test --config e2e/playwright.config.ts raw-response-persistence.spec.ts
 */

const rawResponsePath = () => path.join(getTestAppDataDir(), 'raw_responses.ndjson');

function readConfigToml(): string {
  const configPath = path.join(getTestAppDataDir(), 'config.toml');
  return fs.existsSync(configPath) ? fs.readFileSync(configPath, 'utf-8') : '';
}

async function openRawResponseSettings(page: Page): Promise<void> {
  await navigateToTab(page, 'Settings');
  await page.getByRole('button', { name: '生レスポンス保存' }).click();
  await expect(page.getByRole('heading', { name: '生レスポンス保存設定' })).toBeVisible();
}

const enabledToggle = (page: Page) => page.getByLabel('生レスポンス保存を有効化');

test.describe.serial('Raw Response Save Settings', () => {
  // アプリ2回起動が必要なためタイムアウトを延長
  test.setTimeout(180000);

  let browser: Browser;
  let page: Page;

  test.beforeAll(async () => {
    ({ browser, page } = await setupTestEnvironment());
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test('接続中に有効化すると次のレスポンスからアプリデータディレクトリに保存される', async () => {
    // 無効のまま接続する
    await connectToMockStream(page);

    await openRawResponseSettings(page);
    await expect(enabledToggle(page)).not.toBeChecked();
    await enabledToggle(page).check();

    // 「実際の保存先」はアプリデータディレクトリ配下
    await expect(page.locator('code').filter({ hasText: 'raw_responses.ndjson' })).toContainText(
      getTestAppDataDir()
    );

    // config.toml の [raw_response] に保存される
    await expect.poll(readConfigToml).toMatch(/\[raw_response\][^[]*enabled\s*=\s*true/);

    // 再接続せずに、次のポーリングからファイルに書き込まれる
    await expect.poll(() => fs.existsSync(rawResponsePath()), { timeout: 15000 }).toBe(true);
  });

  test('再起動後も有効のまま', async () => {
    await browser.close();
    ({ browser, page } = await restartApp());
    await page.getByRole('heading', { name: 'Chat Monitor' }).waitFor({ state: 'visible', timeout: 30000 });

    await openRawResponseSettings(page);
    await expect(enabledToggle(page)).toBeChecked();
  });
});
