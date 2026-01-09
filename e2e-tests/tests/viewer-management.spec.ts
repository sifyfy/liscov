/**
 * E2E Tests for Viewer Management Tab
 *
 * Tests the viewer data management functionality including:
 * - Tab navigation
 * - Broadcaster selection
 * - Viewer list display
 * - Viewer information editing (reading, notes, tags, membership level)
 */

import { test, expect, Page } from '@playwright/test';
import { connectToApp, closeApp, closeModals, AppContext } from './app-launcher';

let appContext: AppContext;

test.beforeAll(async () => {
  appContext = await connectToApp();
});

test.afterAll(async () => {
  if (appContext) {
    await closeApp(appContext);
  }
});

test.beforeEach(async () => {
  // Close any open modals before each test
  await closeModals(appContext.page);
});

test.describe('Viewer Management Tab', () => {
  test('should display viewer management tab in navigation', async () => {
    const { page } = appContext;

    // Check that the viewer management tab exists in the navigation
    const viewerManagementTab = page.locator('button:has-text("視聴者管理")');
    await expect(viewerManagementTab).toBeVisible({ timeout: 10000 });
  });

  test('should navigate to viewer management tab when clicked', async () => {
    const { page } = appContext;

    // Click on the viewer management tab
    await page.click('button:has-text("視聴者管理")');
    await page.waitForTimeout(1000);

    // Check that the header is visible
    const header = page.locator('h2:has-text("視聴者管理")');
    await expect(header).toBeVisible({ timeout: 5000 });

    // Check that the broadcaster selector is visible
    const broadcasterLabel = page.locator('label:has-text("配信者チャンネル")');
    await expect(broadcasterLabel).toBeVisible({ timeout: 5000 });
  });

  test('should display empty state when no broadcaster is selected', async () => {
    const { page } = appContext;

    // Navigate to viewer management tab
    await navigateToViewerManagement(page);

    // Reset broadcaster selection by selecting placeholder option
    const select = page.locator('.broadcaster-selector select');

    if (await select.isVisible({ timeout: 3000 }).catch(() => false)) {
      // Select the placeholder option to reset state
      await select.selectOption({ index: 0 });
      await page.waitForTimeout(500);
    }

    // Check for placeholder message
    const placeholder = page.locator('text=配信者チャンネルを選択してください');
    await expect(placeholder).toBeVisible({ timeout: 5000 });
  });

  test('should display viewer list when broadcaster is selected', async () => {
    const { page } = appContext;

    // Navigate to viewer management tab
    await navigateToViewerManagement(page);

    // Select a broadcaster
    const broadcasterSelected = await selectFirstBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Check if viewer list table is displayed
    const table = page.locator('table');
    await expect(table).toBeVisible({ timeout: 5000 });

    // Check for table headers
    await expect(page.locator('th:has-text("表示名")')).toBeVisible();
    await expect(page.locator('th:has-text("読み仮名")')).toBeVisible();
    await expect(page.locator('th:has-text("操作")')).toBeVisible();
  });

  test('should open edit modal when edit button is clicked', async () => {
    const { page } = appContext;

    // Navigate and select broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectFirstBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Click edit button on first viewer
    const editButton = page.locator('button:has-text("編集")').first();
    await expect(editButton).toBeVisible({ timeout: 5000 });
    await editButton.click();
    await page.waitForTimeout(500);

    // Check that modal is visible
    const modal = page.locator('.modal-overlay');
    await expect(modal).toBeVisible({ timeout: 5000 });

    // Check that modal has expected fields
    await expect(page.locator('label:has-text("読み仮名")')).toBeVisible();
    await expect(page.locator('label:has-text("メモ")')).toBeVisible();
    await expect(page.locator('label:has-text("タグ")')).toBeVisible();
    await expect(page.locator('label:has-text("メンバーシップレベル")')).toBeVisible();
  });

  test('should save viewer information when edited', async () => {
    const { page } = appContext;

    // Navigate and select broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectFirstBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Click edit button on first viewer
    await page.locator('button:has-text("編集")').first().click();
    await page.waitForTimeout(500);

    // Generate unique test data
    const timestamp = Date.now();
    const testData = {
      reading: `テスト読み_${timestamp}`,
      notes: `テストメモ_${timestamp}`,
      tags: 'テストタグ1, テストタグ2',
      membership: 'テストレベル',
    };

    // Fill in the form
    const readingInput = page.locator('input[placeholder*="やまだたろう"]');
    const notesTextarea = page.locator('textarea[placeholder*="視聴者に関するメモ"]');
    const tagsInput = page.locator('input[placeholder*="常連"]');
    const membershipInput = page.locator('input[placeholder*="Gold"]');

    await readingInput.fill(testData.reading);
    await notesTextarea.fill(testData.notes);
    await tagsInput.fill(testData.tags);
    await membershipInput.fill(testData.membership);

    // Click save button
    await page.locator('button:has-text("保存")').first().click();
    await page.waitForTimeout(1500);

    // Verify modal is closed
    const modal = page.locator('.modal-overlay');
    await expect(modal).not.toBeVisible({ timeout: 5000 });

    // Re-open the edit modal to verify saved data
    await page.locator('button:has-text("編集")').first().click();
    await page.waitForTimeout(500);

    // Verify saved values
    await expect(readingInput).toHaveValue(testData.reading);
    await expect(notesTextarea).toHaveValue(testData.notes);
    await expect(tagsInput).toHaveValue(testData.tags);
    await expect(membershipInput).toHaveValue(testData.membership);

    // Close modal
    await page.locator('button:has-text("キャンセル")').click();
  });

  test('should display search box in viewer list header', async () => {
    const { page } = appContext;

    // Navigate and select broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectFirstBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Check that the search box is visible
    const searchInput = page.locator('input[placeholder*="検索"]');
    await expect(searchInput).toBeVisible({ timeout: 5000 });

    // Check that the total count display is visible
    const totalCount = page.locator('text=/全 \\d+ 件/');
    await expect(totalCount).toBeVisible({ timeout: 5000 });
  });

  test('should filter viewers when searching', async () => {
    const { page } = appContext;

    // Navigate and select broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectFirstBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Get initial row count
    const table = page.locator('table');
    await expect(table).toBeVisible({ timeout: 5000 });

    const initialRows = await page.locator('table tbody tr').count();
    if (initialRows <= 1) {
      // Not enough data to test filtering
      test.skip();
      return;
    }

    // Type in search box
    const searchInput = page.locator('input[placeholder*="検索"]');
    await searchInput.fill('nonexistent_search_term_12345');
    await page.waitForTimeout(1000);

    // Either rows should be filtered or empty message should appear
    const rowsAfterSearch = await page.locator('table tbody tr').count();
    const emptyMessage = page.locator('text=視聴者データがありません');

    const isFiltered = rowsAfterSearch < initialRows || await emptyMessage.isVisible();
    expect(isFiltered).toBe(true);

    // Clear search
    await searchInput.fill('');
    await page.waitForTimeout(1000);

    // Rows should be restored
    const rowsAfterClear = await page.locator('table tbody tr').count();
    expect(rowsAfterClear).toBe(initialRows);
  });

  test('should display refresh button for broadcaster info', async () => {
    const { page } = appContext;

    // Navigate to viewer management tab
    await navigateToViewerManagement(page);

    // Check that the refresh button is visible (contains emoji + text)
    const refreshButton = page.locator('button', { hasText: '情報を更新' });
    await expect(refreshButton).toBeVisible({ timeout: 5000 });
  });

  test('should have clear button when search has text', async () => {
    const { page } = appContext;

    // Navigate and select broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectFirstBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Type in search box
    const searchInput = page.locator('input[placeholder*="検索"]');
    await searchInput.fill('test');
    await page.waitForTimeout(500);

    // Check that clear button appears (✕)
    const clearButton = page.locator('button:has-text("✕")');
    await expect(clearButton).toBeVisible({ timeout: 3000 });

    // Click clear button
    await clearButton.click();
    await page.waitForTimeout(500);

    // Search input should be empty
    await expect(searchInput).toHaveValue('');
  });

  test('should open delete confirmation dialog when delete button is clicked', async () => {
    const { page } = appContext;

    // Navigate and select test broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectTestBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Click delete button on first viewer
    const deleteButton = page.locator('button:has-text("削除")').first();
    await expect(deleteButton).toBeVisible({ timeout: 5000 });
    await deleteButton.click();
    await page.waitForTimeout(500);

    // Check that confirmation dialog is visible
    const dialog = page.locator('.modal-overlay');
    await expect(dialog).toBeVisible({ timeout: 5000 });

    // Check dialog contains confirmation text
    const confirmText = page.locator('text=/削除.*確認|本当に削除/');
    await expect(confirmText).toBeVisible({ timeout: 3000 });

    // Check that cancel and confirm buttons exist
    await expect(page.locator('button:has-text("キャンセル")')).toBeVisible();

    // Close dialog
    await page.locator('button:has-text("キャンセル")').click();
    await page.waitForTimeout(500);
  });

  test('should delete viewer when confirmed', async () => {
    const { page } = appContext;

    // Navigate and select test broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectTestBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Search for the deletion test viewer
    const searchInput = page.locator('input[placeholder*="検索"]');
    await searchInput.fill('削除テスト用視聴者');
    await page.waitForTimeout(1000);

    // Check viewer is found
    const viewerRow = page.locator('tr', { hasText: '削除テスト用視聴者' });
    const viewerExists = await viewerRow.isVisible({ timeout: 3000 }).catch(() => false);
    if (!viewerExists) {
      console.log('Deletion test viewer not found, skipping test');
      test.skip();
      return;
    }

    // Get initial count
    const initialRows = await page.locator('table tbody tr').count();

    // Click delete button for this viewer
    const deleteButton = viewerRow.locator('button:has-text("削除")');
    await deleteButton.click();
    await page.waitForTimeout(500);

    // Confirm deletion - look for the confirm/delete button in the dialog
    const confirmButton = page.locator('.modal-overlay button:has-text("削除")');
    await expect(confirmButton).toBeVisible({ timeout: 3000 });
    await confirmButton.click();
    await page.waitForTimeout(1500);

    // Verify dialog is closed
    const dialog = page.locator('.modal-overlay');
    await expect(dialog).not.toBeVisible({ timeout: 5000 });

    // Verify viewer is removed from list
    await searchInput.fill('削除テスト用視聴者');
    await page.waitForTimeout(1000);

    const emptyMessage = page.locator('text=視聴者データがありません');
    const deletedViewerRow = page.locator('tr', { hasText: '削除テスト用視聴者' });

    const isDeleted = await emptyMessage.isVisible().catch(() => false) ||
                      !(await deletedViewerRow.isVisible().catch(() => false));
    expect(isDeleted).toBe(true);
  });

  test('should cancel deletion when cancel is clicked', async () => {
    const { page } = appContext;

    // Navigate and select test broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectTestBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Get first viewer name for later verification
    const firstViewerCell = page.locator('table tbody tr td').first();
    await expect(firstViewerCell).toBeVisible({ timeout: 5000 });
    const viewerName = await firstViewerCell.textContent();

    // Click delete button
    const deleteButton = page.locator('button:has-text("削除")').first();
    await deleteButton.click();
    await page.waitForTimeout(500);

    // Cancel deletion
    const cancelButton = page.locator('.modal-overlay button:has-text("キャンセル")');
    await cancelButton.click();
    await page.waitForTimeout(500);

    // Verify dialog is closed
    const dialog = page.locator('.modal-overlay');
    await expect(dialog).not.toBeVisible({ timeout: 3000 });

    // Verify viewer is still in the list
    const viewerStillExists = page.locator(`table tbody tr td:has-text("${viewerName?.split('\n')[0]}")`);
    await expect(viewerStillExists).toBeVisible({ timeout: 3000 });
  });

  test('should display viewer list refresh button', async () => {
    const { page } = appContext;

    // Navigate and select broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectFirstBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Check that the list refresh button is visible
    const refreshButton = page.locator('button', { hasText: 'リスト更新' });
    await expect(refreshButton).toBeVisible({ timeout: 5000 });
  });

  test('should show hamburger menu always visible with backup option', async () => {
    const { page } = appContext;

    // Navigate to viewer management tab
    await navigateToViewerManagement(page);

    // Reset selection first
    const select = page.locator('.broadcaster-selector select');
    await select.selectOption({ index: 0 }); // Select placeholder
    await page.waitForTimeout(500);

    // Check hamburger menu is ALWAYS visible (even without broadcaster selected)
    const menuButton = page.locator('button:has-text("⋮")');
    await expect(menuButton).toBeVisible({ timeout: 5000 });

    // Click hamburger menu
    await menuButton.click();
    await page.waitForTimeout(300);

    // Backup option should always be visible
    const backupOption = page.locator('button:has-text("バックアップを作成")');
    await expect(backupOption).toBeVisible({ timeout: 3000 });

    // Delete option should NOT be visible when no broadcaster is selected
    const deleteOption = page.locator('button:has-text("配信者を削除")');
    await expect(deleteOption).not.toBeVisible({ timeout: 2000 });

    // Close menu by clicking elsewhere
    await page.click('body', { position: { x: 10, y: 10 } });
    await page.waitForTimeout(300);

    // Select a broadcaster
    const broadcasterSelected = await selectFirstBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Click hamburger menu again
    await menuButton.click();
    await page.waitForTimeout(300);

    // Now both options should be visible
    await expect(backupOption).toBeVisible({ timeout: 3000 });
    await expect(deleteOption).toBeVisible({ timeout: 3000 });

    // Close menu
    await page.click('body', { position: { x: 10, y: 10 } });
  });

  test('should create backup and show success dialog with open folder button', async () => {
    const { page } = appContext;

    // Navigate to viewer management tab
    await navigateToViewerManagement(page);

    // Click hamburger menu
    const menuButton = page.locator('button:has-text("⋮")');
    await expect(menuButton).toBeVisible({ timeout: 5000 });
    await menuButton.click();
    await page.waitForTimeout(300);

    // Click backup option
    const backupOption = page.locator('button:has-text("バックアップを作成")');
    await expect(backupOption).toBeVisible({ timeout: 3000 });
    await backupOption.click();

    // Wait for backup to complete and dialog to appear
    await page.waitForTimeout(1000);

    // Check success dialog is visible
    const successDialog = page.locator('h3:has-text("バックアップが完了しました")');
    await expect(successDialog).toBeVisible({ timeout: 10000 });

    // Check that backup path is shown (contains backups directory)
    const pathDisplay = page.locator('text=/liscov_backup_\\d{8}_\\d{6}\\.db/');
    await expect(pathDisplay).toBeVisible({ timeout: 3000 });

    // Check that "Open Folder" button is visible
    const openFolderButton = page.locator('button:has-text("フォルダを開く")');
    await expect(openFolderButton).toBeVisible({ timeout: 3000 });

    // Check that "Close" button is visible
    const closeButton = page.locator('button:has-text("閉じる")');
    await expect(closeButton).toBeVisible({ timeout: 3000 });

    // Click "Open Folder" button (we just verify it's clickable, not that Explorer opens)
    await openFolderButton.click();
    await page.waitForTimeout(500);

    // Dialog should still be open after clicking "Open Folder"
    await expect(successDialog).toBeVisible({ timeout: 3000 });

    // Close the dialog
    await closeButton.click();

    // Verify dialog is closed
    await expect(successDialog).not.toBeVisible({ timeout: 3000 });
  });

  test('should close backup success dialog when clicking overlay', async () => {
    const { page } = appContext;

    // Navigate to viewer management tab
    await navigateToViewerManagement(page);

    // Click hamburger menu
    const menuButton = page.locator('button:has-text("⋮")');
    await menuButton.click();
    await page.waitForTimeout(300);

    // Click backup option
    const backupOption = page.locator('button:has-text("バックアップを作成")');
    await backupOption.click();

    // Wait for success dialog
    const successDialog = page.locator('h3:has-text("バックアップが完了しました")');
    await expect(successDialog).toBeVisible({ timeout: 10000 });

    // Click the overlay (outside the dialog) to close
    // The overlay has position: fixed and covers the entire screen
    await page.click('div[style*="position: fixed"][style*="background: rgba"]', {
      position: { x: 10, y: 10 },
      force: true
    });

    // Verify dialog is closed
    await expect(successDialog).not.toBeVisible({ timeout: 3000 });
  });

  test('should open broadcaster delete confirmation dialog', async () => {
    const { page } = appContext;

    // Navigate and select broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectDeletionTestBroadcaster(page);
    if (!broadcasterSelected) {
      test.skip();
      return;
    }

    // Open hamburger menu
    const menuButton = page.locator('button:has-text("⋮")');
    await expect(menuButton).toBeVisible({ timeout: 5000 });
    await menuButton.click();
    await page.waitForTimeout(300);

    // Click delete option
    const deleteOption = page.locator('button:has-text("配信者を削除")');
    await expect(deleteOption).toBeVisible({ timeout: 3000 });
    await deleteOption.click();
    await page.waitForTimeout(500);

    // Check that confirmation dialog is visible
    const dialog = page.locator('.modal-overlay');
    await expect(dialog).toBeVisible({ timeout: 5000 });

    // Check dialog contains header text
    const headerText = page.locator('h3:has-text("配信者データの削除")');
    await expect(headerText).toBeVisible({ timeout: 3000 });

    // Check dialog shows viewer count to be deleted
    const viewerCountText = page.locator('text=/視聴者カスタム情報.*件/');
    await expect(viewerCountText).toBeVisible({ timeout: 3000 });

    // Cancel the dialog
    await page.locator('.modal-overlay button:has-text("キャンセル")').click();
    await page.waitForTimeout(500);

    // Verify dialog is closed
    await expect(dialog).not.toBeVisible({ timeout: 3000 });
  });

  test('should delete broadcaster and associated viewers when confirmed', async () => {
    const { page } = appContext;

    // Navigate and select the deletion test broadcaster
    await navigateToViewerManagement(page);
    const broadcasterSelected = await selectDeletionTestBroadcaster(page);
    if (!broadcasterSelected) {
      console.log('Deletion test broadcaster not found, skipping test');
      test.skip();
      return;
    }

    // Verify we have viewers for this broadcaster
    const table = page.locator('table');
    await expect(table).toBeVisible({ timeout: 5000 });

    const initialRows = await page.locator('table tbody tr').count();
    expect(initialRows).toBeGreaterThan(0);

    // Open hamburger menu
    const menuButton = page.locator('button:has-text("⋮")');
    await menuButton.click();
    await page.waitForTimeout(300);

    // Click delete option
    const deleteOption = page.locator('button:has-text("配信者を削除")');
    await deleteOption.click();
    await page.waitForTimeout(500);

    // Confirm deletion
    const confirmButton = page.locator('.modal-overlay button:has-text("削除")');
    await expect(confirmButton).toBeVisible({ timeout: 3000 });
    await confirmButton.click();

    // Wait for dialog to close and dropdown to refresh
    const dialog = page.locator('.modal-overlay');
    await expect(dialog).not.toBeVisible({ timeout: 5000 });

    // Wait for dropdown to refresh
    await page.waitForTimeout(2000);

    // Verify broadcaster is no longer in the dropdown
    const select = page.locator('.broadcaster-selector select');
    const options = await select.locator('option').all();

    let broadcasterStillExists = false;
    for (const opt of options) {
      const value = await opt.getAttribute('value');
      if (value === 'UC_TEST_BROADCASTER_DEL') {
        broadcasterStillExists = true;
        break;
      }
    }
    expect(broadcasterStillExists).toBe(false);

    // Verify placeholder message is shown (no broadcaster selected)
    const placeholder = page.locator('text=配信者チャンネルを選択してください');
    await expect(placeholder).toBeVisible({ timeout: 5000 });
  });
});

