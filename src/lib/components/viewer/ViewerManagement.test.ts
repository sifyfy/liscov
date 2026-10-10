// spec: docs/specs/06_viewer.md「視聴者のタブ (配信の後・準備)」
// - 開いたら主の配信の配信者 (なければ記録のある最初の配信者) の視聴者を出す
// - 1 人の行: 1 行目に名前 (読み仮名)・タグ、2 行目に最終確認の日付・コメント数・貢献額 (0 なら出さない)・メモ
// - 「読み仮名なしだけ」・探す欄 (打ち終えて 300ms) で絞る。最初のページに戻る
// - 行を押すとその行の下に編集を広げる。保存したらチャットの行にも読み仮名を出す。別の行を押すと前の編集は捨てる
// - 「この配信者の記録を消す…」は確認のダイアログ (フォーカスは「やめる」)。消したら記録のある最初の配信者を選ぶ
// - ページ: 「前の 50 人」「次の 50 人」「1〜50 人目」
// - 読み込み中・記録が無い・見つからない・配信者がいない のそれぞれの文
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/dom';
import { invoke } from '@tauri-apps/api/core';
import type { BroadcasterChannel, ViewerWithCustomInfo } from '$lib/types';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const THIS_YEAR = new Date().getFullYear();

function viewer(id: number, display_name: string, rest: Partial<ViewerWithCustomInfo> = {}): ViewerWithCustomInfo {
	return {
		id,
		broadcaster_channel_id: 'UC_A',
		channel_id: `UC_v${id}`,
		display_name,
		first_seen: `${THIS_YEAR}-03-02T12:00:00`,
		last_seen: `${THIS_YEAR}-10-09T12:00:00`,
		message_count: 1,
		total_contribution: 0,
		membership_level: null,
		tags: [],
		reading: null,
		notes: null,
		custom_data: null,
		...rest,
	};
}

const BROADCASTERS: BroadcasterChannel[] = [
	{ channel_id: 'UC_A', channel_name: 'あおば', handle: null, viewer_count: 3 },
	{ channel_id: 'UC_B', channel_name: '試作の配信者', handle: null, viewer_count: 1 },
];

const VIEWERS: Record<string, ViewerWithCustomInfo[]> = {
	UC_A: [
		viewer(1, '夜ふかし研究会', {
			reading: 'よふかし',
			notes: 'ホラーが苦手',
			tags: ['常連', '攻略'],
			message_count: 1284,
			total_contribution: 12000,
			custom_data: '{"x":1}',
		}),
		viewer(2, 'しおこんぶ', { message_count: 842 }),
		viewer(3, 'みかん箱', { reading: '', notes: '箱推し' }),
	],
	UC_B: [viewer(4, 'のりたま', { broadcaster_channel_id: 'UC_B', reading: 'のりたま' })],
};

type ListArgs = { broadcasterId: string; searchQuery?: string; withoutReading?: boolean; limit: number; offset: number };

/** バックエンドの代わり。一覧の絞り込みは crud.rs と同じ条件 (名前・読み仮名・メモの部分一致、読み仮名なし) */
function mockBackend(
	overrides: Partial<{ broadcasters: BroadcasterChannel[]; list: (args: ListArgs) => ViewerWithCustomInfo[] | Promise<ViewerWithCustomInfo[]> }> = {}
) {
	let broadcasters = overrides.broadcasters ?? BROADCASTERS;
	vi.mocked(invoke).mockImplementation(async (command, args) => {
		switch (command) {
			case 'broadcaster_get_list':
				return broadcasters;
			case 'viewer_get_list': {
				const a = args as ListArgs;
				if (overrides.list) return overrides.list(a);
				const q = a.searchQuery;
				return (VIEWERS[a.broadcasterId] ?? [])
					.filter((v) => !q || [v.display_name, v.reading, v.notes].some((s) => s?.includes(q)))
					.filter((v) => !a.withoutReading || !v.reading);
			}
			case 'broadcaster_delete':
				broadcasters = broadcasters.filter((b) => b.channel_id !== (args as { broadcasterId: string }).broadcasterId);
				return [true, 3];
			case 'viewer_delete':
			case 'viewer_update_info':
				return true;
			default:
				return null;
		}
	});
}

const listCalls = () => vi.mocked(invoke).mock.calls.filter(([c]) => c === 'viewer_get_list').map(([, a]) => a as ListArgs);

// ストアはシングルトンなので、テストごとにモジュールを読み直して絞り込みなどを初期化する。
// svelte も読み直されるので、testing-library も読み直したものを使う (古い svelte で描くと effect_orphan になる)
let tl: typeof import('@testing-library/svelte');

async function renderManagement(props: { broadcasterId?: string } = {}) {
	const { default: ViewerManagement } = await import('./ViewerManagement.svelte');
	const { chatStore } = await import('$lib/stores');
	const setViewerInfo = vi.spyOn(chatStore, 'setViewerInfo');
	tl.render(ViewerManagement, props);
	return { setViewerInfo };
}

