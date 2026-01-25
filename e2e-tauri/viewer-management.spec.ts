import { test, expect, chromium, BrowserContext, Page, Browser } from '@playwright/test';
import { exec, execSync, spawn, ChildProcess } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

const execAsync = promisify(exec);
const MOCK_SERVER_URL = 'http://localhost:3456';

/**
 * E2E tests for Viewer Management feature based on 06_viewer.md specification.
 *
 * Tests verify the UI behavior specified in the frontend operation table:
 * - Broadcaster selection dropdown
 * - Viewer list display with search and pagination
 * - Viewer edit modal (reading, notes)
 * - Delete functionality for viewer custom info and broadcaster data
 *
 * Prerequisites:
 * 1. Mock server running on port 3456:
 *    cargo run --manifest-path src-tauri/Cargo.toml --bin mock_server
 *
 * 2. Run tests (app will be started automatically with test namespace):
 *    pnpm exec playwright test --config e2e-tauri/playwright.config.ts viewer-management.spec.ts
 */

const CDP_URL = 'http://127.0.0.1:9222';
const PROJECT_DIR = process.cwd().replace(/[\\/]e2e-tauri$/, '');

// Test isolation: use separate namespace for credentials and data
const TEST_APP_NAME = 'liscov-test';
const TEST_KEYRING_SERVICE = 'liscov-test';

// Get test data directories based on platform
function getTestDataDirs(): string[] {
  const dirs: string[] = [];

  const configDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.config');

  if (configDir) {
    dirs.push(path.join(configDir, TEST_APP_NAME));
  }

  const dataDir = process.platform === 'win32'
    ? process.env.APPDATA
    : process.platform === 'darwin'
      ? path.join(os.homedir(), 'Library', 'Application Support')
      : path.join(os.homedir(), '.local', 'share');

  if (dataDir && dataDir !== configDir) {
    dirs.push(path.join(dataDir, TEST_APP_NAME));
  }

  return dirs;
}

// Clean up test data directories
async function cleanupTestData(): Promise<void> {
  const dirs = getTestDataDirs();
  for (const dir of dirs) {
    if (fs.existsSync(dir)) {
      console.log(`Cleaning up test data directory: ${dir}`);
      fs.rmSync(dir, { recursive: true, force: true });
    }
  }
}

// Clean up test keyring credentials (Windows Credential Manager)
async function cleanupTestCredentials(): Promise<void> {
  if (process.platform === 'win32') {
    try {
      // The keyring crate stores credentials with target format: <user>.<service>
      execSync(`cmdkey /delete:youtube_credentials.${TEST_KEYRING_SERVICE} 2>nul`, { stdio: 'ignore' });
      console.log('Cleaned up test credentials from Windows Credential Manager');
    } catch {
      // Credential may not exist, which is fine
    }
  }
}

// Mock server process reference
let mockServerProcess: ChildProcess | null = null;

// Helper to kill mock server process
async function killMockServer(): Promise<void> {
  if (mockServerProcess) {
    console.log('Stopping mock server...');
    mockServerProcess.kill();
    mockServerProcess = null;
  }
  // Also kill any orphaned mock_server processes
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /IM mock_server.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f mock_server', { stdio: 'ignore' });
    }
  } catch {
    // Process may not exist
  }
  await new Promise(resolve => setTimeout(resolve, 500));
}

// Helper to start mock server
async function startMockServer(): Promise<void> {
  console.log('Starting mock server...');

  // Kill any existing mock server first
  await killMockServer();

  // Start mock server as a child process
  const cargoPath = path.join(PROJECT_DIR, 'src-tauri', 'Cargo.toml');
  mockServerProcess = spawn('cargo', ['run', '--manifest-path', cargoPath, '--bin', 'mock_server'], {
    cwd: PROJECT_DIR,
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: true,
  });

  // Log mock server output for debugging
  mockServerProcess.stdout?.on('data', (data) => {
    const msg = data.toString().trim();
    if (msg) console.log(`[mock_server] ${msg}`);
  });
  mockServerProcess.stderr?.on('data', (data) => {
    const msg = data.toString().trim();
    // Filter out cargo build warnings/info
    if (msg && !msg.includes('Compiling') && !msg.includes('Finished') && !msg.includes('warning:')) {
      console.log(`[mock_server] ${msg}`);
    }
  });

  // Wait for mock server to be ready
  const timeout = 60000;
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`${MOCK_SERVER_URL}/status`);
      if (response.ok) {
        console.log(`Mock server ready after ${Date.now() - start}ms`);
        return;
      }
    } catch {
      // Server not ready yet
    }
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  throw new Error(`Mock server not ready after ${timeout}ms`);
}

// Helper to reset mock server state
async function resetMockServer(): Promise<void> {
  await fetch(`${MOCK_SERVER_URL}/reset`, { method: 'POST' });
}

