/**
 * E2Eテスト共通ヘルパー関数
 */

import { chromium, BrowserContext, Page, Browser, Locator, expect } from '@playwright/test';
import { execSync, spawn, ChildProcess } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { log } from './logger';
import {
  acquireWorktreeLock,
  describeFrontendServerMismatch,
  parseDevToolsActivePort,
  parseMockServerUrl,
  releaseWorktreeLock,
  webView2BrowserArguments,
  worktreeTestName,
  type DevToolsEndpoint,
  type FrontendServerIdFetch,
} from './e2e-isolation';
import { FRONTEND_SERVER_ID_PATH, startStaticFrontendServer, type StaticFrontendServer } from './static-frontend-server';

export const PROJECT_DIR = process.cwd().replace(/[\\/]e2e$/, '');

// テスト分離: 認証情報・データに専用名前空間を使用する。ワークツリーごとに分け、並列に走らせても干渉しない（ADR-005）
export const TEST_APP_NAME = worktreeTestName(PROJECT_DIR);
export const TEST_KEYRING_SERVICE = TEST_APP_NAME;

// 同じワークツリーの E2E の二重実行を検知するロック（e2e/README.md「同じワークツリーでの二重実行」）
const WORKTREE_LOCK_PATH = path.join(PROJECT_DIR, '.tmp', 'e2e.lock');

// モックサーバープロセス参照と、標準出力から読んだ実際の URL（ポートは起動ごとに OS が選ぶ）
let mockServerProcess: ChildProcess | null = null;
let mockServerBaseUrl: string | null = null;

// Tauriアプリプロセス参照と、DevToolsActivePort から読んだ CDP のエンドポイント
let tauriProcess: ChildProcess | null = null;
let cdpEndpoint: DevToolsEndpoint | null = null;
let staticFrontendServer: StaticFrontendServer | null = null;

/**
 * モックサーバーの URL（例: http://127.0.0.1:51234）。ポートは起動ごとに変わるので、起動後に呼ぶ
 */
export function mockServerUrl(): string {
  if (mockServerBaseUrl === null) {
    throw new Error('モックサーバーがまだ起動していません。startMockServer() / setupTestEnvironment() の後に呼んでください。');
  }
  return mockServerBaseUrl;
}

// プリビルドバイナリのパス（Windowsのみ対応）
// 注: workspace 化により cargo build の出力先は <root>/target/ (旧: src-tauri/target/)
const PREBUILT_TAURI_APP_PATH = path.join(PROJECT_DIR, 'target', 'debug', 'liscov-tauri.exe');
const PREBUILT_MOCK_SERVER_PATH = path.join(PROJECT_DIR, 'target', 'debug', 'mock-server.exe');
const PREBUILT_FRONTEND_INDEX_PATH = path.join(PROJECT_DIR, 'build', 'index.html');
const PREBUILT_FRONTEND_DIR = path.join(PROJECT_DIR, 'build');
// debug ビルドの exe が読む URL (http://localhost:5173)。WebView2 の接続先をここから静的サーバーへ付け替える
const DEV_URL: string = JSON.parse(fs.readFileSync(path.join(PROJECT_DIR, 'src-tauri', 'tauri.conf.json'), 'utf8')).build.devUrl;

/**
 * このワークツリーの E2E のロックを取る。共有の名前空間に触る操作（アプリ・モックの停止と起動、テストデータ・資格情報の削除）の
 * 先頭で呼ぶ。setupTestEnvironment() を通らない spec もあるので、個々の関数の中で呼ぶ。
 * 停止もロックの後にするのは、exe のパスで止める killProcessesStartedFrom() が同じワークツリーの別の実行のアプリまで止めるため
 */
function ensureWorktreeLock(): void {
  acquireWorktreeLock(WORKTREE_LOCK_PATH, process.pid);
}
// ワーカーのプロセスが終わるときに解放する（落ちて残ったロックは、次の実行が PID の生死で判定する）
process.once('exit', () => releaseWorktreeLock(WORKTREE_LOCK_PATH, process.pid));

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

