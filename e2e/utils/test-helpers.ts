/**
 * E2Eテスト共通ヘルパー関数
 */

import { chromium, BrowserContext, Page, Browser, expect } from '@playwright/test';
import { execSync, spawn, spawnSync, ChildProcess, SpawnOptionsWithoutStdio } from 'child_process';
import * as fs from 'fs';
import * as http from 'http';
import * as path from 'path';
import * as os from 'os';
import { log } from './logger';

export const CDP_URL = 'http://127.0.0.1:9222';
export const MOCK_SERVER_URL = 'http://127.0.0.1:3456';
export const PROJECT_DIR = process.cwd().replace(/[\\/]e2e$/, '');

// Test isolation: use separate namespace for credentials and data
export const TEST_APP_NAME = 'liscov-test';
export const TEST_KEYRING_SERVICE = 'liscov-test';

// Mock server process reference
let mockServerProcess: ChildProcess | null = null;

// Tauri app process reference
let tauriProcess: ChildProcess | null = null;
let staticFrontendServer: http.Server | null = null;

const WINDOWS_DEFAULT_PATHEXT = '.COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC';
const WINDOWS_VSDEVCMD_CANDIDATES = [
  'C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\Common7\\Tools\\VsDevCmd.bat',
  'C:\\Program Files\\Microsoft Visual Studio\\2022\\BuildTools\\Common7\\Tools\\VsDevCmd.bat',
  'C:\\Program Files\\Microsoft Visual Studio\\2022\\Professional\\Common7\\Tools\\VsDevCmd.bat',
  'C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\Common7\\Tools\\VsDevCmd.bat',
];
const WINDOWS_TEMP_DIR = path.join(PROJECT_DIR, '.tmp', 'e2e', 'temp');
const PREBUILT_TAURI_APP_PATH = path.join(PROJECT_DIR, 'src-tauri', 'target', 'debug', 'liscov-tauri.exe');
const PREBUILT_MOCK_SERVER_PATH = path.join(PROJECT_DIR, 'src-tauri', 'target', 'debug', 'mock_server.exe');
const PREBUILT_FRONTEND_INDEX_PATH = path.join(PROJECT_DIR, 'build', 'index.html');
const PREBUILT_FRONTEND_DIR = path.join(PROJECT_DIR, 'build');
const PREBUILT_FRONTEND_PORT = 5173;

let cachedWindowsShellEnv: NodeJS.ProcessEnv | null = null;
let cachedVsDevCmdPath: string | null | undefined;

function ensureDir(dir: string): void {
  fs.mkdirSync(dir, { recursive: true });
}

function getWindowsHomeDir(): string {
  return process.env.USERPROFILE ?? process.env.HOME ?? os.homedir();
}

