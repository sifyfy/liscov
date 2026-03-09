/**
 * E2Eテスト共通ヘルパー関数
 */

import { chromium, BrowserContext, Page, Browser, expect } from '@playwright/test';
import { execSync, spawn, ChildProcess } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { log } from './logger';

export const CDP_URL = 'http://127.0.0.1:9222';
export const MOCK_SERVER_URL = 'http://localhost:3456';
export const PROJECT_DIR = process.cwd().replace(/[\\/]e2e$/, '');

// Test isolation: use separate namespace for credentials and data
export const TEST_APP_NAME = 'liscov-test';
export const TEST_KEYRING_SERVICE = 'liscov-test';

// Mock server process reference
let mockServerProcess: ChildProcess | null = null;

// Tauri app process reference
let tauriProcess: ChildProcess | null = null;

/**
 * Get test data directories based on platform
 */
export function getTestDataDirs(): string[] {
  const dirs: string[] = [];

  const configDir =
    process.platform === 'win32'
      ? process.env.APPDATA
      : process.platform === 'darwin'
        ? path.join(os.homedir(), 'Library', 'Application Support')
        : path.join(os.homedir(), '.config');

  if (configDir) {
    dirs.push(path.join(configDir, TEST_APP_NAME));
  }

  const dataDir =
    process.platform === 'win32'
      ? process.env.APPDATA
      : process.platform === 'darwin'
        ? path.join(os.homedir(), 'Library', 'Application Support')
        : path.join(os.homedir(), '.local', 'share');

  if (dataDir && dataDir !== configDir) {
    dirs.push(path.join(dataDir, TEST_APP_NAME));
  }

  return dirs;
}

/**
 * Clean up test data directories
 */
export async function cleanupTestData(): Promise<void> {
  const dirs = getTestDataDirs();
  for (const dir of dirs) {
    if (fs.existsSync(dir)) {
      log.debug(`Cleaning up test data directory: ${dir}`);
      fs.rmSync(dir, { recursive: true, force: true });
    }
  }
}

/**
 * Clean up test keyring credentials (Windows Credential Manager)
 */
export async function cleanupTestCredentials(): Promise<void> {
  if (process.platform === 'win32') {
    try {
      execSync(`cmdkey /delete:youtube_credentials.${TEST_KEYRING_SERVICE} 2>nul`, { stdio: 'ignore' });
      log.debug('Cleaned up test credentials from Windows Credential Manager');
    } catch {
      // Credential may not exist, which is fine
    }
  }
}

/**
 * 指定ポートが解放される（LISTENING 状態でなくなる）まで待機する
 */
