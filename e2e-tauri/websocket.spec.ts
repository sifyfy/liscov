import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
import { WebSocket } from 'ws';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
} from './utils/test-helpers';

/**
 * E2E tests for WebSocket API based on 03_websocket.md specification.
 *
 * Tests verify:
 * - Server auto-starts on app launch (no manual start/stop)
 * - Port selection within 8765-8774 range
 * - Tauri events: websocket-client-connected, websocket-client-disconnected
 * - Connected clients count in UI
 * - Complete data flow: YouTube (mock) → Tauri App → WebSocket API → External Client
 * - Message format verification per spec
 */

// Helper to get the actual WebSocket port from the UI status display
// Format: "WS:8765(0)" or status text showing port
async function getWebSocketPort(page: Page): Promise<number> {
  const wsStatusLocator = page.locator('text=/WS:\\d{4}\\(\\d+\\)/');
  const timeout = 15000;
  const start = Date.now();

  while (Date.now() - start < timeout) {
    const count = await wsStatusLocator.count();
    if (count > 0) {
      const wsStatus = await wsStatusLocator.first().textContent();
      if (wsStatus) {
        const match = wsStatus.match(/WS:(\d+)/i);
        if (match) {
          return parseInt(match[1], 10);
        }
      }
    }
    await page.waitForTimeout(500);
  }

  throw new Error('Could not find WebSocket port in UI - WebSocket server may not have started');
}

// Helper to connect WebSocket client and wait for Connected message
function connectWebSocket(
  url: string,
  timeout = 10000,
): Promise<{ ws: WebSocket; connectedMsg: unknown }> {
  return new Promise((resolve, reject) => {
    log.debug(`Connecting to WebSocket at ${url}...`);
    const ws = new WebSocket(url);
    const timer = setTimeout(() => {
      log.debug(`WebSocket connection timeout (waiting for Connected message) to ${url}`);
      ws.close();
      reject(new Error(`WebSocket connection timeout after ${timeout}ms`));
    }, timeout);

    ws.once('message', (data) => {
      clearTimeout(timer);
      try {
        const parsed = JSON.parse(data.toString());
        log.debug(`WebSocket connected to ${url}, received: ${parsed.type || 'unknown'}`);
        resolve({ ws, connectedMsg: parsed });
      } catch (e) {
        reject(e);
      }
    });

    ws.on('error', (err) => {
      log.debug(`WebSocket error connecting to ${url}: ${err.message}`);
      clearTimeout(timer);
      reject(err);
    });

    ws.on('close', (code, reason) => {
      clearTimeout(timer);
      reject(new Error(`WebSocket closed before Connected message: code=${code}, reason=${reason.toString()}`));
    });
  });
}

