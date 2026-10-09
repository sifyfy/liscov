import { test, expect } from './utils/fixtures';
import type { BrowserContext, Page, Browser } from '@playwright/test';
import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import { log } from './utils/logger';
import {
  MOCK_SERVER_URL,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  startTauriApp,
  connectToApp,
  killTauriApp,
  cleanupTestData,
  cleanupTestCredentials,
  TEST_APP_NAME,
  getTestDatabasePath,
  openUrlInput,
  navigateToTab,
  viewerTabRow,
  viewerTabBroadcasterSelect,
} from './utils/test-helpers';

/**
 * E2E tests for actual data flow - NOT using fixtures.
 *
 * These tests verify the REAL user workflow:
 * 1. Connect to a stream
 * 2. Receive chat messages
 * 3. Viewer profiles are automatically created in DB
 * 4. Viewer Management shows the data
 * 5. Data persists across app restarts
 */

test.describe.serial('Viewer Data Flow - Real E2E Tests', () => {
  let browser: Browser;
  let context: BrowserContext;
  let mainPage: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000);

    // Clean start - no fixtures, no pre-seeded data
    log.info('=== CLEAN START - Testing actual data flow ===');
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    context = connection.context;
    mainPage = connection.page;
    log.info('Connected to Tauri app');
  });

  test.afterAll(async () => {
    log.info('Cleaning up...');
    if (browser) await browser.close();
    await killTauriApp();
    await cleanupTestData();
    await cleanupTestCredentials();
  });

  test('Step 1: Before connecting - Viewer Management should be empty', async () => {
    // 接続する前は記録のある配信者がいない (06_viewer.md「視聴者のタブ」: 記録のある配信者がいない)
    await navigateToTab(mainPage, 'Viewers');
    await expect(mainPage.getByText('まだ記録がありません。配信に接続してコメントが届くと記録されます')).toBeVisible({ timeout: 5000 });
    await expect(viewerTabBroadcasterSelect(mainPage)).toHaveCount(0);
  });

  test('Step 2: Authenticate', async () => {
    await mainPage.locator('nav button[aria-label="設定"]').click();
    await expect(mainPage.getByRole('heading', { name: 'YouTube認証' })).toBeVisible();

    const loginButton = mainPage.getByRole('button', { name: 'YouTubeにログイン' });
    if (await loginButton.isVisible()) {
      await loginButton.click();
      const logoutButton = mainPage.getByRole('button', { name: 'ログアウト' });
      await expect(logoutButton).toBeVisible({ timeout: 15000 });
      log.info('Authentication successful');
    }
  });

  test('Step 2.5: Verify mock server returns broadcaster info', async () => {
    // Directly verify the mock server HTML contains broadcaster info
    const response = await fetch(`${MOCK_SERVER_URL}/watch?v=test123`);
    const html = await response.text();

    // Extract ytInitialData
    const startMarker = 'var ytInitialData = ';
    const startIdx = html.indexOf(startMarker);
    if (startIdx === -1) {
      throw new Error('ytInitialData not found in HTML');
    }

    const jsonStart = startIdx + startMarker.length;
    const jsonEnd = html.indexOf(';</script>', jsonStart);
    const jsonStr = html.substring(jsonStart, jsonEnd);

    const data = JSON.parse(jsonStr);
    log.debug('Mock server ytInitialData:', { preview: JSON.stringify(data, null, 2).slice(0, 500) });

    // Verify broadcaster info exists in the response
    const owner = data?.contents?.twoColumnWatchNextResults?.results?.results?.contents?.[1]?.videoSecondaryInfoRenderer?.owner?.videoOwnerRenderer;
    log.debug('Owner data:', { owner: JSON.stringify(owner, null, 2) });

    const browseId = owner?.navigationEndpoint?.browseEndpoint?.browseId;
    log.debug(`BrowseId (broadcaster channel ID): ${browseId}`);

    expect(browseId).toBeDefined();
    expect(browseId).toBe('UC_mock');
  });

  test('Step 3: Connect to stream and receive messages', async () => {
    // Add messages to mock server queue BEFORE connecting
    // These will be delivered when the app polls for messages
    log.info('Adding messages to mock server...');
    await fetch(`${MOCK_SERVER_URL}/add_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message_type: 'text',
        author: 'RealViewer1',
        channel_id: 'UC_real_viewer_1',
        content: 'Hello from RealViewer1!'
      })
    });
    await fetch(`${MOCK_SERVER_URL}/add_message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message_type: 'superchat',
        author: 'RealSuperChatter',
        channel_id: 'UC_real_superchat',
        content: 'Super chat!',
        amount: '¥1000'
      })
    });

    // Navigate to Chat and connect
    await mainPage.locator('nav button[aria-label="チャット"]').click();
    const urlInput = (await openUrlInput(mainPage));
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=real_test_123`);

    const connectButton = mainPage.getByRole('button', { name: '接続', exact: true });
    await connectButton.click();

    // Wait for connection and stream title
    await expect(mainPage.locator('[data-testid="connection-chip"][title="Mock Live"]').first()).toBeVisible({ timeout: 15000 });
    log.info('Connected to stream');

    // Debug: Check connection status display
    // The connection info should show broadcaster details
    const connectionInfo = mainPage.locator('[data-testid="connection-info"]');
    if (await connectionInfo.isVisible()) {
      const infoText = await connectionInfo.textContent();
      log.debug(`Connection info: ${infoText}`);
    }

    // Check for any error messages
    const errorMessage = mainPage.getByText(/error|failed/i);
    if (await errorMessage.count() > 0) {
      log.error(`Error found: ${await errorMessage.first().textContent()}`);
    }

    // Wait for messages to be fetched and processed
    // The app polls every 1.5 seconds, so wait a bit
    await new Promise(resolve => setTimeout(resolve, 5000));

    // Verify messages appeared in chat (use first() to avoid strict mode violation)
    await expect(mainPage.getByText('RealViewer1').first()).toBeVisible({ timeout: 10000 });
    log.info('Messages received in chat');

    // Check the database file directly
    const dbPath = getTestDatabasePath();
    log.debug(`Database path: ${dbPath}`);
    log.debug(`Database exists: ${fs.existsSync(dbPath)}`);

    if (fs.existsSync(dbPath)) {
      // Use sqlite3 command to check tables
      try {
        const result = execSync(`sqlite3 "${dbPath}" "SELECT * FROM broadcaster_profiles;"`, { encoding: 'utf-8' });
        log.debug(`broadcaster_profiles content: ${result || '(empty)'}`);
      } catch (e) {
        log.debug(`Error reading broadcaster_profiles: ${e}`);
      }

      try {
        const result = execSync(`sqlite3 "${dbPath}" "SELECT id, broadcaster_channel_id, broadcaster_name FROM sessions;"`, { encoding: 'utf-8' });
        log.debug(`sessions content: ${result || '(empty)'}`);
      } catch (e) {
        log.debug(`Error reading sessions: ${e}`);
      }
    }
  });

  test('Step 4: Viewer Management should show broadcaster and viewers from stream', async () => {
    // 接続した配信の配信者が選ばれ、届いたコメントの視聴者が出る
    await navigateToTab(mainPage, 'Viewers');
    await expect(viewerTabBroadcasterSelect(mainPage)).not.toHaveValue('', { timeout: 5000 });
    await expect(viewerTabRow(mainPage, 'RealViewer1')).toBeVisible({ timeout: 5000 });
    await expect(viewerTabRow(mainPage, 'RealSuperChatter')).toBeVisible();
    log.info('Viewers from stream are visible in Viewer Management');
  });

  test('Step 5: Edit viewer info from Viewer Management', async () => {
    const row = viewerTabRow(mainPage, 'RealViewer1');
    await row.getByRole('button', { name: 'RealViewer1' }).click();
    const reading = row.getByLabel('読み仮名', { exact: true });
    await reading.fill('テスト読み仮名');
    await reading.press('Enter');

    await expect(reading).toHaveCount(0);
    await expect(row).toContainText('(テスト読み仮名)');
    log.info('Viewer info saved successfully');
  });

  test('Step 6: Data persists after app restart', async () => {
    test.setTimeout(180000);

    // Close and restart the app
    log.info('Restarting app to test persistence...');
    await browser.close();
    await killTauriApp();

    // Wait a moment for the app to fully close
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Restart (keeping mock server running)
    await startTauriApp();

    const connection = await connectToApp();
    browser = connection.browser;
    mainPage = connection.page;
    log.info('App restarted');

    // 接続せずに視聴者のタブを開く (記録のある最初の配信者が選ばれる)
    await navigateToTab(mainPage, 'Viewers');
    await expect(viewerTabBroadcasterSelect(mainPage)).not.toHaveValue('', { timeout: 10000 });

    // CRITICAL: 視聴者と読み仮名が DB に残っている
    await expect(viewerTabRow(mainPage, 'RealViewer1')).toContainText('(テスト読み仮名)', { timeout: 5000 });
    await expect(viewerTabRow(mainPage, 'RealSuperChatter')).toBeVisible();
    log.info('Data persisted successfully after restart');
  });
});
