import { defineConfig } from '@playwright/test';

/**
 * Playwright config for E2E testing the actual Tauri application.
 *
 * This config connects to a running Tauri app via CDP (Chrome DevTools Protocol).
 * The CDP port is chosen by WebView2 (--remote-debugging-port=0) and read from DevToolsActivePort,
 * so that E2E runs in multiple worktrees do not interfere (docs/decisions/005_e2e_worktree_isolation.md).
 *
 * Usage: `pnpm test:e2e` (builds, then runs). The mock server, the static frontend
 * server and the Tauri app are started by setupTestEnvironment() in utils/test-helpers.ts.
 * See e2e/README.md.
 */
export default defineConfig({
  testDir: '.',
  // utils/*.test.ts は E2E 基盤の vitest の単体テスト。Playwright の既定の testMatch は *.test.ts も拾うので、spec だけに絞る
  testMatch: '**/*.spec.ts',
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
      // CDP の接続先は起動ごとに変わるので、utils/test-helpers.ts の connectToApp() が接続する (ADR-005)
      name: 'tauri-webview',
    },
  ],
});