/**
 * Navigate to the viewer management tab
 */
async function navigateToViewerManagement(page: Page): Promise<void> {
  await closeModals(page);
  await page.click('button:has-text("視聴者管理")');
  await page.waitForTimeout(1000);
}

/**
 * Select the first available broadcaster from the dropdown
 * Returns true if a broadcaster was selected, false if none available
 */
async function selectFirstBroadcaster(page: Page): Promise<boolean> {
  // Find the broadcaster selector within the broadcaster-selector container
  const select = page.locator('.broadcaster-selector select');

  if (!(await select.isVisible({ timeout: 3000 }).catch(() => false))) {
    console.log('Broadcaster selector not found');
    return false;
  }

  // Get available options
  const options = await select.locator('option').all();
  if (options.length <= 1) {
    console.log('No broadcasters available');
    return false;
  }

  // Find and select first broadcaster option (starts with UC)
  for (const opt of options) {
    const value = await opt.getAttribute('value');
    if (value && value.startsWith('UC')) {
      await select.selectOption(value);
      await page.waitForTimeout(1500);
      return true;
    }
  }

  // If no UC channel found, select the second option (first non-placeholder)
  await select.selectOption({ index: 1 });
  await page.waitForTimeout(1500);
  return true;
}

/**
 * Select the test broadcaster (UC_TEST_BROADCASTER_001) from the dropdown
 * Returns true if selected, false if not found
 */
