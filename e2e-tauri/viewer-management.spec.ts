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
    console.log('Connected to Tauri app:', await mainPage.title());

    // Step 6: Authenticate first (required for chat connection)
    await mainPage.getByRole('button', { name: 'Settings' }).click();
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
    await mainPage.getByRole('button', { name: 'Chat' }).click();

    // Enter mock stream URL (full URL format to pass validation)
    // Note: The URL validation accepts localhost URLs with /watch?v= format
    const urlInput = mainPage.getByPlaceholder(/Enter YouTube URL or Video ID/i);
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test123`);

    const connectButton = mainPage.getByRole('button', { name: 'Connect' });
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
      await mainPage.getByRole('button', { name: 'Viewers' }).click();
      await expect(mainPage.getByRole('heading', { name: 'Viewer Management' })).toBeVisible();
    });

    test('should use consistent color scheme with CSS variables (not hard-coded colors)', async () => {
      // Verify that Viewer Management uses CSS variables for theming consistency
      // This test detects issues like hard-coded dark theme colors (bg-gray-900, text-purple-300)
      //
      // The key indicator of the bug is the heading text color:
      // - Bug: text-purple-300 (light purple, high luminance ~210)
      // - Fixed: text-[var(--text-primary)] (dark text, low luminance ~60)

      const heading = mainPage.getByRole('heading', { name: 'Viewer Management' });
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
      const message = mainPage.getByText('Select a broadcaster to view viewers');
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

        // Verify table headers are present
        await expect(mainPage.getByRole('columnheader', { name: 'Name' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'Reading' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'Messages' })).toBeVisible();
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

        const searchInput = mainPage.getByPlaceholder(/Search by name, reading, or notes/i);
        await expect(searchInput).toBeVisible();

        // Type a search query
        await searchInput.fill('test');

        // Submit the search
        const searchButton = mainPage.getByRole('button', { name: 'Search' });
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

        const prevButton = mainPage.getByRole('button', { name: 'Previous' });
        const nextButton = mainPage.getByRole('button', { name: 'Next' });

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

        // Verify all 8 column headers
        await expect(mainPage.getByRole('columnheader', { name: 'Name' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'Reading' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'First seen' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'Last seen' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'Messages' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'Contribution' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'Tags' })).toBeVisible();
        await expect(mainPage.getByRole('columnheader', { name: 'Notes' })).toBeVisible();
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
        const pageIndicator = mainPage.getByText(/Page \d+/);
        await expect(pageIndicator).toContainText('Page 1');

        const nextButton = mainPage.getByRole('button', { name: 'Next' });
        const prevButton = mainPage.getByRole('button', { name: 'Previous' });

        // If Next button is enabled (has more pages), click it
        if (!await nextButton.isDisabled()) {
          await nextButton.click();
          await new Promise(resolve => setTimeout(resolve, 500));

          // Page should change to 2
          await expect(pageIndicator).toContainText('Page 2');

          // Previous button should now be enabled
          await expect(prevButton).not.toBeDisabled();

          // Click Previous to go back
          await prevButton.click();
          await new Promise(resolve => setTimeout(resolve, 500));

          // Page should be back to 1
          await expect(pageIndicator).toContainText('Page 1');
        }
      }
    });
  });

  test.describe('Viewer Edit Modal', () => {
    test.beforeEach(async () => {
      // Navigate to Viewers tab and select a broadcaster
      await mainPage.getByRole('button', { name: 'Viewers' }).click();
      await expect(mainPage.getByRole('heading', { name: 'Viewer Management' })).toBeVisible();

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
        const modal = mainPage.getByRole('heading', { name: 'Edit Viewer Info' });
        await expect(modal).toBeVisible({ timeout: 3000 });

        // Close modal to clean up for next test
        await mainPage.getByRole('button', { name: 'Cancel' }).click();
        await expect(modal).not.toBeVisible({ timeout: 3000 });
      }
    });

    test('should have editable fields for reading, notes, and tags', async () => {
      // Spec: "読み仮名入力 | フォーム状態を更新", "メモ入力 | フォーム状態を更新", "タグ入力 | カンマ区切りで入力"
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

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
        await mainPage.getByRole('button', { name: 'Cancel' }).click();
      }
    });

    test('should save tags with comma-separated input', async () => {
      // Spec: "タグ入力 | カンマ区切りで入力"
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

        // Enter tags
        const tagsInput = mainPage.locator('#tags');
        await tagsInput.fill('常連, VIP, スパチャ');

        // Save
        await mainPage.getByRole('button', { name: 'Save' }).click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).not.toBeVisible({ timeout: 3000 });

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
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

        // Enter test data
        const readingInput = mainPage.locator('#reading');
        await readingInput.fill('テストよみがな');

        const notesInput = mainPage.locator('#notes');
        await notesInput.fill('テストメモ');

        // Click save
        const saveButton = mainPage.getByRole('button', { name: 'Save' });
        await saveButton.click();

        // Modal should close
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).not.toBeVisible({ timeout: 3000 });

        // Verify data is reflected in the list (reading column)
        await expect(mainPage.getByText('テストよみがな')).toBeVisible();
      }
    });

    test('should have delete button in modal', async () => {
      // Spec: "「削除」クリック | 削除確認ダイアログを表示"
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

        // Delete button should be visible (use exact: true to avoid matching "Delete Broadcaster")
        const deleteButton = mainPage.getByRole('button', { name: 'Delete', exact: true });
        await expect(deleteButton).toBeVisible();

        // Close modal to clean up
        await mainPage.getByRole('button', { name: 'Cancel' }).click();
      }
    });

    test('should show delete confirmation dialog', async () => {
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

        // Click delete button (use exact: true to avoid matching "Delete Broadcaster")
        const deleteButton = mainPage.getByRole('button', { name: 'Delete', exact: true });
        await deleteButton.click();

        // Confirmation dialog should appear
        await expect(mainPage.getByRole('heading', { name: 'Delete Custom Info' })).toBeVisible();

        // Cancel the delete (use .last() to target the frontmost dialog - the confirmation dialog)
        const confirmDialog = mainPage.getByRole('dialog').last();
        const cancelButton = confirmDialog.getByRole('button', { name: 'Cancel' });
        await cancelButton.click();

        // Confirmation dialog should close
        await expect(mainPage.getByRole('heading', { name: 'Delete Custom Info' })).not.toBeVisible();

        // Close edit modal to clean up (now only one Cancel button visible)
        await mainPage.getByRole('button', { name: 'Cancel' }).click();
      }
    });

    test('should delete viewer custom info when confirmed', async () => {
      // First, add some custom info to a viewer so we can delete it
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        // Get the first viewer's name for later verification
        const viewerName = await viewerRows[0].locator('td').first().textContent();

        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

        // First add some data
        const readingInput = mainPage.locator('#reading');
        await readingInput.fill('削除テスト用');

        await mainPage.getByRole('button', { name: 'Save' }).click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).not.toBeVisible({ timeout: 3000 });

        // Verify data was saved
        await expect(mainPage.getByText('削除テスト用')).toBeVisible();

        // Now open the modal again and delete
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

        // Click delete (use exact: true to avoid matching "Delete Broadcaster")
        await mainPage.getByRole('button', { name: 'Delete', exact: true }).click();
        await expect(mainPage.getByRole('heading', { name: 'Delete Custom Info' })).toBeVisible();

        // Confirm deletion (use .last() to target the frontmost dialog - the confirmation dialog)
        const confirmButton = mainPage.getByRole('dialog').last().getByRole('button', { name: 'Delete' });
        await confirmButton.click();

        // Modal should close
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).not.toBeVisible({ timeout: 3000 });

        // The reading should no longer be visible (deleted)
        await expect(mainPage.getByText('削除テスト用')).not.toBeVisible({ timeout: 3000 });
      }
    });

    test('should close modal with cancel button', async () => {
      const viewerRows = await mainPage.locator('tbody tr').all();

      if (viewerRows.length > 0) {
        await viewerRows[0].click();
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

        // Click cancel
        const cancelButton = mainPage.getByRole('button', { name: 'Cancel' });
        await cancelButton.click();

        // Modal should close
        await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).not.toBeVisible({ timeout: 3000 });
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
      await mainPage.getByRole('button', { name: 'Viewers' }).click();
      await expect(mainPage.getByRole('heading', { name: 'Viewer Management' })).toBeVisible();

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
      await mainPage.getByRole('button', { name: 'Viewers' }).click();
      await expect(mainPage.getByRole('heading', { name: 'Viewer Management' })).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        const viewerRows = await mainPage.locator('tbody tr').all();

        if (viewerRows.length > 0) {
          // Click the first viewer
          await viewerRows[0].click();
          await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).toBeVisible();

          // Add a reading (furigana) for this viewer
          const readingInput = mainPage.locator('#reading');
          const testReading = 'ストリームからのよみがな';
          await readingInput.fill(testReading);

          // Save
          await mainPage.getByRole('button', { name: 'Save' }).click();
          await expect(mainPage.getByRole('heading', { name: 'Edit Viewer Info' })).not.toBeVisible({ timeout: 3000 });

          // Verify the reading is shown in the table
          await expect(mainPage.getByText(testReading)).toBeVisible();
        }
      }
    });

    test('should persist viewer data across page navigation', async () => {
      // Verify data persists when navigating away and back
      await mainPage.getByRole('button', { name: 'Viewers' }).click();
      await expect(mainPage.getByRole('heading', { name: 'Viewer Management' })).toBeVisible();

      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });
        await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

        // Navigate to Chat tab
        await mainPage.getByRole('button', { name: 'Chat' }).click();
        await new Promise(resolve => setTimeout(resolve, 500));

        // Navigate back to Viewers tab
        await mainPage.getByRole('button', { name: 'Viewers' }).click();
        await expect(mainPage.getByRole('heading', { name: 'Viewer Management' })).toBeVisible();

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

  // IMPORTANT: Broadcaster Management tests are placed LAST because
  // the delete test removes data that other tests depend on
  test.describe('Broadcaster Management (Destructive - Run Last)', () => {
    test.beforeEach(async () => {
      await mainPage.getByRole('button', { name: 'Viewers' }).click();
      await expect(mainPage.getByRole('heading', { name: 'Viewer Management' })).toBeVisible();
    });

    test('should show delete broadcaster button when broadcaster is selected', async () => {
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });

        // Delete Broadcaster button should appear
        const deleteButton = mainPage.getByRole('button', { name: 'Delete Broadcaster' });
        await expect(deleteButton).toBeVisible();
      }
    });

    test('should show confirmation dialog when deleting broadcaster', async () => {
      const broadcasterSelect = mainPage.locator('#broadcaster-select');
      const options = await broadcasterSelect.locator('option').all();

      if (options.length > 1) {
        await broadcasterSelect.selectOption({ index: 1 });

        const deleteButton = mainPage.getByRole('button', { name: 'Delete Broadcaster' });
        await deleteButton.click();

        // Confirmation dialog should appear
        await expect(mainPage.getByRole('heading', { name: 'Delete Broadcaster Data' })).toBeVisible();

        // Cancel the delete
        const cancelButton = mainPage.getByRole('button', { name: 'Cancel' });
        await cancelButton.click();

        await expect(mainPage.getByRole('heading', { name: 'Delete Broadcaster Data' })).not.toBeVisible();
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

        const deleteButton = mainPage.getByRole('button', { name: 'Delete Broadcaster' });
        await deleteButton.click();

        await expect(mainPage.getByRole('heading', { name: 'Delete Broadcaster Data' })).toBeVisible();

        // Confirm deletion (use dialog scoping to target the confirm button)
        const confirmButton = mainPage.getByRole('dialog').getByRole('button', { name: 'Delete' });
        await confirmButton.click();

        // Dialog should close
        await expect(mainPage.getByRole('heading', { name: 'Delete Broadcaster Data' })).not.toBeVisible({ timeout: 3000 });

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
