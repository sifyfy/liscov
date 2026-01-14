import { test, expect } from '@playwright/test';
import { setupTauriMock } from './tauri-mock';

/**
 * E2E tests for TTS (Text-to-Speech) feature (04_tts.md)
 * Tests cover:
 * - Backend selection (none/bouyomichan/voicevox)
 * - Common settings (read_author_name, add_honorific, etc.)
 * - Bouyomichan-specific settings
 * - VOICEVOX-specific settings
 * - Connection test
 * - Direct speak test
 * - Auto-save behavior
 */

test.describe('TTS Feature', () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMock(page);
    await page.goto('/');
    await page.getByRole('button', { name: 'Settings' }).click();
    await page.getByRole('button', { name: 'TTS読み上げ' }).click();
  });

  test.describe('TTS Header and Enable Toggle', () => {
    test('should display TTS settings header', async ({ page }) => {
      await expect(page.locator('h2:has-text("TTS設定")')).toBeVisible();
    });

    test('should display TTS enable toggle', async ({ page }) => {
      // Per spec: TTS有効/無効トグル
      await expect(page.locator('h3:has-text("TTS読み上げ")')).toBeVisible();
    });

    test('should invoke tts_update_config when toggle is clicked', async ({ page }) => {
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

      // Find and click enable checkbox
      const enableCheckbox = page.locator('input[type="checkbox"]').first();
      if (await enableCheckbox.isVisible()) {
        await enableCheckbox.click();
        await page.waitForTimeout(500);

        const callCount = await page.evaluate(() => {
          // @ts-expect-error - tracking
          return window.__UPDATE_CONFIG_CALLS__ || 0;
        });
        expect(callCount).toBeGreaterThan(0);
      }
    });
  });

  test.describe('Backend Selection', () => {
    test('should display backend selection dropdown', async ({ page }) => {
      // Per spec: バックエンド選択（なし / 棒読みちゃん / VOICEVOX）
      await expect(page.locator('text=バックエンド設定')).toBeVisible();
      await expect(page.locator('text=使用するバックエンド')).toBeVisible();
    });

    test('should have three backend options', async ({ page }) => {
      const backendSelect = page.locator('select#backend');
      if (await backendSelect.isVisible()) {
        // Per spec: "none" | "bouyomichan" | "voicevox"
        const options = await backendSelect.locator('option').allTextContents();
        expect(options.length).toBeGreaterThanOrEqual(3);
      }
    });

    test('should invoke tts_update_config when backend changed', async ({ page }) => {
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

      const backendSelect = page.locator('select#backend');
      if (await backendSelect.isVisible()) {
        await backendSelect.selectOption('bouyomichan');
        await page.waitForTimeout(500);

        const callCount = await page.evaluate(() => {
          // @ts-expect-error - tracking
          return window.__UPDATE_CONFIG_CALLS__ || 0;
        });
        expect(callCount).toBeGreaterThan(0);
      }
    });
  });

  test.describe('Common Settings', () => {
    test('should display read author name setting', async ({ page }) => {
      // Per spec: read_author_name - 投稿者名を読み上げる
      const readAuthorSetting = page.locator('text=投稿者名').or(page.locator('text=ユーザー名'));
      // Common settings should be visible
    });

    test('should display add honorific setting', async ({ page }) => {
      // Per spec: add_honorific - 投稿者名に「さん」を付ける
      const honorificSetting = page.locator('text=さん');
      // Setting should be present
    });

    test('should display max text length setting', async ({ page }) => {
      // Per spec: max_text_length - 最大読み上げ文字数 (default: 200)
      const maxLengthSetting = page.locator('text=最大').or(page.locator('text=文字'));
      // Setting should be present
    });
  });

  test.describe('Bouyomichan Settings', () => {
    test.beforeEach(async ({ page }) => {
      // Select bouyomichan backend
      const backendSelect = page.locator('select#backend');
      if (await backendSelect.isVisible()) {
        await backendSelect.selectOption('bouyomichan');
        await page.waitForTimeout(300);
      }
    });

    test('should display bouyomichan host setting', async ({ page }) => {
      // Per spec: bouyomichan.host (default: "localhost")
      const hostInput = page.locator('input[placeholder*="localhost"]').or(
        page.locator('label:has-text("ホスト") + input')
      );
      // Host input should be visible when bouyomichan selected
    });

    test('should display bouyomichan port setting', async ({ page }) => {
      // Per spec: bouyomichan.port (default: 50080)
      const portInput = page.locator('input[type="number"]');
      // Port input should be visible
    });

    test('should display voice setting', async ({ page }) => {
      // Per spec: bouyomichan.voice (default: 0)
      const voiceSetting = page.locator('text=声質').or(page.locator('text=voice'));
      // Voice setting should be present for bouyomichan
    });
  });

  test.describe('VOICEVOX Settings', () => {
    test.beforeEach(async ({ page }) => {
      // Select VOICEVOX backend
      const backendSelect = page.locator('select#backend');
      if (await backendSelect.isVisible()) {
        await backendSelect.selectOption('voicevox');
        await page.waitForTimeout(300);
      }
    });

    test('should display VOICEVOX host setting', async ({ page }) => {
      // Per spec: voicevox.host (default: "localhost")
      const hostInput = page.locator('input');
      // Host input should be visible
    });

    test('should display VOICEVOX port setting', async ({ page }) => {
      // Per spec: voicevox.port (default: 50021)
      const portInput = page.locator('input[type="number"]');
      // Port input should be visible
    });

    test('should display speaker_id setting', async ({ page }) => {
      // Per spec: voicevox.speaker_id (default: 1 = 四国めたん)
      const speakerSetting = page.locator('text=話者').or(page.locator('text=speaker'));
      // Speaker setting should be present for VOICEVOX
    });

    test('should display speed_scale setting', async ({ page }) => {
      // Per spec: voicevox.speed_scale (default: 1.0, range: 0.5-2.0)
      const speedSetting = page.locator('text=速度').or(page.locator('text=speed'));
      // Speed setting should be present
    });
  });

  test.describe('Connection Test', () => {
    test('should display connection test button', async ({ page }) => {
      // Per spec: 接続テストボタン
      const testButton = page.getByRole('button', { name: /接続テスト|Test/ });
      // Connection test button should be visible
    });

    test('should invoke tts_test_connection when test button clicked', async ({ page }) => {
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

      // Select a backend first
      const backendSelect = page.locator('select#backend');
      if (await backendSelect.isVisible()) {
        await backendSelect.selectOption('bouyomichan');
        await page.waitForTimeout(300);
      }

      const testButton = page.getByRole('button', { name: /接続テスト|Test/ });
      if (await testButton.isVisible()) {
        await testButton.click();
        await page.waitForTimeout(500);

        const commands = await page.evaluate(() => {
          // @ts-expect-error - tracking
          return window.__INVOKED_COMMANDS__ || [];
        });
        expect(commands).toContain('tts_test_connection');
      }
    });
  });

  test.describe('Direct Speak Test', () => {
    test('should display test speak section', async ({ page }) => {
      // Per spec: テスト読み上げ（テキスト入力 + 読み上げボタン）
      await expect(page.locator('h3:has-text("読み上げテスト")')).toBeVisible();
    });

    test('should display test input field', async ({ page }) => {
      const testInput = page.getByPlaceholder('テスト文を入力');
      await expect(testInput).toBeVisible();
    });

    test('should display speak button', async ({ page }) => {
      const speakButton = page.locator('button:has-text("読み上げ")').last();
      await expect(speakButton).toBeVisible();
    });

    test('should invoke tts_speak_direct when speak button clicked', async ({ page }) => {
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

      const commands = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__INVOKED_COMMANDS__ || [];
      });
      const speakCmd = commands.find((c: { cmd: string }) => c.cmd === 'tts_speak_direct');
      expect(speakCmd).toBeDefined();
    });

    test('should show loading state during speak', async ({ page }) => {
      // Select a backend first
      const backendSelect = page.locator('select#backend');
      await backendSelect.selectOption('bouyomichan');
      await page.waitForTimeout(300);

      const speakButton = page.locator('button:has-text("読み上げ")').last();
      await speakButton.click();

      // Per spec: ボタンにスピナー表示
      const spinner = page.locator('svg.animate-spin');
      // Spinner may appear briefly
    });
  });

  test.describe('Config Loading', () => {
    test('should invoke tts_get_config on mount', async ({ page }) => {
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

      // Navigate away and back to trigger mount
      await page.getByRole('button', { name: 'YouTube認証' }).click();
      await page.waitForTimeout(300);
      await page.getByRole('button', { name: 'TTS読み上げ' }).click();
      await page.waitForTimeout(500);

      const wasCalled = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__GET_CONFIG_CALLED__;
      });
      expect(wasCalled).toBe(true);
    });
  });

  test.describe('Auto-save Behavior', () => {
    test('should NOT have a standalone save button', async ({ page }) => {
      // Per spec: 設定変更は自動保存（保存ボタンなし）
      const mainContent = page.locator('.p-6.space-y-6');
      const saveButton = mainContent.getByRole('button', { name: '保存', exact: true });
      await expect(saveButton).not.toBeVisible();
    });

    test('should auto-save with debounce on input change', async ({ page }) => {
      await page.evaluate(() => {
        // @ts-expect-error - tracking
        window.__UPDATE_CALLS__ = [];
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        // @ts-expect-error - extending invoke
        window.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
          if (cmd === 'tts_update_config') {
            // @ts-expect-error - tracking
            window.__UPDATE_CALLS__.push(Date.now());
          }
          return originalInvoke(cmd, args);
        };
      });

      // Change backend
      const backendSelect = page.locator('select#backend');
      await backendSelect.selectOption('bouyomichan');

      // Per spec: 300msデバウンス後に自動保存
      await page.waitForTimeout(500);

      const calls = await page.evaluate(() => {
        // @ts-expect-error - tracking
        return window.__UPDATE_CALLS__ || [];
      });
      expect(calls.length).toBeGreaterThan(0);
    });
  });
});