async function selectTestBroadcaster(page: Page): Promise<boolean> {
  const select = page.locator('.broadcaster-selector select');

  if (!(await select.isVisible({ timeout: 3000 }).catch(() => false))) {
    console.log('Broadcaster selector not found');
    return false;
  }

  // Try to select test broadcaster
  const testBroadcasterId = 'UC_TEST_BROADCASTER_001';
  const options = await select.locator('option').all();

  for (const opt of options) {
    const value = await opt.getAttribute('value');
    if (value === testBroadcasterId) {
      await select.selectOption(testBroadcasterId);
      await page.waitForTimeout(1500);
      return true;
    }
  }

  // Test broadcaster not found, fall back to first available
  console.log('Test broadcaster not found, falling back to first available');
  return selectFirstBroadcaster(page);
}

/**
 * Select the deletion test broadcaster (UC_TEST_BROADCASTER_DEL) from the dropdown
 * Returns true if selected, false if not found (does NOT fall back)
 */
async function selectDeletionTestBroadcaster(page: Page): Promise<boolean> {
  const select = page.locator('.broadcaster-selector select');

  if (!(await select.isVisible({ timeout: 3000 }).catch(() => false))) {
    console.log('Broadcaster selector not found');
    return false;
  }

  // Try to select deletion test broadcaster
  const deletionBroadcasterId = 'UC_TEST_BROADCASTER_DEL';
  const options = await select.locator('option').all();

  for (const opt of options) {
    const value = await opt.getAttribute('value');
    if (value === deletionBroadcasterId) {
      await select.selectOption(deletionBroadcasterId);
      await page.waitForTimeout(1500);
      return true;
    }
  }

  console.log('Deletion test broadcaster not found');
  return false;
}
