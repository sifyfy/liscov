import { test, expect } from './utils/fixtures';
import type { Browser, Page } from '@playwright/test';
import { log } from './utils/logger';
import {
  setupTestEnvironment,
  teardownTestEnvironment,
  addMockMessage,
  connectToMockStream,
  disconnectAndInitialize,
  setStreamState,
  navigateToTab,
  chatRow,
  viewerTabRow,
  viewerTabBroadcasterSelect,
} from './utils/test-helpers';

/**
 * 視聴者のタブ (docs/specs/06_viewer.md「視聴者のタブ (配信の後・準備)」「配信者スコープ」「検索機能」)
 *
 * 配信に接続してコメントが届くと視聴者が記録される → 視聴者のタブで読み仮名・メモ・タグを整理する、の流れを通しで確かめる。
 * テストは前のテストの記録を使うので順に走らせる。配信者を消すテストは最後。
 *
 * Run tests:
 *    pnpm exec playwright test --config e2e/playwright.config.ts viewer-management.spec.ts
 */

// 実行ごとに一意の視聴者にする (モックの channel_id を省くと乱数になり、前の記録と重なるため)
const RUN = Date.now();
const VIEWERS = {
  alice: { author: 'TabViewerAlice', channel_id: `UC_tab_alice_${RUN}`, content: 'Alice のコメント' },
  bob: { author: 'TabViewerBob', channel_id: `UC_tab_bob_${RUN}`, content: 'Bob のコメント' },
  carol: { author: 'TabViewerCarol', channel_id: `UC_tab_carol_${RUN}`, content: 'Carol のスパチャ' },
} as const;
const BROADCASTER_B = `UC_tab_broadcaster_b_${RUN}`;

