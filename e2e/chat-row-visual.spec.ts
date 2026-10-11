import { test, expect } from './utils/fixtures';
import type { Browser, Page } from '@playwright/test';
import { setupTestEnvironment, teardownTestEnvironment } from './utils/test-helpers';

/**
 * コメントの行の見た目の回帰テスト (docs/specs/02_chat.md「メッセージ表示仕様」、docs/design/tokens.md)
 *
 * /design-tokens の「コメントの行」(本物の ChatMessage を固定の架空データで並べたもの) と
 * 「流れたものの欄 (スパチャ等の一覧の概要と詳細を含む)」を撮り、基準の画像と比べる。
 * 優先順位 1〜5・種類・相手の配信の札・読み仮名・折り返し・詰めた形・短い札を守る。
 * データは固定で時刻も文字列なので、モックサーバーやタイムゾーンに左右されない。
 *
 * 見た目を意図して変えたときは、撮影で確かめてから基準を更新する:
 *    pnpm exec playwright test --config e2e/playwright.config.ts chat-row-visual --update-snapshots
 */

const THEMES = ['dark', 'light'] as const;

test.describe('コメントの行の見た目', () => {
  let browser: Browser;
  let page: Page;

  test.beforeAll(async () => {
    test.setTimeout(120000);
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    page = connection.page;
    await page.evaluate(() => location.assign('/design-tokens'));
    await expect(page.getByTestId('row-samples')).toBeVisible({ timeout: 15000 });
    // テーマ切替の途中の色を撮らないよう、transition を止める
    await page.addStyleTag({ content: '*, *::before, *::after { transition: none !important; }' });
    // いちばん小さい想定の幅 (docs/design/brief.md §3)
    await page.setViewportSize({ width: 540, height: 960 });
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  for (const theme of THEMES) {
    test(`${theme} テーマ`, async () => {
      await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), theme);
      await page.mouse.move(-1, -1);
      await expect(page.getByTestId('row-samples')).toHaveScreenshot(`chat-rows-${theme}.png`, { animations: 'disabled' });
      // 流れたものの欄 (スパチャ等の一覧の概要と詳細を含む) (docs/specs/02_chat.md「流れたものの欄」「詰めた形」)
      await expect(page.getByTestId('flow-samples')).toHaveScreenshot(`chat-flow-${theme}.png`, { animations: 'disabled' });
    });
  }
});