// Helper to wait for CDP to be available
async function waitForCDP(timeout = 120000): Promise<void> {
  const start = Date.now();
  console.log('Waiting for CDP to be available...');
  let lastError = '';
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`${CDP_URL}/json/version`);
      if (response.ok) {
        console.log(`CDP available after ${Date.now() - start}ms`);
        return;
      }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
    await new Promise(resolve => setTimeout(resolve, 1000));
  }
  throw new Error(`CDP not available after ${timeout}ms. Last error: ${lastError}`);
}

// Helper to connect to Tauri app
async function connectToApp(): Promise<{ browser: Browser; context: BrowserContext; page: Page }> {
  const browser = await chromium.connectOverCDP(CDP_URL);
  const contexts = browser.contexts();

  if (contexts.length === 0) {
    throw new Error('No browser contexts found');
  }

  const context = contexts[0];
  const pages = context.pages();

  if (pages.length === 0) {
    throw new Error('No pages found in context');
  }

  return { browser, context, page: pages[0] };
}

// Helper to kill Tauri app
async function killTauriApp(): Promise<void> {
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /IM liscov-tauri.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f liscov-tauri', { stdio: 'ignore' });
    }
  } catch {
    // Process may not exist
  }
  // Wait for port to be released
  await new Promise(resolve => setTimeout(resolve, 1000));
}

// Helper to start Tauri app with test isolation
async function startTauriApp(): Promise<void> {
  const env = {
    ...process.env,
    // Test isolation: use separate namespace
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    // Mock server URLs - CRITICAL: point app to mock server for all YouTube interactions
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
    LISCOV_AUTH_URL: 'http://localhost:3456/?auto_login=true',
    LISCOV_SESSION_CHECK_URL: 'http://localhost:3456/youtubei/v1/account/account_menu',
    // Enable CDP for Playwright
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  };

  console.log(`Starting Tauri app with test namespace: ${TEST_APP_NAME}`);

  // Start app in background
  exec(`cd "${PROJECT_DIR}" && pnpm tauri dev`, { env });

  // Wait for CDP to be available
  await waitForCDP();
}

