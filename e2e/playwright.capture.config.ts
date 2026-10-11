import { defineConfig } from '@playwright/test';
import baseConfig from './playwright.config';

/**
 * デザインの撮影用の設定 (`pnpm design:capture`)
 *
 * 撮影はテストではないので、通常の E2E (*.spec.ts) とファイル名で分ける。
 * 接続先 (WebView2 の CDP) やレポーターは通常の E2E と同じ。
 */
export default defineConfig({
  ...baseConfig,
  testMatch: '**/*.capture.ts',
  retries: 0,
});
