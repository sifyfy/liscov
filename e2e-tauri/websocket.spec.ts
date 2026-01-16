import { test, expect, chromium, BrowserContext, Page, Browser } from '@playwright/test';
import { exec, execSync, spawn, ChildProcess } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { WebSocket } from 'ws';

const execAsync = promisify(exec);

/**
 * E2E tests for WebSocket API based on 03_websocket.md specification.
 *
 * Tests verify:
 * - Server start/stop and port selection (8765-8774 range)
 * - Tauri events: websocket-client-connected, websocket-client-disconnected
 * - Connected clients count
 * - Complete data flow: YouTube (mock) → Tauri App → WebSocket API → External Client
 * - Message format verification per spec
 */

const CDP_URL = 'http://127.0.0.1:9222';
const MOCK_SERVER_URL = 'http://localhost:3456';
const PROJECT_DIR = process.cwd().replace(/[\\/]e2e-tauri$/, '');

// Test isolation: use separate namespace for credentials and data
const TEST_APP_NAME = 'liscov-test';
const TEST_KEYRING_SERVICE = 'liscov-test';

// Helper to get the actual WebSocket port from the UI
async function getWebSocketPort(page: Page): Promise<number> {
  // Port is displayed in side panel: "Running on port XXXX"
  const portText = await page.locator('text=/Running on port \\d+/').textContent();
  const match = portText?.match(/Running on port (\d+)/);
  if (!match) {
    throw new Error('Could not find WebSocket port in UI');
  }
  return parseInt(match[1], 10);
}

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
      execSync(`cmdkey /delete:${TEST_KEYRING_SERVICE}:youtube_credentials 2>nul`, { stdio: 'ignore' });
      console.log('Cleaned up test credentials from Windows Credential Manager');
    } catch {
      // Credential may not exist, which is fine
    }
  }
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

// Helper to start Tauri app with test isolation and mock server
async function startTauriApp(): Promise<void> {
  const env = {
    ...process.env,
    // Test isolation: use separate namespace
    LISCOV_APP_NAME: TEST_APP_NAME,
    LISCOV_KEYRING_SERVICE: TEST_KEYRING_SERVICE,
    // Mock server URLs - point all YouTube APIs to mock server
    LISCOV_AUTH_URL: `${MOCK_SERVER_URL}/?auto_login=true`,
    LISCOV_SESSION_CHECK_URL: `${MOCK_SERVER_URL}/youtubei/v1/account/account_menu`,
    LISCOV_YOUTUBE_BASE_URL: MOCK_SERVER_URL,
    // Enable CDP for Playwright
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
  };

  console.log(`Starting Tauri app with test namespace: ${TEST_APP_NAME}`);

  // Start app in background
  exec(`cd "${PROJECT_DIR}" && pnpm tauri dev`, { env });

  // Wait for CDP to be available
  await waitForCDP();
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

// Helper to add message to mock server
async function addMockMessage(message: {
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

// Helper to connect WebSocket client and wait for connection
function connectWebSocket(url: string, timeout = 5000): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    console.log(`Connecting to WebSocket at ${url}...`);
    const ws = new WebSocket(url);
    const timer = setTimeout(() => {
      console.log(`WebSocket connection timeout to ${url}`);
      ws.close();
      reject(new Error(`WebSocket connection timeout after ${timeout}ms`));
    }, timeout);

    ws.on('open', () => {
      console.log(`WebSocket connected to ${url}`);
      clearTimeout(timer);
      resolve(ws);
    });

    ws.on('error', (err) => {
      console.log(`WebSocket error connecting to ${url}:`, err);
      clearTimeout(timer);
      reject(err);
    });

    ws.on('close', (code, reason) => {
      console.log(`WebSocket closed: code=${code}, reason=${reason.toString()}`);
    });
  });
}

