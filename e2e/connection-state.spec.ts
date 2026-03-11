import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  disconnectAndInitialize,
} from './utils/test-helpers';

/**
 * E2E tests for connection state transitions based on 02_chat.md specification.
 *
 * Tests verify:
 * - Pause preserves stream title and broadcaster name
 * - Paused state displays stream info correctly (not fallback "配信")
 * - Resume reconnects to the same stream
 * - Initialize clears all state and returns to idle
 */

test.describe('Connection State Transitions (02_chat.md)', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(300000);

    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;

    await mainPage.waitForLoadState('domcontentloaded');
    // Wait for SvelteKit app to render
    await expect(mainPage.locator('nav button:has-text("Chat")')).toBeVisible({ timeout: 15000 });
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test.beforeEach(async () => {
    await resetMockServer();
    await disconnectAndInitialize(mainPage);
  });

  test.describe('Pause State (停止)', () => {
    test('should show stream title in paused state, not fallback text', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();

      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await mainPage.locator('button:has-text("停止")').click();

      const pausedInfo = mainPage.locator('div:has-text("⏸ 一時停止中:")').first();
      await expect(pausedInfo).toBeVisible({ timeout: 5000 });

      const pausedText = await pausedInfo.textContent();
      expect(pausedText).toContain('一時停止中:');

      const hasStreamTitle = pausedText?.includes('Mock Live');
      const hasBroadcasterName = pausedText?.includes('Mock Streamer');
      const onlyFallback = pausedText?.match(/一時停止中:\s*配信\s*$/);

      expect(hasStreamTitle || hasBroadcasterName).toBe(true);
      expect(onlyFallback).toBeFalsy();

      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();
      await expect(mainPage.locator('button:has-text("初期化")')).toBeVisible();

      await mainPage.locator('button:has-text("初期化")').click();
    });

    test('should preserve messages when paused', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      const messageCountBefore = await mainPage.locator('[data-message-id]').count();

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      const messageCountAfter = await mainPage.locator('[data-message-id]').count();
      expect(messageCountAfter).toBe(messageCountBefore);

      await mainPage.locator('button:has-text("初期化")').click();
    });

    test('should show chat mode toggle in paused state', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      const chatModeButton = mainPage.locator('button:has-text("トップ"), button:has-text("全て")');
      await expect(chatModeButton).toBeVisible();

      await mainPage.locator('button:has-text("初期化")').click();
    });
  });

  test.describe('Resume (再開)', () => {
    test('should reconnect to the same stream when resumed', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      await mainPage.locator('button:has-text("再開")').click();

      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      await expect(mainPage.getByText('Mock Live').first()).toBeVisible();

      await disconnectAndInitialize(mainPage);
    });

    test('should not clear messages when resumed', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      const messageCountBefore = await mainPage.locator('[data-message-id]').count();

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();
      await mainPage.locator('button:has-text("再開")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      const messageCountAfter = await mainPage.locator('[data-message-id]').count();
      expect(messageCountAfter).toBeGreaterThanOrEqual(messageCountBefore);

      await disconnectAndInitialize(mainPage);
    });

    test('should receive new messages after resume', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      await mainPage.locator('button:has-text("再開")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      const uniqueContent = `ResumeTest_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'ResumeTestUser',
        content: uniqueContent,
        channel_id: 'UC_resume_test',
      });

      await expect(mainPage.getByText(uniqueContent)).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Initialize (初期化)', () => {
    test('should clear all state and return to idle', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("初期化")')).toBeVisible();

      await mainPage.locator('button:has-text("初期化")').click();

      await expect(mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]')).toBeVisible({ timeout: 5000 });

      await expect(mainPage.locator('button:has-text("開始")')).toBeVisible();

      const messageCount = await mainPage.locator('[data-message-id]').count();
      expect(messageCount).toBe(0);

      await expect(mainPage.getByText('Mock Live')).not.toBeVisible();
    });

    test('should clear URL input after initialize', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 10000 });

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("初期化")')).toBeVisible();
      await mainPage.locator('button:has-text("初期化")').click();

      const newUrlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await expect(newUrlInput).toBeVisible({ timeout: 5000 });

      const inputValue = await newUrlInput.inputValue();
      expect(inputValue).toBe('');
    });
  });

  test.describe('State Transitions', () => {
    test('should follow correct state machine: idle -> connecting -> connected -> paused -> idle', async () => {
      await expect(mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]')).toBeVisible();

      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();

      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();
      await expect(mainPage.locator('button:has-text("初期化")')).toBeVisible();

      await mainPage.locator('button:has-text("初期化")').click();
      await expect(mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]')).toBeVisible({ timeout: 5000 });
    });

    test('should follow correct state machine: paused -> connecting -> connected via resume', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible();

      await mainPage.locator('button:has-text("再開")').click();

      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('Auto-Message Continuous Flow (実際のYouTubeシミュレーション)', () => {
    async function enableAutoMessages(messagesPerPoll: number = 10): Promise<void> {
      await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: true, messages_per_poll: messagesPerPoll }),
      });
    }

    async function disableAutoMessages(): Promise<void> {
      await fetch(`${MOCK_SERVER_URL}/set_auto_message`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: false }),
      });
    }

    async function getAutoMessageStatus(): Promise<{ enabled: boolean; total_generated: number }> {
      const response = await fetch(`${MOCK_SERVER_URL}/auto_message_status`);
      return response.json();
    }

    test('should continue receiving auto-generated messages after pause/resume', async () => {
      await enableAutoMessages(10);

      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_auto_msg`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      log.debug('Waiting for auto-generated messages...');
      await mainPage.waitForTimeout(5000);

      // Use status bar total count (VList virtualizes DOM, so element count != total)
      const statusBefore = await mainPage.locator('text=/全\\d+件/').textContent();
      const messageCountBefore = parseInt(statusBefore?.match(/全(\d+)件/)?.[1] || '0');
      log.debug(`Messages before pause (status bar): ${messageCountBefore}`);
      expect(messageCountBefore).toBeGreaterThanOrEqual(10);

      const lastMessageBefore = await mainPage.locator('[data-message-id]').last().getAttribute('data-message-id');
      log.debug(`Last message ID before pause: ${lastMessageBefore}`);

      log.debug('Pausing...');
      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

      const statusAfterPause = await getAutoMessageStatus();
      log.debug(`Auto-message total after pause: ${statusAfterPause.total_generated}`);

      await mainPage.waitForTimeout(2000);

      log.debug('Resuming...');
      await mainPage.locator('button:has-text("再開")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });
      log.debug('Resume completed');

      log.debug('Waiting for new messages after resume...');
      await mainPage.waitForTimeout(8000);

      // Use status bar total count (VList virtualizes DOM)
      const statusAfter = await mainPage.locator('text=/全\\d+件/').textContent();
      const messageCountAfter = parseInt(statusAfter?.match(/全(\d+)件/)?.[1] || '0');
      log.debug(`Messages after resume wait (status bar): ${messageCountAfter}`);

      const lastMessageAfter = await mainPage.locator('[data-message-id]').last().getAttribute('data-message-id');
      log.debug(`Last message ID after resume: ${lastMessageAfter}`);

      expect(messageCountAfter).toBeGreaterThan(messageCountBefore);

      expect(lastMessageAfter).not.toBe(lastMessageBefore);

      const statusAfterResume = await getAutoMessageStatus();
      log.debug(`Auto-message total after resume: ${statusAfterResume.total_generated}`);
      expect(statusAfterResume.total_generated).toBeGreaterThan(statusAfterPause.total_generated);

      await disableAutoMessages();
      await disconnectAndInitialize(mainPage);
    });

    test('should not lose messages during rapid pause/resume with auto-generation', async () => {
      await enableAutoMessages(20);

      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_rapid_auto`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      await mainPage.waitForTimeout(3000);
      // Use status bar total count (VList virtualizes DOM, so element count != total)
      const initialStatusText = await mainPage.locator('text=/全\\d+件/').textContent();
      const initialCount = parseInt(initialStatusText?.match(/全(\d+)件/)?.[1] || '0');
      log.debug(`Initial message count (status bar): ${initialCount}`);

      for (let i = 0; i < 5; i++) {
        log.debug(`Rapid cycle ${i + 1}/5`);

        await mainPage.locator('button:has-text("停止")').click();
        await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

        await mainPage.waitForTimeout(100);

        await mainPage.locator('button:has-text("再開")').click();
        await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

        await mainPage.waitForTimeout(100);
      }

      await mainPage.waitForTimeout(5000);

      // Use status bar total count
      const finalStatusText = await mainPage.locator('text=/全\\d+件/').textContent();
      const finalCount = parseInt(finalStatusText?.match(/全(\d+)件/)?.[1] || '0');
      log.debug(`Final message count after 5 rapid cycles (status bar): ${finalCount}`);

      expect(finalCount).toBeGreaterThan(initialCount);

      await disableAutoMessages();
      await disconnectAndInitialize(mainPage);
    });
  });

  test.describe('High Volume Resume (UIフリーズ回避)', () => {
    test('should not freeze UI when resuming with high message volume', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      log.debug('Sending 500 messages to fill message buffer...');
      const messagePromises = [];
      for (let i = 0; i < 500; i++) {
        const msgType = i % 20 === 0 ? 'superchat' : i % 10 === 0 ? 'membership' : 'text';
        messagePromises.push(
          addMockMessage({
            message_type: msgType,
            author: `User${i % 50}`,
            content: `Pre-pause message ${i} with some additional text to make it longer and more realistic like actual YouTube chat messages`,
            channel_id: `UC_user_${i % 50}`,
            is_member: i % 5 === 0,
            amount: msgType === 'superchat' ? '¥500' : undefined,
          })
        );
      }
      await Promise.all(messagePromises);

      await mainPage.waitForTimeout(8000);

      // Use status bar total count (VList virtualizes DOM, so element count != total)
      const statusText = await mainPage.locator('text=/全\\d+件/').textContent();
      const messageCount = parseInt(statusText?.match(/全(\d+)件/)?.[1] || '0');
      log.debug(`Messages in UI before pause (status bar): ${messageCount}`);
      expect(messageCount).toBeGreaterThan(100);

      await mainPage.locator('button:has-text("停止")').click();
      await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

      log.debug('Adding 200 messages to queue for resume...');
      const resumeMessages = [];
      for (let i = 0; i < 200; i++) {
        const msgType = i % 15 === 0 ? 'superchat' : i % 8 === 0 ? 'membership' : 'text';
        resumeMessages.push(
          addMockMessage({
            message_type: msgType,
            author: `ResumeUser${i % 30}`,
            content: `Resume batch message ${i} - testing high volume scenario with realistic message length`,
            channel_id: `UC_resume_${i % 30}`,
            is_member: i % 4 === 0,
            amount: msgType === 'superchat' ? '¥1000' : undefined,
          })
        );
      }
      await Promise.all(resumeMessages);

      log.debug('Clicking resume and immediately trying tab switch...');
      const resumeButton = mainPage.locator('button:has-text("再開")');
      const settingsTab = mainPage.locator('button:has-text("Settings")');

      await resumeButton.click();

      const interactionStart = Date.now();

      const freezeTimeout = 3000;

      try {
        await Promise.race([
          (async () => {
            await settingsTab.click({ timeout: freezeTimeout });
            await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible({ timeout: 1000 });
          })(),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error('UI FREEZE DETECTED: Tab click not processed')), freezeTimeout)
          ),
        ]);
      } catch (error) {
        const elapsed = Date.now() - interactionStart;
        log.error(`UI freeze detected after ${elapsed}ms`);
        throw error;
      }

      const interactionDuration = Date.now() - interactionStart;
      log.debug(`Tab switch completed in ${interactionDuration}ms`);

      expect(interactionDuration).toBeLessThan(1000);

      await mainPage.locator('button:has-text("Chat")').click();
      await mainPage.waitForTimeout(500);

      await disconnectAndInitialize(mainPage);
    });

    test('should continue receiving messages after multiple pause/resume cycles with high volume', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      for (let cycle = 0; cycle < 3; cycle++) {
        log.debug(`Pause/Resume cycle ${cycle + 1}/3`);

        const messagePromises = [];
        for (let i = 0; i < 50; i++) {
          messagePromises.push(
            addMockMessage({
              message_type: 'text',
              author: `CycleUser${i % 5}`,
              content: `Cycle ${cycle} message ${i}`,
              channel_id: `UC_cycle_${i % 5}`,
            })
          );
        }
        await Promise.all(messagePromises);
        await mainPage.waitForTimeout(1000);

        await mainPage.locator('button:has-text("停止")').click();
        await expect(mainPage.locator('button:has-text("再開")')).toBeVisible({ timeout: 5000 });

        await mainPage.locator('button:has-text("再開")').click();
        await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

        await mainPage.locator('button:has-text("Settings")').click();
        await mainPage.locator('button:has-text("Chat")').click();
      }

      const finalContent = `FinalCheck_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'FinalCheckUser',
        content: finalContent,
        channel_id: 'UC_final_check',
      });

      await expect(mainPage.getByText(finalContent)).toBeVisible({ timeout: 10000 });
      log.debug('Final message received after 3 pause/resume cycles!');

      await disconnectAndInitialize(mainPage);
    });

    test('should handle rapid pause/resume without UI freeze', async () => {
      const urlInput = mainPage.locator('input[placeholder*="YouTube URL"], input[placeholder*="youtube.com"]');
      await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=test_video_123`);
      await mainPage.locator('button:has-text("開始")').click();
      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      const messagePromises = [];
      for (let i = 0; i < 100; i++) {
        messagePromises.push(
          addMockMessage({
            message_type: 'text',
            author: `RapidUser${i % 10}`,
            content: `Rapid test message ${i}`,
            channel_id: `UC_rapid_${i % 10}`,
          })
        );
      }
      await Promise.all(messagePromises);
      await mainPage.waitForTimeout(2000);

      log.debug('Performing rapid pause/resume cycles...');
      for (let i = 0; i < 5; i++) {
        await mainPage.locator('button:has-text("停止")').click();
        await mainPage.waitForTimeout(200);
        await mainPage.locator('button:has-text("再開")').click();
        await mainPage.waitForTimeout(200);
      }

      await expect(mainPage.locator('button:has-text("停止")')).toBeVisible({ timeout: 10000 });

      const tabClickStart = Date.now();
      await mainPage.locator('button:has-text("Settings")').click();
      await mainPage.locator('button:has-text("Chat")').click();
      const tabClickDuration = Date.now() - tabClickStart;
      log.debug(`Tab switching after rapid cycles: ${tabClickDuration}ms`);
      expect(tabClickDuration).toBeLessThan(3000);

      const rapidContent = `AfterRapid_${Date.now()}`;
      await addMockMessage({
        message_type: 'text',
        author: 'RapidTestUser',
        content: rapidContent,
        channel_id: 'UC_rapid_final',
      });

      await expect(mainPage.getByText(rapidContent)).toBeVisible({ timeout: 10000 });
      log.debug('Message received after rapid pause/resume cycles!');

      await disconnectAndInitialize(mainPage);
    });
  });
});
