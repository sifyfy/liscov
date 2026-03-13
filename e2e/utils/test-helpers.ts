/**
 * E2Eテスト共通ヘルパー関数
 */

import { chromium, BrowserContext, Page, Browser, expect } from '@playwright/test';
import { execSync, spawn, ChildProcess } from 'child_process';
import * as fs from 'fs';
import * as http from 'http';
import * as path from 'path';
import * as os from 'os';
import { log } from './logger';

export const CDP_URL = 'http://127.0.0.1:9222';
export const MOCK_SERVER_URL = 'http://127.0.0.1:3456';
export const PROJECT_DIR = process.cwd().replace(/[\\/]e2e$/, '');

// テスト分離: 認証情報・データに専用名前空間を使用
export const TEST_APP_NAME = 'liscov-test';
export const TEST_KEYRING_SERVICE = 'liscov-test';

// モックサーバープロセス参照
let mockServerProcess: ChildProcess | null = null;

// Tauriアプリプロセス参照
let tauriProcess: ChildProcess | null = null;
let staticFrontendServer: http.Server | null = null;

// プリビルドバイナリのパス（Windowsのみ対応）
const PREBUILT_TAURI_APP_PATH = path.join(PROJECT_DIR, 'src-tauri', 'target', 'debug', 'liscov-tauri.exe');
const PREBUILT_MOCK_SERVER_PATH = path.join(PROJECT_DIR, 'src-tauri', 'target', 'debug', 'mock_server.exe');
const PREBUILT_FRONTEND_INDEX_PATH = path.join(PROJECT_DIR, 'build', 'index.html');
const PREBUILT_FRONTEND_DIR = path.join(PROJECT_DIR, 'build');
const PREBUILT_FRONTEND_PORT = 5173;

/**
 * テスト用プロセス環境変数を生成する
 */
function getTestProcessEnv(extraEnv: NodeJS.ProcessEnv = {}): NodeJS.ProcessEnv {
  return { ...process.env, ...extraEnv };
}

/**
 * プラットフォームに応じた設定ディレクトリを返す
 */
export function getPlatformConfigDir(): string {
  if (process.platform === 'win32') {
    return process.env.APPDATA ?? path.join(os.homedir(), 'AppData', 'Roaming');
  }
  return process.platform === 'darwin'
    ? path.join(os.homedir(), 'Library', 'Application Support')
    : path.join(os.homedir(), '.config');
}

export function getTestAppDataDir(): string {
  return path.join(getPlatformConfigDir(), TEST_APP_NAME);
}

export function getTestDatabasePath(): string {
  return path.join(getTestAppDataDir(), 'liscov.db');
}

/**
 * プラットフォームに応じたテストデータディレクトリ一覧を返す
 */
export function getTestDataDirs(): string[] {
  const dirs: string[] = [];
  const configDir = getPlatformConfigDir();
  dirs.push(path.join(configDir, TEST_APP_NAME));
  // Linux では設定とデータが別ディレクトリになる場合がある
  if (process.platform !== 'win32' && process.platform !== 'darwin') {
    const dataDir = path.join(os.homedir(), '.local', 'share');
    if (dataDir !== configDir) {
      dirs.push(path.join(dataDir, TEST_APP_NAME));
    }
  }
  return dirs;
}

/**
 * テストデータディレクトリを削除する
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
 * テスト用キーリング認証情報を削除する（Windows資格情報マネージャー）
 */
export async function cleanupTestCredentials(): Promise<void> {
  if (process.platform === 'win32') {
    try {
      execSync(`cmdkey /delete:youtube_credentials.${TEST_KEYRING_SERVICE} 2>nul`, { stdio: 'ignore' });
      log.debug('Cleaned up test credentials from Windows Credential Manager');
    } catch {
      // 認証情報が存在しない場合は無視
    }
  }
}

/**
 * レスポンスコンテンツタイプを拡張子から解決する
 */