test.describe.serial('視聴者のタブ (06_viewer.md)', () => {
  let browser: Browser;
  let mainPage: Page;
  // 最初に接続した配信の配信者 (チャンネル ID)
  let broadcasterA = '';

  const row = (name: string) => viewerTabRow(mainPage, name);
  const select = () => viewerTabBroadcasterSelect(mainPage);
  /** 行を押して編集を開き、読み仮名の欄にフォーカスが来るのを待つ */
  async function openEditor(name: string) {
    await row(name).getByRole('button', { name }).click();
    const reading = row(name).getByLabel('読み仮名', { exact: true });
    await expect(reading).toBeFocused();
    return reading;
  }

  test.beforeAll(async () => {
    test.setTimeout(240000);
    const connection = await setupTestEnvironment();
    browser = connection.browser;
    mainPage = connection.page;

    // 接続する前に積んだコメントは接続したときに届き、視聴者として記録される
    await addMockMessage({ message_type: 'text', ...VIEWERS.alice });
    await addMockMessage({ message_type: 'text', ...VIEWERS.bob });
    await addMockMessage({ message_type: 'superchat', ...VIEWERS.carol, amount: '¥500' });
    await connectToMockStream(mainPage);
    for (const v of Object.values(VIEWERS)) {
      await expect(chatRow(mainPage, v.content)).toBeVisible({ timeout: 10000 });
    }
    log.info('視聴者の記録の準備ができた');
  });

  test.afterAll(async () => {
    await teardownTestEnvironment(browser);
  });

  test('開くと主の配信の配信者を選び、1 人を 2 行で出す (貢献が 0 なら額は出さない)', async () => {
    await navigateToTab(mainPage, 'Viewers');
    await expect(select()).not.toHaveValue('');
    broadcasterA = await select().inputValue();
    await expect(row(VIEWERS.alice.author)).toContainText('コメント 1');
    await expect(row(VIEWERS.alice.author)).toContainText('最終 ');
    await expect(row(VIEWERS.alice.author)).not.toContainText('￥');
    await expect(row(VIEWERS.carol.author)).toContainText('￥500');
    await expect(mainPage.getByText(/^1〜\d+ 人目$/)).toBeVisible();
  });

  test('行を押すと下に編集を広げる。Enter で保存すると行とチャットの行に読み仮名が出る', async () => {
    const reading = await openEditor(VIEWERS.alice.author);
    await reading.fill('ありす');
    await reading.press('Enter');

    await expect(row(VIEWERS.alice.author).getByLabel('読み仮名', { exact: true })).toHaveCount(0);
    await expect(row(VIEWERS.alice.author)).toContainText('(ありす)');

    await navigateToTab(mainPage, 'Chat');
    await expect(chatRow(mainPage, VIEWERS.alice.content).getByText('(ありす)')).toBeVisible({ timeout: 5000 });
    await navigateToTab(mainPage, 'Viewers');
  });

  test('Esc は保存せずに閉じる。開いている間に別の行を押すと前の編集は捨てる', async () => {
    let reading = await openEditor(VIEWERS.bob.author);
    await reading.fill('捨てる読み');
    await reading.press('Escape');
    await expect(row(VIEWERS.bob.author).getByLabel('読み仮名', { exact: true })).toHaveCount(0);
    await expect(row(VIEWERS.bob.author)).not.toContainText('捨てる読み');

    reading = await openEditor(VIEWERS.bob.author);
    await reading.fill('これも捨てる');
    await openEditor(VIEWERS.carol.author);
    await expect(mainPage.getByLabel('読み仮名', { exact: true })).toHaveCount(1);
    await row(VIEWERS.carol.author).getByRole('button', { name: 'やめる' }).click();
    await expect(row(VIEWERS.bob.author)).not.toContainText('これも捨てる');
  });

  test('タグは読点・カンマで区切り、メモと一緒に行に出る', async () => {
    await openEditor(VIEWERS.bob.author);
    await row(VIEWERS.bob.author).getByLabel(/タグ/).fill('常連, 攻略、注意');
    const notes = row(VIEWERS.bob.author).getByLabel('メモ', { exact: true });
    await notes.fill('ネタバレしがち');
    await notes.press('Enter');

    for (const text of ['常連', '攻略', '注意', 'ネタバレしがち']) {
      await expect(row(VIEWERS.bob.author).getByText(text, { exact: true })).toBeVisible();
    }
    await openEditor(VIEWERS.bob.author);
    await expect(row(VIEWERS.bob.author).getByLabel(/タグ/)).toHaveValue('常連、攻略、注意');
    await row(VIEWERS.bob.author).getByRole('button', { name: 'やめる' }).click();
  });

  test('「読み仮名なしだけ」で読み仮名の無い人だけになる', async () => {
    const onlyWithoutReading = mainPage.getByLabel('読み仮名なしだけ');
    await onlyWithoutReading.check();
    await expect(row(VIEWERS.alice.author)).toHaveCount(0);
    await expect(row(VIEWERS.bob.author)).toBeVisible();

    await onlyWithoutReading.uncheck();
    await expect(row(VIEWERS.alice.author)).toBeVisible();
  });

  test('探す欄は打つだけで名前・読み仮名・メモから絞る。0 人なら「見つかりません」', async () => {
    const search = mainPage.getByLabel('視聴者を探す');
    await search.fill('ありす');
    await expect(row(VIEWERS.bob.author)).toHaveCount(0);
    await expect(row(VIEWERS.alice.author)).toBeVisible();

    await search.fill('ネタバレ');
    await expect(row(VIEWERS.alice.author)).toHaveCount(0);
    await expect(row(VIEWERS.bob.author)).toBeVisible();

    await search.fill(`見つからない文字 ${RUN}`);
    await expect(mainPage.getByText('見つかりません', { exact: true })).toBeVisible();

    await search.fill('');
    await expect(row(VIEWERS.alice.author)).toBeVisible();
  });

  test('配信者ごとに別の記録になる (同じ視聴者でも別の配信者のもとでは読み仮名が無い)', async () => {
    test.setTimeout(120000);
    await navigateToTab(mainPage, 'Chat');
    await disconnectAndInitialize(mainPage);
    await setStreamState({ channel_id: BROADCASTER_B, title: 'Mock Live B' });
    await addMockMessage({ message_type: 'text', ...VIEWERS.alice, content: 'B での Alice のコメント' });
    await connectToMockStream(mainPage, 'test_b_123', 'Mock Live B');
    await expect(chatRow(mainPage, 'B での Alice のコメント')).toBeVisible({ timeout: 10000 });

    await navigateToTab(mainPage, 'Viewers');
    await expect(select()).toHaveValue(BROADCASTER_B);
    await expect(row(VIEWERS.alice.author)).toBeVisible();
    await expect(row(VIEWERS.alice.author)).not.toContainText('(ありす)');

    await select().selectOption(broadcasterA);
    await expect(row(VIEWERS.alice.author)).toContainText('(ありす)');

    await navigateToTab(mainPage, 'Chat');
    await disconnectAndInitialize(mainPage);
    await setStreamState({ channel_id: '', title: '' });
  });

  test('「この視聴者を消す…」は消える範囲を書いた確認を出し、「消す」で一覧から外す', async () => {
    await navigateToTab(mainPage, 'Viewers');
    await select().selectOption(broadcasterA);
    await expect(row(VIEWERS.carol.author)).toBeVisible();
    await openEditor(VIEWERS.carol.author);
    await row(VIEWERS.carol.author).getByRole('button', { name: 'この視聴者を消す…' }).click();

    const confirm = row(VIEWERS.carol.author).getByRole('alert');
    await expect(confirm).toContainText(
      `取り消せません。${VIEWERS.carol.author} の記録 (読み仮名・メモ・タグ・コメント数など) をすべて消します。次にコメントしたら新しい視聴者として記録します`
    );
    await expect(confirm.getByRole('button', { name: 'やめる' })).toBeFocused();
    await confirm.getByRole('button', { name: '消す', exact: true }).click();
    await expect(row(VIEWERS.carol.author)).toHaveCount(0);
  });

  // 記録を消すので最後
  test('「この配信者の記録を消す…」は確認を出し、消したら残った最初の配信者を選ぶ', async () => {
    const before = await select().locator('option').count();
    const target = await select().inputValue();
    await mainPage.getByRole('button', { name: 'この配信者の記録を消す…' }).click();

    const dialog = mainPage.getByRole('dialog');
    await expect(dialog).toContainText(/取り消せません。.+ のもとの視聴者 \d+ 人の記録 \(読み仮名・メモ・タグ・コメント数など\) をすべて消します/);
    await expect(dialog.getByRole('button', { name: 'やめる' })).toBeFocused();
    await dialog.getByRole('button', { name: 'やめる' }).click();
    await expect(dialog).toHaveCount(0);

    await mainPage.getByRole('button', { name: 'この配信者の記録を消す…' }).click();
    await dialog.getByRole('button', { name: '消す', exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(select().locator('option')).toHaveCount(before - 1);
    await expect(select()).not.toHaveValue(target);
    await expect(select()).toHaveValue(await select().locator('option').first().getAttribute('value') as string);
  });
});
