import { test, expect } from './utils/fixtures';
import type { Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  disconnectAndInitialize,
  setStreamState,
  openUrlInput,
  openControlPanel,
  connectionChips,
} from './utils/test-helpers';

/**
 * E2E tests for Chat Display — visual styling and display settings.
 * Covers: Author Name Color Coding, Font Size and Display Settings,
 * Display Settings, Message Display Styling, Layout and Overflow
 */

test.describe('Chat Display — Styling (02_chat.md)', () => {
  let browser: Browser;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000); // 4 minutes for setup (includes mock server build time)

    log.info('Setting up test environment for Chat Display tests...');
    const connection = await setupTestEnvironment();
    browser = connection.browser;
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

  // 文字の大きさ・時刻・表示件数は操作パネルにある
  const panelFontSize = () => mainPage.getByRole('region', { name: '操作パネル' }).getByText(/^\d+px$/);

  test.describe('Author Name Color Coding', () => {
    test('should display member names in member-text color and non-member names in fg-1 color', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Use unique identifiers to avoid text matching issues
      const nonMemberName = 'BlueViewer123';
      const memberName = 'GreenViewer456';
      const nonMemberContent = 'Hello from non-subscriber';
      const memberContent = 'Hello from subscriber';

      // Add non-member message first
      await addMockMessage({
        message_type: 'text',
        author: nonMemberName,
        content: nonMemberContent,
        channel_id: 'UC_blue_viewer',
        is_member: false,
      });

      // Wait for first message
      await expect(mainPage.getByText(nonMemberName).first()).toBeVisible({ timeout: 5000 });

      // 仕様 (tokens.md): 通常の名前は fg-1。ダークテーマ (E2E の既定) は #ededf0 = rgb(237, 237, 240)
      const nonMemberMessage = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${nonMemberContent}`)
      }).first();
      const nonMemberAuthor = nonMemberMessage.getByTestId('author-name');
      const nonMemberColor = await nonMemberAuthor.evaluate(el => getComputedStyle(el).color);
      expect(nonMemberColor).toMatch(/rgb\(237,\s*237,\s*240\)/);

      // メンバーのコメントを 2 回送る。仕様: 新規さんの初回の行 (優先順位 2) は薄い背景の上で緑が
      // 読みにくいので名前は通常の色。2 回目以降 (優先順位 3) でメンバーの緑になる
      await addMockMessage({
        message_type: 'text',
        author: memberName,
        content: 'Member first comment',
        channel_id: 'UC_green_viewer',
        is_member: true,
      });
      const memberFirst = mainPage.locator('[data-message-id]').filter({ hasText: 'Member first comment' }).first();
      await expect(memberFirst).toHaveAttribute('data-priority', '2', { timeout: 5000 });
      const firstColor = await memberFirst.getByTestId('author-name').evaluate(el => getComputedStyle(el).color);
      expect(firstColor).toMatch(/rgb\(237,\s*237,\s*240\)/);

      await addMockMessage({
        message_type: 'text',
        author: memberName,
        content: memberContent,
        channel_id: 'UC_green_viewer',
        is_member: true,
      });

      await expect(mainPage.getByText(memberContent).first()).toBeVisible({ timeout: 5000 });

      // 仕様: メンバーの名前は member-text。ダークテーマは #4cc26a = rgb(76, 194, 106)
      const memberMessage = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${memberContent}`)
      }).first();
      const memberAuthor = memberMessage.getByTestId('author-name');
      await expect(memberAuthor).toHaveClass(/text-member-text/);
      const memberColor = await memberAuthor.evaluate(el => getComputedStyle(el).color);

      expect(memberColor).toMatch(/rgb\(76,\s*194,\s*106\)/);

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Font Size and Display Settings', () => {
    test('should increase font size when clicking 文字を大きく button', async () => {
      await openControlPanel(mainPage);
      // Get initial font size
      const fontSizeDisplay = panelFontSize();
      const initialSize = await fontSizeDisplay.textContent();
      const initialNum = parseInt(initialSize?.replace('px', '') || '13');

      // Click increase button
      const increaseButton = mainPage.getByRole('button', { name: '文字を大きく' });
      await increaseButton.click();

      // Verify size increased
      const newSize = await fontSizeDisplay.textContent();
      const newNum = parseInt(newSize?.replace('px', '') || '13');
      expect(newNum).toBe(initialNum + 1);
    });

    test('should decrease font size when clicking 文字を小さく button', async () => {
      await openControlPanel(mainPage);
      // Get initial font size
      const fontSizeDisplay = panelFontSize();
      const initialSize = await fontSizeDisplay.textContent();
      const initialNum = parseInt(initialSize?.replace('px', '') || '13');

      // Click decrease button
      const decreaseButton = mainPage.getByRole('button', { name: '文字を小さく' });
      await decreaseButton.click();

      // Verify size decreased
      const newSize = await fontSizeDisplay.textContent();
      const newNum = parseInt(newSize?.replace('px', '') || '13');
      expect(newNum).toBe(initialNum - 1);
    });

    test('should toggle timestamp display', async () => {
      // Connect to stream and add message
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      await addMockMessage({
        message_type: 'text',
        author: 'TimestampUser',
        content: 'Check timestamp',
      });

      // Wait for message to appear
      await expect(mainPage.locator('text=TimestampUser')).toBeVisible({ timeout: 5000 });

      // 操作パネルの「時刻」
      await openControlPanel(mainPage);
      const timestampToggle = mainPage.getByRole('checkbox', { name: '時刻' });

      // 付けると時刻 (HH:MM:SS) が出る
      await timestampToggle.check();
      const timestamp = mainPage.locator('[data-message-id]').getByText(/\d{2}:\d{2}:\d{2}/).first();
      await expect(timestamp).toBeVisible();

      // 外すと時刻が消える
      await timestampToggle.uncheck();
      await expect(timestamp).not.toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Display Settings', () => {
    test('should respect font size boundaries (10-24px)', async () => {
      await openControlPanel(mainPage);
      // Get initial font size
      const fontSizeDisplay = panelFontSize();
      const decreaseButton = mainPage.getByRole('button', { name: '文字を小さく' });
      const increaseButton = mainPage.getByRole('button', { name: '文字を大きく' });

      // Decrease to minimum
      for (let i = 0; i < 20; i++) {
        await decreaseButton.click();
      }

      // Should be at minimum (10px)
      let size = await fontSizeDisplay.textContent();
      expect(parseInt(size?.replace('px', '') || '0')).toBeGreaterThanOrEqual(10);

      // Increase to maximum
      for (let i = 0; i < 20; i++) {
        await increaseButton.click();
      }

      // Should be at maximum (24px)
      size = await fontSizeDisplay.textContent();
      expect(parseInt(size?.replace('px', '') || '0')).toBeLessThanOrEqual(24);

      // Reset to default (13px)
      for (let i = 0; i < 15; i++) {
        await decreaseButton.click();
      }
    });
  });

  test.describe('Message Display Styling', () => {
    test('should display member name in member-text color without a green background or border', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // メンバーの 2 回目のコメントを見る (初回は優先順位 2 で名前が通常の色になるため。仕様「1 行の形」)
      for (const content of ['Member first', 'Member with green background']) {
        await addMockMessage({
          message_type: 'text',
          author: 'GreenBgMember',
          content,
          channel_id: 'UC_green_bg_member',
          is_member: true,
        });
      }

      const memberMessage = mainPage.locator('[data-message-id]').filter({
        hasText: 'Member with green background',
      }).first();

      await expect(memberMessage).toBeVisible({ timeout: 5000 });

      // 仕様「1 行の形」: メンバーは名前を緑の文字にする。背景・枠は付けない
      // ダークテーマの member-text #4cc26a = rgb(76, 194, 106)
      const authorSpan = memberMessage.getByTestId('author-name');
      await expect(authorSpan).toHaveClass(/text-member-text/);
      const authorColor = await authorSpan.evaluate(el => getComputedStyle(el).color);
      expect(authorColor).toMatch(/rgb\(76,\s*194,\s*106\)/);

      // 行の根にも、行の本体にも背景色と枠が付かない (透明・枠なし)
      const decoration = await memberMessage.evaluate((root) => {
        const body = root.firstElementChild as HTMLElement;
        return [root, body].map((el) => {
          const st = getComputedStyle(el);
          return { background: st.backgroundColor, borderWidth: st.borderTopWidth, borderStyle: st.borderTopStyle };
        });
      });
      for (const d of decoration) {
        expect(d.background).toBe('rgba(0, 0, 0, 0)');
        expect(d.borderStyle === 'none' || d.borderWidth === '0px').toBe(true);
      }

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });

    test('should display author icon', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // Add message
      await addMockMessage({
        message_type: 'text',
        author: 'IconUser',
        content: 'User with icon',
      });

      // Find message and check for icon (img element)
      const message = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator('text=IconUser')
      }).first();

      await expect(message).toBeVisible({ timeout: 5000 });
      const icon = message.locator('img').first();
      await expect(icon).toBeVisible();

      // Disconnect
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Layout and Overflow', () => {
    test('should not overflow window width with long stream title', async () => {
      // Set a very long stream title via mock server
      const longTitle = 'This is an extremely long stream title that should be truncated with ellipsis and not cause the application window to overflow beyond the viewport width - Testing overflow handling for Japanese YouTube Live streams with very long titles 超長いタイトルのテスト日本語も含めて配信タイトル';

      await setStreamState({ title: longTitle });

      // Connect to stream with long title
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();

      // 上の行の札は 1 行のまま (タイトルは札のヒント)、全文は操作パネルの接続の一覧に出る
      await expect(connectionChips(mainPage, longTitle)).toBeVisible({ timeout: 10000 });
      await openControlPanel(mainPage);
      const titleElement = mainPage.locator('[data-testid="stream-title"]').first();

      await expect(titleElement).toBeVisible({ timeout: 10000 });

      // 長いタイトルでもウィンドウが横に溢れない (操作パネルの中で折り返す。上の行の札には全文を出さない)
      const overflow = await mainPage.evaluate(() => {
        const panel = document.querySelector('section[aria-label="操作パネル"]');
        return {
          pageScroll: document.documentElement.scrollWidth,
          pageClient: document.documentElement.clientWidth,
          panelScroll: panel?.scrollWidth ?? 0,
          panelClient: panel?.clientWidth ?? 0,
        };
      });
      expect(overflow.pageScroll).toBeLessThanOrEqual(overflow.pageClient);
      expect(overflow.panelScroll).toBeLessThanOrEqual(overflow.panelClient);
      // 全文が省略されずに DOM にある
      expect(await titleElement.textContent()).toContain('超長いタイトルのテスト日本語も含めて配信タイトル');

      // Disconnect
      await disconnectAndInitialize(mainPage);

      // Reset stream title
      await setStreamState({ title: '' });
    });

    // 著者名の表示幅（02_chat.md:584）
    // 仕様「1 行の形」: 長い名前は折り返して全文を出し、省略 (…) しない。
    test('should display a moderately long author name wider than the former 200px cap without truncation', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // 全角約24文字（13px換算で ~300px、旧上限200pxを明確に超える）。
      // 一意トークンを先頭に付与し、メッセージ本文とは別に著者名 span を特定できるようにする。
      const widthMarker = 'WCAPMARK';
      const moderateName = `${widthMarker}_やまだたろうのなまえはとてもながいですね`;
      const content = 'moderate-author-width-marker';

      await addMockMessage({
        message_type: 'text',
        author: moderateName,
        content,
        channel_id: 'UC_moderate_width',
        is_member: false,
      });

      const message = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${content}`),
      }).first();
      await expect(message).toBeVisible({ timeout: 5000 });

      // 名前は inline の span なので、幅は描画矩形で測る (clientWidth は inline だと 0)
      const authorSpan = message.getByTestId('author-name');
      await expect(authorSpan).toHaveText(moderateName);
      const metrics = await authorSpan.evaluate((el) => ({
        width: el.getBoundingClientRect().width,
        textOverflow: getComputedStyle(el).textOverflow,
      }));

      // 固定の 200px 上限が無く、名前が 200px を超えて表示されている
      expect(metrics.width).toBeGreaterThan(200);
      // 省略 (…) の指定が無い
      expect(metrics.textOverflow).not.toBe('ellipsis');

      await disconnectAndInitialize(mainPage);
    });

    test('should wrap an over-long author name in full while keeping the member badge visible and the row from overflowing', async () => {
      // Connect to stream
      const urlInput = (await openUrlInput(mainPage));
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.getByRole('button', { name: '接続', exact: true }).click();
      await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 10000 });

      // どんな現実的なパネル幅も超える長さ（全角300文字）。
      // is_member: true で「メンバー」バッジを同居させ、長い名前がバッジを押し出さない
      // ことを検証する（バッジは showTimestamps の UI 状態に依存せず常に描画される）。
      const overLongName = `OVFMARK_${'あ'.repeat(300)}`;
      const content = 'overlong-author-width-marker';

      await addMockMessage({
        message_type: 'text',
        author: overLongName,
        content,
        channel_id: 'UC_overlong_width',
        is_member: true,
      });

      const message = mainPage.locator('[data-message-id]').filter({
        has: mainPage.locator(`text=${content}`),
      }).first();
      await expect(message).toBeVisible({ timeout: 5000 });

      // 仕様: 省略せず折り返して全文を出す。行は横にはみ出さない
      const authorSpan = message.getByTestId('author-name');
      await expect(authorSpan).toHaveText(overLongName);
      const result = await authorSpan.evaluate((el) => {
        const style = window.getComputedStyle(el);
        const root = el.closest('[data-message-id]') as HTMLElement;
        return {
          textOverflow: style.textOverflow,
          lineCount: el.getClientRects().length,
          rootClientWidth: root.clientWidth,
          rootScrollWidth: root.scrollWidth,
          spanRight: el.getBoundingClientRect().right,
          rootRight: root.getBoundingClientRect().right,
        };
      });

      // 省略の指定が無い
      expect(result.textOverflow).not.toBe('ellipsis');
      // 実際に複数行へ折り返している (1 行に押し込んで切っていない)
      expect(result.lineCount).toBeGreaterThan(1);
      // 行は横方向に溢れず、名前の右端も行の中に収まる
      expect(result.rootScrollWidth).toBeLessThanOrEqual(result.rootClientWidth + 1);
      expect(result.spanRight).toBeLessThanOrEqual(result.rootRight + 1);

      // 成功条件: 長い名前があってもメンバーバッジは表示され、行からはみ出さない。
      // バッジの右端がメッセージ枠内に収まっていることで実証する。
      // メンバーは画像バッジ（authorBadges の customThumbnail）で表示される（02_chat.md「バッジ表示優先順位」）
      const memberBadge = message.locator('img[alt="Member"]').first();
      await expect(memberBadge).toBeVisible();
      const badgeBox = await memberBadge.boundingBox();
      const messageBox = await message.boundingBox();
      expect(badgeBox).not.toBeNull();
      expect(messageBox).not.toBeNull();
      expect(badgeBox!.x + badgeBox!.width).toBeLessThanOrEqual(messageBox!.x + messageBox!.width + 1);

      await disconnectAndInitialize(mainPage);
    });
  });
});