function getStaticContentType(filePath: string): string {
  switch (path.extname(filePath).toLowerCase()) {
    case '.css':
      return 'text/css; charset=utf-8';
    case '.html':
      return 'text/html; charset=utf-8';
    case '.ico':
      return 'image/x-icon';
    case '.js':
      return 'application/javascript; charset=utf-8';
    case '.json':
      return 'application/json; charset=utf-8';
    case '.png':
      return 'image/png';
    case '.svg':
      return 'image/svg+xml';
    case '.txt':
      return 'text/plain; charset=utf-8';
    case '.woff2':
      return 'font/woff2';
    default:
      return 'application/octet-stream';
  }
}

/**
 * リクエストURLからビルド済みフロントエンドのファイルパスを解決する
 */
function resolveStaticFrontendFile(requestUrl?: string): string {
  const requestPath = decodeURIComponent(new URL(requestUrl ?? '/', 'http://127.0.0.1').pathname);
  const relativePath = requestPath === '/' ? 'index.html' : requestPath.replace(/^\/+/, '');
  const resolvedPath = path.resolve(PREBUILT_FRONTEND_DIR, relativePath);

  if (resolvedPath.startsWith(path.resolve(PREBUILT_FRONTEND_DIR)) && fs.existsSync(resolvedPath) && fs.statSync(resolvedPath).isFile()) {
    return resolvedPath;
  }

  return PREBUILT_FRONTEND_INDEX_PATH;
}

/**
 * 静的フロントエンドサーバーを起動する（既に起動済みの場合は再利用）
 */
async function ensureStaticFrontendServer(): Promise<void> {
  if (staticFrontendServer) return;

  staticFrontendServer = http.createServer((req, res) => {
    const filePath = resolveStaticFrontendFile(req.url);
    try {
      const content = fs.readFileSync(filePath);
      res.writeHead(200, { 'Content-Type': getStaticContentType(filePath) });
      res.end(content);
    } catch {
      res.writeHead(404);
      res.end('Not found');
    }
  });

  await new Promise<void>((resolve, reject) => {
    staticFrontendServer!.once('error', (error) => {
      if ((error as NodeJS.ErrnoException).code === 'EADDRINUSE') {
        log.debug(`Port ${PREBUILT_FRONTEND_PORT} already in use, reusing existing server`);
        staticFrontendServer = null;
        resolve();
        return;
      }
      reject(error);
    });
    staticFrontendServer!.listen(PREBUILT_FRONTEND_PORT, '127.0.0.1', resolve);
  });

  log.debug(`Static frontend server started on port ${PREBUILT_FRONTEND_PORT}`);
}

/**
 * プロセスの直近ログ行を記録するバッファを生成する
 */
function buildProcessTailRecorder(maxEntries = 20): {
  lines: string[];
  push: (chunk: string) => void;
} {
  const lines: string[] = [];

  return {
    lines,
    push: (chunk: string) => {
      chunk
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter((line) => line.length > 0)
        .forEach((line) => {
          lines.push(line);
          if (lines.length > maxEntries) {
            lines.shift();
          }
        });
    },
  };
}

/**
 * プロセスが終了していた場合に終了情報の文字列を返す（生存中は null）
 */
function describeExitedProcess(processName: string, process: ChildProcess, tailLines: string[]): string | null {
  if (process.exitCode === null && process.signalCode === null) {
    return null;
  }

  const exitDescription =
    process.exitCode !== null
      ? `${processName} exited with code ${process.exitCode}`
      : `${processName} exited with signal ${process.signalCode}`;
  const outputDescription = tailLines.length > 0 ? ` Recent output: ${tailLines.join(' | ')}` : '';
  return `${exitDescription}.${outputDescription}`;
}

/**
 * 指定ポートが解放されるまで待機する
 */
