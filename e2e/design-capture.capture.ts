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
  openControlPanel,
  closeControlPanel,
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
 * 各状態・テーマで、最後の大きさ (いちばん広い幅) のときに axe (WCAG 2 AA) のコントラスト検査も行い、<名前>-<テーマ>.axe.json に残す。
 */

const OUT_DIR = path.join(PROJECT_DIR, '.tmp', 'design-capture', process.env.LISCOV_DESIGN_CAPTURE_LABEL ?? 'latest');

// 設計の基準にする大きさ (docs/design/brief.md §3): いちばん小さい・いちばんよく使われる想定・広め
const SIZES = [
  { width: 540, height: 960 },
  { width: 640, height: 1080 },
  { width: 960, height: 1080 },
] as const;
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
      for (const { width, height } of SIZES) {
        await page.setViewportSize({ width, height });
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

    // 合間・準備: 操作パネルを開いたところ (02_chat.md「上の行と操作パネル」)
    await openControlPanel(page);
    await captureAll('chat-2-panel');
    // いちばん小さい窓 (10_window_state.md の最小サイズ 540x600) でも操作パネルと流れが使えるか
    for (const theme of THEMES) {
      await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), theme);
      await page.setViewportSize({ width: 540, height: 600 });
      await page.waitForTimeout(300);
      await page.screenshot({ path: path.join(OUT_DIR, `chat-2-panel-${theme}-540x600.png`), animations: 'disabled' });
    }
    await closeControlPanel(page);

    await addMessages(longMessages);
    await expect(page.getByText(longMessages[0].author).first()).toBeVisible({ timeout: 15000 });
    await captureAll('chat-3-long');

    await addMessages(superMessages);
    await expect(page.getByText('¥10,000').first()).toBeVisible({ timeout: 15000 });
    await captureAll('chat-4-supers');

    await addMessages(heavyMessages);
    await page.waitForTimeout(3000);
    await captureAll('chat-5-heavy');

    // 配信の最後のお礼: この配信のスパチャ等の一覧を開く (02_chat.md「配信の最後のお礼」)
    await page.getByRole('button', { name: /この配信のスパチャ等の一覧/ }).click();
    await captureAll('chat-5-thanks');
    await page.locator('section[aria-label="この配信のスパチャ等"]').getByRole('button', { name: '閉じる' }).click();

    // 手でスクロールして最新から離れた: 「最新へ」と、戻るまでの残り時間の帯 (02_chat.md「自動スクロール」)
    await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'));
    await page.setViewportSize({ width: 640, height: 1080 });
    await page.locator('[style*="overflow-y"]').filter({ has: page.locator('[data-message-id]') }).first().hover();
    await page.mouse.wheel(0, -1500);
    await expect(page.getByRole('button', { name: /最新へ/ })).toBeVisible({ timeout: 5000 });
    await page.mouse.move(-1, -1);
    // 帯の縮む途中 (animations: 'disabled' だと縮み切った形で撮れるので、ここは動きを止めずに撮る)
    await page.waitForTimeout(5000);
    await page.screenshot({ path: path.join(OUT_DIR, 'chat-5-away-bar-dark-640.png') });
    await captureAll('chat-5-away');

    // 2 番目の用途: 視聴者の補足情報の確認・登録
    // (押した行のすぐ下に広がる。02_chat.md「視聴者の読み仮名・メモ」)
    await page.getByText(superMessages[0].author).first().click();
    const panel = page.getByRole('region', { name: '視聴者情報' });
    await expect(panel.getByLabel('読み仮名')).toBeEnabled({ timeout: 5000 });
    await captureAll('chat-6-viewer-panel');
    await panel.getByRole('button', { name: /この人のコメント/ }).click();
    await captureAll('chat-6-viewer-panel-history');
    await panel.getByLabel('読み仮名').press('Escape');
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

  // 他の画面を撮り終えてから移動する (見本ページはナビゲーションから開けない)
  test('デザイントークンの見本', async () => {
    test.setTimeout(120000);
    await page.evaluate(() => location.assign('/design-tokens'));
    await expect(page.getByText('デザイントークンの見本')).toBeVisible({ timeout: 15000 });
    await page.addStyleTag({ content: '*, *::before, *::after { transition: none !important; }' });
    await page.setViewportSize(SIZES[2]);
    for (const theme of THEMES) {
      await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), theme);
      await page.waitForTimeout(300);
      await page.screenshot({ path: path.join(OUT_DIR, `design-tokens-${theme}.png`), fullPage: true, animations: 'disabled' });
      await checkContrast(`design-tokens-${theme}`);
    }
  });

  // 画面の枠の試作 (段階 5.2c、src/routes/design-proto/frame)。中身は同じで、操作と状態の置き場所だけを比べる
  test('枠の試作', async () => {
    test.setTimeout(600000);
    for (const v of ['a', 'b', 'c'] as const) {
      for (const st of ['live', 'open', 'empty'] as const) {
        await page.evaluate((p) => location.assign(p), `/design-proto/frame?v=${v}&st=${st}`);
        await expect(page.locator('main')).toBeVisible({ timeout: 15000 });
        await page.waitForTimeout(500);
        await page.addStyleTag({ content: '*, *::before, *::after { transition: none !important; }' });
        await captureAll(`frame-${v}-${st}`);
      }
    }
  });

  // 視聴者情報パネルの置き方の試作 (段階 5.3)。A 右に置く / B 下に開く / C 行の中で広げる
  test('視聴者情報の試作', async () => {
    test.setTimeout(600000);
    for (const v of ['a', 'b', 'c'] as const) {
      await page.evaluate((p) => location.assign(p), `/design-proto/viewer?v=${v}`);
      await expect(page.getByRole('region', { name: '視聴者情報' }).or(page.getByRole('complementary', { name: '視聴者情報' }))).toBeVisible({ timeout: 15000 });
      await page.addStyleTag({ content: '*, *::before, *::after { transition: none !important; }' });
      await captureAll(`viewer-${v}`);
    }
  });

  // 視聴者のタブの試作 (段階 5.3b)。A 表 / B 一覧と詳細 / C 2 行の一覧。一覧・編集中・削除の確認
  test('視聴者のタブの試作', async () => {
    test.setTimeout(600000);
    for (const v of ['a', 'b', 'c'] as const) {
      for (const st of ['list', 'edit', 'delete'] as const) {
        await page.evaluate((p) => location.assign(p), `/design-proto/viewers?v=${v}&st=${st}`);
        await expect(page.getByRole('searchbox', { name: '視聴者を探す' })).toBeVisible({ timeout: 15000 });
        await page.addStyleTag({ content: '*, *::before, *::after { transition: none !important; }' });
        await captureAll(`viewers-${v}-${st}`);
      }
    }
  });

  // 方向性の試作 (src/routes/design-proto)。架空のデータを持っているので、モックサーバーは使わない。
  // 場面 (docs/design/brief.md §1) ごとに撮る: 通常・コラボ・配信の最後のお礼・参加型
  test('方向性の試作', async () => {
    test.setTimeout(600000);
    const SCENES = ['normal', 'collab', 'thanks', 'party'] as const;
    for (const id of ['a', 'b', 'c'] as const) {
      // 選ばれた案 C は、スパチャ等と新規さんが大量に流れた後 (heavy) も撮る
      for (const scene of id === 'c' ? [...SCENES, 'heavy'] : SCENES) {
        await page.evaluate((p) => location.assign(p), `/design-proto/${id}?scene=${scene}`);
        await expect(page.getByText('倒したああああ').first()).toBeVisible({ timeout: 15000 });
        await page.addStyleTag({ content: '*, *::before, *::after { transition: none !important; }' });
        await captureAll(`proto-${id}-${scene}`);
      }
    }
  });
});
