import { defineConfig } from '@playwright/test';

/**
 * Playwright config for E2E testing the actual Tauri application.
 *
 * This config connects to a running Tauri app via CDP (Chrome DevTools Protocol).
 * WebView2 must be started with --remote-debugging-port enabled.
 *
 * Usage: `pnpm test:e2e` (builds, then runs). The mock server, the static frontend
 * server and the Tauri app are started by setupTestEnvironment() in utils/test-helpers.ts.
 * See e2e/README.md.
 */
export default defineConfig({
  testDir: '.',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 1,
  workers: 1,
  // list レポーターの後に buffered-log-reporter を配置することで、
  // テスト結果行（✓/✘）の後にログが表示される
  reporter: [['list'], ['./utils/buffered-log-reporter.ts']],
  timeout: 60000,
  use: {
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'tauri-webview',
      use: {
        // Connect to WebView2 via CDP
        connectOptions: {
          wsEndpoint: 'ws://127.0.0.1:9222',
        },
      },
    },
  ],
});