async function waitForPortFree(port: number, timeout: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      await fetch(`http://127.0.0.1:${port}/`);
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
 * Tauriアプリを終了する（graceful shutdown → 強制終了の順で試行）
 */
export async function killTauriApp(): Promise<void> {
  log.debug('Killing Tauri app...');
  if (tauriProcess) {
    if (process.platform === 'win32' && tauriProcess.pid) {
      // まず graceful shutdown を試行
      try {
        execSync(`taskkill /PID ${tauriProcess.pid} 2>nul`, { stdio: 'ignore' });
        await waitForPortFree(9222, 3000);
      } catch { /* 既に終了していた場合は無視 */ }
      // プロセスツリーごと強制終了（フォールバック）
      try {
        execSync(`taskkill /F /T /PID ${tauriProcess.pid} 2>nul`, { stdio: 'ignore' });
      } catch { /* 既に終了していた場合は無視 */ }
    } else {
      tauriProcess.kill();
    }
    tauriProcess = null;
  }
  // 孤立プロセスのフォールバック: プロセスツリーごと強制終了
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /T /IM liscov-tauri.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f liscov-tauri', { stdio: 'ignore' });
    }
  } catch { /* プロセスが存在しない場合は無視 */ }
  // CDP ポートが解放されるまで待機（Windowsではプロセスツリー終了が遅延するため長めに設定）
  await waitForPortFree(9222, 10000);
}

/**
 * CDPが利用可能になるまで待機する
 */
export async function waitForCDP(timeout = 120000): Promise<void> {
  return waitForCDPWithProcess(timeout);
}

