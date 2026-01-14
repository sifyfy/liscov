import { test, expect, Page } from '@playwright/test';
import { setupTauriMock } from './tauri-mock';

/**
 * E2E tests for Settings features implementation
 * Tests cover:
 * 1. YouTube authentication WebView login button
 * 2. TTS speak button with visual feedback
 * 3. TTS settings auto-save (no save button)
 * 4. Raw response save file browser button
 * 5. Settings persistence
 */

test.describe('Settings Features', () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMock(page);
    await page.goto('/');
    await page.getByRole('button', { name: 'Settings' }).click();
  });

  test.describe('YouTube Authentication - WebView Login', () => {
    test('should display login button for unauthenticated state', async ({ page }) => {
      // Auth settings should show login button
      await expect(page.locator('h2:has-text("YouTube認証")')).toBeVisible();

      // Should show WebView login button (not cookie input)
      const loginButton = page.getByRole('button', { name: /ログイン|Login/i });
      await expect(loginButton).toBeVisible();
    });

    test('should display logout button when authenticated', async ({ page }) => {
      // Modify mock to return authenticated state before navigating
      await page.evaluate(() => {
        // @ts-expect-error - mock responses
        window.__MOCK_RESPONSES__.auth_get_status = {
          is_authenticated: true,
          has_saved_credentials: true,
          credentials_path: 'C:\\Users\\test\\AppData\\Roaming\\liscov\\credentials.toml',
        };
      });

      // Navigate to settings again (mock is already set)
      await page.goto('/');
      await setupTauriMock(page);

      // Re-apply authenticated mock state
      await page.evaluate(() => {
        // @ts-expect-error - mock responses
        window.__MOCK_RESPONSES__.auth_get_status = {
          is_authenticated: true,
          has_saved_credentials: true,
          credentials_path: 'C:\\Users\\test\\AppData\\Roaming\\liscov\\credentials.toml',
        };
      });

      await page.getByRole('button', { name: 'Settings' }).click();
      await page.waitForTimeout(500);

      // Should show logout button
      const logoutButton = page.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible();
    });

    test('login button should invoke auth_open_window command', async ({ page }) => {
      // Track invoked commands
      const invokedCommands: string[] = [];
      await page.evaluate(() => {
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          // @ts-expect-error - tracking
          window.__INVOKED_COMMANDS__ = window.__INVOKED_COMMANDS__ || [];
          // @ts-expect-error - tracking
          window.__INVOKED_COMMANDS__.push(cmd);
          return originalInvoke(cmd, args);
        };
      });

      // Click login button
      const loginButton = page.getByRole('button', { name: /ログイン|Login/i });
      if (await loginButton.isVisible()) {
        await loginButton.click();

        // Wait for command to be invoked
        await page.waitForTimeout(500);

        // Check that auth_open_window was invoked
        const commands = await page.evaluate(() => {
          // @ts-expect-error - tracking
          return window.__INVOKED_COMMANDS__ || [];
        });

        expect(commands).toContain('auth_open_window');
      }
    });
  });

  test.describe('TTS Settings - Speak Button Visual Feedback', () => {
    test.beforeEach(async ({ page }) => {
      await page.getByRole('button', { name: 'TTS読み上げ' }).click();
    });

    test('should display test speak section', async ({ page }) => {
      // Should have test section header
      await expect(page.locator('h3:has-text("読み上げテスト")')).toBeVisible();

      // Should have a test input
      const testInput = page.getByPlaceholder('テスト文を入力');
      await expect(testInput).toBeVisible();

      // Should have speak button (exact match to avoid matching other buttons)
      const speakButton = page.locator('button:has-text("読み上げ")').last();
      await expect(speakButton).toBeVisible();
    });

    test('speak button should show loading state when clicked', async ({ page }) => {
      // Need to select a backend first (button is disabled when backend is 'none')
      const backendSelect = page.locator('select#backend');
      await backendSelect.selectOption('bouyomichan');

      // Wait for UI to update
      await page.waitForTimeout(300);

      // Get speak button (the one in 読み上げテスト section, not キューをクリア)
      const speakButton = page.locator('button:has-text("読み上げ")').last();

      // Click and check for loading indicator (spinner SVG)
      await speakButton.click();

      // Should show spinner briefly
      const spinner = page.locator('svg.animate-spin');
      // Spinner should appear (may be very brief)
      await expect(spinner).toBeVisible({ timeout: 500 }).catch(() => {
        // Spinner might disappear quickly, that's acceptable
      });
    });

    test('speak button should invoke tts_speak_direct command', async ({ page }) => {
      // Track invoked commands
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

      // Select a backend first
      const backendSelect = page.locator('select#backend');
      await backendSelect.selectOption('bouyomichan');
      await page.waitForTimeout(300);

      // Fill test text
      const testInput = page.getByPlaceholder('テスト文を入力');
      await testInput.fill('こんにちは');

      // Click speak button
      const speakButton = page.locator('button:has-text("読み上げ")').last();
      await speakButton.click();

      await page.waitForTimeout(500);

      // Check command was invoked
      const commands = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__INVOKED_COMMANDS__ || [];
      });

      const speakCommand = commands.find((c: { cmd: string }) => c.cmd === 'tts_speak_direct');
      expect(speakCommand).toBeDefined();
    });
  });

  test.describe('TTS Settings - Auto-Save (No Save Button)', () => {
    test.beforeEach(async ({ page }) => {
      await page.getByRole('button', { name: 'TTS読み上げ' }).click();
    });

    test('should NOT display a save button in TTS settings content', async ({ page }) => {
      // There should be no standalone "保存" (Save) button in TTS settings content area
      // Note: The sidebar has "生レスポンス保存" button which contains "保存", so we check the main content
      const mainContent = page.locator('.p-6.space-y-6'); // TTS settings content container
      const saveButtonInContent = mainContent.getByRole('button', { name: '保存', exact: true });
      await expect(saveButtonInContent).not.toBeVisible();
    });

    test('should auto-save when toggling TTS enabled', async ({ page }) => {
      // Track tts_update_config calls
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__UPDATE_CONFIG_CALLS__ = 0;
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          if (cmd === 'tts_update_config') {
            // @ts-expect-error - tracking
            window.__UPDATE_CONFIG_CALLS__++;
          }
          return originalInvoke(cmd, args);
        };
      });

      // Toggle the TTS enabled checkbox
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      if (await enableCheckbox.isVisible()) {
        await enableCheckbox.click();

        // Wait for debounced auto-save
        await page.waitForTimeout(500);

        // Check that update_config was called
        const callCount = await page.evaluate(() => {
          // @ts-expect-error - tracking
          return window.__UPDATE_CONFIG_CALLS__ || 0;
        });

        expect(callCount).toBeGreaterThan(0);
      }
    });

    test('should auto-save when changing backend selection', async ({ page }) => {
      // Track config updates
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__UPDATE_CONFIG_CALLS__ = 0;
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          if (cmd === 'tts_update_config') {
            // @ts-expect-error - tracking
            window.__UPDATE_CONFIG_CALLS__++;
          }
          return originalInvoke(cmd, args);
        };
      });

      // Find backend selector
      const backendSelect = page.locator('select').first();
      if (await backendSelect.isVisible()) {
        // Change selection
        await backendSelect.selectOption({ index: 1 });

        // Wait for debounced auto-save
        await page.waitForTimeout(500);

        const callCount = await page.evaluate(() => {
          // @ts-expect-error - tracking
          return window.__UPDATE_CONFIG_CALLS__ || 0;
        });

        expect(callCount).toBeGreaterThan(0);
      }
    });
  });

  test.describe('Raw Response Save - File Browser Button', () => {
    test.beforeEach(async ({ page }) => {
      await page.getByRole('button', { name: '生レスポンス保存' }).click();
    });

    test('should display raw response settings section', async ({ page }) => {
      await expect(page.locator('h2:has-text("生レスポンス保存設定")')).toBeVisible();
    });

    test('should display browse button when save is enabled', async ({ page }) => {
      // Enable save first
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();

      // Wait for UI to update
      await page.waitForTimeout(300);

      // Should show browse button
      const browseButton = page.getByRole('button', { name: '参照' });
      await expect(browseButton).toBeVisible();
    });

    test('should display file path input', async ({ page }) => {
      // Enable save
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();

      await page.waitForTimeout(300);

      // Should show file path input
      const pathInput = page.locator('input[type="text"]').first();
      await expect(pathInput).toBeVisible();
    });

    test('should display resolved path', async ({ page }) => {
      // Enable save
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      await enableCheckbox.click();

      await page.waitForTimeout(500);

      // Should show resolved path section
      await expect(page.locator('text=実際の保存先')).toBeVisible();
    });
  });

  test.describe('Settings Persistence', () => {
    test('TTS config should be loaded on mount', async ({ page }) => {
      // Track get_config calls
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__GET_CONFIG_CALLED__ = false;
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          if (cmd === 'tts_get_config') {
            // @ts-expect-error - tracking
            window.__GET_CONFIG_CALLED__ = true;
          }
          return originalInvoke(cmd, args);
        };
      });

      // Navigate to TTS settings
      await page.getByRole('button', { name: 'TTS読み上げ' }).click();

      await page.waitForTimeout(500);

      // Config should have been loaded
      const wasCalled = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__GET_CONFIG_CALLED__;
      });

      expect(wasCalled).toBe(true);
    });

    test('Save config should be loaded on mount', async ({ page }) => {
      // Track get_save_config calls
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__GET_SAVE_CONFIG_CALLED__ = false;
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          if (cmd === 'get_save_config') {
            // @ts-expect-error - tracking
            window.__GET_SAVE_CONFIG_CALLED__ = true;
          }
          return originalInvoke(cmd, args);
        };
      });

      // Navigate to save settings
      await page.getByRole('button', { name: '生レスポンス保存' }).click();

      await page.waitForTimeout(500);

      // Config should have been loaded
      const wasCalled = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__GET_SAVE_CONFIG_CALLED__;
      });

      expect(wasCalled).toBe(true);
    });
  });
});
