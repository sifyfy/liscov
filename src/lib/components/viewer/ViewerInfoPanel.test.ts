// spec: docs/specs/02_chat.md「配信中の画面」の「視聴者の読み仮名・メモ (視聴者情報パネル)」と「視聴者情報パネル」の読み仮名機能
// - コメントを押すと行の下に広がり、読み仮名の欄にフォーカスする (読み込みが終わりしだい打てる)
// - 読み仮名・メモの欄で Enter・「保存」→ 保存して閉じる。日本語入力の変換を確定する Enter では保存しない
// - × を押す・Esc → 保存せずに閉じる
// - 欄にフォーカスがある・保存していない変更がある間は「打っている」(30 秒で最新に戻らない)
// - 接続が 2 つ以上なら、どの配信者のもとに保存するかを配信者名の札で出す
// - 「この人のコメント N 件」は畳んである。押すと開き、行を押すと流れのそのコメントへ移る
// - 読み込み中に別の視聴者を開いたら、前の視聴者の結果は捨てる。読み込みが終わるまで欄と保存は押せない
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import ViewerInfoPanel from './ViewerInfoPanel.svelte';
import type { ChatMessage } from '$lib/types';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

function viewer(channelId: string, displayName: string) {
	return { channelId, displayName, message: { id: `${channelId}-msg` } as ChatMessage };
}

function comment(id: string, content: string, timestamp: string): ChatMessage {
	return { id, content, timestamp, author: '夜ふかし研究会', message_type: 'text', amount: null, metadata: null, connection_id: 1n } as ChatMessage;
}

/** 視聴者 (プロフィール id 7) は読み仮名「よふかし」・メモなしで登録済み */
function mockLoaded() {
	vi.mocked(invoke).mockImplementation(async (command) => {
		switch (command) {
			case 'viewer_get_profile':
				return { id: 7 };
			case 'viewer_get_custom_info':
				return { reading: 'よふかし', notes: null };
			default:
				return null;
		}
	});
}

function renderPanel(overrides: Record<string, unknown> = {}) {
	const props = {
		viewer: viewer('UC_X', '夜ふかし研究会'),
		broadcasterChannelId: 'UC_B',
		broadcasterName: null,
		history: [],
		onClose: vi.fn(),
		onSaved: vi.fn(),
		onBusyChange: vi.fn(),
		onMessageClick: vi.fn(),
		...overrides,
	};
	render(ViewerInfoPanel, props);
	return props;
}

const readingInput = () => screen.getByLabelText('読み仮名') as HTMLInputElement;
const notesInput = () => screen.getByLabelText('メモ') as HTMLInputElement;