async function waitForPortFree(port: number, timeout: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/`);
      // まだ応答がある → まだ使用中
      await new Promise((resolve) => setTimeout(resolve, 300));
    } catch {
      // 接続拒否 → ポートが解放された
      return;
    }
  }
  log.warn(`Port ${port} still in use after ${timeout}ms`);
}

/**
 * Wait for CDP to be available
 */
export async function waitForCDP(timeout = 120000): Promise<void> {
  const start = Date.now();
  log.debug('Waiting for CDP to be available...');
  let lastError = '';
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`${CDP_URL}/json/version`);
      if (response.ok) {
        log.debug(`CDP available after ${Date.now() - start}ms`);
        return;
      }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`CDP not available after ${timeout}ms. Last error: ${lastError}`);
}

/**
 * WebView2 + Vite dev server の初回ロードでは SvelteKit の ESM 動的 import が
 * サイレントに失敗し空白ページになることがある。
 * ページリロードをリトライしてハイドレーション完了を保証する。
 */
export async function ensureSvelteHydrated(page: Page): Promise<void> {
  const heading = page.getByRole('heading', { name: 'Chat Monitor' });
  const maxRetries = 5;
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    const visible = await heading.isVisible().catch(() => false);
    if (visible) {
      log.debug(`SvelteKit hydrated (attempt ${attempt})`);
      return;
    }
    log.debug(`SvelteKit not hydrated, reloading... (attempt ${attempt + 1}/${maxRetries})`);
    try {
      await page.goto('http://localhost:5173/', { waitUntil: 'load', timeout: 30000 });
    } catch (e) {
      // Vite dev server がまだ起動していない場合は少し待ってリトライ
      log.debug(`goto failed: ${e instanceof Error ? e.message : String(e)}`);
      await new Promise((resolve) => setTimeout(resolve, 2000));
      continue;
    }
    try {
      await heading.waitFor({ state: 'visible', timeout: 10000 });
      return;
    } catch {
      // タイムアウトした場合は次のリトライ
    }
  }
  // 最終確認（ここで失敗すると例外が飛ぶ）
  await heading.waitFor({ state: 'visible', timeout: 30000 });
}

/**
 * Connect to Tauri app via CDP
 */
export async function connectToApp(): Promise<{ browser: Browser; context: BrowserContext; page: Page }> {
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

  const page = pages[0];

  // WebView2 初回ロードの SvelteKit ハイドレーション失敗を回避
  await ensureSvelteHydrated(page);

  log.info('Connected to Tauri app');
  return { browser, context, page };
}

/**
 * Kill Tauri app
 *
 * Windows では tauriProcess.kill() だけでは子プロセスツリー（cargo, liscov-tauri.exe,
 * msedgewebview2.exe）が残り、CDP ポート 9222 を占有し続ける。
 * /T フラグでプロセスツリーごと強制終了する。
 */
export async function killTauriApp(): Promise<void> {
  log.debug('Killing Tauri app...');
  if (tauriProcess) {
    if (process.platform === 'win32' && tauriProcess.pid) {
      // プロセスツリーごと強制終了（子プロセスの WebView2 も含む）
      try {
        execSync(`taskkill /F /T /PID ${tauriProcess.pid} 2>nul`, { stdio: 'ignore' });
      } catch {
        // プロセスが既に終了している場合
      }
    } else {
      tauriProcess.kill();
    }
    tauriProcess = null;
  }
  // 孤立プロセスのフォールバック: liscov-tauri.exe を /T（ツリーごと）で強制終了
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /T /IM liscov-tauri.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f liscov-tauri', { stdio: 'ignore' });
    }
  } catch {
    // Process may not exist
  }
  // CDP ポートが解放されるまで待機
  await waitForPortFree(9222, 5000);
}

/**
 * Start Tauri app with test isolation
 */
export async function startTauriApp(): Promise<void> {
  const env = {
    ...process.env,
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  };

  log.info(`Starting Tauri app with test namespace: ${TEST_APP_NAME}`);

  tauriProcess = spawn('pnpm', ['tauri', 'dev'], {
    cwd: PROJECT_DIR,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: true,
  });

  const tauriLog = log.child('tauri');
  tauriProcess.stdout?.on('data', (data) => {
    const msg = data.toString().trim();
    if (msg) tauriLog.debug(msg);
  });
  tauriProcess.stderr?.on('data', (data) => {
    const msg = data.toString().trim();
    if (msg && !msg.includes('Compiling') && !msg.includes('Finished')) {
      tauriLog.debug(msg);
    }
  });

  await waitForCDP();
}

/**
 * Kill mock server process
 */
export async function killMockServer(): Promise<void> {
  if (mockServerProcess) {
    log.debug('Stopping mock server...');
    if (process.platform === 'win32' && mockServerProcess.pid) {
      try {
        execSync(`taskkill /F /T /PID ${mockServerProcess.pid} 2>nul`, { stdio: 'ignore' });
      } catch {
        // プロセスが既に終了している場合
      }
    } else {
      mockServerProcess.kill();
    }
    mockServerProcess = null;
  }
  // 孤立プロセスのフォールバック
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /T /IM mock-server.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f mock_server', { stdio: 'ignore' });
    }
  } catch {
    // Process may not exist
  }
  // ポート解放を待機
  await waitForPortFree(3456, 3000);
}

/**
 * Start mock server
 */
export async function startMockServer(): Promise<void> {
  log.info('Starting mock server...');

  // Kill any existing mock server first
  await killMockServer();

  // モックサーバーを独立クレートから起動
  const cargoPath = path.join(PROJECT_DIR, 'crates', 'mock-server', 'Cargo.toml');
  mockServerProcess = spawn('cargo', ['run', '--manifest-path', cargoPath], {
    cwd: PROJECT_DIR,
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: true,
  });

  const mockLog = log.child('mock_server');

  // Log mock server output for debugging
  mockServerProcess.stdout?.on('data', (data) => {
    const msg = data.toString().trim();
    if (msg) mockLog.debug(msg);
  });
  mockServerProcess.stderr?.on('data', (data) => {
    const msg = data.toString().trim();
    // Filter out cargo build warnings/info
    if (msg && !msg.includes('Compiling') && !msg.includes('Finished') && !msg.includes('warning:')) {
      mockLog.debug(msg);
    }
  });

  // Wait for mock server to be ready
  const timeout = 60000;
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`${MOCK_SERVER_URL}/status`);
      if (response.ok) {
        log.debug(`Mock server ready after ${Date.now() - start}ms`);
        return;
      }
    } catch {
      // Server not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`Mock server not ready after ${timeout}ms`);
}

/**
 * Reset mock server state
 */
export async function resetMockServer(): Promise<void> {
  log.debug('Resetting mock server state...');
  await fetch(`${MOCK_SERVER_URL}/reset`, { method: 'POST' });
}

/**
 * Add message to mock server
 */
export async function addMockMessage(message: {
  message_type: string;
  author: string;
  content: string;
  channel_id?: string;
  is_member?: boolean;
  amount?: string;
  tier?: string;
  milestone_months?: number;
  gift_count?: number;
}): Promise<void> {
  await fetch(`${MOCK_SERVER_URL}/add_message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(message),
  });
}

