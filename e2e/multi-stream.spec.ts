import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  mockServerUrl,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  disconnectAndInitialize,
  setStreamState,
  openUrlInput,
  openControlPanel,
  connectionChips,
  openViewerPanel,
  saveReadingInPanel,
} from './utils/test-helpers';

/**
 * E2E tests for Multi-Stream Connection.
 *
 * 複数の配信に同時接続し、メッセージを受信・表示できることを確認する。
 * 対応仕様: 上の行の接続の札, 操作パネルの接続の一覧, 個別切断, 全切断, 配信元インジケーター
 */

/**
 * ストリームに接続し、上の行に接続の札 (title は配信タイトル) が追加されるのを待つ
 * 2 本目以降は「+」で操作パネルを開いて入力する (openUrlInput)
 */
async function connectStream(page: Page, videoId: string, expectedTitle: string): Promise<void> {
  const urlInput = await openUrlInput(page);
  await urlInput.fill(`${mockServerUrl()}/watch?v=${videoId}`);
  await page.getByRole('button', { name: '接続', exact: true }).click();
  await expect(connectionChips(page, expectedTitle).first()).toBeVisible({ timeout: 10000 });
}

test.describe('Multi-Stream Connection', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000);

    log.info('Setting up test environment for Multi-Stream tests...');
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
    await disconnectAndInitialize(mainPage);
  });

  test.describe('同時接続', () => {
    test('2つの配信に同時接続できる', async () => {
      // Stream 1に接続
      await connectStream(mainPage, 'test_video_123', 'Mock Live');

      // 接続リストに1件表示されている
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(1);

      // Stream 2に接続（既存接続を切断しない）
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 接続リストに2件表示されている
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(2);

      // 両方のストリーム情報が表示されている
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live 2"]').first()).toBeVisible();

      log.debug('2つの同時接続が確認された');
    });

    test('接続リストに各配信の情報（タイトル）が表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 上の行の札は 2 つ
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(2);

      // 操作パネルの接続の一覧に、接続ごとに配信タイトルが出る
      await openControlPanel(mainPage);
      const items = mainPage.locator('[data-testid="connection-item"]');
      await expect(items).toHaveCount(2);
      await expect(items.nth(0).locator('[data-testid="stream-title"]')).toHaveText('Mock Live');
      await expect(items.nth(1).locator('[data-testid="stream-title"]')).toHaveText('Mock Live 2');
    });

    test('各接続アイテムに個別切断ボタン（×）が表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 各接続アイテムに切断ボタンがある
      const disconnectBtns = mainPage.locator('[data-testid="connection-chip"] button');
      await expect(disconnectBtns).toHaveCount(2);
    });

    test('札が上の行に収まらないとき、ホイールの上下で札の列が横にスクロールする', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      const strip = mainPage.locator('[data-testid="connection-strip"]');
      const scrollLeft = () => strip.evaluate((el) => el.scrollLeft);
      const [width, height] = await mainPage.evaluate(() => [window.innerWidth, window.innerHeight]);
      try {
        // 札 2 つが入らない幅に縮める (最小幅 540 より狭いが、札の列があふれる状態を作るため)
        await mainPage.setViewportSize({ width: 300, height });
        await expect.poll(() => strip.evaluate((el) => el.scrollWidth > el.clientWidth)).toBe(true);
        expect(await scrollLeft()).toBe(0);

        const box = await strip.boundingBox();
        if (!box) throw new Error('札の列が見つからない');
        await mainPage.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await mainPage.mouse.wheel(0, 100);
        await expect.poll(scrollLeft).toBeGreaterThan(0);
      } finally {
        await mainPage.setViewportSize({ width, height });
      }
    });
  });

  test.describe('メッセージ表示', () => {
    test('両方のストリームのメッセージが表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // Stream 1へのメッセージを追加
      const msg1Content = `Stream1Msg_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'Stream1User',
        content: msg1Content,
        channel_id: 'UC_stream1_user',
      });

      // メッセージが表示される
      await expect(mainPage.locator(`text=${msg1Content}`)).toBeVisible({ timeout: 8000 });

      // Stream 2へのメッセージを追加
      const msg2Content = `Stream2Msg_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'Stream2User',
        content: msg2Content,
        channel_id: 'UC_stream2_user',
      });

      // 両方のメッセージが表示される
      await expect(mainPage.locator(`text=${msg1Content}`)).toBeVisible({ timeout: 8000 });
      await expect(mainPage.locator(`text=${msg2Content}`)).toBeVisible({ timeout: 8000 });

      log.debug('両ストリームのメッセージが表示された');
    });

    test('2接続時にメッセージの配信元インジケーターが表示される', async () => {
      // 2 接続のポーリングの時刻が近いと接続2 にはほとんど届かないので、接続1 がポーリングした直後
      // (同期用のメッセージが見えた時点) から周期の半分ほどずらして接続2 を接続する (「視聴者パネルの配信者」と同じ)
      const syncContent = `IndicatorSync_${Date.now()}`;
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await addMockMessage({ message_type: 'text', author: 'IndicatorSync', content: syncContent, channel_id: 'UC_indicator_sync' });
      await expect(mainPage.locator(`text=${syncContent}`)).toBeVisible({ timeout: 8000 });
      await mainPage.waitForTimeout(700);
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 仕様「1 行の形」: 相手の配信 (2 つ目以降の接続) のコメントには名前の行の上に配信者名の行が付き、
      // 主の配信 (最初の接続) の行には付かない。
      // モックのキューは全接続共通で、積んだあと最初にポーリングした接続に届く。どちらの接続に届くかは
      // 決まらないので、積む時刻を 130ms ずつずらして周期を掃き、両方の行が見えるまで積む
      const runId = Date.now();
      let sawPartner = false;
      let sawPrimary = false;
      for (let i = 0; i < 16 && !(sawPartner && sawPrimary); i++) {
        await mainPage.waitForTimeout(i * 130);
        const content = `IndicatorTest_${runId}_${i}`;
        await addMockMessage({
          message_type: 'text',
          author: 'IndicatorUser',
          content,
          channel_id: 'UC_indicator_user',
        });
        const messageEl = mainPage.locator('[data-message-id]').filter({ hasText: content }).first();
        await expect(messageEl).toBeVisible({ timeout: 8000 });

        const origin = await messageEl.getAttribute('data-origin');
        const label = messageEl.locator('[data-testid="source-label"]');
        log.debug(`${content} の data-origin: ${origin}`);
        if (origin === 'partner') {
          await expect(label).toHaveText('MockBroadcaster2');
          sawPartner = true;
        } else {
          expect(origin).toBe('primary');
          await expect(label).toHaveCount(0);
          sawPrimary = true;
        }
      }
      expect(sawPartner, '2 つ目の接続の行 (配信者名の行つき) が見えた').toBe(true);
      expect(sawPrimary, '最初の接続の行 (札なし) が見えた').toBe(true);

      log.debug('配信元インジケーターの表示が確認された');
    });
  });

  // spec: 02_chat.md「視聴者情報パネル」の「どの配信者のもとで開くか」
  test.describe('視聴者パネルの配信者', () => {
    test('2つ目の接続に届いたコメントの視聴者は、その接続の配信者のもとで読み仮名を保存できる', async () => {
      // モックのメッセージキューは全接続で共通で、積んだあと最初にポーリング（1,500ms 周期）した接続に届く。
      // 2 接続のポーリングの時刻が近いと接続2 にはほとんど届かないので、接続1 がポーリングした直後
      // （同期用のメッセージが見えた時点）から周期の半分ほどずらして接続2 を接続する
      const runId = Date.now();
      await setStreamState({ channel_id: 'UC_panel_broadcaster_a' });
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      const syncContent = `PanelSync_${runId}`;
      await addMockMessage({ message_type: 'text', author: 'PanelSync', content: syncContent, channel_id: 'UC_panel_sync' });
      await expect(mainPage.locator(`text=${syncContent}`)).toBeVisible({ timeout: 8000 });
      await mainPage.waitForTimeout(700);
      // 接続1（配信者A）と接続2（配信者B）で配信者を分ける。モックの /watch は接続時に上書きを読む
      await setStreamState({ channel_id: 'UC_panel_broadcaster_b' });
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 接続2（配信者名 MockBroadcaster2）に届くまで、視聴者を変えて積み直す。
      // 前のメッセージが見えた直後に積むと毎回同じ接続に拾われるので、積む時刻を 130ms ずつずらして周期を掃く
      let target: { content: string; channelId: string } | null = null;
      for (let i = 0; i < 12 && target === null; i++) {
        await mainPage.waitForTimeout(i * 130);
        const content = `PanelBroadcaster_${runId}_${i}`;
        const channelId = `UC_panel_viewer_${runId}_${i}`;
        await addMockMessage({ message_type: 'text', author: `PanelViewer${i}`, content, channel_id: channelId });
        const messageEl = mainPage.locator('[data-message-id]').filter({ hasText: content }).first();
        await expect(messageEl).toBeVisible({ timeout: 8000 });
        // 札があるのは相手の配信 (2 つ目の接続) の行だけ。主の配信の行には無い
        const source = await messageEl.locator('[data-testid="source-label"]').textContent({ timeout: 500 }).catch(() => null);
        log.debug(`${content} は ${source ?? '主の配信'} の接続に届いた`);
        if (source?.trim() === 'MockBroadcaster2') target = { content, channelId };
      }
      expect(target, '12 件とも接続1 に届いた').not.toBeNull();
      const { content, channelId } = target!;

      // 接続2 の配信者 B のもとにはこの視聴者のプロフィールがある。最初の接続の配信者 A のもとには無い。
      // 接続が 2 つなので、保存先の配信者名をパネルに出す (02_chat.md「視聴者の読み仮名・メモ」)
      const panel = await openViewerPanel(mainPage, content);
      await expect(panel.getByTestId('viewer-broadcaster')).toHaveText('保存先 MockBroadcaster2');
      await saveReadingInPanel(mainPage, 'びーのしちょうしゃ');
      log.debug(`${channelId} を配信者 B のもとに保存した`);

      // 開き直すと、B のもとに保存した読み仮名が読み込まれる
      const reopened = await openViewerPanel(mainPage, content);
      await expect(reopened.getByLabel('読み仮名')).toHaveValue('びーのしちょうしゃ', { timeout: 5000 });
      await reopened.getByLabel('読み仮名').press('Escape');
    });
  });

  test.describe('個別切断', () => {
    test('1つ目の×ボタンをクリックすると1件だけ切断される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(2);

      // 1つ目の接続の×ボタンをクリック
      const firstDisconnectBtn = mainPage.locator('[data-testid="connection-chip"] button').first();
      await firstDisconnectBtn.click();

      // 接続リストが1件になる
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(1, { timeout: 10000 });

      log.debug('1件の個別切断が確認された');
    });

    test('操作パネルの「切断」ボタンでもその接続だけ切断される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      await openControlPanel(mainPage);
      const items = mainPage.locator('[data-testid="connection-item"]');
      await expect(items).toHaveCount(2);
      await items.nth(1).getByRole('button', { name: '切断', exact: true }).click();

      // 2つ目だけが切れ、1つ目の札が残る
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(1, { timeout: 10000 });
      await expect(connectionChips(mainPage, 'Mock Live')).toBeVisible();
    });

    test('個別切断後も残りの接続が正常に機能する', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      // 1つ目の接続を切断
      const firstDisconnectBtn = mainPage.locator('[data-testid="connection-chip"] button').first();
      await firstDisconnectBtn.click();
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(1, { timeout: 10000 });

      // 残りの接続でメッセージを受信できる
      const remainingMsg = `RemainingStream_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'RemainingUser',
        content: remainingMsg,
        channel_id: 'UC_remaining_user',
      });

      await expect(mainPage.locator(`text=${remainingMsg}`)).toBeVisible({ timeout: 8000 });

      log.debug('個別切断後の残接続が正常に動作している');
    });
  });

  test.describe('全切断', () => {
    test('全切断ボタンで全ての接続が切断される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(2);

      // 全切断ボタンは操作パネルにあり、2件以上の接続時に表示される
      await openControlPanel(mainPage);
      const disconnectAllBtn = mainPage.getByRole('button', { name: '全切断' });
      await expect(disconnectAllBtn).toBeVisible({ timeout: 5000 });

      await disconnectAllBtn.click();

      // 接続リストが空になる
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0, { timeout: 10000 });

      log.debug('全切断が確認された');
    });

    test('1件の接続時は全切断ボタンが表示されない', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');

      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(1);

      // 1件の場合は全切断ボタンは表示されない (操作パネルを開いて確かめる)
      await openControlPanel(mainPage);
      await expect(mainPage.locator('[data-testid="connection-item"]')).toHaveCount(1);
      await expect(mainPage.getByRole('button', { name: '全切断' })).not.toBeVisible();
    });
  });

  test.describe('接続状態の管理', () => {
    test('全切断後にURL入力フォームが引き続き表示される', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      await openControlPanel(mainPage);
      await mainPage.getByRole('button', { name: '全切断' }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0, { timeout: 10000 });

      // 全切断後もURLフォームは表示される
      await expect(mainPage.locator('#connect-url')).toBeVisible();
      await expect(mainPage.getByRole('button', { name: '接続', exact: true })).toBeVisible();
    });

    test('全切断後に再度接続できる', async () => {
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await connectStream(mainPage, 'test_video_456', 'Mock Live 2');

      await openControlPanel(mainPage);
      await mainPage.getByRole('button', { name: '全切断' }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(0, { timeout: 10000 });

      // 再接続
      await connectStream(mainPage, 'test_video_123', 'Mock Live');
      await expect(mainPage.locator('[data-testid="connection-chip"]')).toHaveCount(1);
    });
  });
});
