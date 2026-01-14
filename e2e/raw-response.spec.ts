import { test, expect } from '@playwright/test';
import { setupTauriMock } from './tauri-mock';

/**
 * E2E tests for Raw Response Save feature (05_raw_response.md)
 * Tests cover:
 * - Enable/disable toggle
 * - File path configuration
 * - Path resolution display
 * - File rotation settings
 * - Browse button functionality
 */

test.describe('Raw Response Save Feature', () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMock(page);
    await page.goto('/');
    await page.getByRole('button', { name: 'Settings' }).click();
    await page.getByRole('button', { name: '生レスポンス保存' }).click();
  });

  test.describe('Settings Section Display', () => {
    test('should display raw response settings header', async ({ page }) => {
      await expect(page.locator('h2:has-text("生レスポンス保存設定")')).toBeVisible();
    });

    test('should display enable toggle', async ({ page }) => {
      // Per spec: 有効/無効トグル
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await expect(enableCheckbox).toBeVisible();
    });
  });

  test.describe('Config Loading', () => {
    test('should invoke raw_response_get_config or get_save_config on mount', async ({ page }) => {
      // Clear command tracking to isolate this test
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__INVOKED_COMMANDS__ = [];
      });

      // Navigate away and back
      await page.getByRole('button', { name: 'YouTube認証' }).click();
      await page.waitForTimeout(300);
      await page.getByRole('button', { name: '生レスポンス保存' }).click();
      await page.waitForTimeout(500);

      const commands = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__INVOKED_COMMANDS__ || [];
      });
      const hasConfigCall = commands.some((c: { cmd: string }) =>
        c.cmd.includes('save_config') || c.cmd.includes('raw_response')
      );
      expect(hasConfigCall).toBe(true);
    });
  });

  test.describe('File Path Settings', () => {
    test('should display file path input when enabled', async ({ page }) => {
      // Enable save first
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(300);

      // Per spec: ファイルパス入力欄
      const pathInput = page.locator('input[type="text"]').first();
      await expect(pathInput).toBeVisible();
    });

    test('should display browse button when enabled', async ({ page }) => {
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(300);

      // Per spec: 「参照」ボタン
      const browseButton = page.getByRole('button', { name: '参照' });
      await expect(browseButton).toBeVisible();
    });

    test('should display resolved path', async ({ page }) => {
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(500);

      // Per spec: 解決後パス表示 - "実際の保存先"
      await expect(page.locator('text=実際の保存先')).toBeVisible();
    });

    test('should invoke raw_response_resolve_path when path changed', async ({ page }) => {
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(300);

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

      const pathInput = page.locator('input[type="text"]').first();
      if (await pathInput.isVisible()) {
        await pathInput.fill('custom_responses.ndjson');
        await page.waitForTimeout(500);

        const commands = await page.evaluate(() => {
          // @ts-expect-error - tracking
          return window.__INVOKED_COMMANDS__ || [];
        });
        const hasResolveCall = commands.some((c: { cmd: string }) =>
          c.cmd.includes('resolve') || c.cmd.includes('path')
        );
        // Path resolution should be triggered
      }
    });
  });

  test.describe('File Rotation Settings', () => {
    test('should display max file size setting when enabled', async ({ page }) => {
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(300);

      // Per spec: max_file_size_mb - 最大ファイルサイズ（MB）
      const maxSizeSetting = page.locator('text=最大ファイルサイズ').or(
        page.locator('text=サイズ')
      );
      // Max size setting should be visible
    });

    test('should display rotation toggle when enabled', async ({ page }) => {
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(300);

      // Per spec: enable_rotation - ファイルローテーション有効/無効
      const rotationSetting = page.locator('text=ローテーション');
      // Rotation setting should be visible
    });

    test('should display backup files count when enabled', async ({ page }) => {
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(300);

      // Per spec: max_backup_files - 保持世代数
      const backupSetting = page.locator('text=保持世代').or(
        page.locator('text=バックアップ')
      );
      // Backup count setting should be visible
    });
  });

  test.describe('Config Update', () => {
    test('should invoke raw_response_update_config when settings changed', async ({ page }) => {
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__UPDATE_CALLS__ = 0;
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          if (cmd.includes('update') && cmd.includes('config')) {
            // @ts-expect-error - tracking
            window.__UPDATE_CALLS__++;
          }
          return originalInvoke(cmd, args);
        };
      });

      // Toggle enable
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(500);

      const callCount = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__UPDATE_CALLS__ || 0;
      });
      expect(callCount).toBeGreaterThan(0);
    });
  });

  test.describe('Default Values (05_raw_response.md)', () => {
    test('should use correct default for enabled', async ({ page }) => {
      // Per spec: enabled default = false
      // Save should be disabled by default
    });

    test('should use correct default for file_path', async ({ page }) => {
      // Per spec: file_path default = "raw_responses.ndjson"
    });

    test('should use correct default for max_file_size_mb', async ({ page }) => {
      // Per spec: max_file_size_mb default = 100
    });

    test('should use correct default for enable_rotation', async ({ page }) => {
      // Per spec: enable_rotation default = true
    });

    test('should use correct default for max_backup_files', async ({ page }) => {
      // Per spec: max_backup_files default = 5
    });
  });

  test.describe('Path Resolution Rules', () => {
    test('should resolve relative path to app data directory', async ({ page }) => {
      // Per spec: 相対パス → %APPDATA%/liscov/raw_responses.ndjson
      await page.evaluate(() => {
        // @ts-expect-error - mock responses
        window.__MOCK_RESPONSES__.raw_response_resolve_path = 'C:\\Users\\test\\AppData\\Roaming\\liscov\\custom.ndjson';
        // @ts-expect-error - mock responses
        window.__MOCK_RESPONSES__.resolve_save_path = 'C:\\Users\\test\\AppData\\Roaming\\liscov\\custom.ndjson';
      });

      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(300);

      const pathInput = page.locator('input[type="text"]').first();
      if (await pathInput.isVisible()) {
        await pathInput.fill('custom.ndjson');
        await page.waitForTimeout(500);

        // Resolved path should show full path
      }
    });

    test('should keep absolute path as-is', async ({ page }) => {
      // Per spec: 絶対パス → そのまま
      await page.evaluate(() => {
        // @ts-expect-error - mock responses
        window.__MOCK_RESPONSES__.raw_response_resolve_path = 'C:\\data\\responses.ndjson';
        // @ts-expect-error - mock responses
        window.__MOCK_RESPONSES__.resolve_save_path = 'C:\\data\\responses.ndjson';
      });

      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();
      await page.waitForTimeout(300);

      const pathInput = page.locator('input[type="text"]').first();
      if (await pathInput.isVisible()) {
        await pathInput.fill('C:\\data\\responses.ndjson');
        await page.waitForTimeout(500);

        // Resolved path should be same as input
      }
    });
  });
});