/**
 * Common setup for E2E tests
 */
export async function setupTestEnvironment(): Promise<{ browser: Browser; context: BrowserContext; page: Page }> {
  // Step 1: Kill any existing processes
  log.info('Setting up test environment...');
  await killTauriApp();

  // Step 2: Clean up test data and credentials for a fresh start
  await cleanupTestData();
  await cleanupTestCredentials();

  // Step 3: Start mock server
  await startMockServer();

  // Step 4: Reset mock server state
  await resetMockServer();

  // Step 5: Start Tauri app with test namespace
  await startTauriApp();

  // Step 6: Connect to the running Tauri app（ensureSvelteHydrated は connectToApp 内で実行）
  return await connectToApp();
}

/**
 * Common teardown for E2E tests
 */
export async function teardownTestEnvironment(browser?: Browser): Promise<void> {
  log.info('Tearing down test environment...');
  const errors: Error[] = [];

  for (const [name, cleanup] of [
    ['browser.close', () => browser?.close()],
    ['killTauriApp', killTauriApp],
    ['killMockServer', killMockServer],
    ['cleanupTestData', cleanupTestData],
    ['cleanupTestCredentials', cleanupTestCredentials],
  ] as [string, () => Promise<void> | undefined][]) {
    try {
      await cleanup();
    } catch (e) {
      const error = e instanceof Error ? e : new Error(String(e));
      log.warn(`Teardown step "${name}" failed: ${error.message}`);
      errors.push(error);
    }
  }

  if (errors.length > 0) {
    log.warn(`Teardown completed with ${errors.length} error(s)`);
  }
}

/**
 * アプリを再起動する（設定永続化テスト用）
 * 既存のアプリを終了し、新しいインスタンスを起動してCDP接続を確立する
 */
export async function restartApp(): Promise<{ browser: Browser; context: BrowserContext; page: Page }> {
  // 既存のアプリを終了
  await killTauriApp();
  // Vite dev server のポートも解放されるまで待機
  await waitForPortFree(5173, 5000);
  // 新しいインスタンスを起動
  await startTauriApp();
  // CDP接続を確立（ensureSvelteHydrated は connectToApp 内で実行）
  return await connectToApp();
}

/**
 * Fully disconnect (stop + initialize) and return app to idle state.
 * Clicks 停止 if visible, then 初期化, and waits for the URL input to reappear.
 */
export async function disconnectAndInitialize(page: Page): Promise<void> {
  const stopButton = page.locator('button:has-text("停止")');
  if (await stopButton.isVisible({ timeout: 1000 }).catch(() => false)) {
    await stopButton.click();
    await page.locator('button:has-text("初期化")').click();
    await expect(page.locator('input[placeholder*="youtube.com"]')).toBeVisible({ timeout: 5000 });
  }
}

/**
 * Connect to mock server stream and wait for stream title to appear.
 * @param videoId - optional video ID, defaults to "test_video_123"
 */
export async function connectToMockStream(page: Page, videoId = 'test_video_123'): Promise<void> {
  const urlInput = page.locator('input[placeholder*="youtube.com"]');
  await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=${videoId}`);
  await page.locator('button:has-text("開始")').click();
  await expect(page.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });
}

/**
 * Navigate to a tab by its display name using the nav button.
 */
export async function navigateToTab(page: Page, tabName: string): Promise<void> {
  const tab = page.locator(`nav button:has-text("${tabName}")`);
  await tab.click();
}

/**
 * Set stream state on the mock server.
 */
export async function setStreamState(state: { member_only?: boolean; require_auth?: boolean; title?: string }): Promise<void> {
  await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(state),
  });
}
