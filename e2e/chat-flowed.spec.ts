import { test, expect } from './utils/fixtures';
import type { Browser, Page } from '@playwright/test';
import fs from 'fs';
import path from 'path';
import {
  APP_READY,
  getTestAppDataDir,
  setupTestEnvironment,
  teardownTestEnvironment,
  resetMockServer,
  addMockMessage,
  connectToMockStream,
  disconnectAndInitialize,
} from './utils/test-helpers';

/**
 * 流れたものの欄とお礼の一覧 (docs/specs/02_chat.md「配信中の画面」の「流れたもの」「配信の最後のお礼」)
 *
 * 仕様の例: ¥10,000 の赤スパのあとにコメントが 40 件来て流れの画面から外れた → 「流れたスパチャ等」の欄の先頭に出る
 * 仕様の例 (欄の高さ): 境目を 60px 上へ引くと 60px 高くなり、離すと割合を覚える。ダブルクリックで既定 (1/8) に戻る
 */

/** config.toml に保存された流れた新規さんの欄の高さの割合 (無ければ null) */
function readNewcomersHeightRatio(): number | null {
  const configPath = path.join(getTestAppDataDir(), 'config.toml');
  if (!fs.existsSync(configPath)) return null;
  const match = fs.readFileSync(configPath, 'utf-8').match(/flowed_newcomers_height_ratio\s*=\s*([\d.]+)/);
  return match ? Number(match[1]) : null;
}

// 視聴者は実行ごとに新しい channel_id にする (初見の判定はテスト用の DB に残るため)
const RUN = Date.now().toString(36);

test.describe('流れたものの欄とお礼の一覧', () => {
  let browser: Browser;
  let page: Page;

  test.beforeAll(async () => {
    test.setTimeout(120000);
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    page = connection.page;
    await page.setViewportSize({ width: 640, height: 1080 });
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test('流れて画面の外に出たスパチャと新規さんが欄に残り、お礼の一覧で時刻順に見られる', async () => {
    test.setTimeout(120000);
    await resetMockServer();
    await connectToMockStream(page);

    await addMockMessage({ message_type: 'text', author: 'NewViewer', content: 'はじめまして', channel_id: `UC_new_${RUN}` });
    await addMockMessage({
      message_type: 'superchat',
      author: 'RedDonor',
      content: '遅れてきました！',
      amount: '¥10,000',
      tier: 'red',
      channel_id: `UC_red_${RUN}`,
    });
    // 同じ視聴者の 40 件 (初回の 1 件だけが新規さんの初回)
    for (let i = 0; i < 40; i++) {
      await addMockMessage({ message_type: 'text', author: 'Filler', content: `コメント ${i}`, channel_id: `UC_filler_${RUN}` });
    }
    await expect(page.getByText('コメント 39')).toBeVisible({ timeout: 15000 });

    // 流れた: スパチャと、新規さんの初回コメントが、それぞれの欄に出る
    const flowedSupers = page.locator('section[aria-label="流れたスパチャ等"]');
    const flowedNewcomers = page.locator('section[aria-label="流れた新規さん"]');
    await expect(flowedSupers).toContainText('RedDonor', { timeout: 5000 });
    await expect(flowedSupers).toContainText('¥10,000');
    await expect(flowedSupers).toContainText('遅れてきました！');
    await expect(flowedNewcomers).toContainText('NewViewer');

    // 配信の最後のお礼: 一覧を開く (1 手)。流れたスパチャ等の欄は一覧に含まれるので消え、最新のコメントは見えたまま
    await page.getByRole('button', { name: /この配信のスパチャ等の一覧/ }).click();
    const thanks = page.locator('section[aria-label="この配信のスパチャ等"]');
    await expect(thanks).toContainText('RedDonor');
    await expect(thanks).toContainText('1 件・時刻順');
    await expect(flowedSupers).toHaveCount(0);
    await expect(flowedNewcomers).toContainText('NewViewer');
    await expect(page.getByText('コメント 39')).toBeVisible();

    await thanks.getByRole('button', { name: '閉じる' }).click();
    await expect(thanks).toHaveCount(0);
    await expect(flowedSupers).toContainText('RedDonor');

    // 手でスクロールして戻り (自動スクロールは止まる)、また見えたら欄から外れる
    await page.getByText('コメント 39').hover();
    await page.mouse.wheel(0, -20000);
    await expect(page.getByText('遅れてきました！').first()).toBeVisible({ timeout: 5000 });
    await expect(flowedSupers).toHaveCount(0, { timeout: 5000 });
    await expect(flowedNewcomers).toHaveCount(0);

    await disconnectAndInitialize(page);
  });

  test('欄の上の境目をドラッグすると高さが変わり、再読み込みしても覚えている。ダブルクリックで既定に戻る', async () => {
    test.setTimeout(180000);
    const flowedNewcomers = page.locator('section[aria-label="流れた新規さん"]');
    const handle = flowedNewcomers.getByRole('separator', { name: '流れた新規さんの高さ' });

    /** 新規さんを 30 人積んで流し、欄を中身が多い状態にする (実行ごと・回ごとに新しい視聴者) */
    const fillFlowedNewcomers = async (round: string) => {
      await resetMockServer();
      await connectToMockStream(page);
      for (let i = 0; i < 30; i++) {
        await addMockMessage({ message_type: 'text', author: `LongNewcomerName${i}`, content: `はじめまして ${i}`, channel_id: `UC_h_${round}_${i}_${RUN}` });
      }
      for (let i = 0; i < 40; i++) {
        await addMockMessage({ message_type: 'text', author: 'Filler', content: `埋め ${round} ${i}`, channel_id: `UC_hfill_${RUN}` });
      }
      await expect(page.getByText(`埋め ${round} 39`)).toBeVisible({ timeout: 15000 });
      await expect(flowedNewcomers).toContainText('LongNewcomerName0', { timeout: 5000 });
    };
    const height = async () => (await flowedNewcomers.boundingBox())?.height ?? 0;

    await fillFlowedNewcomers('a');
    const before = await height();

    // 境目を 60px 上へ引いて離す
    const box = await handle.boundingBox();
    if (!box) throw new Error('境目が見つからない');
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 - 60, { steps: 5 });
    await page.mouse.up();
    await expect.poll(height).toBeGreaterThan(before + 55);
    expect(await height()).toBeLessThan(before + 65);
    await expect.poll(readNewcomersHeightRatio).toBeGreaterThan(0.125);
    const saved = readNewcomersHeightRatio();
    const after = await height();
    await disconnectAndInitialize(page);

    // 再読み込み (F5) しても、同じ高さで出る
    await page.reload();
    await expect(page.locator(APP_READY)).toBeVisible({ timeout: 30000 });
    await fillFlowedNewcomers('b');
    await expect.poll(height).toBeGreaterThan(after - 3);
    expect(await height()).toBeLessThan(after + 3);
    expect(readNewcomersHeightRatio()).toBe(saved);

    // ダブルクリックで既定 (1/8) に戻る
    await handle.dblclick();
    await expect.poll(readNewcomersHeightRatio).toBe(0.125);
    await expect.poll(height).toBeLessThan(before + 3);

    await disconnectAndInitialize(page);
  });
});