// Helper to wait for WebSocket message
function waitForMessage(ws: WebSocket, timeout = 10000): Promise<unknown> {
  return new Promise((resolve, reject) => {
    log.debug('Waiting for WebSocket message...');
    const timer = setTimeout(() => {
      log.debug('WebSocket message wait timed out');
      reject(new Error(`WebSocket message timeout after ${timeout}ms`));
    }, timeout);

    ws.once('message', (data) => {
      clearTimeout(timer);
      try {
        const parsed = JSON.parse(data.toString());
        log.debug(`Received WebSocket message: ${parsed.type || 'unknown'}`);
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

// Helper to fully disconnect (stop + initialize) and return to idle state
async function disconnectAndInitialize(page: Page): Promise<void> {
  const stopButton = page.locator('button:has-text("停止")');
  if (await stopButton.isVisible({ timeout: 1000 }).catch(() => false)) {
    await stopButton.click();
    await page.locator('button:has-text("初期化")').click();
    await expect(page.locator('input[placeholder*="youtube.com"], input[placeholder*="youtube.com"]')).toBeVisible({
      timeout: 5000,
    });
  }
}

test.describe('WebSocket API (03_websocket.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000); // 5 minutes for setup

    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test.beforeEach(async () => {
    await resetMockServer();
    await disconnectAndInitialize(mainPage);
  });

  test.describe('Auto-Start Behavior', () => {
    test('should have WebSocket server running on app launch', async () => {
      const port = await getWebSocketPort(mainPage);
      expect(port).toBeGreaterThanOrEqual(8765);
      expect(port).toBeLessThanOrEqual(8774);

      const { ws, connectedMsg } = await connectWebSocket(`ws://127.0.0.1:${port}`);
      const connected = connectedMsg as { type: string; data: { client_id: number } };
      expect(connected.type).toBe('Connected');
      expect(connected.data.client_id).toBeGreaterThan(0);

      ws.close();
    });

    test('should display port number in header status', async () => {
      const wsStatus = await mainPage.locator('text=/WS:\\d+/i').textContent();
      expect(wsStatus).toMatch(/WS:\d{4}/i);

      const port = await getWebSocketPort(mainPage);
      expect(port).toBeGreaterThanOrEqual(8765);
      expect(port).toBeLessThanOrEqual(8774);
    });
  });

  test.describe('Port Range (8765-8774)', () => {
    test('should use port within valid range', async () => {
      const port = await getWebSocketPort(mainPage);
      expect(port).toBeGreaterThanOrEqual(8765);
      expect(port).toBeLessThanOrEqual(8774);
    });
  });

  test.describe('Tauri Events', () => {
    test('should update UI when client connects (websocket-client-connected event)', async () => {
      const port = await getWebSocketPort(mainPage);

      await expect(mainPage.locator(`text=/WS:${port}\\(0\\)/`)).toBeVisible({ timeout: 5000 });

      const { ws } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      await expect(mainPage.locator(`text=/WS:${port}\\(1\\)/`)).toBeVisible({ timeout: 5000 });

      ws.close();
    });

    test('should update UI when client disconnects (websocket-client-disconnected event)', async () => {
      const port = await getWebSocketPort(mainPage);

      const { ws } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      await expect(mainPage.locator(`text=/WS:${port}\\(1\\)/`)).toBeVisible({ timeout: 5000 });

      ws.close();

      await expect(mainPage.locator(`text=/WS:${port}\\(0\\)/`)).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Connected Clients Count', () => {
    test('should display connected clients count in header', async () => {
      const port = await getWebSocketPort(mainPage);

      await expect(mainPage.locator(`text=/WS:${port}\\(0\\)/`)).toBeVisible({ timeout: 5000 });

      const { ws } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      await expect(mainPage.locator(`text=/WS:${port}\\(1\\)/`)).toBeVisible({ timeout: 5000 });

      const { ws: ws2 } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      await expect(mainPage.locator(`text=/WS:${port}\\(2\\)/`)).toBeVisible({ timeout: 5000 });

      ws.close();
      await expect(mainPage.locator(`text=/WS:${port}\\(1\\)/`)).toBeVisible({ timeout: 5000 });

      ws2.close();
      await expect(mainPage.locator(`text=/WS:${port}\\(0\\)/`)).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Connected Message', () => {
    test('should send Connected message with unique client_id on connection', async () => {
      const port = await getWebSocketPort(mainPage);

      const { ws: ws1, connectedMsg: connectedMsg1 } = await connectWebSocket(`ws://127.0.0.1:${port}`);
      const msg1 = connectedMsg1 as { type: string; data: { client_id: number } };

      await mainPage.waitForTimeout(200);
      const { ws: ws2, connectedMsg: connectedMsg2 } = await connectWebSocket(`ws://127.0.0.1:${port}`);
      const msg2 = connectedMsg2 as { type: string; data: { client_id: number } };

      expect(msg1.type).toBe('Connected');
      expect(msg1.data).toHaveProperty('client_id');
      expect(typeof msg1.data.client_id).toBe('number');

      expect(msg2.type).toBe('Connected');
      expect(msg2.data).toHaveProperty('client_id');
      expect(typeof msg2.data.client_id).toBe('number');

      expect(msg1.data.client_id).not.toBe(msg2.data.client_id);

      ws1.close();
      ws2.close();
    });
  });

  test.describe('ServerInfo Message', () => {
    test('should respond to GetInfo with correct format', async () => {
      const port = await getWebSocketPort(mainPage);
      const { ws } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      ws.send(JSON.stringify({ type: 'GetInfo' }));

      const infoMsg = (await waitForMessage(ws)) as {
        type: string;
        data: {
          version: string;
          connected_clients: number;
        };
      };

      expect(infoMsg.type).toBe('ServerInfo');
      expect(infoMsg.data).toHaveProperty('version');
      expect(infoMsg.data).toHaveProperty('connected_clients');
      expect(typeof infoMsg.data.version).toBe('string');
      expect(typeof infoMsg.data.connected_clients).toBe('number');
      expect(infoMsg.data.connected_clients).toBeGreaterThanOrEqual(1);

      ws.close();
    });
  });

  test.describe('Complete Data Flow: YouTube → App → WebSocket → Client', () => {
    test('should receive text message through WebSocket API', async () => {
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      const port = await getWebSocketPort(mainPage);
      const { ws, connectedMsg } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      const connected = connectedMsg as { type: string; data: { client_id: number } };
      expect(connected.type).toBe('Connected');
      expect(connected.data.client_id).toBeGreaterThan(0);

      await addMockMessage({
        message_type: 'text',
        author: 'TestViewer',
        content: 'Hello WebSocket!',
        channel_id: 'UC_test_viewer',
        is_member: false,
      });

      const messages = await collectMessages(ws, 5000);

      const chatMessages = messages.filter((m: unknown) => (m as { type: string }).type === 'ChatMessage');
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

      expect(chatMsg.type).toBe('ChatMessage');
      expect(chatMsg.data.author).toBe('TestViewer');
      expect(chatMsg.data.content).toBe('Hello WebSocket!');
      expect(chatMsg.data.message_type).toBe('Text');

      expect(chatMsg.data.runs).toBeInstanceOf(Array);
      expect(chatMsg.data.runs.length).toBeGreaterThan(0);
      expect(chatMsg.data.runs[0]).toHaveProperty('Text');
      expect(chatMsg.data.runs[0].Text).toHaveProperty('content');

      ws.close();
      await disconnectAndInitialize(mainPage);
    });

    test('should receive SuperChat message with correct format', async () => {
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      const port = await getWebSocketPort(mainPage);
      const { ws } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      await addMockMessage({
        message_type: 'superchat',
        author: 'SuperChatter',
        content: 'Thanks for the stream!',
        amount: '¥1,000',
        tier: 'green',
        is_member: true,
      });

      const messages = await collectMessages(ws, 5000);
      const chatMessages = messages.filter((m: unknown) => (m as { type: string }).type === 'ChatMessage');
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

      expect(scMsg.data.message_type).toHaveProperty('SuperChat');
      expect(scMsg.data.message_type.SuperChat).toHaveProperty('amount');
      expect(scMsg.data.message_type.SuperChat.amount).toBe('¥1,000');

      expect(scMsg.data.metadata.amount).toBe('¥1,000');
      expect(scMsg.data.metadata.superchat_colors).not.toBeNull();

      ws.close();
      await disconnectAndInitialize(mainPage);
    });

    test('should receive membership message with milestone_months', async () => {
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      const port = await getWebSocketPort(mainPage);
      const { ws } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      await addMockMessage({
        message_type: 'membership_milestone',
        author: 'LoyalMember',
        content: 'ありがとうございます！',
        milestone_months: 12,
      });

      const messages = await collectMessages(ws, 5000);
      const chatMessages = messages.filter((m: unknown) => (m as { type: string }).type === 'ChatMessage');
      expect(chatMessages.length).toBeGreaterThan(0);

      const memberMsg = chatMessages[0] as {
        type: string;
        data: {
          author: string;
          message_type: { Membership: { milestone_months: number | null } };
          is_member: boolean;
        };
      };

      expect(memberMsg.data.message_type).toHaveProperty('Membership');
      expect(memberMsg.data.message_type.Membership).toHaveProperty('milestone_months');
      expect(memberMsg.data.is_member).toBe(true);

      ws.close();
      await disconnectAndInitialize(mainPage);
    });

    test('should receive membership gift message with gift_count', async () => {
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      const port = await getWebSocketPort(mainPage);
      const { ws } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      await addMockMessage({
        message_type: 'membership_gift',
        author: 'GenerousGifter',
        content: '',
        gift_count: 5,
      });

      const messages = await collectMessages(ws, 5000);
      const chatMessages = messages.filter((m: unknown) => (m as { type: string }).type === 'ChatMessage');
      expect(chatMessages.length).toBeGreaterThan(0);

      const giftMsg = chatMessages[0] as {
        type: string;
        data: {
          author: string;
          message_type: { MembershipGift: { gift_count: number } };
        };
      };

      expect(giftMsg.data.message_type).toHaveProperty('MembershipGift');
      expect(giftMsg.data.message_type.MembershipGift).toHaveProperty('gift_count');
      expect(giftMsg.data.message_type.MembershipGift.gift_count).toBe(5);

      ws.close();
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Message Format Verification (03_websocket.md spec)', () => {
    test('should have correct ChatMessage structure', async () => {
      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      const port = await getWebSocketPort(mainPage);
      const { ws } = await connectWebSocket(`ws://127.0.0.1:${port}`);

      await addMockMessage({
        message_type: 'text',
        author: 'StructTestUser',
        content: 'Structure test message',
        channel_id: 'UC_struct_test',
        is_member: true,
      });

      const messages = await collectMessages(ws, 5000);
      const chatMessages = messages.filter((m: unknown) => (m as { type: string }).type === 'ChatMessage');
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
          is_first_time_viewer: boolean;
          in_stream_comment_count: number | null;
        };
      };

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
      expect(msg.data).toHaveProperty('is_first_time_viewer');
      expect(msg.data).toHaveProperty('in_stream_comment_count');

      if (msg.data.metadata) {
        expect(msg.data.metadata).toHaveProperty('amount');
        expect(msg.data.metadata).toHaveProperty('badges');
        expect(msg.data.metadata).toHaveProperty('badge_info');
        expect(msg.data.metadata).toHaveProperty('color');
        expect(msg.data.metadata).toHaveProperty('is_moderator');
        expect(msg.data.metadata).toHaveProperty('is_verified');
        expect(msg.data.metadata).toHaveProperty('superchat_colors');
      }

      expect(msg.data.timestamp).toMatch(/^\d{2}:\d{2}:\d{2}$|^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
      expect(msg.data.timestamp_usec).toMatch(/^\d+$/);

      ws.close();
      await disconnectAndInitialize(mainPage);
    });

    test('should broadcast to multiple connected clients', async () => {
      await resetMockServer();

      const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await mainPage.waitForTimeout(1000);

      const port = await getWebSocketPort(mainPage);

      const messages1: unknown[] = [];
      const messages2: unknown[] = [];
      const messages3: unknown[] = [];

      const { ws: ws1 } = await connectWebSocket(`ws://127.0.0.1:${port}`);
      ws1.on('message', (data) => {
        try {
          messages1.push(JSON.parse(data.toString()));
        } catch {
          /* ignore */
        }
      });

      await mainPage.waitForTimeout(300);
      const { ws: ws2 } = await connectWebSocket(`ws://127.0.0.1:${port}`);
      ws2.on('message', (data) => {
        try {
          messages2.push(JSON.parse(data.toString()));
        } catch {
          /* ignore */
        }
      });

      await mainPage.waitForTimeout(300);
      const { ws: ws3 } = await connectWebSocket(`ws://127.0.0.1:${port}`);
      ws3.on('message', (data) => {
        try {
          messages3.push(JSON.parse(data.toString()));
        } catch {
          /* ignore */
        }
      });

      await addMockMessage({
        message_type: 'text',
        author: 'BroadcastTest',
        content: 'Message to all clients',
      });

      await mainPage.waitForTimeout(5000);

      const chatMsg1 = messages1.find((m: unknown) => (m as { type: string }).type === 'ChatMessage');
      const chatMsg2 = messages2.find((m: unknown) => (m as { type: string }).type === 'ChatMessage');
      const chatMsg3 = messages3.find((m: unknown) => (m as { type: string }).type === 'ChatMessage');

      expect(chatMsg1).toBeDefined();
      expect(chatMsg2).toBeDefined();
      expect(chatMsg3).toBeDefined();

      expect((chatMsg1 as { data: { content: string } }).data.content).toBe('Message to all clients');
      expect((chatMsg2 as { data: { content: string } }).data.content).toBe('Message to all clients');
      expect((chatMsg3 as { data: { content: string } }).data.content).toBe('Message to all clients');

      ws1.close();
      ws2.close();
      ws3.close();
      await disconnectAndInitialize(mainPage);
    });
  });
});