function getWindowsPathParts(fullPath: string): { drive: string; relativePath: string } {
  const parsed = path.parse(fullPath);
  const drive = process.env.HOMEDRIVE ?? parsed.root.replace(/[\\\/]+$/, '');
  const relativePath = process.env.HOMEPATH ?? fullPath.slice(drive.length).replace(/\//g, '\\');
  return {
    drive,
    relativePath: relativePath || '\\',
  };
}

function getWindowsBaseEnv(): NodeJS.ProcessEnv {
  const homeDir = getWindowsHomeDir();
  const { drive, relativePath } = getWindowsPathParts(homeDir);
  const appData = process.env.APPDATA ?? path.join(homeDir, 'AppData', 'Roaming');
  const localAppData = process.env.LOCALAPPDATA ?? path.join(homeDir, 'AppData', 'Local');
  const systemRoot = process.env.SystemRoot ?? 'C:\\Windows';

  ensureDir(WINDOWS_TEMP_DIR);

  return {
    ALLUSERSPROFILE: process.env.ALLUSERSPROFILE ?? 'C:\\ProgramData',
    APPDATA: appData,
    COMSPEC: process.env.COMSPEC ?? path.join(systemRoot, 'System32', 'cmd.exe'),
    HOME: process.env.HOME ?? homeDir,
    HOMEDRIVE: drive,
    HOMEPATH: relativePath,
    LOCALAPPDATA: localAppData,
    PATHEXT: process.env.PATHEXT ?? WINDOWS_DEFAULT_PATHEXT,
    ProgramData: process.env.ProgramData ?? 'C:\\ProgramData',
    SystemRoot: systemRoot,
    TEMP: process.env.TEMP ?? WINDOWS_TEMP_DIR,
    TMP: process.env.TMP ?? WINDOWS_TEMP_DIR,
    USERPROFILE: homeDir,
    WINDIR: process.env.WINDIR ?? systemRoot,
  };
}

function applyWindowsBaseEnvToProcess(): void {
  if (process.platform !== 'win32') {
    return;
  }

  for (const [key, value] of Object.entries(getWindowsBaseEnv())) {
    if (value && !process.env[key]) {
      process.env[key] = value;
    }
  }
}

applyWindowsBaseEnvToProcess();

function resolveVsDevCmdPath(): string | null {
  if (cachedVsDevCmdPath !== undefined) {
    return cachedVsDevCmdPath;
  }

  cachedVsDevCmdPath = WINDOWS_VSDEVCMD_CANDIDATES.find((candidate) => fs.existsSync(candidate)) ?? null;
  return cachedVsDevCmdPath;
}

function parseWindowsShellEnv(output: string): NodeJS.ProcessEnv {
  return output
    .split(/\r?\n/)
    .filter((line) => line.includes('='))
    .reduce<NodeJS.ProcessEnv>((env, line) => {
      const separatorIndex = line.indexOf('=');
      const key = line.slice(0, separatorIndex).trim();
      const value = line.slice(separatorIndex + 1);
      if (key.length > 0) {
        env[key] = value;
      }
      return env;
    }, {});
}

function getWindowsShellEnv(): NodeJS.ProcessEnv {
  if (process.platform !== 'win32') {
    return {};
  }

  if (cachedWindowsShellEnv) {
    return cachedWindowsShellEnv;
  }

  const baseEnv = {
    ...process.env,
    ...getWindowsBaseEnv(),
    VSCMD_SKIP_SENDTELEMETRY: '1',
  };
  const vsDevCmdPath = resolveVsDevCmdPath();

  if (!vsDevCmdPath) {
    log.warn('VsDevCmd.bat が見つからないため、現在の環境変数のみで E2E を実行します');
    cachedWindowsShellEnv = baseEnv;
    return cachedWindowsShellEnv;
  }

  const vsEnvScriptPath = path.join(PROJECT_DIR, '.tmp', 'e2e', 'load-vsdevcmd-env.bat');
  ensureDir(path.dirname(vsEnvScriptPath));
  fs.writeFileSync(
    vsEnvScriptPath,
    [
      '@echo off',
      `call "${vsDevCmdPath}" -no_logo -arch=amd64`,
      'if errorlevel 1 exit /b %errorlevel%',
      'set',
      '',
    ].join('\r\n'),
    'utf-8'
  );

  const result = spawnSync(
    baseEnv.COMSPEC!,
    ['/d', '/c', vsEnvScriptPath],
    {
      cwd: PROJECT_DIR,
      encoding: 'utf-8',
      env: baseEnv,
      windowsHide: true,
      maxBuffer: 10 * 1024 * 1024,
    }
  );

  if (result.status !== 0) {
    log.warn(`VsDevCmd.bat の読み込みに失敗したため、現在の環境変数のみで続行します (status=${result.status ?? 'unknown'})`);
    cachedWindowsShellEnv = baseEnv;
    return cachedWindowsShellEnv;
  }

  cachedWindowsShellEnv = {
    ...baseEnv,
    ...parseWindowsShellEnv(result.stdout),
  };

  return cachedWindowsShellEnv;
}

function getTestProcessEnv(extraEnv: NodeJS.ProcessEnv = {}): NodeJS.ProcessEnv {
  return process.platform === 'win32'
    ? {
        ...getWindowsShellEnv(),
        ...extraEnv,
      }
    : {
        ...process.env,
        ...extraEnv,
    };
}

function spawnCommand(
  executable: string,
  args: string[],
  options: SpawnOptionsWithoutStdio
): ChildProcess {
  return spawn(executable, args, {
    ...options,
    env: getTestProcessEnv(options.env),
    shell: false,
    windowsHide: process.platform === 'win32',
  });
}

function spawnProjectCommand(
  command: 'cargo' | 'pnpm',
  args: string[],
  options: SpawnOptionsWithoutStdio
): ChildProcess {
  const executable =
    process.platform === 'win32'
      ? command === 'cargo'
        ? 'cargo.exe'
        : 'pnpm.cmd'
      : command;

  return spawnCommand(executable, args, options);
}

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

function resolveStaticFrontendFile(requestUrl?: string): string {
  const requestPath = decodeURIComponent(new URL(requestUrl ?? '/', 'http://127.0.0.1').pathname);
  const relativePath = requestPath === '/' ? 'index.html' : requestPath.replace(/^\/+/, '');
  const resolvedPath = path.resolve(PREBUILT_FRONTEND_DIR, relativePath);

  if (resolvedPath.startsWith(path.resolve(PREBUILT_FRONTEND_DIR)) && fs.existsSync(resolvedPath) && fs.statSync(resolvedPath).isFile()) {
    return resolvedPath;
  }

  return PREBUILT_FRONTEND_INDEX_PATH;
}

async function ensureStaticFrontendServer(): Promise<void> {
  if (staticFrontendServer || !fs.existsSync(PREBUILT_FRONTEND_INDEX_PATH)) {
    return;
  }

  staticFrontendServer = http.createServer((request, response) => {
    try {
      const filePath = resolveStaticFrontendFile(request.url);
      const body = fs.readFileSync(filePath);
      response.writeHead(200, { 'Content-Type': getStaticContentType(filePath) });
      response.end(body);
    } catch (error) {
      response.writeHead(500, { 'Content-Type': 'text/plain; charset=utf-8' });
      response.end(error instanceof Error ? error.message : String(error));
    }
  });

  await new Promise<void>((resolve, reject) => {
    staticFrontendServer!.once('error', (error) => {
      if ((error as NodeJS.ErrnoException).code === 'EADDRINUSE') {
        log.debug(`Static frontend server already running on port ${PREBUILT_FRONTEND_PORT}`);
        staticFrontendServer = null;
        resolve();
        return;
      }

      reject(error);
    });

    staticFrontendServer!.listen(PREBUILT_FRONTEND_PORT, () => {
      log.debug(`Static frontend server ready on port ${PREBUILT_FRONTEND_PORT}`);
      resolve();
    });
  });
}

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

export function getPlatformConfigDir(): string {
  if (process.platform === 'win32') {
    return getTestProcessEnv().APPDATA ?? path.join(getWindowsHomeDir(), 'AppData', 'Roaming');
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
 * Get test data directories based on platform
 */
export function getTestDataDirs(): string[] {
  const dirs: string[] = [];

  const configDir =
    process.platform === 'win32'
      ? getPlatformConfigDir()
      : process.platform === 'darwin'
        ? path.join(os.homedir(), 'Library', 'Application Support')
        : path.join(os.homedir(), '.config');

  if (configDir) {
    dirs.push(path.join(configDir, TEST_APP_NAME));
  }

  const dataDir =
    process.platform === 'win32'
      ? getPlatformConfigDir()
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
 * Wait for CDP to be available
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

  log.info('Connected to Tauri app');
  return { browser, context, page: pages[0] };
}

/**
 * Kill Tauri app
 */
export async function killTauriApp(): Promise<void> {
  log.debug('Killing Tauri app...');
  if (tauriProcess) {
    tauriProcess.kill();
    tauriProcess = null;
  }
  // Also kill any orphaned processes as fallback
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /F /IM liscov-tauri.exe 2>nul', { stdio: 'ignore' });
    } else {
      execSync('pkill -f liscov-tauri', { stdio: 'ignore' });
    }
  } catch {
    // Process may not exist
  }
  await new Promise((resolve) => setTimeout(resolve, 1000));
}

/**
 * Start Tauri app with test isolation
 */
export async function startTauriApp(): Promise<void> {
  await startTauriAppWithEnv({
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  });
}

export async function startTauriAppWithEnv(extraEnv: NodeJS.ProcessEnv): Promise<void> {
  const env = getTestProcessEnv({
    ...extraEnv,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  });

  log.info(`Starting Tauri app with test namespace: ${TEST_APP_NAME}`);

  const tauriTail = buildProcessTailRecorder();

  if (process.platform === 'win32' && fs.existsSync(PREBUILT_TAURI_APP_PATH) && fs.existsSync(PREBUILT_FRONTEND_INDEX_PATH)) {
    await ensureStaticFrontendServer();
    log.debug(`Using prebuilt Tauri app binary: ${PREBUILT_TAURI_APP_PATH}`);
    tauriProcess = spawnCommand(PREBUILT_TAURI_APP_PATH, [], {
      cwd: PROJECT_DIR,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } else {
    tauriProcess = spawnProjectCommand('pnpm', ['tauri', 'dev'], {
      cwd: PROJECT_DIR,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  }

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
 * Kill mock server process
 */
export async function killMockServer(): Promise<void> {
  if (mockServerProcess) {
    log.debug('Stopping mock server...');
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
  await new Promise((resolve) => setTimeout(resolve, 500));
}

/**
 * Start mock server
 */
export async function startMockServer(): Promise<void> {
  log.info('Starting mock server...');

  // Kill any existing mock server first
  await killMockServer();

  // Start mock server as a child process
  const cargoPath = path.join(PROJECT_DIR, 'src-tauri', 'Cargo.toml');
  const mockTail = buildProcessTailRecorder();

  if (process.platform === 'win32' && fs.existsSync(PREBUILT_MOCK_SERVER_PATH)) {
    log.debug(`Using prebuilt mock server binary: ${PREBUILT_MOCK_SERVER_PATH}`);
    mockServerProcess = spawnCommand(PREBUILT_MOCK_SERVER_PATH, [], {
      cwd: PROJECT_DIR,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } else {
    mockServerProcess = spawnProjectCommand('cargo', ['run', '--manifest-path', cargoPath, '--bin', 'mock_server'], {
      cwd: PROJECT_DIR,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  }

  const mockLog = log.child('mock_server');

  // Log mock server output for debugging
  mockServerProcess.stdout?.on('data', (data) => {
    const msg = data.toString().trim();
    mockTail.push(msg);
    if (msg) mockLog.debug(msg);
  });
  mockServerProcess.stderr?.on('data', (data) => {
    const msg = data.toString().trim();
    mockTail.push(msg);
    // Filter out cargo build warnings/info
    if (msg && !msg.includes('Compiling') && !msg.includes('Finished') && !msg.includes('warning:')) {
      mockLog.debug(msg);
    }
  });

  // Wait for mock server to be ready
  const timeout = 60000;
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const exitInfo = describeExitedProcess('Mock server', mockServerProcess, mockTail.lines);
    if (exitInfo) {
      throw new Error(`Mock server failed to start because ${exitInfo}`);
    }

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

  // Step 6: Connect to the running Tauri app
  const connection = await connectToApp();

  // Wait for Svelte app to fully mount (not just HTML load)
  await connection.page.waitForLoadState('load');
  // Wait for a known UI element that only exists after Svelte renders
  await connection.page.getByRole('heading', { name: 'Chat Monitor' }).waitFor({ state: 'visible', timeout: 30000 });

  return connection;
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