async function waitForCDPWithProcess(timeout = 120000, process?: ChildProcess, tailLines: string[] = []): Promise<void> {
  const start = Date.now();
  log.debug('Waiting for CDP to be available...');
  let lastError = '';
  while (Date.now() - start < timeout) {
    const exitInfo = process ? describeExitedProcess('Tauri app', process, tailLines) : null;
    if (exitInfo) {
      throw new Error(`CDP not available because ${exitInfo}`);
    }

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
 * CDPでTauriアプリに接続する
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

  log.info('Connected to Tauri app');
  return { browser, context, page: pages[0] };
}

/**
 * テスト分離用の環境変数でTauriアプリを起動する
 */
export async function startTauriApp(): Promise<void> {
  await startTauriAppWithEnv({
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
  });
}

/**
 * 指定した環境変数でTauriアプリを起動する（プリビルドバイナリ必須）
 */
export async function startTauriAppWithEnv(extraEnv: NodeJS.ProcessEnv): Promise<void> {
  if (!fs.existsSync(PREBUILT_TAURI_APP_PATH)) {
    throw new Error(
      `プリビルドバイナリが見つかりません: ${PREBUILT_TAURI_APP_PATH}\n` +
        '`pnpm test:e2e:build` を実行してビルドしてください。'
    );
  }
  if (!fs.existsSync(PREBUILT_FRONTEND_INDEX_PATH)) {
    throw new Error(
      `ビルド済みフロントエンドが見つかりません: ${PREBUILT_FRONTEND_INDEX_PATH}\n` +
        '`pnpm test:e2e:build` を実行してビルドしてください。'
    );
  }

  await ensureStaticFrontendServer();

  const env = getTestProcessEnv({
    ...extraEnv,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  });

  log.info(`Starting prebuilt Tauri app: ${PREBUILT_TAURI_APP_PATH}`);
  const tauriTail = buildProcessTailRecorder();

  tauriProcess = spawn(PREBUILT_TAURI_APP_PATH, [], {
    cwd: PROJECT_DIR,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  const tauriLog = log.child('tauri');
  tauriProcess.stdout?.on('data', (data) => {
    const msg = data.toString().trim();
    tauriTail.push(msg);
    if (msg) tauriLog.debug(msg);
  });
  tauriProcess.stderr?.on('data', (data) => {
    const msg = data.toString().trim();
    tauriTail.push(msg);
    if (msg && !msg.includes('Compiling') && !msg.includes('Finished')) {
      tauriLog.debug(msg);
    }
  });

  await waitForCDPWithProcess(120000, tauriProcess, tauriTail.lines);
}

/**
 * モックサーバープロセスを終了する（graceful shutdown → 強制終了の順で試行）
 */
export async function killMockServer(): Promise<void> {
  if (mockServerProcess) {
    log.debug('Stopping mock server...');
    if (process.platform === 'win32' && mockServerProcess.pid) {
      // まず graceful shutdown を試行
      try {
        execSync(`taskkill /PID ${mockServerProcess.pid} 2>nul`, { stdio: 'ignore' });
        await waitForPortFree(3456, 3000);
      } catch { /* 既に終了していた場合は無視 */ }
      // プロセスツリーごと強制終了（フォールバック）
      try {
        execSync(`taskkill /F /T /PID ${mockServerProcess.pid} 2>nul`, { stdio: 'ignore' });
      } catch { /* 既に終了していた場合は無視 */ }
    } else {
      mockServerProcess.kill();
    }
    mockServerProcess = null;
  }
  // 孤立プロセスのフォールバック
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /T /IM mock_server.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f mock_server', { stdio: 'ignore' });
    }
  } catch { /* プロセスが存在しない場合は無視 */ }
  await waitForPortFree(3456, 3000);
}

/**
 * モックサーバーを起動する（プリビルドバイナリ必須）
 */
export async function startMockServer(): Promise<void> {
  log.info('Starting mock server...');
  await killMockServer();

  if (!fs.existsSync(PREBUILT_MOCK_SERVER_PATH)) {
    throw new Error(
      `モックサーバーバイナリが見つかりません: ${PREBUILT_MOCK_SERVER_PATH}\n` +
        '`pnpm test:e2e:build` を実行してビルドしてください。'
    );
  }

  const mockTail = buildProcessTailRecorder();

  mockServerProcess = spawn(PREBUILT_MOCK_SERVER_PATH, [], {
    cwd: PROJECT_DIR,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  const mockLog = log.child('mock_server');
  mockServerProcess.stdout?.on('data', (data) => {
    const msg = data.toString().trim();
    mockTail.push(msg);
    if (msg) mockLog.debug(msg);
  });
  mockServerProcess.stderr?.on('data', (data) => {
    const msg = data.toString().trim();
    mockTail.push(msg);
    if (msg && !msg.includes('Compiling') && !msg.includes('Finished') && !msg.includes('warning:')) {
      mockLog.debug(msg);
    }
  });

  // モックサーバーの起動を待機
  const timeout = 60000;
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const exitInfo = describeExitedProcess('mock_server', mockServerProcess, mockTail.lines);
    if (exitInfo) throw new Error(`モックサーバーが起動できませんでした: ${exitInfo}`);

    try {
      const response = await fetch(`${MOCK_SERVER_URL}/status`);
      if (response.ok) {
        log.debug(`Mock server ready after ${Date.now() - start}ms`);
        return;
      }
    } catch {
      // まだ起動していない
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`Mock server not ready after ${timeout}ms`);
}

/**
 * モックサーバーの状態をリセットする
 */
export async function resetMockServer(): Promise<void> {
  log.debug('Resetting mock server state...');
  await fetch(`${MOCK_SERVER_URL}/reset`, { method: 'POST' });
}

/**
 * モックサーバーにメッセージを追加する
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
 * E2Eテスト共通セットアップ
 */
export async function setupTestEnvironment(): Promise<{ browser: Browser; context: BrowserContext; page: Page }> {
  log.info('Setting up test environment...');

  // Step 1: 既存のプロセスを終了
  await killTauriApp();

  // Step 2: テストデータ・認証情報を削除してクリーンな状態にする
  await cleanupTestData();
  await cleanupTestCredentials();

  // Step 3: モックサーバーを起動
  await startMockServer();

  // Step 4: モックサーバーの状態をリセット
  await resetMockServer();

  // Step 5: テスト用名前空間でTauriアプリを起動
  await startTauriApp();

  // Step 6: Tauriアプリに接続
  const connection = await connectToApp();

  // Svelteアプリが完全にマウントされるまで待機
  await connection.page.waitForLoadState('load');
  // Svelteレンダリング後にのみ表示される既知のUI要素を待機
  await connection.page.getByRole('heading', { name: 'Chat Monitor' }).waitFor({ state: 'visible', timeout: 30000 });

  return connection;
}

/**
 * E2Eテスト共通ティアダウン（静的サーバーも停止する）
 */
export async function teardownTestEnvironment(browser?: Browser): Promise<void> {
  log.info('Tearing down test environment...');
  const errors: Error[] = [];

  for (const [name, cleanup] of [
    ['browser.close', () => browser?.close()],
    ['killTauriApp', killTauriApp],
    ['killMockServer', killMockServer],
    ['stopStaticServer', () => new Promise<void>((resolve) => {
      if (!staticFrontendServer) { resolve(); return; }
      staticFrontendServer.close(() => { staticFrontendServer = null; resolve(); });
    })],
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
 * 全接続を切断し、蓄積されたメッセージをクリアしてアプリをアイドル状態に戻す。
 * 多接続リファクタリングで「初期化」ボタンが廃止されたため、
 * 切断後にFilterPanelの「クリア」ボタンでメッセージを消去する。
 */
export async function disconnectAndInitialize(page: Page): Promise<void> {
  // Step 1: 全接続を切断
  const disconnectAllBtn = page.locator('button:has-text("全切断")');
  if (await disconnectAllBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await disconnectAllBtn.click();
    await expect(page.locator('.connection-item')).toHaveCount(0, { timeout: 10000 });
  } else {
    const disconnectBtn = page.locator('.connection-item .disconnect-btn').first();
    if (await disconnectBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await disconnectBtn.click();
      await expect(page.locator('.connection-item')).toHaveCount(0, { timeout: 10000 });
    }
  }

  // Step 2: 蓄積メッセージをクリア（テスト間の状態分離のため）
  // FilterPanelの「クリア」ボタン → 確認ダイアログ → 実行
  const clearButton = page.locator('button:has-text("クリア")').first();
  if (await clearButton.isEnabled({ timeout: 1000 }).catch(() => false)) {
    await clearButton.click();
    // 確認ダイアログ内のクリアボタン
    const dialog = page.locator('.fixed.inset-0');
    await expect(dialog).toBeVisible({ timeout: 3000 });
    const confirmBtn = dialog.locator('button:has-text("クリア")');
    await confirmBtn.click();
    await expect(dialog).not.toBeVisible({ timeout: 3000 });
  }
}

/**
 * モックサーバーのストリームに接続し、接続リストへの追加を待機する。
 * URLフォームは常に表示されているため、接続後も入力欄は残る。
 * @param videoId - 動画ID（省略時は "test_video_123"）
 * @param expectedTitle - 接続確認に使うストリームタイトル（省略時は "Mock Live"）
 */
export async function connectToMockStream(page: Page, videoId = 'test_video_123', expectedTitle = 'Mock Live'): Promise<void> {
  const urlInput = page.locator('input[placeholder*="youtube.com"]');
  await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=${videoId}`);
  await page.locator('button:has-text("開始")').click();
  // 接続リストにエントリが追加されるのを待つ
  await expect(page.getByText(expectedTitle).first()).toBeVisible({ timeout: 10000 });
}

/**
 * ナビゲーションボタンから指定タブに遷移する
 */
export async function navigateToTab(page: Page, tabName: string): Promise<void> {
  const tab = page.locator(`nav button:has-text("${tabName}")`);
  await tab.click();
}

/**
 * モックサーバーのストリーム状態を設定する
 */
export async function setStreamState(state: { member_only?: boolean; require_auth?: boolean; title?: string }): Promise<void> {
  await fetch(`${MOCK_SERVER_URL}/set_stream_state`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(state),
  });
}

/**
 * アプリを再起動して新しいブラウザ接続を返す
 */
export async function restartApp(): Promise<{
  browser: Browser;
  context: BrowserContext;
  page: Page;
}> {
  await killTauriApp();
  await startTauriApp();
  return await connectToApp();
}
