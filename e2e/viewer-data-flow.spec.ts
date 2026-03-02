import { test, expect, BrowserContext, Page, Browser } from '@playwright/test';
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
    // Navigate to Viewers tab WITHOUT connecting to a stream first
    await mainPage.locator('button:has-text("Viewer")').click();
    await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

    // Wait for the broadcaster list to load (async operation)
    await new Promise(resolve => setTimeout(resolve, 2000));

    const broadcasterSelect = mainPage.locator('#broadcaster-select');
    await expect(broadcasterSelect).toBeVisible();

    // Should have NO broadcasters (only placeholder)
    const options = await broadcasterSelect.locator('option').all();
    log.debug(`Broadcasters before connecting: ${options.length - 1}`);

    // CRITICAL: Without connecting to a stream, there should be NO broadcasters
    expect(options.length).toBe(1); // Only placeholder

    // Should show "配信者を選択してください" message (Japanese UI)
    await expect(mainPage.getByText('配信者を選択してください')).toBeVisible();
  });

  test('Step 2: Authenticate', async () => {
    await mainPage.locator('button:has-text("Settings")').click();
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
    await mainPage.locator('button:has-text("Chat")').click();
    const urlInput = mainPage.locator('input[placeholder*="youtube.com"]');
    await urlInput.fill(`${MOCK_SERVER_URL}/watch?v=real_test_123`);

    const connectButton = mainPage.locator('button:has-text("開始")');
    await connectButton.click();

    // Wait for connection and stream title
    await expect(mainPage.getByText('Mock Live').first()).toBeVisible({ timeout: 15000 });
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
    const dbPath = path.join(process.env.APPDATA || '', TEST_APP_NAME, 'liscov.db');
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
    // Capture browser console logs
    const consoleLogs: string[] = [];
    mainPage.on('console', msg => {
      const text = `[browser] ${msg.type()}: ${msg.text()}`;
      consoleLogs.push(text);
      if (msg.text().includes('[viewerStore]')) {
        log.debug(text);
      }
    });

    // Navigate to Viewers tab
    await mainPage.locator('button:has-text("Viewer")').click();
    await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

    // Wait for the store to load and log
    await new Promise(resolve => setTimeout(resolve, 3000));

    // Log any viewerStore related messages
    log.debug('Relevant console logs:');
    consoleLogs.filter(l => l.includes('[viewerStore]')).forEach(l => log.debug(l));

    const broadcasterSelect = mainPage.locator('#broadcaster-select');

    // CRITICAL TEST: After connecting to a stream, broadcaster should appear
    const options = await broadcasterSelect.locator('option').all();
    log.debug(`Broadcasters after connecting: ${options.length - 1}`);

    // Log all options for debugging
    for (let i = 0; i < options.length; i++) {
      const text = await options[i].textContent();
      log.debug(`  Option ${i}: ${text}`);
    }

    // ASSERTION: There should be at least 1 broadcaster (from the connected stream)
    expect(options.length).toBeGreaterThan(1);

    // Select the broadcaster
    await broadcasterSelect.selectOption({ index: 1 });

    // Wait for viewer list to load
    await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

    // CRITICAL TEST: Viewers from the stream should appear
    const viewerRows = await mainPage.locator('tbody tr').all();
    log.debug(`Viewers found: ${viewerRows.length}`);

    // ASSERTION: Should have at least 2 viewers (RealViewer1 and RealSuperChatter)
    expect(viewerRows.length).toBeGreaterThanOrEqual(2);

    // Verify specific viewers exist
    const tableText = await mainPage.locator('table').textContent();
    expect(tableText).toContain('RealViewer1');
    expect(tableText).toContain('RealSuperChatter');
    log.info('Viewers from stream are visible in Viewer Management');
  });

  test('Step 5: Edit viewer info from Viewer Management', async () => {
    // Click on a viewer to edit
    const viewerRow = mainPage.locator('tbody tr').first();
    await viewerRow.click();

    await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).toBeVisible();

    // Set reading
    const readingInput = mainPage.locator('#reading');
    await readingInput.fill('テスト読み仮名');

    // Save
    await mainPage.getByRole('button', { name: '保存' }).click();

    // Modal should close
    await expect(mainPage.getByRole('heading', { name: '視聴者情報の編集' })).not.toBeVisible({ timeout: 3000 });

    // Verify reading appears in table
    await expect(mainPage.getByText('テスト読み仮名')).toBeVisible();
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

    // Navigate directly to Viewers tab (without connecting to stream)
    await mainPage.locator('button:has-text("Viewer")').click();
    await expect(mainPage.getByRole('heading', { name: '視聴者管理' }).first()).toBeVisible();

    // Wait for the broadcaster list to load (async operation)
    await new Promise(resolve => setTimeout(resolve, 3000));

    const broadcasterSelect = mainPage.locator('#broadcaster-select');
    const options = await broadcasterSelect.locator('option').all();

    log.debug(`Broadcasters after restart: ${options.length - 1}`);

    // CRITICAL: Broadcaster should still exist (persisted in DB)
    expect(options.length).toBeGreaterThan(1);

    // Select the broadcaster
    await broadcasterSelect.selectOption({ index: 1 });
    await expect(mainPage.locator('table')).toBeVisible({ timeout: 5000 });

    // CRITICAL: Viewers should still exist
    const viewerRows = await mainPage.locator('tbody tr').all();
    log.debug(`Viewers after restart: ${viewerRows.length}`);
    expect(viewerRows.length).toBeGreaterThanOrEqual(2);

    // CRITICAL: Custom reading should be persisted
    await expect(mainPage.getByText('テスト読み仮名')).toBeVisible();
    log.info('Data persisted successfully after restart');
  });
});
