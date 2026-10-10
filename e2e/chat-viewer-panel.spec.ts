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
  openUrlInput,
  openControlPanel,
  connectToMockStream,
  chatRow,
  viewerPanel,
  openViewerPanel,
  saveReadingInPanel,
} from './utils/test-helpers';

/**
 * E2E tests for Chat Display — ViewerInfoPanel and scroll behavior.
 * Covers: ViewerInfoPanel and Scroll, ViewerInfoPanel Details,
 * Auto-Scroll Behavior, Scroll Behavior with Auto-scroll, Display Limit (displayLimit)
 */

test.describe('Chat Display — Viewer Panel (02_chat.md)', () => {
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

  test.describe('ViewerInfoPanel and Scroll', () => {
    test('should have data-message-id attribute on messages', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${mockServerUrl()}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add message
      await addMockMessage({
        message_type: 'text',
        author: 'DataAttrUser',
        content: 'Check data attribute',
      });

      // Check for data-message-id attribute
      const messageElement = mainPage.locator('[data-message-id]').first();
      await expect(messageElement).toBeVisible({ timeout: 5000 });
      const messageId = await messageElement.getAttribute('data-message-id');
      expect(messageId).toBeTruthy();
      expect(messageId).toMatch(/^mock_msg_\d+$/);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  // spec: 02_chat.md「配信中の画面」の「自動スクロール」
  // 新着は最新まで追う / 手でスクロールして離れたら止め「最新へ」を出す / 30 秒操作が無ければ戻る / 手で最新まで戻れば再開
  test.describe('Auto-Scroll Behavior', () => {
    const latestButton = () => mainPage.getByRole('button', { name: /最新へ/ });
    const chatContainer = () =>
      mainPage.locator('[style*="overflow-y"]').filter({ has: mainPage.locator('[data-message-id]') }).first();
    const distanceFromBottom = () =>
      chatContainer().evaluate((el) => el.scrollHeight - el.scrollTop - el.clientHeight);

    async function connect() {
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${mockServerUrl()}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });
    }

    async function addMessages(prefix: string, from: number, to: number) {
      for (let i = from; i <= to; i++) {
        await addMockMessage({ message_type: 'text', author: `${prefix}User${i}`, content: `${prefix}_${String(i).padStart(2, '0')}` });
      }
    }

    /** 流れの上でホイールを上に回す (手でスクロールして最新から離れる) */
    async function scrollUpByHand() {
      const box = await chatContainer().boundingBox();
      if (!box) throw new Error('流れが見つからない');
      await mainPage.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await mainPage.mouse.wheel(0, -10000);
    }

    test('新着が来たら最新まで追う', async () => {
      await connect();
      await addMessages('Follow', 1, 20);
      await expect(mainPage.locator('text=Follow_20')).toBeVisible({ timeout: 8000 });
      await expect(latestButton()).toHaveCount(0);
      await disconnectAndInitialize(mainPage);
    });

    test('should scroll exactly to bottom (not partial scroll)', async () => {
      // This test verifies that auto-scroll reaches the exact bottom of the chat container
      // Bug: Sometimes scroll stops short of the actual bottom

      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${mockServerUrl()}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add many messages to create scrollable content
      for (let i = 1; i <= 30; i++) {
        await addMockMessage({ message_type: 'text', author: `ScrollUser${i}`, content: `Bottom scroll test message ${i}` });
      }

      // Wait for messages to be received and auto-scroll to complete
      await expect(mainPage.locator('text=Bottom scroll test message 30')).toBeVisible({ timeout: 10000 });

      // Find the virtua VList scroll container (has overflow-y in inline style)
      const chatContainer = mainPage.locator('[style*="overflow-y"]').filter({ has: mainPage.locator('[data-message-id]') }).first();

      // Check scroll position - should be at the exact bottom
      const scrollInfo = await chatContainer.evaluate(el => ({
        scrollTop: el.scrollTop,
        scrollHeight: el.scrollHeight,
        clientHeight: el.clientHeight,
        distanceFromBottom: el.scrollHeight - el.scrollTop - el.clientHeight,
      }));

      log.debug(`Scroll to bottom test: scrollTop=${scrollInfo.scrollTop}, scrollHeight=${scrollInfo.scrollHeight}, clientHeight=${scrollInfo.clientHeight}, distanceFromBottom=${scrollInfo.distanceFromBottom}`);

      // The distance from bottom should be very small (within threshold of 30px)
      // Perfect scroll would have distanceFromBottom === 0
      expect(scrollInfo.distanceFromBottom).toBeLessThanOrEqual(50);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('一度にたくさん届いても追うのをやめない (「最新へ」は出ない)', async () => {
      // 以前の不具合: たくさん届くと自動スクロールが外れていた。virtua が行の高さを測り直すときのスクロールでも止めない
      await connect();
      await addMessages('Init', 1, 10);
      await expect(mainPage.locator('text=Init_10')).toBeVisible({ timeout: 8000 });
      await addMessages('Burst', 1, 20);
      await expect(mainPage.locator('text=Burst_20')).toBeVisible({ timeout: 10000 });
      await expect(latestButton()).toHaveCount(0);
      expect(await distanceFromBottom()).toBeLessThanOrEqual(50);
      await disconnectAndInitialize(mainPage);
    });

    test('手でスクロールして離れたら止まり「最新へ」が出る。押すと最新に戻って追うのを再開する', async () => {
      await connect();
      await addMessages('Manual', 1, 40);
      await expect(mainPage.locator('text=Manual_40')).toBeVisible({ timeout: 10000 });

      await scrollUpByHand();
      await expect(mainPage.locator('text=Manual_01')).toBeVisible({ timeout: 5000 });
      await expect(latestButton()).toBeVisible();

      // ボタンはコメントの行に重ねない (流れの下端の帯に置く)
      const flowBox = await chatContainer().boundingBox();
      const buttonBox = await latestButton().boundingBox();
      if (!flowBox || !buttonBox) throw new Error('流れか「最新へ」が見つからない');
      expect(buttonBox.y).toBeGreaterThanOrEqual(flowBox.y + flowBox.height - 1);

      // 離れている間に届いても、見ている場所は動かない
      await addMessages('Manual', 41, 50);
      await expect(mainPage.locator('text=Manual_01')).toBeVisible({ timeout: 5000 });
      await expect(mainPage.locator('text=Manual_50')).not.toBeVisible({ timeout: 3000 });

      await latestButton().click();
      await expect(mainPage.locator('text=Manual_50')).toBeVisible({ timeout: 5000 });
      await expect(latestButton()).toHaveCount(0);

      // 再開したので、次に届いたものも追う
      await addMessages('Manual', 51, 55);
      await expect(mainPage.locator('text=Manual_55')).toBeVisible({ timeout: 8000 });
      await disconnectAndInitialize(mainPage);
    });

    test('最新へ合わせ直している途中でも、手でスクロールすれば離れられる (最新へ引き戻されない)', async () => {
      // 以前の不具合: 最新へ合わせる処理 (virtua の scrollToIndex) は行の高さが測れるまで合わせ直し続けるため、
      // その間 (新着・画面の大きさの変化の直後) に手でスクロールすると最新へ引き戻されていた。
      // 行の高さがばらばらで、まだ測っていない行へスクロールしたときに起きる
      await connect();
      for (let i = 1; i <= 60; i++) {
        await addMockMessage({ message_type: 'text', author: `PullUser${i}`, content: `Pull_${String(i).padStart(2, '0')} ${'ながいコメント'.repeat((i % 5) * 6)}` });
      }
      await expect(mainPage.locator('text=Pull_60')).toBeVisible({ timeout: 10000 });
      const [width, height] = await mainPage.evaluate(() => [window.innerWidth, window.innerHeight]);
      const box = await chatContainer().boundingBox();
      if (!box) throw new Error('流れが見つからない');
      try {
        // 画面の大きさを変える (最新へ合わせ直しが始まる) → すぐに手でスクロール
        await mainPage.setViewportSize({ width: width - 40, height });
        await mainPage.mouse.move(box.x + box.width / 2, box.y + 40);
        await mainPage.mouse.wheel(0, -1500);
        await expect(latestButton()).toBeVisible({ timeout: 3000 });
        await mainPage.waitForTimeout(500);
        await expect(latestButton()).toBeVisible();
        expect(await distanceFromBottom()).toBeGreaterThan(500);
      } finally {
        await mainPage.setViewportSize({ width, height });
      }
      await disconnectAndInitialize(mainPage);
    });

    test('手で最新までスクロールして戻ったら、追うのを再開する', async () => {
      await connect();
      await addMessages('Back', 1, 40);
      await expect(mainPage.locator('text=Back_40')).toBeVisible({ timeout: 10000 });
      await scrollUpByHand();
      await expect(latestButton()).toBeVisible();

      await mainPage.mouse.wheel(0, 20000);
      await expect(latestButton()).toHaveCount(0, { timeout: 5000 });
      await addMessages('Back', 41, 45);
      await expect(mainPage.locator('text=Back_45')).toBeVisible({ timeout: 8000 });
      await disconnectAndInitialize(mainPage);
    });

    test('離れたまま 30 秒、流れの上で操作が無ければ最新に戻る', async () => {
      test.setTimeout(120000);
      await connect();
      await addMessages('Idle', 1, 40);
      await expect(mainPage.locator('text=Idle_40')).toBeVisible({ timeout: 10000 });
      await scrollUpByHand();
      await expect(latestButton()).toBeVisible();
      // 流れの外にマウスを出し、操作しないで待つ
      await mainPage.mouse.move(0, 0);
      await addMessages('Idle', 41, 42);

      await expect(latestButton()).toHaveCount(0, { timeout: 40000 });
      await expect(mainPage.locator('text=Idle_42')).toBeVisible({ timeout: 5000 });
      await disconnectAndInitialize(mainPage);
    });
  });

  // spec: 02_chat.md「配信中の画面」の「視聴者の読み仮名・メモ (視聴者情報パネル)」と「自動スクロール」
  test.describe('視聴者情報パネル (行の中で広げる)', () => {
    const latestButton = () => mainPage.getByRole('button', { name: /最新へ/ });
    const chatContainer = () =>
      mainPage.locator('[style*="overflow-y"]').filter({ has: mainPage.locator('[data-message-id]') }).first();

    async function addFiller(prefix: string, count: number) {
      for (let i = 1; i <= count; i++) {
        await addMockMessage({ message_type: 'text', author: `${prefix}User${i}`, content: `${prefix}_${String(i).padStart(2, '0')}` });
      }
      await expect(chatRow(mainPage, `${prefix}_${String(count).padStart(2, '0')}`)).toBeVisible({ timeout: 10000 });
    }

    test('コメントを押すと行のすぐ下に広がり、読み仮名の欄にフォーカスする。打っている間に新着が来ても流れていかない', async () => {
      await connectToMockStream(mainPage);
      await addFiller('Before', 20);
      await addMockMessage({ message_type: 'text', author: 'InlineUser', content: 'Open me inline', channel_id: 'UC_inline_user' });

      // 最新のコメントを押す (下端に広がっても、流れは止まったまま)
      const panel = await openViewerPanel(mainPage, 'Open me inline');
      const rowBox = (await chatRow(mainPage, 'Open me inline').boundingBox())!;
      const panelBox = (await panel.boundingBox())!;
      expect(panelBox.y).toBeGreaterThanOrEqual(rowBox.y + rowBox.height - 1);
      expect(panelBox.y).toBeLessThan(rowBox.y + rowBox.height + 16);
      await expect(panel.getByText('InlineUser')).toBeVisible();
      await expect(panel.getByLabel('読み仮名')).toBeFocused();
      await expect(latestButton()).toBeVisible();

      await panel.getByLabel('読み仮名').pressSequentially('いんらいん');
      // 視聴者は前のテストで来ていない人にする (来たことがあると新規さんにならない。
      // channel_id を省くとモックが UC_user_0〜999 から乱数で選び、前の視聴者と重なることがある)
      const runId = Date.now();
      for (let i = 1; i <= 20; i++) {
        await addMockMessage({ message_type: 'text', author: `AfterUser${i}`, content: `After_${i}`, channel_id: `UC_after_${runId}_${i}` });
      }
      // 止まっているので新着は画面に出ず、「最新へ」に数で出る (どの視聴者も初めてなので新規さん)
      await expect(latestButton()).toContainText('新規さん 20', { timeout: 15000 });
      await expect(panel).toBeInViewport();
      await expect(panel.getByLabel('読み仮名')).toHaveValue('いんらいん');

      await panel.getByRole('button', { name: '閉じる' }).click();
      await expect(panel).toHaveCount(0);
      await disconnectAndInitialize(mainPage);
    });

    test('読み仮名を打って Enter で保存すると閉じ、行の名前の横に読み仮名が出て最新に戻る', async () => {
      await connectToMockStream(mainPage);
      await addMockMessage({
        message_type: 'text',
        author: 'ReadingTestUser',
        content: 'Test message for reading',
        channel_id: 'UC_reading_test_user',
      });
      await openViewerPanel(mainPage, 'Test message for reading');
      await saveReadingInPanel(mainPage, 'りーでぃんぐてすと');

      await expect(chatRow(mainPage, 'Test message for reading').getByText('(りーでぃんぐてすと)')).toBeVisible({ timeout: 5000 });
      await expect(latestButton()).toHaveCount(0);

      // 開き直すと、保存した読み仮名が読み込まれる
      const panel = await openViewerPanel(mainPage, 'Test message for reading');
      await expect(panel.getByLabel('読み仮名')).toHaveValue('りーでぃんぐてすと');
      await panel.getByLabel('読み仮名').press('Escape');
      await expect(panel).toHaveCount(0);
      await disconnectAndInitialize(mainPage);
    });

    // 02_chat.md「1 行の形」: メモが登録されている視聴者は、本文の下に「メモ」の札とメモを毎回出す
    test('メモを保存すると、画面の行の本文の下にすぐ出て、その後に届いたコメントにも出る', async () => {
      const channelId = `UC_notes_user_${Date.now()}`;
      await connectToMockStream(mainPage);
      await addMockMessage({ message_type: 'text', author: 'NotesUser', content: 'Notes before save', channel_id: channelId });
      const panel = await openViewerPanel(mainPage, 'Notes before save');
      const notes = panel.getByLabel('メモ');
      await notes.fill('ネタバレしがち。注意');
      await notes.press('Enter');
      await expect(panel).toHaveCount(0);

      const notesOf = (text: string) => chatRow(mainPage, text).getByTestId('author-notes');
      await expect(notesOf('Notes before save')).toHaveText(/メモ\s*ネタバレしがち。注意/, { timeout: 5000 });

      // 保存した後に届いたコメントには、バックエンドがメモを付けて送る
      await addMockMessage({ message_type: 'text', author: 'NotesUser', content: 'Notes after save', channel_id: channelId });
      await expect(notesOf('Notes after save')).toHaveText(/メモ\s*ネタバレしがち。注意/, { timeout: 10000 });
      await disconnectAndInitialize(mainPage);
    });

    test('Esc で保存せずに閉じる', async () => {
      await connectToMockStream(mainPage);
      await addMockMessage({ message_type: 'text', author: 'EscUser', content: 'Escape test', channel_id: 'UC_esc_user' });
      const panel = await openViewerPanel(mainPage, 'Escape test');
      await panel.getByLabel('読み仮名').fill('かきかけ');
      await panel.getByLabel('読み仮名').press('Escape');
      await expect(panel).toHaveCount(0);

      const reopened = await openViewerPanel(mainPage, 'Escape test');
      await expect(reopened.getByLabel('読み仮名')).toHaveValue('');
      await reopened.getByLabel('読み仮名').press('Escape');
      await disconnectAndInitialize(mainPage);
    });

    test('開く前から最新を離れていたら、閉じても離れたまま', async () => {
      await connectToMockStream(mainPage);
      await addFiller('Away', 40);
      const box = (await chatContainer().boundingBox())!;
      await mainPage.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await mainPage.mouse.wheel(0, -10000);
      await expect(latestButton()).toBeVisible();

      const panel = await openViewerPanel(mainPage, 'Away_03');
      await panel.getByLabel('読み仮名').press('Escape');
      await expect(panel).toHaveCount(0);
      await expect(latestButton()).toBeVisible();
      await expect(chatRow(mainPage, 'Away_03')).toBeInViewport();
      await disconnectAndInitialize(mainPage);
    });

    test('「この人のコメント」の行を押すと、流れのそのコメントへ移って強調する。パネルは元の行の下のまま', async () => {
      await connectToMockStream(mainPage);
      const target = 'UC_scroll_target_user';
      await addMockMessage({ message_type: 'text', author: 'ScrollTargetUser', content: 'FIRST_TARGET_MSG', channel_id: target });
      await addFiller('Filler', 25);
      await addMockMessage({ message_type: 'text', author: 'ScrollTargetUser', content: 'LATEST_TARGET_MSG', channel_id: target });

      const panel = await openViewerPanel(mainPage, 'LATEST_TARGET_MSG');
      const toggle = panel.getByRole('button', { name: /この人のコメント 2 件/ });
      await expect(toggle).toHaveAttribute('aria-expanded', 'false');
      await toggle.click();
      const scrollBefore = await chatContainer().evaluate((el) => el.scrollTop);
      await panel.getByTestId('viewer-history-item').filter({ hasText: 'FIRST_TARGET_MSG' }).click();

      const first = chatRow(mainPage, 'FIRST_TARGET_MSG');
      await expect(first).toBeInViewport({ timeout: 5000 });
      expect(await chatContainer().evaluate((el) => el.scrollTop)).toBeLessThan(scrollBefore);
      // 選択の強調: 行の根に class outline-accent (outline 2px)
      await expect(first).toHaveClass(/outline-accent/);
      expect(await first.evaluate((el) => window.getComputedStyle(el).outlineWidth)).toBe('2px');
      // パネルは画面の外に出ても、元の行の下に残っている
      await expect(panel).toHaveCount(1);

      await panel.getByRole('button', { name: '閉じる' }).click();
      await disconnectAndInitialize(mainPage);
    });

    test('打っている間は 30 秒たっても最新に戻らない', async () => {
      test.setTimeout(120000);
      await connectToMockStream(mainPage);
      await addFiller('Typing', 30);
      const panel = await openViewerPanel(mainPage, 'Typing_28');
      await panel.getByLabel('読み仮名').fill('うちかけ');
      await mainPage.mouse.move(0, 0);
      await expect(latestButton()).toBeVisible();
      // 戻らない間は、残り時間の帯を出さない
      await expect(mainPage.getByTestId('latest-bar')).toHaveCount(0);

      await mainPage.waitForTimeout(33000);
      await expect(panel).toBeVisible();
      await expect(panel.getByLabel('読み仮名')).toHaveValue('うちかけ');
      await expect(latestButton()).toBeVisible();

      // 閉じると、開く前に追っていたので最新に戻る
      await panel.getByLabel('読み仮名').press('Escape');
      await expect(latestButton()).toHaveCount(0, { timeout: 5000 });
      await disconnectAndInitialize(mainPage);
    });

    test('止まっている間に届いたスパチャ等・新規さんの件数を「最新へ」に出す', async () => {
      await connectToMockStream(mainPage);
      await addMockMessage({ message_type: 'text', author: 'CountBaseUser', content: 'Count base', channel_id: 'UC_count_base' });
      const panel = await openViewerPanel(mainPage, 'Count base');
      await expect(latestButton()).not.toContainText('スパチャ等');

      await addMockMessage({ message_type: 'superchat', author: 'CountSuperChatter', content: 'Paused superchat', amount: '¥500', tier: 'green' });
      await addMockMessage({ message_type: 'text', author: 'CountNewbie', content: 'はじめまして', channel_id: `UC_count_newbie_${Date.now()}` });
      await expect(latestButton()).toContainText('最新へ スパチャ等 1・新規さん 1', { timeout: 8000 });

      await panel.getByLabel('読み仮名').press('Escape');
      await expect(latestButton()).toHaveCount(0, { timeout: 5000 });
      await disconnectAndInitialize(mainPage);
    });

    test('流れたスパチャ等の欄で押すと、流れのそのコメントまで移ってその下に広げる', async () => {
      await connectToMockStream(mainPage);
      await addMockMessage({ message_type: 'superchat', author: 'FlowedChatter', content: 'Flowed superchat body', amount: '¥1,000', tier: 'yellow' });
      await addFiller('Flow', 40);
      const flowed = mainPage.getByRole('region', { name: '流れたスパチャ等' });
      await expect(flowed.getByText('Flowed superchat body')).toBeVisible({ timeout: 5000 });

      await flowed.locator('[data-message-id]').filter({ hasText: 'Flowed superchat body' }).click();
      const panel = viewerPanel(mainPage);
      await expect(panel.getByLabel('読み仮名')).toBeEnabled({ timeout: 5000 });
      await expect(panel).toBeInViewport();
      // 流れの中に広げる (欄の中ではない)。流れにそのコメントが見えるので、欄からは外れる
      await expect(chatContainer().getByRole('region', { name: '視聴者情報' })).toHaveCount(1);
      await expect(flowed).toHaveCount(0, { timeout: 5000 });

      await panel.getByLabel('読み仮名').press('Escape');
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Display Limit (displayLimit)', () => {
    test('should limit displayed messages when displayLimit is set', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${mockServerUrl()}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add 80 messages
      const addPromises = [];
      for (let i = 1; i <= 80; i++) {
        addPromises.push(addMockMessage({
          message_type: 'text',
          author: `LimitUser${i}`,
          content: `LimitMsg_${String(i).padStart(3, '0')}`,
        }));
      }
      await Promise.all(addPromises);

      // Verify all messages received (状態の行の件数)
      await expect(mainPage.getByText('80 件', { exact: true })).toBeVisible({ timeout: 10000 });

      // Set displayLimit to 50 via select
      await openControlPanel(mainPage);
      const displayLimitSelect = mainPage.getByRole('combobox', { name: '表示件数' });
      await displayLimitSelect.selectOption('50');
      // 選んだ件数が選択欄に出たまま (以前の不具合: 数値と選択肢の文字列が合わず空欄になっていた)
      await expect(displayLimitSelect).toHaveValue('50');

      // 状態の行に「表示中 50 / 全 80 件」が出る (全 80 件のうち 50 件だけ表示)
      await expect(mainPage.getByText('表示中 50 / 全 80 件')).toBeVisible({ timeout: 3000 });

      // Scroll VList to bottom to ensure latest message is rendered in DOM
      const vlistContainer = mainPage.locator('[style*="overflow-y"]').filter({ has: mainPage.locator('[data-message-id]') }).first();
      await vlistContainer.evaluate(el => { el.scrollTop = el.scrollHeight; });

      // Latest message should be visible (displayedMessages slices from end)
      await expect(mainPage.locator('text=LimitMsg_080')).toBeVisible({ timeout: 5000 });

      // Scroll to top to verify early messages are excluded
      await vlistContainer.evaluate(el => { el.scrollTop = 0; });

      // Early messages should NOT be in the DOM (excluded by displayLimit)
      await expect(mainPage.locator('text=LimitMsg_001')).not.toBeVisible();

      // Reset to unlimited
      await displayLimitSelect.selectOption('unlimited');

      // 隠しているものが無くなるので「表示中」は消える
      await expect(mainPage.getByText(/表示中/)).toHaveCount(0, { timeout: 3000 });

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should update status bar counts when displayLimit changes', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${mockServerUrl()}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add 30 messages sequentially (preserves order for VList scroll verification)
      for (let i = 1; i <= 30; i++) {
        await addMockMessage({
          message_type: 'text',
          author: `StatusUser${i}`,
          content: `StatusMsg_${String(i).padStart(3, '0')}`,
        });
      }

      // Verify initial state: unlimited (隠しているものが無いので「表示中」は出ない)
      await expect(mainPage.getByText('30 件', { exact: true })).toBeVisible({ timeout: 10000 });
      await expect(mainPage.getByText(/表示中/)).toHaveCount(0);

      // Switch to 50 (all 30 should still display since 30 < 50)
      await openControlPanel(mainPage);
      const displayLimitSelect = mainPage.getByRole('combobox', { name: '表示件数' });
      await displayLimitSelect.selectOption('50');

      // 30 件は 50 件の枠に収まるので、隠しているものが無く「表示中」は出ないまま
      await expect(mainPage.getByText('30 件', { exact: true })).toBeVisible();
      await expect(mainPage.getByText(/表示中/)).toHaveCount(0);

      // All messages should still be in displayedMessages (30 < 50)
      // VList virtualizes rendering, so scroll to check specific items
      const vlistContainer = mainPage.locator('[style*="overflow-y"]').filter({ has: mainPage.locator('[data-message-id]') }).first();

      // 手でいちばん上までスクロールして最初のコメントを確かめる
      // (最新を追っている間にプログラムで scrollTop を変えると、最新への合わせ直しで戻される。手でのスクロールなら追うのが止まる)
      await vlistContainer.hover();
      await mainPage.mouse.wheel(0, -20000);
      await expect(mainPage.locator('text=StatusMsg_001')).toBeVisible({ timeout: 5000 });

      // Scroll to bottom to verify last message
      await vlistContainer.evaluate(el => { el.scrollTop = el.scrollHeight; });
      await expect(mainPage.locator('text=StatusMsg_030')).toBeVisible({ timeout: 5000 });

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should restrict display to displayLimit with large message count', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${mockServerUrl()}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Set displayLimit to 100 before adding messages
      await openControlPanel(mainPage);
      const displayLimitSelect = mainPage.getByRole('combobox', { name: '表示件数' });
      await displayLimitSelect.selectOption('100');
      // 効いたことは、下で件数を受けたあとの「表示中 100 / 全 150 件」で確かめる

      // Add 150 messages in parallel batches for speed
      // 並列バッチ内の到着順は非決定的なので、最終メッセージは順次送信で保証
      for (let batch = 0; batch < 3; batch++) {
        const addPromises = [];
        for (let i = 1; i <= 50; i++) {
          const num = batch * 50 + i;
          addPromises.push(addMockMessage({
            message_type: 'text',
            author: `BulkUser${num}`,
            content: `BulkMsg_${String(num).padStart(3, '0')}`,
          }));
        }
        await Promise.all(addPromises);
      }

      // 状態の行: 150 件を受けたが、表示は 100 件に絞られる
      await expect(mainPage.getByText('150 件', { exact: true })).toBeVisible({ timeout: 5000 });
      await expect(mainPage.getByText('表示中 100 / 全 150 件')).toBeVisible();

      // Scroll to top to check early messages are trimmed
      const vlistContainer = mainPage.locator('[style*="overflow-y"]').filter({ has: mainPage.locator('[data-message-id]') }).first();
      await vlistContainer.evaluate(el => { el.scrollTop = 0; });

      // Early messages (within first 50) should NOT be visible
      // displayedMessages = filteredMessages.slice(-100), so only messages 1-50 are trimmed
      await expect(mainPage.locator('text=BulkMsg_001')).not.toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });
});
