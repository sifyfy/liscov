import * as fs from 'fs';
import * as path from 'path';
import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  setStreamState,
  connectToMockStream,
  disconnectAndInitialize,
  getTestAppDataDir,
} from './utils/test-helpers';

/**
 * チャット取得の失敗と切断（02_chat.md「取得に失敗したとき」「メッセージ受信」）
 * モックの get_live_chat を遅らせる・失敗させて、監視ループの振る舞いを実 Tauri で確かめる。
 */
test.describe('チャット取得の失敗 (02_chat.md)', () => {
  let browser: Browser;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000);
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    mainPage = connection.page;
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test.beforeEach(async () => {
    await resetMockServer();
  });

  test.afterEach(async () => {
    await resetMockServer();
    await disconnectAndInitialize(mainPage);
  });

  const connectionItems = () => mainPage.locator('.connection-item');
  const logText = () => {
    const logFile = path.join(getTestAppDataDir(), 'logs', 'liscov.log');
    return fs.existsSync(logFile) ? fs.readFileSync(logFile, 'utf-8') : '';
  };

  // 仕様: 取得の応答を待っている間に切断 → 応答を待たずに止まる
  test('取得の応答を待っている間に切断しても、すぐに切断が終わる', async () => {
    await setStreamState({ chat_delay_ms: 20000 });
    await connectToMockStream(mainPage, 'test_video_fetch_hang');
    // 最初の取得が応答待ちになるまで少し待つ
    await mainPage.waitForTimeout(1000);

    const started = Date.now();
    await mainPage.locator('.connection-item .disconnect-btn').first().click();
    await expect(connectionItems()).toHaveCount(0, { timeout: 10000 });

    expect(Date.now() - started).toBeLessThan(3000);
  });

  // 仕様: リクエストのタイムアウトは全体15秒。応答が返らなければ失敗として数える
  test('応答が返らない取得は15秒でタイムアウトして失敗として記録される', async () => {
    test.setTimeout(60000);
    await setStreamState({ chat_delay_ms: 30000 });
    await connectToMockStream(mainPage, 'test_video_fetch_timeout');

    await expect
      .poll(() => /メッセージ取得失敗.*timed out/.test(logText()), { timeout: 25000 })
      .toBe(true);
  });

  // 仕様: 15回続けて失敗 → 切断し、エラーを表示する（待ちは 1.5秒×10、3・6・12・24秒で約60秒）
  test('取得に15回続けて失敗したら自動で切断してエラーを表示する', async () => {
    test.setTimeout(180000);
    await setStreamState({ chat_fail: true });
    await connectToMockStream(mainPage, 'test_video_fetch_fail');

    await expect(
      mainPage.getByText('チャットの取得に15回続けて失敗したため切断しました')
    ).toBeVisible({ timeout: 120000 });
    await expect(connectionItems()).toHaveCount(0);
  });
});