/** E2E 用の WebView2 データフォルダ（本番は %LOCALAPPDATA%\com.liscov-tauri.app\EBWebView） */
export function getTestWebViewDataDir(): string {
  const localAppData = process.env.LOCALAPPDATA ?? path.join(os.homedir(), 'AppData', 'Local');
  return path.join(localAppData, TEST_APP_NAME, 'EBWebView');
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
  ensureWorktreeLock();
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
  ensureWorktreeLock();
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
 * このワークツリーの build/ を空きポートで配信する（同じプロセス内で起動済みならそれを返す）
 */
async function ensureStaticFrontendServer(): Promise<StaticFrontendServer> {
  if (!staticFrontendServer) {
    staticFrontendServer = await startStaticFrontendServer(PREBUILT_FRONTEND_DIR);
    log.debug(`Static frontend server started on port ${staticFrontendServer.port}`);
  }
  return staticFrontendServer;
}

/** WebView2 が CDP の実際のポートを書くファイル（WEBVIEW2_USER_DATA_FOLDER の下の EBWebView にできる） */
function getDevToolsActivePortPath(): string {
  return path.join(getTestWebViewDataDir(), 'EBWebView', 'DevToolsActivePort');
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
/**
 * 指定した実行ファイルから起動したプロセス（とその子プロセス）だけを強制終了する
 *
 * 名前で止めると、同じ名前の本番アプリ（liscov-tauri.exe）やユーザーのプロセスまで止めてしまう。
 * 配信中に E2E を走らせても本番に触れないよう、実行ファイルのパスで絞る。
 */
export function killProcessesStartedFrom(exePath: string): void {
  try {
    if (process.platform === 'win32') {
      const name = path.basename(exePath, '.exe').replace(/'/g, "''");
      const target = exePath.replace(/'/g, "''");
      const ids = execSync(
        `powershell -NoProfile -NonInteractive -Command "Get-Process -Name '${name}' -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq '${target}' } | ForEach-Object { $_.Id }"`,
        { encoding: 'utf8' }
      )
        .split(/\s+/)
        .filter(Boolean);
      for (const id of ids) {
        execSync(`taskkill /F /T /PID ${id} 2>nul`, { stdio: 'ignore' });
      }
    } else {
      execSync(`pkill -f '${exePath}'`, { stdio: 'ignore' });
    }
  } catch { /* プロセスが存在しない場合は無視 */ }
}

export async function killTauriApp(): Promise<void> {
  ensureWorktreeLock();
  log.debug('Killing Tauri app...');
  const cdpPort = cdpEndpoint?.port;
  if (tauriProcess) {
    if (process.platform === 'win32' && tauriProcess.pid) {
      // まず graceful shutdown を試行
      try {
        execSync(`taskkill /PID ${tauriProcess.pid} 2>nul`, { stdio: 'ignore' });
        if (cdpPort) await waitForPortFree(cdpPort, 3000);
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
  // 孤立プロセスのフォールバック: テスト用の実行ファイルから起動したものだけをプロセスツリーごと強制終了
  killProcessesStartedFrom(PREBUILT_TAURI_APP_PATH);
  // CDP ポートが解放されるまで待機（Windowsではプロセスツリー終了が遅延するため長めに設定）
  if (cdpPort) await waitForPortFree(cdpPort, 10000);
  cdpEndpoint = null;
}

/**
 * WebView2 が DevToolsActivePort を書くのを待ち、CDP のエンドポイントを返す
 *
 * 起動前に消したファイルを、自分のデータフォルダから読むので、別のワークツリーのアプリに繋がることはない
 */
async function waitForDevToolsEndpoint(timeout: number, process: ChildProcess, tailLines: string[]): Promise<DevToolsEndpoint> {
  const start = Date.now();
  const portFile = getDevToolsActivePortPath();
  log.debug(`Waiting for ${portFile}...`);
  while (Date.now() - start < timeout) {
    const exitInfo = describeExitedProcess('Tauri app', process, tailLines);
    if (exitInfo) {
      throw new Error(`CDP not available because ${exitInfo}`);
    }

    const endpoint = fs.existsSync(portFile) ? parseDevToolsActivePort(fs.readFileSync(portFile, 'utf8')) : null;
    if (endpoint) {
      log.debug(`CDP available on port ${endpoint.port} after ${Date.now() - start}ms`);
      return endpoint;
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(
    `CDP not available after ${timeout}ms: ${portFile} が書かれませんでした。` +
      '同じ WebView2 のデータフォルダを使うアプリが既に動いていないか確認してください (e2e/README.md のトラブルシューティング)。'
  );
}

/**
 * ページが devUrl へ遷移するのを待ち、E2E の静的サーバーから読み込まれたかを確かめる（違えば例外で止める）
 */
async function verifyFrontendServer(page: Page, expectedId: string): Promise<void> {
  const deadline = Date.now() + 30000;
  while (page.url() === 'about:blank' && Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  const idUrl = new URL(FRONTEND_SERVER_ID_PATH, DEV_URL).href;
  const result: FrontendServerIdFetch = await page
    .evaluate(async (url) => (await fetch(url, { cache: 'no-store' })).text(), idUrl)
    .then(
      (body): FrontendServerIdFetch => ({ kind: 'fetched', body }),
      (error: unknown): FrontendServerIdFetch => ({ kind: 'failed', error: error instanceof Error ? error.message : String(error) })
    );
  const mismatch = describeFrontendServerMismatch(result, expectedId);
  if (mismatch) throw new Error(`${mismatch} (ページの URL: ${page.url()})`);
}

/**
 * CDPでTauriアプリに接続する
 */
export async function connectToApp(): Promise<{ browser: Browser; context: BrowserContext; page: Page }> {
  if (!cdpEndpoint || !staticFrontendServer) {
    throw new Error('Tauri アプリがまだ起動していません。startTauriApp() / startTauriAppWithEnv() の後に呼んでください。');
  }
  const browser = await chromium.connectOverCDP(cdpEndpoint.browserWsUrl);
  const contexts = browser.contexts();

  if (contexts.length === 0) {
    throw new Error('No browser contexts found');
  }

  const context = contexts[0];
  const pages = context.pages();

  if (pages.length === 0) {
    throw new Error('No pages found in context');
  }

  await verifyFrontendServer(pages[0], staticFrontendServer.serverId);
  log.info('Connected to Tauri app');
  return { browser, context, page: pages[0] };
}

/**
 * テスト分離用の環境変数でTauriアプリを起動する
 *
 * モックサーバーが起動していなければ起動する。モックを使わない spec でも、アプリの YouTube 宛ての通信
 * (起動時のセッション確認など) は本物の YouTube や別のワークツリーのモックではなく、このモックに向ける
 */
export async function startTauriApp(): Promise<void> {
  if (mockServerBaseUrl === null) await startMockServer();
  await startTauriAppWithEnv({
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    LISCOV_AUTH_URL: `${mockServerUrl()}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${mockServerUrl()}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: mockServerUrl(),
  });
}

/**
 * 指定した環境変数でTauriアプリを起動する（プリビルドバイナリ必須）
 */
export async function startTauriAppWithEnv(extraEnv: NodeJS.ProcessEnv): Promise<void> {
  ensureWorktreeLock();
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

  const frontend = await ensureStaticFrontendServer();
  // 前回の実行のポートを読まないよう、WebView2 が書き直すファイルを先に消す
  fs.rmSync(getDevToolsActivePortPath(), { force: true });

  const env = getTestProcessEnv({
    ...extraEnv,
    // CDP は空きポートで開き、devUrl の接続先はこのワークツリーの静的サーバーへ付け替える（ADR-005）
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: webView2BrowserArguments(DEV_URL, frontend.port),
    // WebView2 のデータ (Cookie・キャッシュ) は identifier で決まるフォルダにあり、LISCOV_APP_NAME では分かれない。
    // 同じフォルダを使うアプリ (本番の liscov や別のワークツリーの E2E) が起動中だとブラウザプロセスを共有し、
    // 上の引数が効かず CDP が開かない。ワークツリーごとのフォルダに分ける（環境変数がアプリの指定より優先される）
    WEBVIEW2_USER_DATA_FOLDER: getTestWebViewDataDir(),
    // WebSocket サーバーは OS が選ぶ空きポートで動かす。本番の 8765 を取ると、本番の liscov 向けにつなぎに来る
    // オーバーレイ (OBS のブラウザソースなど) がこのアプリにつながる（03_websocket.md「開始ポートの上書き」、ADR-005）
    LISCOV_WEBSOCKET_PORT: '0',
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

  cdpEndpoint = await waitForDevToolsEndpoint(120000, tauriProcess, tauriTail.lines);
}

/**
 * モックサーバープロセスを終了する（graceful shutdown → 強制終了の順で試行）
 */
export async function killMockServer(): Promise<void> {
  ensureWorktreeLock();
  const port = mockServerBaseUrl ? Number(new URL(mockServerBaseUrl).port) : null;
  if (mockServerProcess) {
    log.debug('Stopping mock server...');
    if (process.platform === 'win32' && mockServerProcess.pid) {
      // まず graceful shutdown を試行
      try {
        execSync(`taskkill /PID ${mockServerProcess.pid} 2>nul`, { stdio: 'ignore' });
        if (port) await waitForPortFree(port, 3000);
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
  // 孤立プロセスのフォールバック（テスト用の実行ファイルから起動したものだけ）
  killProcessesStartedFrom(PREBUILT_MOCK_SERVER_PATH);
  if (port) await waitForPortFree(port, 3000);
  mockServerBaseUrl = null;
}

/**
 * モックサーバーを起動する（プリビルドバイナリ必須）
 *
 * ポートは OS に選ばせ (--port 0)、実際の URL は自分が起動したプロセスの標準出力から読む（ADR-005）
 */
export async function startMockServer(): Promise<void> {
  ensureWorktreeLock();
  log.info('Starting mock server...');
  await killMockServer();

  if (!fs.existsSync(PREBUILT_MOCK_SERVER_PATH)) {
    throw new Error(
      `モックサーバーバイナリが見つかりません: ${PREBUILT_MOCK_SERVER_PATH}\n` +
        '`pnpm test:e2e:build` を実行してビルドしてください。'
    );
  }

  const mockTail = buildProcessTailRecorder();
  let stdout = '';
  let url: string | null = null;

  mockServerProcess = spawn(PREBUILT_MOCK_SERVER_PATH, ['--port', '0'], {
    cwd: PROJECT_DIR,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  const mockLog = log.child('mock_server');
  mockServerProcess.stdout?.on('data', (data) => {
    const text = data.toString();
    if (url === null) {
      stdout += text;
      url = parseMockServerUrl(stdout);
    }
    const msg = text.trim();
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

    if (url !== null) {
      try {
        const response = await fetch(`${url}/status`);
        if (response.ok) {
          mockServerBaseUrl = url;
          log.debug(`Mock server ready on ${url} after ${Date.now() - start}ms`);
          return;
        }
      } catch {
        // まだ起動していない
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`Mock server not ready after ${timeout}ms (URL: ${url ?? '標準出力にまだ出ていない'})`);
}

/**
 * モックサーバーの状態をリセットする
 */
export async function resetMockServer(): Promise<void> {
  log.debug('Resetting mock server state...');
  await fetch(`${mockServerUrl()}/reset`, { method: 'POST' });
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
  /** モデレーターのバッジを付ける */
  is_moderator?: boolean;
  amount?: string;
  tier?: string;
  milestone_months?: number;
  gift_count?: number;
  gift_image_url?: string;
  /** 同じ id を再び積むと YouTube の再送を再現できる。省略時はモックが採番する */
  id?: string;
}): Promise<void> {
  await fetch(`${mockServerUrl()}/add_message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(message),
  });
}

/**
 * モックサーバーにライブリアクションを 1 回分積む（次のポーリング応答に載る）
 * durationSeconds が 2 以上なら、残りは 0 件のバケットになる
 */
export async function addMockReaction(counts: Record<string, number>, durationSeconds = 1): Promise<void> {
  await fetch(`${mockServerUrl()}/add_reaction`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ counts, duration_seconds: durationSeconds }),
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
  await connection.page.locator(APP_READY).waitFor({ state: 'visible', timeout: 30000 });

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
    ['stopStaticServer', async () => {
      const server = staticFrontendServer;
      staticFrontendServer = null;
      await server?.close();
    }],
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

/** Svelte の描画が終わったら出る、上の行の画面のタブ (アプリの準備ができた目印) */
export const APP_READY = 'nav[aria-label="画面"]';

/** 上の行の接続の札 (02_chat.md「上の行と操作パネル」)。title は配信タイトル */
export function connectionChips(page: Page, streamTitle?: string): Locator {
  return page.locator(streamTitle ? `[data-testid="connection-chip"][title="${streamTitle}"]` : '[data-testid="connection-chip"]');
}

/** 操作パネルを開く (開いていればそのまま) */
export async function openControlPanel(page: Page): Promise<void> {
  const open = page.getByRole('button', { name: '操作パネルを開く' });
  if (await open.isVisible().catch(() => false)) await open.click();
  await expect(page.getByRole('region', { name: '操作パネル' })).toBeVisible({ timeout: 5000 });
}

/** 操作パネルを閉じる (閉じていればそのまま) */
export async function closeControlPanel(page: Page): Promise<void> {
  const close = page.getByRole('button', { name: '操作パネルを閉じる' });
  if (await close.isVisible().catch(() => false)) await close.click();
}

/**
 * 配信の URL の入力欄を出して返す。未接続でコメントも無ければ流れの場所に出ている。
 * 出ていなければ上の行の「+」で操作パネルを開く
 */
export async function openUrlInput(page: Page): Promise<Locator> {
  const input = page.locator('#connect-url');
  if (!(await input.isVisible().catch(() => false))) {
    await page.getByRole('button', { name: '配信に接続' }).click();
  }
  await expect(input).toBeVisible({ timeout: 5000 });
  return input;
}

/**
 * 「すべてのコメントを取る」のチェック (チャットモード「全て」)。配信に接続する欄にあるので、
 * 欄が出ていなければ「+」で操作パネルを開いて返す
 */
export async function chatModeCheckbox(page: Page): Promise<Locator> {
  await openUrlInput(page);
  return page.getByRole('checkbox', { name: /すべてのコメントを取る/ });
}

/** 「接続」のボタン */
export function connectButton(page: Page): Locator {
  return page.getByRole('button', { name: '接続', exact: true });
}

/**
 * 全接続を切断し、蓄積されたメッセージを消してアプリをアイドル状態に戻す。
 * 切断は上の行の札の ×、コメントを消すのは操作パネルの「コメントを消す」。最後に操作パネルを閉じる
 */
export async function disconnectAndInitialize(page: Page): Promise<void> {
  // Step 1: 全接続を切断 (切断できるものから順に)
  const chips = connectionChips(page);
  for (let i = 0; i < 40 && (await chips.count()) > 0; i++) {
    const enabled = chips.locator('button:enabled').first();
    if (await enabled.isVisible().catch(() => false)) {
      await enabled.click();
    }
    await page.waitForTimeout(200);
  }
  await expect(chips).toHaveCount(0, { timeout: 10000 });

  // Step 2: 蓄積メッセージを消す（テスト間の状態分離のため）
  await openControlPanel(page);
  const clearButton = page.getByRole('button', { name: 'コメントを消す' });
  if (await clearButton.isEnabled({ timeout: 1000 }).catch(() => false)) {
    await clearButton.click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible({ timeout: 3000 });
    await dialog.getByRole('button', { name: '消す', exact: true }).click();
    await expect(dialog).not.toBeVisible({ timeout: 3000 });
  }
  await closeControlPanel(page);
}

/**
 * モックサーバーのストリームに接続し、上の行に札が出るのを待つ。
 * @param expectedTitle - 接続確認に使うストリームタイトル（省略時は "Mock Live"）
 */
export async function connectToMockStream(page: Page, videoId = 'test_video_123', expectedTitle = 'Mock Live'): Promise<void> {
  const urlInput = await openUrlInput(page);
  await urlInput.fill(`${mockServerUrl()}/watch?v=${videoId}`);
  await connectButton(page).click();
  await expect(connectionChips(page, expectedTitle).first()).toBeVisible({ timeout: 10000 });
}

/** 上の行の画面のタブの名前 (読み上げ用の名前) */
const TAB_LABELS: Record<string, string> = { Chat: 'チャット', Viewers: '視聴者', Settings: '設定' };

/**
 * 上の行の画面のタブで指定タブに遷移する (Chat / Viewers / Settings)
 */
export async function navigateToTab(page: Page, tabName: string): Promise<void> {
  await page.locator(`nav[aria-label="画面"] button[aria-label="${TAB_LABELS[tabName] ?? tabName}"]`).click();
}

/** 流れ・流れたものの欄 (スパチャ等の一覧の詳細を含む) の中で、text を含むコメントの行 (いちばん上のもの) */
export function chatRow(page: Page, text: string): Locator {
  return page.locator('[data-message-id]').filter({ hasText: text }).first();
}

/** 視聴者情報パネル (押した行のすぐ下に広がる。02_chat.md「視聴者の読み仮名・メモ」) */
export function viewerPanel(page: Page): Locator {
  return page.getByRole('region', { name: '視聴者情報' });
}

/** text を含むコメントを押して視聴者情報パネルを開き、読み込みが終わる (読み仮名の欄が押せる) まで待つ */
export async function openViewerPanel(page: Page, text: string): Promise<Locator> {
  await chatRow(page, text).click();
  await expect(viewerPanel(page).getByLabel('読み仮名')).toBeEnabled({ timeout: 5000 });
  return viewerPanel(page);
}

/** 視聴者のタブの配信者の選択 (06_viewer.md「視聴者のタブ」) */
export function viewerTabBroadcasterSelect(page: Page): Locator {
  return page.getByRole('combobox', { name: '配信者' });
}

/** 視聴者のタブの一覧で、name を含む 1 人の行 (押すと下に編集が広がる。広がった編集もこの行の中) */
export function viewerTabRow(page: Page, name: string): Locator {
  return page.getByRole('listitem').filter({ has: page.getByRole('button', { name }) }).first();
}

/** 開いている視聴者情報パネルで読み仮名を入れ、Enter で保存する (保存すると閉じる) */
export async function saveReadingInPanel(page: Page, reading: string): Promise<void> {
  const input = viewerPanel(page).getByLabel('読み仮名');
  await input.fill(reading);
  await input.press('Enter');
  await expect(viewerPanel(page)).toHaveCount(0, { timeout: 5000 });
}

/**
 * モックサーバーのストリーム状態を設定する
 */
export async function setStreamState(state: {
  member_only?: boolean;
  require_auth?: boolean;
  title?: string;
  chat_delay_ms?: number;
  chat_fail?: boolean;
  /** 応答の timeoutMs（既定 1000。liscov の下限 1.5 秒より短いので間隔は延びない） */
  chat_timeout_ms?: number;
  /** 次に接続する配信の配信者チャンネル ID（空文字で既定に戻す）。接続ごとに配信者を分けるときに使う */
  channel_id?: string;
}): Promise<void> {
  await fetch(`${mockServerUrl()}/set_stream_state`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(state),
  });
}

/**
 * モックサーバーが起動してから受けたチャット取得（get_live_chat）の回数。reset では戻らない
 */
export async function mockChatRequestCount(): Promise<number> {
  const response = await fetch(`${mockServerUrl()}/status`);
  const status = (await response.json()) as { request_count: number };
  return status.request_count;
}

/** モックの通知のサーバー（signaler）の状態（02_chat.md「プッシュ通知」） */
export type MockSignalerStatus = {
  readonly choose_server_count: number;
  readonly refresh_count: number;
  /** いま開いている受信の数（liscov が受信を閉じると減る） */
  readonly open_receivers: number;
  /** 最後の get_live_chat に付いていた invalidationPayloadLastPublishAtUsec */
  readonly last_publish_at: string | null;
};

export async function mockSignalerStatus(): Promise<MockSignalerStatus> {
  const response = await fetch(`${mockServerUrl()}/signaler_status`);
  return (await response.json()) as MockSignalerStatus;
}

/** モックの通知のサーバーから、開いている受信すべてにチャットの更新の通知を送る。発行時刻 (µs) を返す */
export async function sendMockPushNotification(): Promise<string> {
  const response = await fetch(`${mockServerUrl()}/signaler_notify`, { method: 'POST' });
  return ((await response.json()) as { publish_at: string }).publish_at;
}

/**
 * テスト用に起動したアプリだけを強制終了する（終了処理を走らせない。クラッシュ・強制終了の再現用）
 *
 * 自分が起動したプロセスの PID だけを止める。名前で止めると本番のアプリまで止めてしまう。
 */
export async function forceKillTauriApp(): Promise<void> {
  ensureWorktreeLock();
  const cdpPort = cdpEndpoint?.port;
  if (tauriProcess?.pid) {
    if (process.platform === 'win32') {
      try {
        execSync(`taskkill /F /T /PID ${tauriProcess.pid} 2>nul`, { stdio: 'ignore' });
      } catch { /* 既に終了していた場合は無視 */ }
    } else {
      tauriProcess.kill('SIGKILL');
    }
    tauriProcess = null;
  }
  if (cdpPort) await waitForPortFree(cdpPort, 10000);
  cdpEndpoint = null;
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