describe('ViewerInfoPanel', () => {
	beforeEach(() => vi.mocked(invoke).mockReset());

	it('読み込みが終わったら読み仮名の欄にフォーカスし、登録済みの値を入れる', async () => {
		mockLoaded();
		renderPanel();
		await waitFor(() => expect(readingInput().value).toBe('よふかし'));
		await waitFor(() => expect(document.activeElement).toBe(readingInput()));
	});

	it('読み仮名の欄で Enter を押すと、読み仮名とメモを保存して閉じる (onSaved)', async () => {
		mockLoaded();
		const props = renderPanel();
		await waitFor(() => expect(readingInput().disabled).toBe(false));
		await fireEvent.input(readingInput(), { target: { value: 'よふかしけんきゅうかい' } });
		await fireEvent.input(notesInput(), { target: { value: '攻略情報をよくくれる' } });
		await fireEvent.keyDown(readingInput(), { key: 'Enter' });
		await waitFor(() => expect(props.onSaved).toHaveBeenCalledOnce());
		expect(invoke).toHaveBeenCalledWith('viewer_upsert_custom_info', {
			viewerProfileId: 7,
			reading: 'よふかしけんきゅうかい',
			notes: '攻略情報をよくくれる',
			customData: null,
		});
		expect(props.onClose).not.toHaveBeenCalled();
	});

	it('メモの欄で Enter を押しても保存する', async () => {
		mockLoaded();
		const props = renderPanel();
		await waitFor(() => expect(notesInput().disabled).toBe(false));
		await fireEvent.keyDown(notesInput(), { key: 'Enter' });
		await waitFor(() => expect(props.onSaved).toHaveBeenCalledOnce());
	});

	it('日本語入力の変換を確定する Enter では保存しない', async () => {
		mockLoaded();
		const props = renderPanel();
		await waitFor(() => expect(readingInput().disabled).toBe(false));
		await fireEvent.keyDown(readingInput(), { key: 'Enter', isComposing: true });
		await fireEvent.keyDown(readingInput(), { key: 'Enter', keyCode: 229 });
		expect(invoke).not.toHaveBeenCalledWith('viewer_upsert_custom_info', expect.anything());
		expect(props.onSaved).not.toHaveBeenCalled();
	});

	it('空の読み仮名は「読み仮名なし」(null) として保存する', async () => {
		mockLoaded();
		renderPanel();
		await waitFor(() => expect(readingInput().disabled).toBe(false));
		await fireEvent.input(readingInput(), { target: { value: '' } });
		await fireEvent.click(screen.getByRole('button', { name: '保存' }));
		expect(invoke).toHaveBeenCalledWith('viewer_upsert_custom_info', expect.objectContaining({ reading: null, notes: null }));
	});

	it('Esc・× を押すと保存せずに閉じる', async () => {
		mockLoaded();
		const props = renderPanel();
		await waitFor(() => expect(readingInput().disabled).toBe(false));
		await fireEvent.input(readingInput(), { target: { value: 'かきかけ' } });
		await fireEvent.keyDown(readingInput(), { key: 'Escape' });
		await fireEvent.click(screen.getByRole('button', { name: '閉じる' }));
		expect(props.onClose).toHaveBeenCalledTimes(2);
		expect(invoke).not.toHaveBeenCalledWith('viewer_upsert_custom_info', expect.anything());
	});

	it('保存に失敗したら閉じずに、失敗したことを出す', async () => {
		mockLoaded();
		vi.mocked(invoke).mockImplementation(async (command) => {
			if (command === 'viewer_get_profile') return { id: 7 };
			if (command === 'viewer_upsert_custom_info') throw new Error('db locked');
			return null;
		});
		const props = renderPanel();
		await waitFor(() => expect(readingInput().disabled).toBe(false));
		await fireEvent.keyDown(readingInput(), { key: 'Enter' });
		await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('保存に失敗しました'));
		expect(props.onSaved).not.toHaveBeenCalled();
	});

	it('欄にフォーカスがある間・保存していない変更がある間は「打っている」', async () => {
		mockLoaded();
		const props = renderPanel();
		await waitFor(() => expect(document.activeElement).toBe(readingInput()));
		expect(props.onBusyChange).toHaveBeenLastCalledWith(true);

		// フォーカスが外れ、変更も無い → 打っていない
		await fireEvent.focusOut(readingInput(), { relatedTarget: document.body });
		expect(props.onBusyChange).toHaveBeenLastCalledWith(false);

		// 変更してからフォーカスが外れた → 打っている (保存していない変更がある)
		await fireEvent.focusIn(readingInput());
		await fireEvent.input(readingInput(), { target: { value: 'よふかしけん' } });
		await fireEvent.focusOut(readingInput(), { relatedTarget: document.body });
		expect(props.onBusyChange).toHaveBeenLastCalledWith(true);
	});

	it('接続が 2 つ以上なら保存先の配信者名を出す。1 つなら出さない', async () => {
		mockLoaded();
		renderPanel({ broadcasterName: 'あおば' });
		expect(screen.getByTestId('viewer-broadcaster').textContent).toContain('あおば');
	});

	it('配信者名が無ければ (接続が 1 つ) 札を出さない', async () => {
		mockLoaded();
		renderPanel();
		expect(screen.queryByTestId('viewer-broadcaster')).toBeNull();
	});

	it('「この人のコメント N 件」は畳んである。押すと新しい順に時刻と本文を出し、行を押すと onMessageClick', async () => {
		mockLoaded();
		const older = comment('c1', '前回のボス戦の続きからだよね', '2026-10-09T12:14:00Z');
		const newer = comment('c2', 'ここのボス、最初は足を狙うと楽', '2026-10-09T12:27:00Z');
		const props = renderPanel({ history: [older, newer] });
		const toggle = screen.getByRole('button', { name: /この人のコメント 2 件/ });
		expect(toggle.getAttribute('aria-expanded')).toBe('false');
		expect(screen.queryByText('前回のボス戦の続きからだよね')).toBeNull();

		await fireEvent.click(toggle);
		expect(toggle.getAttribute('aria-expanded')).toBe('true');
		const rows = screen.getAllByTestId('viewer-history-item');
		expect(rows.map((r) => r.textContent)).toEqual([
			expect.stringContaining('ここのボス、最初は足を狙うと楽'),
			expect.stringContaining('前回のボス戦の続きからだよね'),
		]);
		// 名前は頭に 1 度だけ (行ごとに繰り返さない)
		expect(rows.some((r) => r.textContent?.includes('夜ふかし研究会'))).toBe(false);

		await fireEvent.click(rows[1]);
		expect(props.onMessageClick).toHaveBeenCalledWith(older);
	});

	it('A を開いてすぐ B を開き、A の応答があとから届いても、読み仮名と保存先は B のまま', async () => {
		let resolveProfileA: (profile: { id: number }) => void = () => {};
		vi.mocked(invoke).mockImplementation(async (command, args) => {
			const a = args as Record<string, unknown>;
			switch (command) {
				case 'viewer_get_profile':
					return a.channelId === 'UC_A' ? new Promise((resolve) => (resolveProfileA = resolve)) : { id: 2 };
				case 'viewer_get_custom_info':
					return { reading: a.viewerProfileId === 1 ? 'えー' : 'びー', notes: null };
				default:
					return null;
			}
		});

		const props = {
			broadcasterChannelId: 'UC_B0',
			broadcasterName: null,
			history: [],
			onClose: () => {},
			onSaved: () => {},
		};
		const { rerender } = render(ViewerInfoPanel, { ...props, viewer: viewer('UC_A', 'A') });
		await rerender({ ...props, viewer: viewer('UC_B', 'B') });
		await waitFor(() => expect(readingInput().value).toBe('びー'));

		resolveProfileA({ id: 1 });
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(readingInput().value).toBe('びー');

		await fireEvent.click(screen.getByRole('button', { name: '保存' }));
		expect(invoke).toHaveBeenCalledWith('viewer_upsert_custom_info', expect.objectContaining({ viewerProfileId: 2, reading: 'びー' }));
	});

	it('読み込みが終わるまで読み仮名・メモ・保存は押せない', async () => {
		let resolveProfile: (profile: { id: number }) => void = () => {};
		vi.mocked(invoke).mockImplementation(async (command) => {
			if (command === 'viewer_get_profile') return new Promise((resolve) => (resolveProfile = resolve));
			return null;
		});

		renderPanel();
		const saveButton = screen.getByRole('button', { name: '保存' }) as HTMLButtonElement;
		expect([readingInput().disabled, notesInput().disabled, saveButton.disabled]).toEqual([true, true, true]);

		resolveProfile({ id: 1 });
		await waitFor(() => expect(readingInput().disabled).toBe(false));
		expect(notesInput().disabled).toBe(false);
		expect(saveButton.disabled).toBe(false);
	});
});
