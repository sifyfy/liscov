import { defineConfig } from '@playwright/test';

/**
 * Playwright config for E2E testing the actual Tauri application.
 *
 * This config connects to a running Tauri app via CDP (Chrome DevTools Protocol).
 * WebView2 must be started with --remote-debugging-port enabled.
 *
 * Usage:
 * 1. Start mock server: cargo run --manifest-path src-tauri/Cargo.toml --bin mock_server
 * 2. Start Tauri app with debug port:
 *    set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
 *    set LISCOV_AUTH_URL=http://localhost:3456/
 *    pnpm tauri dev
 * 3. Run tests: pnpm exec playwright test --config e2e/playwright.config.ts
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