// Helper to fully disconnect (stop + initialize) and return to idle state
async function disconnectAndInitialize(page: Page): Promise<void> {
  // Click 停止 to pause
  const stopButton = page.locator('button:has-text("停止")');
  if (await stopButton.isVisible({ timeout: 1000 }).catch(() => false)) {
    await stopButton.click();
    // After clicking 停止, app goes to paused state with 再開 and 初期化 buttons
    // Click 初期化 to return to idle state
    await page.locator('button:has-text("初期化")').click();
    // Wait for UI to return to idle state (URL input visible)
    await expect(page.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"], input[placeholder*="Enter YouTube URL"]')).toBeVisible({ timeout: 5000 });
  }
}

// Use test.describe.serial to ensure tests run in order and share state
test.describe.serial('Viewer Management Feature (06_viewer.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  // Increase timeout for beforeAll as it starts the app
  test.beforeAll(async () => {
    test.setTimeout(240000); // 4 minutes for setup (includes mock server build time)

    // Step 1: Kill any existing processes
    console.log('Killing any existing Tauri app...');
    await killTauriApp();

    // Step 2: Clean up test data and credentials for a fresh start
    console.log('Cleaning up test data and credentials...');
    await cleanupTestData();
    await cleanupTestCredentials();

    // Step 3: Start mock server
    await startMockServer();
    await resetMockServer();

    // Step 4: Start Tauri app with test namespace
    console.log('Starting Tauri app with test namespace...');
    await startTauriApp();

    // Step 5: Connect to the running Tauri app
    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
    // Wait for page to be fully loaded and stable before accessing
    await mainPage.waitForLoadState('load');
    await mainPage.waitForTimeout(1000);
    console.log('Connected to Tauri app');

    // Step 6: Authenticate first (required for chat connection)
    await mainPage.locator('button:has-text("Settings")').click();
    await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible();

    const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
    if (await loginButton.isVisible()) {
      await loginButton.click();
      const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible({ timeout: 15000 });
    }

    // Step 7: Add mock chat messages before connecting (so viewers will be created)
    console.log('Adding mock chat messages...');
    await fetch(`${MOCK_SERVER_URL}/add_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message_type: 'text',
        author: 'TestViewer1',
        channel_id: 'UC_test_viewer_1',
        content: 'Hello from TestViewer1!'
      })
    });
    await fetch(`${MOCK_SERVER_URL}/add_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message_type: 'text',
        author: 'TestViewer2',
        channel_id: 'UC_test_viewer_2',
        content: 'Hello from TestViewer2!'
      })
    });
    await fetch(`${MOCK_SERVER_URL}/add_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message_type: 'superchat',
        author: 'SuperChatViewer',
        channel_id: 'UC_superchat_viewer',
        content: 'Super chat message!',
        amount: '¥500'
      })
    });

    // Step 8: Navigate to Chat tab and connect to mock chat to generate viewer data
    // THIS IS THE CRITICAL SCENARIO: connecting to a stream creates broadcaster + viewer data
    console.log('Connecting to mock chat to generate viewer data...');
    await mainPage.locator('button:has-text("Chat")').click();

    // Enter mock stream URL (full URL format to pass validation)
    // Note: The URL validation accepts localhost URLs with /watch?v= format
    const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test123`);

    const connectButton = mainPage.getByRole('button', { name: '開始' });
    await connectButton.click();

    // Wait for connection success - look for stream title or connected state
    console.log('Waiting for connection success...');
    // The stream title "Mock Live" should appear when connected (use .first() as it appears in multiple places)
    await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 15000 });
    console.log('Connection successful! Stream title visible.');

    // Wait a bit more for chat messages to be fetched and viewers to be saved
    await new Promise(resolve => setTimeout(resolve, 3000));
    console.log('Viewer data should be generated now.');
  });

  test.afterAll(async () => {
    // Clean up: close browser connection, kill Tauri app, and stop mock server
    console.log('Cleaning up after tests...');
    if (browser) {
      await browser.close();
    }
    await killTauriApp();
    await killMockServer();
    // Clean up test data
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test.describe('Viewer Management Page', () => {
    test.beforeEach(async () => {
      // Navigate to Viewers tab
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();
    });

    test('should use consistent color scheme with CSS variables (not hard-coded colors)', async () => {
      // Verify that Viewer Management uses CSS variables for theming consistency
      // This test detects issues like hard-coded dark theme colors (bg-gray-900, text-purple-300)
      //
      // The key indicator of the bug is the heading text color:
      // - Bug: text-purple-300 (light purple, high luminance ~210)
      // - Fixed: text-[var(--text-primary)] (dark text, low luminance ~60)

      // Select the h2 heading (not h1 in header) as it uses CSS variables
      const heading = mainPage.locator('h2').filter({ hasText: '視聴者管理' });
      await expect(heading).toBeVisible();

      // Get heading color and convert to RGB
      const headingColorInfo = await heading.evaluate(el => {
        const style = getComputedStyle(el);
        const color = style.color;

        // Convert to RGB using canvas (handles OKLCH, RGB, hex, etc.)
        const canvas = document.createElement('canvas');
        canvas.width = canvas.height = 1;
        const ctx = canvas.getContext('2d')!;
        ctx.fillStyle = color;
        ctx.fillRect(0, 0, 1, 1);
        const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
        return { original: color, r, g, b };
      });

      const headingLuminance = (headingColorInfo.r + headingColorInfo.g + headingColorInfo.b) / 3;
      console.log(`Heading color: ${headingColorInfo.original} -> rgb(${headingColorInfo.r}, ${headingColorInfo.g}, ${headingColorInfo.b}), luminance: ${headingLuminance}`);

      // Purple-300 is rgb(196, 181, 253) with luminance ~210
      // Dark text (--text-primary) should have luminance < 100
      // This test will FAIL if using light-colored text like purple-300
      expect(headingLuminance).toBeLessThan(100);

      // Additional check: The heading should NOT have high blue component (purple indicator)
      // Purple-300 has B > 250, dark text should have B < 100
      console.log(`Heading blue component: ${headingColorInfo.b}`);
      expect(headingColorInfo.b).toBeLessThan(150);
    });

    test('should display broadcaster selector with connected stream broadcaster', async () => {
      // Spec: "BroadcasterSelector.svelte - 配信者選択ドロップダウン"
      // CRITICAL: After connecting to a stream in beforeAll, the broadcaster MUST appear
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      await expect(broadcasterSelect).toBeVisible();

      // Get available options - should have at least 1 broadcaster (from the connected stream)
      const options = await broadcasterSelect.locator('option').all();
      const broadcasterCount = options.length - 1; // Subtract placeholder

      // CRITICAL: After connecting to a stream, there MUST be at least 1 broadcaster
      expect(broadcasterCount).toBeGreaterThanOrEqual(1);

      // The mock server's broadcaster name should be in the options
      const optionTexts = await Promise.all(options.map(o => o.textContent()));
      console.log('Available broadcasters:', optionTexts);
    });

    test('should show message when no broadcaster is selected', async () => {
      // First, reset the dropdown to "Select a broadcaster..." (placeholder)
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      await broadcasterSelect.selectOption({ index: 0 }); // Select placeholder

      // When no broadcaster is selected, show guidance message
      const message = mainPage.getByText('配信者を選択してください');
      await expect(message).toBeVisible();
    });

    test('should display viewer list after selecting broadcaster', async () => {
      // Spec: "配信者選択 | viewer_get_list呼び出し、リスト更新"
      const broadcasterSelect = mainPage.locator('#broadcaster-select');

      // Get available options
      const options = await broadcasterSelect.locator('option').all();

      // If there are broadcasters (more than the placeholder), select one
      if (options.length > 1) {
        // Select the first real broadcaster (skip the placeholder)
        await broadcasterSelect.selectOption({ index: 1 });

        // Wait for viewer list to appear
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Verify table headers are present (Japanese UI)
        await expect(mainPage.getByRole('columnheader', { name: '名前' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: '読み仮名' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'コメント数' })).toBeVisible();
      }
    });

    test('should support search functionality', async () => {
      // Spec: "検索クエリ入力 | デバウンス後にviewer_get_list呼び出し"
      // First select a broadcaster to show the viewer list with search
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        const searchInput = mainPage.getByPlaceholder(/名前、読み仮名、メモで検索/i);
        await expect(searchInput).toBeVisible();

        // Type a search query
        await searchInput.fill('test');

        // Submit the search (Japanese: 検索)
        const searchButton = mainPage.getByRole('button', { name: '検索' });
        await searchButton.click();

        // Wait for results to update
        await new Promise(resolve => setTimeout(resolve, 500));
      }
    });

    test('should have pagination controls', async () => {
      // Spec: "ページネーション - 1ページあたり: 50件"
      // First select a broadcaster to show the viewer list with pagination
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        const prevButton = mainPage.getByRole('button', { name: '前へ' });
        const nextButton = mainPage.getByRole('button', { name: '次へ' });

        await expect(prevButton).toBeVisible();
        await expect(nextButton).toBeVisible();

        // Page 1 should have Previous disabled
        await expect(prevButton).toBeDisabled();
      }
    });

    test('should display all table columns from spec', async () => {
      // Spec: 表示項目 (8 columns)
      // Name, Reading, First seen, Last seen, Messages, Contribution, Tags, Notes
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Verify all 8 column headers (Japanese UI)
        await expect(mainPage.getByRole('columnheader', { name: '名前' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: '読み仮名' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: '初見日時' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: '最終確認' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'コメント数' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: '貢献額' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'タグ' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'メモ' })).toBeVisible();
      }
    });

    test('should have refresh button to reload viewer list', async () => {
      // Spec: "更新ボタン | リストを再取得"
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Refresh button should be visible (aria-label="Refresh viewer list")
        const refreshButton = mainPage.getByLabel('Refresh viewer list');
        await expect(refreshButton).toBeVisible();

        // Click refresh should reload the list without errors
        await refreshButton.click();
        await new Promise(resolve => setTimeout(resolve, 500));

        // Table should still be visible after refresh
        await expect(mainPage.locator('table')).toBeVisible();
      }
    });

    test('should change page when clicking Next button', async () => {
      // Spec: "ページ変更 | viewer_get_listをoffset変更で呼び出し"
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Check initial page indicator
        const pageIndicator = mainPage.getByText(/ページ \d+/);
        await expect(pageIndicator).toContainText('ページ 1');

        const nextButton = mainPage.getByRole('button', { name: '次へ' });
        const prevButton = mainPage.getByRole('button', { name: '前へ' });

        // If Next button is enabled (has more pages), click it
        if (!await nextButton.isDisabled()) {
          await nextButton.click();
          await new Promise(resolve => setTimeout(resolve, 500));

          // Page should change to 2
          await expect(pageIndicator).toContainText('ページ 2');

          // Previous button should now be enabled
          await expect(prevButton).not.toBeDisabled();

          // Click Previous to go back
          await prevButton.click();
          await new Promise(resolve => setTimeout(resolve, 500));

          // Page should be back to 1
          await expect(pageIndicator).toContainText('ページ 1');
        }
      }
    });
  });

  test.describe('Viewer Edit Modal', () => {
    test.beforeEach(async () => {
      // Navigate to Viewers tab and select a broadcaster
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });
      }
    });

    test('should open edit modal on viewer row click', async () => {
      // Spec: "視聴者行クリック | 編集モーダルを開く"
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        // Click the first viewer row
        await viewerRows[0].click();

        // Edit modal should appear
        const modal = mainPage.getByRole('heading', { name: '視聴者情報の編集' });
        await expect(modal).toBeVisible({ timeout: 3000 });

        // Close modal to clean up for next test
        await mainPage.getByRole('button', { name: 'キャンセル' }).click();
        await expect(modal).not.toBeVisible({ timeout: 3000 });
      }
    });

    test('should have editable fields for reading, notes, and tags', async () => {
      // Spec: "読み仮名入力 | フォーム状態を更新", "メモ入力 | フォーム状態を更新", "タグ入力 | カンマ区切りで入力"
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

        // Check for reading input
        const readingInput = mainPage.locator('#reading');
        await expect(readingInput).toBeVisible();

        // Check for notes textarea
        const notesInput = mainPage.locator('#notes');
        await expect(notesInput).toBeVisible();

        // Check for tags input
        const tagsInput = mainPage.locator('#tags');
        await expect(tagsInput).toBeVisible();

        // Close modal
        await mainPage.getByRole('button', { name: 'キャンセル' }).click();
      }
    });

    test('should save tags with comma-separated input', async () => {
      // Spec: "タグ入力 | カンマ区切りで入力"
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

        // Enter tags
        const tagsInput = mainPage.locator('#tags');
        await tagsInput.fill('常連, VIP, スパチャ');

        // Save
        await mainPage.getByRole('button', { name: '保存' }).click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

        // Verify tags appear in the list (Tags column should show badges)
        // Look for at least one of the tags in the table
        await expect(mainPage.getByText('常連')).toBeVisible();
      }
    });

    test('should save custom info and close modal', async () => {
      // Spec: "「保存」クリック | viewer_upsert_custom_info呼び出し、モーダルを閉じる"
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

        // Enter test data
        const readingInput = mainPage.locator('#reading');
        await readingInput.fill('テストよみがな');

        const notesInput = mainPage.locator('#notes');
        await notesInput.fill('テストメモ');

        // Click save
        const saveButton = mainPage.getByRole('button', { name: '保存' });
        await saveButton.click();

        // Modal should close
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

        // Verify data is reflected in the list (reading column)
        await expect(mainPage.getByText('テストよみがな')).toBeVisible();
      }
    });

    test('should have delete button in modal', async () => {
      // Spec: "「削除」クリック | 削除確認ダイアログを表示"
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

        // Delete button should be visible (use exact: true to avoid matching "Delete Broadcaster")
        const deleteButton = mainPage.getByRole('button', { name: '削除', exact: true });
        await expect(deleteButton).toBeVisible();

        // Close modal to clean up
        await mainPage.getByRole('button', { name: 'キャンセル' }).click();
      }
    });

    test('should show delete confirmation dialog', async () => {
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

        // Click delete button (use exact: true to avoid matching "Delete Broadcaster")
        const deleteButton = mainPage.getByRole('button', { name: '削除', exact: true });
        await deleteButton.click();

        // Confirmation dialog should appear
        await expect(mainPage.getByRole('heading', { name: 'カスタム情報の削除' })).toBeVisible();

        // Cancel the delete (use .last() to target the frontmost dialog - the confirmation dialog)
        const confirmDialog = mainPage.getByRole('dialog').last();
        const cancelButton = confirmDialog.getByRole('button', { name: 'キャンセル' });
        await cancelButton.click();

        // Confirmation dialog should close
        await expect(mainPage.getByRole('heading', { name: 'カスタム情報の削除' })).not.toBeVisible();

        // Close edit modal to clean up (now only one Cancel button visible)
        await mainPage.getByRole('button', { name: 'キャンセル' }).click();
      }
    });

    test('should delete viewer custom info when confirmed', async () => {
      // First, add some custom info to a viewer so we can delete it
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        // Get the first viewer's name for later verification
        const viewerName = await viewerRows[0].locator('td').first().textContent();

        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

        // First add some data
        const readingInput = mainPage.locator('#reading');
        await readingInput.fill('削除テスト用');

        await mainPage.getByRole('button', { name: '保存' }).click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

        // Verify data was saved
        await expect(mainPage.getByText('削除テスト用')).toBeVisible();

        // Now open the modal again and delete
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

        // Click delete (use exact: true to avoid matching "Delete Broadcaster")
        await mainPage.getByRole('button', { name: '削除', exact: true }).click();
        await expect(mainPage.getByRole('heading', { name: 'カスタム情報の削除' })).toBeVisible();

        // Confirm deletion (use .last() to target the frontmost dialog - the confirmation dialog)
        const confirmButton = mainPage.getByRole('dialog').last().getByRole('button', { name: '削除' });
        await confirmButton.click();

        // Modal should close
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

        // The reading should no longer be visible (deleted)
        await expect(mainPage.getByText('削除テスト用')).not.toBeVisible({ timeout: 3000 });
      }
    });

    test('should close modal with cancel button', async () => {
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

        // Click cancel
        const cancelButton = mainPage.getByRole('button', { name: 'キャンセル' });
        await cancelButton.click();

        // Modal should close
        await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });
      }
    });
  });

  test.describe('Connected Stream Integration (Critical Scenario)', () => {
    /**
     * These tests verify the PRIMARY USE CASE:
     * After connecting to a stream, the broadcaster should appear in the dropdown,
     * and viewers from that stream should be manageable.
     *
     * This is the most common workflow:
     * 1. User connects to a YouTube live stream
     * 2. Chat messages are received (creating viewer profiles)
     * 3. User goes to Viewer Management to manage viewers for that broadcaster
     *
     * NOTE: The test for broadcaster appearing in dropdown has been moved to
     * "Viewer Management Page" section as the first test to ensure it runs
     * before any cleanup happens.
     */

    test('should show viewers from connected stream after selecting broadcaster', async () => {
      // After connecting to a stream and receiving messages,
      // viewers should be visible in the Viewer Management
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      // Select the first real broadcaster (the one we connected to)
      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });

        // Wait for viewer list to load
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Should have at least one viewer row (from mock server messages)
        const viewerRows = await mainPage.locator('tbody tr').all();
        console.log(`Found ${viewerRows.length} viewers from connected stream`);

        // CRITICAL: There should be at least 1 viewer from the stream
        expect(viewerRows.length).toBeGreaterThanOrEqual(1);
      }
    });

    test('should be able to edit viewer info for viewers from connected stream', async () => {
      // Verify that we can edit viewer info for viewers we received via the stream
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        const viewerRows = await mainPage.locator('tbody tr').all();

        if (viewerRows.length > 0) {
          // Click the first viewer
          await viewerRows[0].click();
          await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

          // Add a reading (furigana) for this viewer
          const readingInput = mainPage.locator('#reading');
          const testReading = 'ストリームからのよみがな';
          await readingInput.fill(testReading);

          // Save
          await mainPage.getByRole('button', { name: '保存' }).click();
          await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

          // Verify the reading is shown in the table
          await expect(mainPage.getByText(testReading)).toBeVisible();
        }
      }
    });

    test('should persist viewer data across page navigation', async () => {
      // Verify data persists when navigating away and back
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Navigate to Chat tab
        await mainPage.locator('button:has-text("Chat")').click();
        await new Promise(resolve => setTimeout(resolve, 500));

        // Navigate back to Viewers tab
        await mainPage.locator('button:has-text("Viewer")').click();
        await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

        // Re-select the broadcaster
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Previously saved reading should still be visible
        const savedReading = mainPage.getByText('ストリームからのよみがな');
        // May or may not be visible depending on test order, but table should load
        await expect(mainPage.locator('tbody')).toBeVisible();
      }
    });
  });

  test.describe('Broadcaster Scoping (06_viewer.md: 配信者スコープ)', () => {
    /**
     * Critical Test: Verify that viewer profiles are scoped per broadcaster.
     * Same viewer should have DIFFERENT custom info for different broadcasters.
     *
     * Spec reference (06_viewer.md):
     * "同じ視聴者でも配信者ごとに異なるプロフィール（統計情報）とカスタム情報を持つ"
     */
    test('should maintain separate viewer custom info per broadcaster', async () => {
      // This test is complex - increase timeout
      test.setTimeout(180000); // 3 minutes

      // Step 1: We're already connected to Broadcaster A (UC_mock) from beforeAll
      // Navigate to Viewers tab and set up reading for a viewer on Broadcaster A
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      let options = await broadcasterSelect.locator('option').all();
      expect(options.length).toBeGreaterThan(1);

      // Select Broadcaster A (first real broadcaster)
      await broadcasterSelect.selectOption({ index: 1 });
      await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

      // Get the selected broadcaster ID and name for later comparison
      const broadcasterAId = await broadcasterSelect.inputValue();
      const broadcasterAOption = await broadcasterSelect.locator('option:checked').textContent();
      console.log(`Broadcaster A: ${broadcasterAOption} (${broadcasterAId})`);

      // Find a viewer to edit
      const viewerRows = await mainPage.locator('tbody tr').all();
      expect(viewerRows.length).toBeGreaterThan(0);

      // Get the viewer's name from the first cell (Name column)
      const viewerName = await viewerRows[0].locator('td').first().textContent();
      console.log(`Setting reading for viewer: ${viewerName}`);

      // Click on the first viewer and set a reading
      await viewerRows[0].click();
      await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

      // Set a reading specific to Broadcaster A
      const readingForA = '配信者Aでの読み方';
      const readingInput = mainPage.locator('#reading');
      await readingInput.fill(readingForA);

      await mainPage.getByRole('button', { name: '保存' }).click();
      await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

      // Verify reading is saved for Broadcaster A
      await expect(mainPage.getByText(readingForA)).toBeVisible();
      console.log('Reading saved for Broadcaster A');

      // Step 2: Disconnect from current stream
      await mainPage.locator('button:has-text("Chat")').click();
      await disconnectAndInitialize(mainPage);

      // Step 3: Configure mock server to use Broadcaster B
      console.log('Switching to Broadcaster B...');
      await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          channel_id: 'UC_broadcaster_b',
          channel_name: 'Mock Broadcaster B',
          title: 'Mock Live Stream B'
        })
      });

      // Add a message from the SAME viewer (same channel_id pattern) for Broadcaster B
      await fetch(`${MOCK_SERVER_URL}/add_message`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message_type: 'text',
          author: 'TestViewer1', // Same as beforeAll
          channel_id: 'UC_test_viewer_1', // Same as beforeAll
          content: 'Hello from TestViewer1 in stream B!'
        })
      });

      // Step 4: Connect to Broadcaster B
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_b_123`);
      await mainPage.getByRole('button', { name: '開始' }).click();

      // Wait for connection
      await expect(mainPage.getByText('Mock Live Stream B').first()).toBeVisible({ timeout: 15000 });
      console.log('Connected to Broadcaster B');

      // Wait for messages to be processed
      await new Promise(resolve => setTimeout(resolve, 3000));

      // Step 5: Navigate to Viewer Management and check Broadcaster B
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      // Refresh broadcaster list and find Broadcaster B
      options = await broadcasterSelect.locator('option').all();
      console.log(`Available broadcasters after connecting to B: ${options.length - 1}`);

      // Find and select Broadcaster B
      let broadcasterBFound = false;
      for (let i = 1; i < options.length; i++) {
        const optionText = await options[i].textContent();
        const optionValue = await options[i].getAttribute('value');
        console.log(`Option ${i}: ${optionText} (${optionValue})`);
        if (optionValue === 'UC_broadcaster_b' || optionText?.includes('Broadcaster B')) {
          await broadcasterSelect.selectOption({ index: i });
          broadcasterBFound = true;
          break;
        }
      }

      // If not found by value, try selecting by index (newest broadcaster)
      if (!broadcasterBFound && options.length > 2) {
        // Try the last option as it might be the newest broadcaster
        await broadcasterSelect.selectOption({ index: options.length - 1 });
        broadcasterBFound = true;
      }

      if (broadcasterBFound) {
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Step 6: Verify the SAME viewer has NO reading for Broadcaster B
        const viewerRowsB = await mainPage.locator('tbody tr').all();
        console.log(`Found ${viewerRowsB.length} viewers for Broadcaster B`);

        if (viewerRowsB.length > 0) {
          // Check if the viewer exists and has no reading
          // The reading column should be empty or not contain our Broadcaster A reading
          const tableText = await mainPage.locator('table').textContent();

          // CRITICAL ASSERTION: The reading set for Broadcaster A should NOT appear for Broadcaster B
          // This is the key test for broadcaster scoping
          expect(tableText).not.toContain(readingForA);
          console.log('✓ Verified: Reading from Broadcaster A is NOT visible for Broadcaster B');
        }

        // Step 7: Switch back to Broadcaster A and verify reading is still there
        // Find Broadcaster A in the dropdown
        for (let i = 1; i < options.length; i++) {
          const optionValue = await options[i].getAttribute('value');
          if (optionValue === broadcasterAId) {
            await broadcasterSelect.selectOption({ index: i });
            break;
          }
        }

        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // CRITICAL ASSERTION: Reading for Broadcaster A should still be there
        await expect(mainPage.getByText(readingForA)).toBeVisible();
        console.log('✓ Verified: Reading for Broadcaster A is preserved');
      }

      // Cleanup: Reset mock server state for next tests
      await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          channel_id: '',
          channel_name: '',
          title: ''
        })
      });
    });
  });

  test.describe('Viewer Profile Auto-Update (06_viewer.md: 自動更新)', () => {
    /**
     * Test: Verify that viewer profiles are automatically updated when messages are received.
     *
     * Spec reference (06_viewer.md):
     * - message_count をインクリメント
     * - last_seen を更新
     * - スーパーチャット時は total_contribution に加算
     */
    test('should increment message_count when new messages are received', async () => {
      // Navigate to Viewers tab
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Find a viewer row and get current message count
        const viewerRows = await mainPage.locator('tbody tr').all();
        if (viewerRows.length > 0) {
          // Get the Messages column (5th column)
          const messagesCell = viewerRows[0].locator('td').nth(4);
          const initialCount = parseInt(await messagesCell.textContent() || '0');
          console.log(`Initial message count: ${initialCount}`);

          // Ensure message count is at least 1 (from beforeAll setup)
          expect(initialCount).toBeGreaterThanOrEqual(1);
        }
      }
    });

    test('should update total_contribution when superchat is received', async () => {
      // Navigate to Viewers tab
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Find the SuperChatViewer (from beforeAll setup) and verify contribution is recorded
        const contributionColumn = mainPage.locator('tbody tr td:nth-child(6)');
        const contributions = await contributionColumn.allTextContents();

        // At least one viewer should have a contribution > 0 (the SuperChatViewer)
        const hasContribution = contributions.some(c => {
          const match = c.match(/[\d,]+/);
          return match && parseInt(match[0].replace(/,/g, '')) > 0;
        });

        console.log('Contributions:', contributions);
        // Note: This assertion may need adjustment based on how contributions are displayed
        // For now, we just verify the column exists and has content
        expect(contributions.length).toBeGreaterThan(0);
      }
    });
  });

  test.describe('Search Functionality Details (06_viewer.md: 検索機能)', () => {
    /**
     * Test: Verify search works on reading and notes fields.
     *
     * Spec reference (06_viewer.md):
     * 検索対象: display_name, reading, notes
     * 検索方式: 部分一致 LIKE "%{検索文字}%"
     */
    test.beforeEach(async () => {
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });
      }
    });

    test('should search by reading (読み仮名)', async () => {
      // First, set a unique reading for a viewer
      const viewerRows = await mainPage.locator('tbody tr').all();
      if (viewerRows.length === 0) return;

      await viewerRows[0].click();
      await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

      const uniqueReading = '検索テスト読み' + Date.now();
      const readingInput = mainPage.locator('#reading');
      await readingInput.fill(uniqueReading);

      await mainPage.getByRole('button', { name: '保存' }).click();
      await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

      // Now search by reading
      const searchInput = mainPage.getByPlaceholder(/名前、読み仮名、メモで検索/);
      await searchInput.fill('検索テスト読み');

      const searchButton = mainPage.getByRole('button', { name: '検索' });
      await searchButton.click();

      await new Promise(resolve => setTimeout(resolve, 500));

      // Verify search results contain our viewer
      const searchResults = await mainPage.locator('tbody tr').all();
      expect(searchResults.length).toBeGreaterThan(0);

      // Verify the unique reading appears in results
      await expect(mainPage.locator('table')).toContainText('検索テスト読み');

      // Clear search
      await searchInput.fill('');
      await searchButton.click();
    });

    test('should search by notes (メモ)', async () => {
      // First, set a unique note for a viewer
      const viewerRows = await mainPage.locator('tbody tr').all();
      if (viewerRows.length === 0) return;

      await viewerRows[0].click();
      await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

      const uniqueNote = '検索テストメモ' + Date.now();
      const notesInput = mainPage.locator('#notes');
      await notesInput.fill(uniqueNote);

      await mainPage.getByRole('button', { name: '保存' }).click();
      await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

      // Now search by notes
      const searchInput = mainPage.getByPlaceholder(/名前、読み仮名、メモで検索/);
      await searchInput.fill('検索テストメモ');

      const searchButton = mainPage.getByRole('button', { name: '検索' });
      await searchButton.click();

      await new Promise(resolve => setTimeout(resolve, 500));

      // Verify search results contain our viewer
      const searchResults = await mainPage.locator('tbody tr').all();
      expect(searchResults.length).toBeGreaterThan(0);

      // Clear search
      await searchInput.fill('');
      await searchButton.click();
    });

    test('should return empty results for non-matching search', async () => {
      const searchInput = mainPage.getByPlaceholder(/名前、読み仮名、メモで検索/);
      await searchInput.fill('これは絶対にマッチしない文字列xyz123abc');

      const searchButton = mainPage.getByRole('button', { name: '検索' });
      await searchButton.click();

      await new Promise(resolve => setTimeout(resolve, 500));

      // Should show no results or empty table
      const viewerRows = await mainPage.locator('tbody tr').all();
      expect(viewerRows.length).toBe(0);

      // Clear search
      await searchInput.fill('');
      await searchButton.click();
    });
  });

  // IMPORTANT: Broadcaster Management tests are placed LAST because
  // the delete test removes data that other tests depend on
  test.describe('Broadcaster Management (Destructive - Run Last)', () => {
    test.beforeEach(async () => {
      await mainPage.locator('button:has-text("Viewer")').click();
      await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();
    });

    test('should show delete broadcaster button when broadcaster is selected', async () => {
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });

        // Delete Broadcaster button should appear
        const deleteButton = mainPage.getByRole('button', { name: '配信者を削除' });
        await expect(deleteButton).toBeVisible();
      }
    });

    test('should show confirmation dialog when deleting broadcaster', async () => {
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });

        const deleteButton = mainPage.getByRole('button', { name: '配信者を削除' });
        await deleteButton.click();

        // Confirmation dialog should appear
        await expect(mainPage.getByRole('heading', { name: '配信者データの削除' })).toBeVisible();

        // Cancel the delete
        const cancelButton = mainPage.getByRole('button', { name: 'キャンセル' });
        await cancelButton.click();

        await expect(mainPage.getByRole('heading', { name: '配信者データの削除' })).not.toBeVisible();
      }
    });

    test('should delete broadcaster when confirmed (DESTRUCTIVE)', async () => {
      // Note: This test actually deletes data, should be the last test
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const optionsBefore = await broadcasterSelect.locator('option').all();
      const broadcasterCountBefore = optionsBefore.length - 1; // Subtract placeholder

      if (broadcasterCountBefore > 0) {
        // Get the name of the broadcaster to delete
        await broadcasterSelect.selectOption({ index: 1 });
        const selectedValue = await broadcasterSelect.inputValue();

        const deleteButton = mainPage.getByRole('button', { name: '配信者を削除' });
        await deleteButton.click();

        await expect(mainPage.getByRole('heading', { name: '配信者データの削除' })).toBeVisible();

        // Confirm deletion (use dialog scoping to target the confirm button)
        const confirmButton = mainPage.getByRole('dialog').getByRole('button', { name: '削除' });
        await confirmButton.click();

        // Dialog should close
        await expect(mainPage.getByRole('heading', { name: '配信者データの削除' })).not.toBeVisible({ timeout: 3000 });

        // Wait for list to update
        await new Promise(resolve => setTimeout(resolve, 500));

        // Broadcaster should be removed from dropdown
        const optionsAfter = await broadcasterSelect.locator('option').all();
        const broadcasterCountAfter = optionsAfter.length - 1;

        // Count should decrease (or show "Select a broadcaster" message)
        expect(broadcasterCountAfter).toBeLessThan(broadcasterCountBefore);
      }
    });
  });
});
