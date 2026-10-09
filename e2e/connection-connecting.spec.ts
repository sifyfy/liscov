import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  connectToMockStream,
  disconnectAndInitialize,
  openUrlInput,
} from './utils/test-helpers';

/**
 * 接続中（connecting）の操作（02_chat.md「多接続」「接続状態遷移」）
 * モックの /watch を遅らせて接続中の時間を作る。
 */
test.describe('接続中の操作 (02_chat.md)', () => {
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

  const items = () => mainPage.locator('[data-testid="connection-chip"]');

  async function startConnecting(videoId: string) {
    await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ watch_delay_ms: 3000 }),
    });
    await (await openUrlInput(mainPage)).fill(`${MOCK_SERVER_URL}/watch?v=${videoId}`);
    await mainPage.getByRole('button', { name: '接続', exact: true }).click();
    await expect(mainPage.getByRole('button', { name: '接続中…' })).toBeVisible();
  }

  // 仕様: 接続中のエントリの切断ボタンは押せない。成立したら切断できる
  test('接続中は切断ボタンを押せず、成立したら切断できる', async () => {
    await startConnecting('test_video_connecting');
    const disconnectBtn = items().first().locator('button');
    await expect(disconnectBtn).toBeDisabled();

    await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });
    await expect(disconnectBtn).toBeEnabled();
    await expect(await openUrlInput(mainPage)).toHaveValue('');
    await disconnectBtn.click();
    await expect(items()).toHaveCount(0);
  });

  // 仕様: 全切断は接続済みだけを切る。接続中のものは残り、成立したら一覧に出る
  test('接続中に全切断すると、接続済みだけが切れて接続中のものは成立する', async () => {
    await connectToMockStream(mainPage, 'test_video_connected_first');
    await startConnecting('test_video_connecting_second');
    await expect(items()).toHaveCount(2);

    await mainPage.locator('button:has-text("全切断")').click();
    await expect(items()).toHaveCount(1);

    await expect(items().first().locator('button')).toBeEnabled({ timeout: 10000 });
    const connections = await mainPage.evaluate(() =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke: (cmd: string) => Promise<unknown[]> } })
        .__TAURI_INTERNALS__.invoke('get_connections')
    );
    expect(connections).toHaveLength(1);
  });
});