const rowOf = (name: string) => screen.getByText(name).closest('li') as HTMLElement;
const broadcasterSelect = () => screen.getByLabelText('配信者') as HTMLSelectElement;

describe('ViewerManagement', () => {
	beforeEach(async () => {
		vi.resetModules();
		tl = await import('@testing-library/svelte');
		vi.mocked(invoke).mockReset();
		mockBackend();
	});
	afterEach(() => tl.cleanup());

	it('開いたら主の配信の配信者の視聴者を出す', async () => {
		await renderManagement({ broadcasterId: 'UC_B' });
		await screen.findByText('のりたま');
		expect(broadcasterSelect().value).toBe('UC_B');
		expect(broadcasterSelect().selectedOptions[0].textContent?.trim()).toBe('試作の配信者 (1 人)');
	});

	it('接続していなければ記録のある最初の配信者を選ぶ', async () => {
		await renderManagement();
		await screen.findByText('夜ふかし研究会');
		expect(broadcasterSelect().value).toBe('UC_A');
	});

	it('配信者を選び直すと、その配信者の視聴者を出す', async () => {
		await renderManagement();
		await screen.findByText('夜ふかし研究会');
		await tl.fireEvent.change(broadcasterSelect(), { target: { value: 'UC_B' } });
		await screen.findByText('のりたま');
		expect(screen.queryByText('夜ふかし研究会')).toBeNull();
	});

	it('1 人の行: 名前 (読み仮名)・タグ、最終確認の日付・コメント数・貢献額・メモ。貢献が 0 なら額は出さない', async () => {
		await renderManagement();
		await screen.findByText('夜ふかし研究会');
		const r = within(rowOf('夜ふかし研究会'));
		r.getByText('(よふかし)');
		r.getByText('常連');
		r.getByText('攻略');
		r.getByText('最終 10/9');
		r.getByText('コメント 1,284');
		r.getByText('￥12,000');
		r.getByText('ホラーが苦手');

		const plain = within(rowOf('しおこんぶ'));
		plain.getByText('コメント 842');
		expect(rowOf('しおこんぶ').textContent).not.toContain('￥');
	});

	it('「読み仮名なしだけ」をオンにすると、読み仮名の無い人だけを最初のページから出す', async () => {
		await renderManagement();
		await screen.findByText('夜ふかし研究会');
		await tl.fireEvent.click(screen.getByLabelText('読み仮名なしだけ'));
		await waitFor(() => expect(screen.queryByText('夜ふかし研究会')).toBeNull());
		screen.getByText('しおこんぶ');
		screen.getByText('みかん箱');
		expect(listCalls().at(-1)).toMatchObject({ broadcasterId: 'UC_A', withoutReading: true, offset: 0 });
	});

	it('探す欄は打ち終えて 300ms で絞る', async () => {
		await renderManagement();
		await screen.findByText('夜ふかし研究会');
		await tl.fireEvent.input(screen.getByLabelText('視聴者を探す'), { target: { value: 'ホラー' } });
		expect(listCalls().some((a) => a.searchQuery === 'ホラー')).toBe(false);
		await waitFor(() => expect(screen.queryByText('しおこんぶ')).toBeNull());
		expect(listCalls().at(-1)).toMatchObject({ searchQuery: 'ホラー', offset: 0 });
		screen.getByText('夜ふかし研究会');
	});

	it('行を押すと、その行の下に編集を広げて読み仮名の欄にフォーカスする', async () => {
		await renderManagement();
		await tl.fireEvent.click(await screen.findByText('しおこんぶ'));
		const reading = within(rowOf('しおこんぶ')).getByLabelText('読み仮名');
		await waitFor(() => expect(document.activeElement).toBe(reading));
	});

	it('Enter で保存して閉じ、チャットの行にも読み仮名とメモを出す', async () => {
		const { setViewerInfo } = await renderManagement();
		await tl.fireEvent.click(await screen.findByText('夜ふかし研究会'));
		const reading = screen.getByLabelText('読み仮名');
		await tl.fireEvent.input(reading, { target: { value: 'よふかしけんきゅうかい' } });
		await tl.fireEvent.keyDown(reading, { key: 'Enter' });

		await waitFor(() => expect(screen.queryByLabelText('読み仮名')).toBeNull());
		expect(invoke).toHaveBeenCalledWith('viewer_update_info', {
			viewerProfileId: 1,
			reading: 'よふかしけんきゅうかい',
			notes: 'ホラーが苦手',
			customData: '{"x":1}',
			tags: ['常連', '攻略'],
		});
		expect(setViewerInfo).toHaveBeenCalledWith('UC_A', 'UC_v1', { reading: 'よふかしけんきゅうかい', notes: 'ホラーが苦手' });
	});

	it('空にした読み仮名・メモは消す (null で保存する)', async () => {
		await renderManagement();
		await tl.fireEvent.click(await screen.findByText('夜ふかし研究会'));
		await tl.fireEvent.input(screen.getByLabelText('読み仮名'), { target: { value: '' } });
		await tl.fireEvent.input(screen.getByLabelText('メモ'), { target: { value: '' } });
		await tl.fireEvent.click(screen.getByRole('button', { name: '保存' }));
		await waitFor(() =>
			expect(invoke).toHaveBeenCalledWith('viewer_update_info', expect.objectContaining({ reading: null, notes: null }))
		);
	});

	it('開いている間に別の行を押すと、前の編集を捨てて押した行の下に開く', async () => {
		await renderManagement();
		await tl.fireEvent.click(await screen.findByText('夜ふかし研究会'));
		await tl.fireEvent.input(screen.getByLabelText('読み仮名'), { target: { value: '捨てる' } });
		await tl.fireEvent.click(screen.getByText('しおこんぶ'));

		expect(screen.getAllByLabelText('読み仮名')).toHaveLength(1);
		within(rowOf('しおこんぶ')).getByLabelText('読み仮名');
		expect(invoke).not.toHaveBeenCalledWith('viewer_update_info', expect.anything());
	});

	it('「この視聴者を消す…」→「消す」で消して一覧から外す', async () => {
		await renderManagement();
		await tl.fireEvent.click(await screen.findByText('しおこんぶ'));
		await tl.fireEvent.click(screen.getByRole('button', { name: 'この視聴者を消す…' }));
		await tl.fireEvent.click(screen.getByRole('button', { name: '消す' }));
		expect(invoke).toHaveBeenCalledWith('viewer_delete', { viewerProfileId: 2 });
		await waitFor(() => expect(screen.queryByLabelText('読み仮名')).toBeNull());
	});

	it('「この配信者の記録を消す…」は消える範囲を書いた確認を出し、フォーカスは「やめる」', async () => {
		await renderManagement();
		await screen.findByText('夜ふかし研究会');
		await tl.fireEvent.click(screen.getByRole('button', { name: 'この配信者の記録を消す…' }));
		const dialog = screen.getByRole('dialog');
		expect(dialog.textContent).toContain(
			'取り消せません。あおば のもとの視聴者 3 人の記録 (読み仮名・メモ・タグ・コメント数など) をすべて消します'
		);
		await waitFor(() => expect(document.activeElement).toBe(within(dialog).getByRole('button', { name: 'やめる' })));

		await tl.fireEvent.click(within(dialog).getByRole('button', { name: 'やめる' }));
		expect(screen.queryByRole('dialog')).toBeNull();
		expect(invoke).not.toHaveBeenCalledWith('broadcaster_delete', expect.anything());
	});

	it('配信者の記録を消したら、記録のある最初の配信者を選ぶ', async () => {
		await renderManagement();
		await screen.findByText('夜ふかし研究会');
		await tl.fireEvent.click(screen.getByRole('button', { name: 'この配信者の記録を消す…' }));
		await tl.fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: '消す' }));

		expect(invoke).toHaveBeenCalledWith('broadcaster_delete', { broadcasterId: 'UC_A' });
		await screen.findByText('のりたま');
		expect(broadcasterSelect().value).toBe('UC_B');
		expect(screen.queryByRole('dialog')).toBeNull();
	});

	it('ページ: 50 人ずつ。次が無ければ「次の 50 人」は押せない', async () => {
		const page = (offset: number) => Array.from({ length: offset === 0 ? 50 : 3 }, (_, i) => viewer(offset + i + 1, `視聴者${offset + i + 1}`));
		mockBackend({ list: (a) => page(a.offset) });
		await renderManagement();
		await screen.findByText('視聴者1');
		screen.getByText('1〜50 人目');
		expect(screen.getByRole('button', { name: '前の 50 人' })).toHaveProperty('disabled', true);

		await tl.fireEvent.click(screen.getByRole('button', { name: '次の 50 人' }));
		await screen.findByText('視聴者51');
		screen.getByText('51〜53 人目');
		expect(screen.getByRole('button', { name: '次の 50 人' })).toHaveProperty('disabled', true);
		expect(screen.getByRole('button', { name: '前の 50 人' })).toHaveProperty('disabled', false);
	});

	it('読み込み中は「読み込み中…」', async () => {
		mockBackend({ list: () => new Promise(() => {}) });
		await renderManagement();
		await screen.findByText('読み込み中…');
	});

	it('配信者に視聴者がいなければ「まだ視聴者の記録がありません」、絞り込んで 0 人なら「見つかりません」', async () => {
		mockBackend({ list: () => [] });
		await renderManagement();
		await screen.findByText('まだ視聴者の記録がありません');
		await tl.fireEvent.click(screen.getByLabelText('読み仮名なしだけ'));
		await screen.findByText('見つかりません');
	});

	it('記録のある配信者がいなければ、記録される条件を出す', async () => {
		mockBackend({ broadcasters: [] });
		await renderManagement();
		await screen.findByText('まだ記録がありません。配信に接続してコメントが届くと記録されます');
	});
});
