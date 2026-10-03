import { DatabaseSync } from 'node:sqlite';
import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  connectToMockStream,
  connectToApp,
  startTauriApp,
  killTauriApp,
  forceKillTauriApp,
  getTestDatabasePath,
} from './utils/test-helpers';

/**
 * セッションの閉じ方（08_database.md「セッションライフサイクル」）
 * アプリを止めたあと、アプリを介さずにテスト用 DB を読んで end_time を確かめる。
 */
test.describe('セッションの終了 (08_database.md)', () => {
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

  function sessionOf(videoId: string): { end_time: string | null; total_messages: number } {
    const db = new DatabaseSync(getTestDatabasePath(), { readOnly: true });
    try {
      return db
        .prepare('SELECT end_time, total_messages FROM sessions WHERE video_id = ? ORDER BY start_time DESC LIMIT 1')
        .get(videoId) as { end_time: string | null; total_messages: number };
    } finally {
      db.close();
    }
  }

  async function connectAndReceive(videoId: string) {
    await connectToMockStream(mainPage, videoId);
    await addMockMessage({ message_type: 'text', author: '@viewer-session', content: `${videoId} のコメント` });
    await expect(mainPage.getByText(`${videoId} のコメント`)).toBeVisible({ timeout: 10000 });
  }

  async function startAgain() {
    await startTauriApp();
    const connection = await connectToApp();
    browser = connection.browser;
    mainPage = connection.page;
    await expect(mainPage.locator('nav button:has-text("Chat")')).toBeVisible({ timeout: 30000 });
  }

  // 仕様: アプリを終了 → 接続中のセッションを切断と同じく閉じる
  test('アプリを終了すると、接続中のセッションを閉じる', async () => {
    test.setTimeout(120000);
    await connectAndReceive('test_video_session_exit');

    await killTauriApp(); // ウィンドウを閉じる終了を先に試す

    const session = sessionOf('test_video_session_exit');
    expect(session.end_time).not.toBeNull();
    expect(session.total_messages).toBe(1);
    await startAgain();
  });

  // 仕様: 起動時に end_time が NULL のセッション → 閉じる（最後のメッセージを保存した時刻）
  test('強制終了で閉じられなかったセッションは、次の起動で閉じる', async () => {
    test.setTimeout(120000);
    await connectAndReceive('test_video_session_crash');

    await forceKillTauriApp();
    expect(sessionOf('test_video_session_crash').end_time).toBeNull();

    await startAgain();
    const session = sessionOf('test_video_session_crash');
    expect(session.end_time).not.toBeNull();
    expect(session.total_messages).toBe(1);
  });
});
