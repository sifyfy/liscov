/**
 * App Launcher for E2E Testing
 *
 * This module provides utilities to connect to the liscov desktop app
 * via Chrome DevTools Protocol (CDP) for E2E testing with Playwright.
 *
 * Usage:
 *   1. Build the app: cargo build --release
 *   2. Launch with CDP: WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 ./target/release/liscov.exe
 *   3. Run tests: npm test
 */

import { spawn, ChildProcess } from 'child_process';
import { chromium, Browser, Page } from 'playwright';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

/** Default CDP port for WebView2 debugging */
const DEFAULT_CDP_PORT = 9223;

/** Get CDP port from environment or use default */
export function getCdpPort(): number {
  return parseInt(process.env.CDP_PORT || String(DEFAULT_CDP_PORT), 10);
}

/** Project root directory */
const PROJECT_ROOT = path.resolve(__dirname, '../..');

/** App context containing process and browser handles */
export interface AppContext {
  process: ChildProcess | null;
  browser: Browser;
  page: Page;
}

/**
 * Wait for CDP to be available on the specified port
 */
async function waitForCDP(port: number, timeout: number = 30000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`http://localhost:${port}/json/version`);
      if (response.ok) {
        console.log(`CDP available on port ${port}`);
        return;
      }
    } catch {
      // CDP not ready yet
    }
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  throw new Error(`CDP did not become available on port ${port} within ${timeout}ms`);
}

/**
 * Launch the liscov desktop app with CDP enabled
 *
 * Note: This requires the app to be pre-built with `cargo build --release`
 */
export async function launchApp(): Promise<AppContext> {
  const cdpPort = getCdpPort();

  // Create a unique user data folder for this test run
  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'liscov-test-'));

  console.log(`Launching app with CDP on port ${cdpPort}...`);

  // Determine the executable path based on platform
  const exeName = process.platform === 'win32' ? 'liscov.exe' : 'liscov';
  const exePath = path.join(PROJECT_ROOT, 'target', 'release', exeName);

  if (!fs.existsSync(exePath)) {
    throw new Error(
      `App executable not found at ${exePath}. Please build the app first with: cargo build --release`
    );
  }

  // Launch the app with CDP enabled
  const appProcess = spawn(exePath, [], {
    cwd: PROJECT_ROOT,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
      WEBVIEW2_USER_DATA_FOLDER: userDataDir,
    },
    detached: false,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  appProcess.stdout?.on('data', (data) => {
    if (process.env.DEBUG) {
      console.log(`[App STDOUT]: ${data}`);
    }
  });

  appProcess.stderr?.on('data', (data) => {
    if (process.env.DEBUG) {
      console.error(`[App STDERR]: ${data}`);
    }
  });

  // Wait for CDP to be available
  console.log('Waiting for CDP...');
  await waitForCDP(cdpPort);

  // Connect Playwright to the app via CDP
  console.log('Connecting Playwright via CDP...');
  const browser = await chromium.connectOverCDP(`http://localhost:${cdpPort}`);

  // Get the existing page (WebView2 content)
  const contexts = browser.contexts();
  if (contexts.length === 0) {
    throw new Error('No browser contexts found');
  }

  const pages = contexts[0].pages();
  if (pages.length === 0) {
    throw new Error('No pages found in the browser context');
  }

  const page = pages[0];

  // Wait for the app to be fully loaded
  await page.waitForLoadState('domcontentloaded');

  return { process: appProcess, browser, page };
}

/**
 * Connect to an already running app via CDP
 *
 * This is useful when you want to manually launch the app and then run tests.
 */
export async function connectToApp(): Promise<AppContext> {
  const cdpPort = getCdpPort();

  console.log(`Connecting to app via CDP on port ${cdpPort}...`);

  // Wait for CDP to be available
  await waitForCDP(cdpPort, 5000).catch(() => {
    throw new Error(
      `App is not running with CDP enabled on port ${cdpPort}.\n` +
      `Please start the app with:\n` +
      `  Windows: set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=${cdpPort} && target\\release\\liscov.exe\n` +
      `  Linux/Mac: WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=${cdpPort} ./target/release/liscov`
    );
  });

  // Connect Playwright to the app via CDP
  const browser = await chromium.connectOverCDP(`http://localhost:${cdpPort}`);

  const contexts = browser.contexts();
  if (contexts.length === 0) {
    throw new Error('No browser contexts found');
  }

  const pages = contexts[0].pages();
  if (pages.length === 0) {
    throw new Error('No pages found in the browser context');
  }

  const page = pages[0];
  await page.waitForLoadState('domcontentloaded');

  return { process: null, browser, page };
}

/**
 * Close the app and cleanup
 */
export async function closeApp(context: AppContext): Promise<void> {
  await context.browser.close();

  // Kill the app process if we launched it
  if (context.process && !context.process.killed) {
    context.process.kill();
  }
}

/**
 * Close any open modals by pressing Escape or clicking cancel button
 */
export async function closeModals(page: Page): Promise<void> {
  const modal = page.locator('.modal-overlay');
  if (await modal.isVisible()) {
    // Try pressing Escape
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);

    // If still visible, try clicking cancel button
    if (await modal.isVisible()) {
      const cancelBtn = page.locator('button:has-text("キャンセル"), button:has-text("×")').first();
      if (await cancelBtn.isVisible()) {
        await cancelBtn.click();
        await page.waitForTimeout(300);
      }
    }
  }
}
