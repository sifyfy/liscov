import { test, expect } from './utils/fixtures';
import type { Browser, Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import fs from 'fs';
import path from 'path';
import { log } from './utils/logger';
import {
  PROJECT_DIR,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  connectToMockStream,
  navigateToTab,
} from './utils/test-helpers';
import {
  type MockMessage,
  normalMessages,
  longMessages,
  superMessages,
  heavyMessages,
} from './utils/design-capture-data';

/**
 * デザインの撮影 (テストではない)
 *
 * 主要な画面を、データの状態 × テーマ × 幅 で撮影し、デザインの診断と批評に使う。
 * 通常の E2E では動かさない (ファイル名が *.spec.ts ではない)。`pnpm design:capture` で動かす。
 * 出力先のフォルダ名は LISCOV_DESIGN_CAPTURE_LABEL で変えられる (既定は latest)。
 * 出力は .tmp/design-capture/ (git に入れない)。データは架空のものだけを使う。
 * 各状態・テーマで、いちばん広い幅のときに axe (WCAG 2 AA) のコントラスト検査も行い、<名前>-<テーマ>.axe.json に残す。
 */

const OUT_DIR = path.join(PROJECT_DIR, '.tmp', 'design-capture', process.env.LISCOV_DESIGN_CAPTURE_LABEL ?? 'latest');

// 幅は、サブモニターの細いウィンドウ・半分・広めの 3 つ (ブリーフで面積の目標が決まったら見直す)
const WIDTHS = [360, 640, 1024];
const HEIGHT = 800;
const THEMES = ['dark', 'light'] as const;

test.describe('Design capture', () => {
  let browser: Browser;
  let page: Page;

  test.beforeAll(async () => {
    test.setTimeout(240000);
    fs.rmSync(OUT_DIR, { recursive: true, force: true });
    fs.mkdirSync(OUT_DIR, { recursive: true });
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    page = connection.page;
    // テーマ切替の途中の色を撮ったり検査したりしないよう、transition を止める
    await page.addStyleTag({ content: '*, *::before, *::after { transition: none !important; }' });
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  /** 現在の画面を、テーマ × 幅 のすべてで撮る */
  async function captureAll(name: string): Promise<void> {
    for (const theme of THEMES) {
      await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), theme);
      for (const width of WIDTHS) {
        await page.setViewportSize({ width, height: HEIGHT });
        await page.mouse.move(-1, -1);
        // レイアウトの再計算と仮想リストの再描画を待つ
        await page.waitForTimeout(300);
        const file = path.join(OUT_DIR, `${name}-${theme}-${width}.png`);
        await page.screenshot({ path: file, animations: 'disabled' });
        log.info(`captured ${path.basename(file)}`);
      }
      await checkContrast(`${name}-${theme}`);
    }
  }

  /** コントラストの検査結果を残す。未検査 (incomplete) も合格ではないので一緒に残す */
  async function checkContrast(name: string): Promise<void> {
    const result = await new AxeBuilder({ page }).withRules(['color-contrast']).analyze();
    const pick = (items: typeof result.violations) =>
      items.flatMap((item) =>
        item.nodes.map((node) => ({ target: node.target.join(' '), summary: node.failureSummary ?? '' })),
      );
    const report = { violations: pick(result.violations), incomplete: pick(result.incomplete) };
    fs.writeFileSync(path.join(OUT_DIR, `${name}.axe.json`), JSON.stringify(report, null, 2));
    log.info(`contrast ${name}: violations=${report.violations.length} incomplete=${report.incomplete.length}`);
  }

  async function addMessages(messages: readonly MockMessage[]): Promise<void> {
    for (const m of messages) await addMockMessage(m);
  }

  test('chat: 状態を積み上げながら撮る', async () => {
    test.setTimeout(600000);
    await resetMockServer();

    await captureAll('chat-0-disconnected');

    await connectToMockStream(page);
    await captureAll('chat-1-empty');

    await addMessages(normalMessages);
    await expect(page.getByText(normalMessages.at(-1)!.content).first()).toBeVisible({ timeout: 15000 });
    await captureAll('chat-2-normal');

    await addMessages(longMessages);
    await expect(page.getByText(longMessages[0].author).first()).toBeVisible({ timeout: 15000 });
    await captureAll('chat-3-long');

    await addMessages(superMessages);
    await expect(page.getByText('¥10,000').first()).toBeVisible({ timeout: 15000 });
    await captureAll('chat-4-supers');

    await addMessages(heavyMessages);
    await page.waitForTimeout(3000);
    await captureAll('chat-5-heavy');

    // 2 番目の用途: 視聴者の補足情報の確認・登録
    await page.getByText(superMessages[0].author).first().click();
    await expect(page.locator('h2:has-text("視聴者情報")')).toBeVisible({ timeout: 5000 });
    await captureAll('chat-6-viewer-panel');
    await page.locator('button:has-text("✕")').first().click();
  });

  test('その他のタブ', async () => {
    test.setTimeout(600000);
    await navigateToTab(page, 'Viewers');
    await captureAll('viewers');

    await navigateToTab(page, 'Analytics');
    await captureAll('analytics');

    await navigateToTab(page, 'Settings');
    for (const [label, name] of [
      ['YouTube認証', 'settings-auth'],
      ['TTS読み上げ', 'settings-tts'],
      ['生レスポンス保存', 'settings-raw'],
      ['UIテーマ', 'settings-theme'],
    ] as const) {
      await page.locator(`button:has-text("${label}")`).first().click();
      await captureAll(name);
    }
  });
});