// Helper to wait for WebSocket message
function waitForMessage(ws: WebSocket, timeout = 10000): Promise<unknown> {
  return new Promise((resolve, reject) => {
    console.log('Waiting for WebSocket message...');
    const timer = setTimeout(() => {
      console.log('WebSocket message wait timed out');
      reject(new Error(`WebSocket message timeout after ${timeout}ms`));
    }, timeout);

    ws.once('message', (data) => {
      clearTimeout(timer);
      try {
        const parsed = JSON.parse(data.toString());
        console.log('Received WebSocket message:', parsed.type || 'unknown');
        resolve(parsed);
      } catch (e) {
        reject(e);
      }
    });
  });
}

// Helper to collect WebSocket messages for a duration
function collectMessages(ws: WebSocket, duration: number): Promise<unknown[]> {
  return new Promise((resolve) => {
    const messages: unknown[] = [];
    const handler = (data: Buffer) => {
      try {
        messages.push(JSON.parse(data.toString()));
      } catch {
        // Ignore non-JSON messages
      }
    };

    ws.on('message', handler);

    setTimeout(() => {
      ws.off('message', handler);
      resolve(messages);
    }, duration);
  });
}

test.describe('WebSocket API (03_websocket.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000); // 5 minutes for setup

    // Step 1: Kill any existing processes
    console.log('Killing any existing Tauri app...');
    await killTauriApp();

    // Step 2: Clean up test data and credentials for a fresh start
    console.log('Cleaning up test data and credentials...');
    await cleanupTestData();
    await cleanupTestCredentials();

    // Step 3: Start mock server
    await startMockServer();

    // Step 4: Reset mock server state
    console.log('Resetting mock server state...');
    await resetMockServer();

    // Step 5: Start Tauri app with test namespace
    console.log('Starting Tauri app with test namespace...');
    await startTauriApp();

    // Step 6: Connect to the running Tauri app
    const connection = await connectToApp();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
    console.log('Connected to Tauri app:', await mainPage.title());
  });

  test.afterAll(async () => {
    // Kill the Tauri app
    await killTauriApp();
    // Stop mock server
    await killMockServer();
    // Clean up test data
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test.beforeEach(async () => {
    // Reset mock server before each test
    await resetMockServer();

    // Ensure clean app state before each test
    // Stop WebSocket server if running
    const stopButton = mainPage.getByRole('button', { name: 'Stop Server' });
    if (await stopButton.isVisible({ timeout: 1000 }).catch(() => false)) {
      await stopButton.click();
      await mainPage.waitForTimeout(500);
    }

    // Disconnect from YouTube if connected
    const disconnectButton = mainPage.locator('button:has-text("Disconnect")');
    if (await disconnectButton.isVisible({ timeout: 1000 }).catch(() => false)) {
      await disconnectButton.click();
      await mainPage.waitForTimeout(500);
    }
  });

  test.describe('Server Start/Stop', () => {
    test('should show Start Server button initially', async () => {
      // Spec: 起動/停止ボタン - サーバーの起動・停止を切り替え
      // WebSocket controls are on Chat tab side panel
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await expect(startButton).toBeVisible();
    });

    test('should start server and show Stop Server button', async () => {
      // Spec: 「Start Server」クリック → WebSocketサーバー起動、ポート番号表示
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();

      // Stop button should appear
      const stopButton = mainPage.getByRole('button', { name: 'Stop Server' });
      await expect(stopButton).toBeVisible({ timeout: 5000 });

      // Cleanup
      await stopButton.click();
      await expect(startButton).toBeVisible({ timeout: 5000 });
    });

    test('should display actual port number after starting', async () => {
      // Spec: ポート番号表示 - 実際に使用中のポート
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Should display port in range 8765-8774
      const port = await getWebSocketPort(mainPage);
      expect(port).toBeGreaterThanOrEqual(8765);
      expect(port).toBeLessThanOrEqual(8774);

      // Cleanup
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
    });

    test('should stop server and show Start Server button', async () => {
      // Spec: 「Stop Server」クリック → サーバー停止、ボタンが「Start Server」に戻る
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      const stopButton = mainPage.getByRole('button', { name: 'Stop Server' });
      await stopButton.click();

      await expect(startButton).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Port Range (8765-8774)', () => {
    test('should use port within valid range', async () => {
      // Spec: ポート範囲: 8765 〜 8774
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Verify port is within valid range
      const port = await getWebSocketPort(mainPage);
      expect(port).toBeGreaterThanOrEqual(8765);
      expect(port).toBeLessThanOrEqual(8774);

      // Cleanup
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
    });

    test('should fallback to next port when preferred port is in use', async () => {
      // Spec: ポート範囲: 8765 〜 8774（自動フォールバック用）
      const { createServer } = await import('net');

      // First, start WebSocket server to see which port it uses
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });
      const firstPort = await getWebSocketPort(mainPage);
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await expect(startButton).toBeVisible({ timeout: 5000 });

      // Wait for port to be released
      await mainPage.waitForTimeout(500);

      // Block the port that was used
      const blockingServer = createServer();
      await new Promise<void>((resolve, reject) => {
        blockingServer.on('error', reject);
        blockingServer.listen(firstPort, '127.0.0.1', () => resolve());
      });

      try {
        // Start WebSocket server again - should fallback to next port
        await startButton.click();
        await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

        const secondPort = await getWebSocketPort(mainPage);
        // Should use a different port (next available)
        expect(secondPort).not.toBe(firstPort);
        expect(secondPort).toBeGreaterThanOrEqual(8765);
        expect(secondPort).toBeLessThanOrEqual(8774);

        // Cleanup
        await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      } finally {
        blockingServer.close();
      }
    });

    test('should fail when all ports (8765-8774) are in use', async () => {
      // Spec: 全ポート使用中 → エラーを返却、サーバー起動失敗
      const { createServer } = await import('net');
      const servers: ReturnType<typeof createServer>[] = [];

      // Occupy all ports 8765-8774
      for (let port = 8765; port <= 8774; port++) {
        const server = createServer();
        try {
          await new Promise<void>((resolve, reject) => {
            server.on('error', reject);
            server.listen(port, '127.0.0.1', () => resolve());
          });
          servers.push(server);
        } catch {
          // Port may already be in use, skip it
        }
      }

      try {
        // Try to start WebSocket server - should fail
        const startButton = mainPage.getByRole('button', { name: 'Start Server' });
        await startButton.click();

        // Wait for UI to respond
        await mainPage.waitForTimeout(2000);

        // Check for error message or verify server didn't start
        const errorVisible = await mainPage.getByText(/error|failed|no.*port/i).isVisible().catch(() => false);
        const startButtonStillVisible = await startButton.isVisible().catch(() => false);

        // Either an error is shown, or the start button is still visible
        expect(errorVisible || startButtonStillVisible).toBe(true);
      } finally {
        // Clean up all blocking servers
        for (const server of servers) {
          server.close();
        }
      }
    });
  });

  test.describe('Tauri Events', () => {
    test('should update UI when client connects (websocket-client-connected event)', async () => {
      // Spec: websocket-client-connected | { client_id: u64 } | クライアント接続時
      // This test verifies that the Tauri event is emitted by observing UI updates

      // Start server
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      const port = await getWebSocketPort(mainPage);

      // Verify initial state: 0 clients
      await expect(mainPage.getByText('0 clients connected')).toBeVisible({ timeout: 5000 });

      // Connect a WebSocket client
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws); // Wait for Connected message

      // Verify UI updated via Tauri event: 1 client
      await expect(mainPage.getByText('1 clients connected')).toBeVisible({ timeout: 5000 });

      ws.close();

      // Clean up - stop server
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Start Server' })).toBeVisible({ timeout: 5000 });
    });

    test('should update UI when client disconnects (websocket-client-disconnected event)', async () => {
      // Spec: websocket-client-disconnected | { client_id: u64 } | クライアント切断時
      // This test verifies that the Tauri event is emitted by observing UI updates

      // Start server
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      const port = await getWebSocketPort(mainPage);

      // Connect a WebSocket client
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws); // Wait for Connected message

      // Verify 1 client connected
      await expect(mainPage.getByText('1 clients connected')).toBeVisible({ timeout: 5000 });

      // Close the WebSocket connection
      ws.close();

      // Verify UI updated via Tauri event: 0 clients
      await expect(mainPage.getByText('0 clients connected')).toBeVisible({ timeout: 5000 });

      // Clean up - stop server
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Start Server' })).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Connected Clients Count', () => {
    test('should display connected clients count', async () => {
      // Spec: 接続数表示 - 現在接続中のクライアント数

      // Start server
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      const port = await getWebSocketPort(mainPage);

      // Initially 0 clients - UI shows "0 clients connected"
      await expect(mainPage.getByText('0 clients connected')).toBeVisible({ timeout: 5000 });

      // Connect a client
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws); // Wait for Connected message

      // Wait for UI to update
      await expect(mainPage.getByText('1 clients connected')).toBeVisible({ timeout: 5000 });

      // Connect another client
      const ws2 = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws2);

      // Should show 2 clients
      await expect(mainPage.getByText('2 clients connected')).toBeVisible({ timeout: 5000 });

      // Disconnect one
      ws.close();
      await expect(mainPage.getByText('1 clients connected')).toBeVisible({ timeout: 5000 });

      // Disconnect all
      ws2.close();
      await expect(mainPage.getByText('0 clients connected')).toBeVisible({ timeout: 5000 });

      // Cleanup
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
    });
  });

  test.describe('Connected Message', () => {
    test('should send Connected message with unique client_id on connection', async () => {
      // Start WebSocket server
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      await mainPage.getByRole('button', { name: 'Start Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Wait for server to be fully ready
      await mainPage.waitForTimeout(1000);

      // Get actual port and connect multiple clients to verify unique IDs (staggered to avoid race conditions)
      const port = await getWebSocketPort(mainPage);
      const ws1 = await connectWebSocket(`ws://127.0.0.1:${port}`);
      const msg1 = await waitForMessage(ws1) as { type: string; data: { client_id: number } };

      await mainPage.waitForTimeout(200);
      const ws2 = await connectWebSocket(`ws://127.0.0.1:${port}`);
      const msg2 = await waitForMessage(ws2) as { type: string; data: { client_id: number } };

      // Verify Connected message format
      expect(msg1.type).toBe('Connected');
      expect(msg1.data).toHaveProperty('client_id');
      expect(typeof msg1.data.client_id).toBe('number');

      expect(msg2.type).toBe('Connected');
      expect(msg2.data).toHaveProperty('client_id');
      expect(typeof msg2.data.client_id).toBe('number');

      // Verify unique client IDs
      expect(msg1.data.client_id).not.toBe(msg2.data.client_id);

      // Cleanup
      ws1.close();
      ws2.close();
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
    });
  });

  test.describe('ServerInfo Message', () => {
    test('should respond to GetInfo with correct format', async () => {
      // Start WebSocket server
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      await mainPage.getByRole('button', { name: 'Start Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Get actual port and connect WebSocket client
      const port = await getWebSocketPort(mainPage);
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws); // Connected message

      // Send GetInfo request
      ws.send(JSON.stringify({ type: 'GetInfo' }));

      // Wait for ServerInfo response
      const infoMsg = await waitForMessage(ws) as {
        type: string;
        data: {
          version: string;
          connected_clients: number;
        };
      };

      // Verify ServerInfo format per spec
      expect(infoMsg.type).toBe('ServerInfo');
      expect(infoMsg.data).toHaveProperty('version');
      expect(infoMsg.data).toHaveProperty('connected_clients');
      expect(typeof infoMsg.data.version).toBe('string');
      expect(typeof infoMsg.data.connected_clients).toBe('number');
      expect(infoMsg.data.connected_clients).toBeGreaterThanOrEqual(1);

      // Cleanup
      ws.close();
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
    });
  });

  test.describe('Complete Data Flow: YouTube → App → WebSocket → Client', () => {
    test('should receive text message through WebSocket API', async () => {
      // Step 1: Connect to YouTube stream (via mock)
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("Connect")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Step 2: Start WebSocket server (on Chat tab side panel)
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      const startButton = mainPage.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Step 3: Get actual port and connect WebSocket client
      const port = await getWebSocketPort(mainPage);
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);

      // Step 4: Wait for Connected message
      const connectedMsg = await waitForMessage(ws) as { type: string; data: { client_id: number } };
      expect(connectedMsg.type).toBe('Connected');
      expect(connectedMsg.data.client_id).toBeGreaterThan(0);

      // Step 5: Add message via mock
      await addMockMessage({
        message_type: 'text',
        author: 'TestViewer',
        content: 'Hello WebSocket!',
        channel_id: 'UC_test_viewer',
        is_member: false,
      });

      // Step 6: Collect messages from WebSocket
      const messages = await collectMessages(ws, 5000);

      // Step 7: Verify ChatMessage was received
      const chatMessages = messages.filter((m: unknown) =>
        (m as { type: string }).type === 'ChatMessage'
      );
      expect(chatMessages.length).toBeGreaterThan(0);

      const chatMsg = chatMessages[0] as {
        type: string;
        data: {
          author: string;
          content: string;
          message_type: string;
          runs: Array<{ Text?: { content: string } }>;
        };
      };

      // Verify message structure per spec
      expect(chatMsg.type).toBe('ChatMessage');
      expect(chatMsg.data.author).toBe('TestViewer');
      expect(chatMsg.data.content).toBe('Hello WebSocket!');
      expect(chatMsg.data.message_type).toBe('Text');

      // Verify runs format: [{ "Text": { "content": "..." } }]
      expect(chatMsg.data.runs).toBeInstanceOf(Array);
      expect(chatMsg.data.runs.length).toBeGreaterThan(0);
      expect(chatMsg.data.runs[0]).toHaveProperty('Text');
      expect(chatMsg.data.runs[0].Text).toHaveProperty('content');

      // Cleanup
      ws.close();
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await mainPage.locator('button:has-text("Disconnect")').click();
    });

    test('should receive SuperChat message with correct format', async () => {
      // Connect to YouTube stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("Connect")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Start WebSocket server (on Chat tab side panel)
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      await mainPage.getByRole('button', { name: 'Start Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Get actual port and connect WebSocket client
      const port = await getWebSocketPort(mainPage);
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws); // Connected message

      // Add SuperChat via mock
      await addMockMessage({
        message_type: 'superchat',
        author: 'SuperChatter',
        content: 'Thanks for the stream!',
        amount: '¥1,000',
        tier: 'green',
        is_member: true,
      });

      // Collect messages
      const messages = await collectMessages(ws, 5000);
      const chatMessages = messages.filter((m: unknown) =>
        (m as { type: string }).type === 'ChatMessage'
      );
      expect(chatMessages.length).toBeGreaterThan(0);

      const scMsg = chatMessages[0] as {
        type: string;
        data: {
          author: string;
          message_type: { SuperChat: { amount: string } };
          metadata: {
            amount: string;
            superchat_colors: {
              header_background: string;
              body_background: string;
            } | null;
          };
        };
      };

      // Verify SuperChat message_type format: { "SuperChat": { "amount": "..." } }
      expect(scMsg.data.message_type).toHaveProperty('SuperChat');
      expect(scMsg.data.message_type.SuperChat).toHaveProperty('amount');
      expect(scMsg.data.message_type.SuperChat.amount).toBe('¥1,000');

      // Verify metadata
      expect(scMsg.data.metadata.amount).toBe('¥1,000');
      expect(scMsg.data.metadata.superchat_colors).not.toBeNull();

      // Cleanup
      ws.close();
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await mainPage.locator('button:has-text("Disconnect")').click();
    });

    test('should receive membership message with milestone_months', async () => {
      // Connect to YouTube stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("Connect")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Start WebSocket server (on Chat tab side panel)
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      await mainPage.getByRole('button', { name: 'Start Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Get actual port and connect WebSocket client
      const port = await getWebSocketPort(mainPage);
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws); // Connected message

      // Add membership milestone via mock
      await addMockMessage({
        message_type: 'membership_milestone',
        author: 'LoyalMember',
        content: 'ありがとうございます！',
        milestone_months: 12,
      });

      // Collect messages
      const messages = await collectMessages(ws, 5000);
      const chatMessages = messages.filter((m: unknown) =>
        (m as { type: string }).type === 'ChatMessage'
      );
      expect(chatMessages.length).toBeGreaterThan(0);

      const memberMsg = chatMessages[0] as {
        type: string;
        data: {
          author: string;
          message_type: { Membership: { milestone_months: number | null } };
          is_member: boolean;
        };
      };

      // Verify Membership message_type format
      expect(memberMsg.data.message_type).toHaveProperty('Membership');
      expect(memberMsg.data.message_type.Membership).toHaveProperty('milestone_months');
      expect(memberMsg.data.is_member).toBe(true);

      // Cleanup
      ws.close();
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await mainPage.locator('button:has-text("Disconnect")').click();
    });

    test('should receive membership gift message with gift_count', async () => {
      // Connect to YouTube stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("Connect")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Start WebSocket server (on Chat tab side panel)
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      await mainPage.getByRole('button', { name: 'Start Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Get actual port and connect WebSocket client
      const port = await getWebSocketPort(mainPage);
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws); // Connected message

      // Add membership gift via mock
      await addMockMessage({
        message_type: 'membership_gift',
        author: 'GenerousGifter',
        content: '',
        gift_count: 5,
      });

      // Collect messages
      const messages = await collectMessages(ws, 5000);
      const chatMessages = messages.filter((m: unknown) =>
        (m as { type: string }).type === 'ChatMessage'
      );
      expect(chatMessages.length).toBeGreaterThan(0);

      const giftMsg = chatMessages[0] as {
        type: string;
        data: {
          author: string;
          message_type: { MembershipGift: { gift_count: number } };
        };
      };

      // Verify MembershipGift message_type format
      expect(giftMsg.data.message_type).toHaveProperty('MembershipGift');
      expect(giftMsg.data.message_type.MembershipGift).toHaveProperty('gift_count');
      expect(giftMsg.data.message_type.MembershipGift.gift_count).toBe(5);

      // Cleanup
      ws.close();
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await mainPage.locator('button:has-text("Disconnect")').click();
    });
  });

  test.describe('Message Format Verification (03_websocket.md spec)', () => {
    test('should have correct ChatMessage structure', async () => {
      // Connect to YouTube stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("Connect")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Start WebSocket server (on Chat tab side panel)
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      await mainPage.getByRole('button', { name: 'Start Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Get actual port and connect WebSocket client
      const port = await getWebSocketPort(mainPage);
      const ws = await connectWebSocket(`ws://127.0.0.1:${port}`);
      await waitForMessage(ws); // Connected message

      // Add message with member badge
      await addMockMessage({
        message_type: 'text',
        author: 'StructTestUser',
        content: 'Structure test message',
        channel_id: 'UC_struct_test',
        is_member: true,
      });

      // Collect messages
      const messages = await collectMessages(ws, 5000);
      const chatMessages = messages.filter((m: unknown) =>
        (m as { type: string }).type === 'ChatMessage'
      );
      expect(chatMessages.length).toBeGreaterThan(0);

      const msg = chatMessages[0] as {
        type: string;
        data: {
          id: string;
          timestamp: string;
          timestamp_usec: string;
          message_type: string;
          author: string;
          author_icon_url: string | null;
          channel_id: string;
          content: string;
          runs: unknown[];
          metadata: {
            amount: string | null;
            badges: string[];
            badge_info: unknown[];
            color: string | null;
            is_moderator: boolean;
            is_verified: boolean;
            superchat_colors: unknown | null;
          } | null;
          is_member: boolean;
          comment_count: number | null;
        };
      };

      // Verify all required fields per 03_websocket.md spec
      expect(msg.type).toBe('ChatMessage');
      expect(msg.data).toHaveProperty('id');
      expect(msg.data).toHaveProperty('timestamp');
      expect(msg.data).toHaveProperty('timestamp_usec');
      expect(msg.data).toHaveProperty('message_type');
      expect(msg.data).toHaveProperty('author');
      expect(msg.data).toHaveProperty('author_icon_url');
      expect(msg.data).toHaveProperty('channel_id');
      expect(msg.data).toHaveProperty('content');
      expect(msg.data).toHaveProperty('runs');
      expect(msg.data).toHaveProperty('metadata');
      expect(msg.data).toHaveProperty('is_member');
      expect(msg.data).toHaveProperty('comment_count');

      // Verify metadata structure
      if (msg.data.metadata) {
        expect(msg.data.metadata).toHaveProperty('amount');
        expect(msg.data.metadata).toHaveProperty('badges');
        expect(msg.data.metadata).toHaveProperty('badge_info');
        expect(msg.data.metadata).toHaveProperty('color');
        expect(msg.data.metadata).toHaveProperty('is_moderator');
        expect(msg.data.metadata).toHaveProperty('is_verified');
        expect(msg.data.metadata).toHaveProperty('superchat_colors');
      }

      // Verify timestamp format (ISO 8601 or HH:MM:SS)
      expect(msg.data.timestamp).toMatch(/^\d{2}:\d{2}:\d{2}$|^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);

      // Verify timestamp_usec is numeric string
      expect(msg.data.timestamp_usec).toMatch(/^\d+$/);

      // Cleanup
      ws.close();
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await mainPage.locator('button:has-text("Disconnect")').click();
    });

    test('should broadcast to multiple connected clients', async () => {
      // Reset mock server to ensure clean state
      await resetMockServer();

      // Connect to YouTube stream
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("Connect")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      // Wait for YouTube connection to stabilize
      await mainPage.waitForTimeout(1000);

      // Start WebSocket server (on Chat tab side panel)
      await expect(mainPage.getByText('WebSocket API')).toBeVisible({ timeout: 5000 });
      await mainPage.getByRole('button', { name: 'Start Server' }).click();
      await expect(mainPage.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Wait for server to be fully ready
      await mainPage.waitForTimeout(1000);

      // Get actual port and connect multiple WebSocket clients with delays to avoid race conditions
      const port = await getWebSocketPort(mainPage);

      // Store messages as they arrive (don't wait for timeout)
      const messages1: unknown[] = [];
      const messages2: unknown[] = [];
      const messages3: unknown[] = [];

      const ws1 = await connectWebSocket(`ws://127.0.0.1:${port}`);
      ws1.on('message', (data) => {
        try { messages1.push(JSON.parse(data.toString())); } catch { /* ignore */ }
      });
      await waitForMessage(ws1); // Wait for Connected message

      await mainPage.waitForTimeout(300);
      const ws2 = await connectWebSocket(`ws://127.0.0.1:${port}`);
      ws2.on('message', (data) => {
        try { messages2.push(JSON.parse(data.toString())); } catch { /* ignore */ }
      });
      await waitForMessage(ws2); // Wait for Connected message

      await mainPage.waitForTimeout(300);
      const ws3 = await connectWebSocket(`ws://127.0.0.1:${port}`);
      ws3.on('message', (data) => {
        try { messages3.push(JSON.parse(data.toString())); } catch { /* ignore */ }
      });
      await waitForMessage(ws3); // Wait for Connected message

      // Add message via mock
      await addMockMessage({
        message_type: 'text',
        author: 'BroadcastTest',
        content: 'Message to all clients',
      });

      // Wait for messages to be processed and broadcasted
      await mainPage.waitForTimeout(5000);

      // Verify all clients received the message
      const chatMsg1 = messages1.find((m: unknown) => (m as { type: string }).type === 'ChatMessage');
      const chatMsg2 = messages2.find((m: unknown) => (m as { type: string }).type === 'ChatMessage');
      const chatMsg3 = messages3.find((m: unknown) => (m as { type: string }).type === 'ChatMessage');

      expect(chatMsg1).toBeDefined();
      expect(chatMsg2).toBeDefined();
      expect(chatMsg3).toBeDefined();

      // Verify message content is the same
      expect((chatMsg1 as { data: { content: string } }).data.content).toBe('Message to all clients');
      expect((chatMsg2 as { data: { content: string } }).data.content).toBe('Message to all clients');
      expect((chatMsg3 as { data: { content: string } }).data.content).toBe('Message to all clients');

      // Cleanup
      ws1.close();
      ws2.close();
      ws3.close();
      await mainPage.getByRole('button', { name: 'Stop Server' }).click();
      await mainPage.locator('button:has-text("Disconnect")').click();
    });
  });
});
