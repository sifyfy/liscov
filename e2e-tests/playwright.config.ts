import { defineConfig } from '@playwright/test';

/**
 * Playwright configuration for liscov E2E tests
 *
 * Note: This configuration is for testing a desktop app via CDP (Chrome DevTools Protocol).
 * The app must be launched with CDP enabled before running tests.
 */
export default defineConfig({
  testDir: './tests',

  // Timeout settings
  timeout: 60000,
  expect: {
    timeout: 10000,
  },

  // Run tests sequentially (desktop app can only have one instance)
  fullyParallel: false,
  workers: 1,

  // CI settings
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,

  // Reporter settings
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['list'],
  ],

  // Global settings
  use: {
    // Trace and screenshot settings
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'on-first-retry',
  },

  // Output directory for test artifacts
  outputDir: 'test-results',
});
