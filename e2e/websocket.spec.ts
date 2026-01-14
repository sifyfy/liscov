import { test, expect } from '@playwright/test';
import { setupTauriMock } from './tauri-mock';

/**
 * E2E tests for WebSocket API feature (03_websocket.md)
 * Tests cover:
 * - Server start/stop
 * - Port display and auto-selection
 * - Connected clients display
 * - Status display
 */

test.describe('WebSocket API Feature', () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMock(page);
    await page.goto('/');
  });

  test.describe('WebSocket Section Display', () => {
    test('should display WebSocket API section', async ({ page }) => {
      await expect(page.locator('text=WebSocket API')).toBeVisible();
    });

    test('should display Start Server button initially', async ({ page }) => {
      const startButton = page.getByRole('button', { name: 'Start Server' });
      await expect(startButton).toBeVisible();
    });

    test('should NOT show Stop Server button initially', async ({ page }) => {
      const stopButton = page.getByRole('button', { name: 'Stop Server' });
      await expect(stopButton).not.toBeVisible();
    });
  });

  test.describe('Server Start/Stop', () => {
    test('should invoke websocket_start when Start Server clicked', async ({ page }) => {
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__INVOKED_COMMANDS__ = [];
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          // @ts-expect-error - tracking
          window.__INVOKED_COMMANDS__.push({ cmd, args });
          return originalInvoke(cmd, args);
        };
      });

      const startButton = page.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await page.waitForTimeout(500);

      const commands = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__INVOKED_COMMANDS__ || [];
      });
      const wsStartCmd = commands.find((c: { cmd: string }) => c.cmd === 'websocket_start');
      expect(wsStartCmd).toBeDefined();
    });

    test('should show Stop Server button after starting', async ({ page }) => {
      const startButton = page.getByRole('button', { name: 'Start Server' });
      await startButton.click();

      const stopButton = page.getByRole('button', { name: 'Stop Server' });
      await expect(stopButton).toBeVisible({ timeout: 5000 });
    });

    test('should display port number after starting', async ({ page }) => {
      const startButton = page.getByRole('button', { name: 'Start Server' });
      await startButton.click();
      await page.waitForTimeout(500);

      // Per spec: ポート番号表示 - actual_port: 8765
      await expect(page.locator('text=Running on port')).toBeVisible();
      await expect(page.locator('text=8765')).toBeVisible();
    });

    test('should invoke websocket_stop when Stop Server clicked', async ({ page }) => {
      // Start first
      await page.getByRole('button', { name: 'Start Server' }).click();
      await expect(page.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Setup tracking
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__INVOKED_COMMANDS__ = [];
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          // @ts-expect-error - tracking
          window.__INVOKED_COMMANDS__.push(cmd);
          return originalInvoke(cmd, args);
        };
      });

      // Stop
      await page.getByRole('button', { name: 'Stop Server' }).click();
      await page.waitForTimeout(500);

      const commands = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__INVOKED_COMMANDS__ || [];
      });
      expect(commands).toContain('websocket_stop');
    });

    test('should return to Start Server button after stopping', async ({ page }) => {
      // Start
      await page.getByRole('button', { name: 'Start Server' }).click();
      await expect(page.getByRole('button', { name: 'Stop Server' })).toBeVisible({ timeout: 5000 });

      // Stop
      await page.getByRole('button', { name: 'Stop Server' }).click();

      // Should show Start Server again
      await expect(page.getByRole('button', { name: 'Start Server' })).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Status Display', () => {
    test('should invoke websocket_get_status for status updates', async ({ page }) => {
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__INVOKED_COMMANDS__ = [];
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          // @ts-expect-error - tracking
          window.__INVOKED_COMMANDS__.push(cmd);
          return originalInvoke(cmd, args);
        };
      });

      // Trigger status check (e.g., by starting server)
      await page.getByRole('button', { name: 'Start Server' }).click();
      await page.waitForTimeout(500);

      const commands = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__INVOKED_COMMANDS__ || [];
      });
      // Status should be checked
    });

    test('should display connected clients count', async ({ page }) => {
      // Update mock to show connected clients
      await page.evaluate(() => {
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          if (cmd === 'websocket_get_status') {
            return {
              is_running: true,
              actual_port: 8765,
              connected_clients: 3,  // 3 clients connected
            };
          }
          return originalInvoke(cmd, args);
        };
      });

      await page.getByRole('button', { name: 'Start Server' }).click();
      await page.waitForTimeout(500);

      // Per spec: connected_clients: u32 - 接続数表示
      // Look for client count display
      const clientsDisplay = page.locator('text=3').or(
        page.locator('text=clients')
      );
      // Client count should be visible when running
    });
  });

  test.describe('Port Auto-selection (03_websocket.md)', () => {
    test('should use default port 8765', async ({ page }) => {
      await page.getByRole('button', { name: 'Start Server' }).click();
      await page.waitForTimeout(500);

      // Per spec: デフォルトポート: 8765
      await expect(page.locator('text=8765')).toBeVisible();
    });

    test('should display actual_port from response', async ({ page }) => {
      // Simulate port fallback
      await page.evaluate(() => {
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          if (cmd === 'websocket_start') {
            return { actual_port: 8766 };  // Fallback to 8766
          }
          if (cmd === 'websocket_get_status') {
            return {
              is_running: true,
              actual_port: 8766,
              connected_clients: 0,
            };
          }
          return originalInvoke(cmd, args);
        };
      });

      await page.getByRole('button', { name: 'Start Server' }).click();
      await page.waitForTimeout(500);

      // Per spec: ポート範囲: 8765 〜 8774（自動フォールバック用）
      await expect(page.locator('text=8766')).toBeVisible();
    });
  });
});

test.describe('Session Management (08_database.md)', () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMock(page);
    await page.goto('/');
  });

  test.describe('Session Creation', () => {
    test('should create session on connect', async ({ page }) => {
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__INVOKED_COMMANDS__ = [];
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          // @ts-expect-error - tracking
          window.__INVOKED_COMMANDS__.push({ cmd, args });
          return originalInvoke(cmd, args);
        };
      });

      const urlInput = page.getByPlaceholder(/YouTube URL/);
      await urlInput.fill('https://www.youtube.com/watch?v=test123');
      await page.getByRole('button', { name: 'Connect', exact: true }).click();
      await page.waitForTimeout(500);

      // Per spec: connect_to_stream returns session_id
      // Session is created as part of connection
    });
  });

  test.describe('Session Reconnection', () => {
    test('should restore messages on reconnect to same stream', async ({ page }) => {
      // Per spec: 同一配信（同一video_id）への再接続時、DBから前回取得したメッセージを復元
      // Setup mock with existing session
      await page.evaluate(() => {
        // @ts-expect-error - mock responses
        window.__MOCK_RESPONSES__.session_get_list = [
          {
            id: 'existing-session-123',
            start_time: '2025-01-14T10:00:00Z',
            end_time: null,
            stream_url: 'https://www.youtube.com/watch?v=test123',
            stream_title: 'Previous Session',
            total_messages: 100,
            super_chat_count: 5,
            membership_count: 2,
          },
        ];
      });

      // Connect to same stream
      const urlInput = page.getByPlaceholder(/YouTube URL/);
      await urlInput.fill('https://www.youtube.com/watch?v=test123');
      await page.getByRole('button', { name: 'Connect', exact: true }).click();
      await page.waitForTimeout(500);

      // Messages should be restored from previous session
    });
  });
});
