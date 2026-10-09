// spec: docs/specs/06_viewer.md「視聴者のタブ (配信の後・準備)」
// - 行を押すと行の下に編集を広げ、読み仮名の欄にフォーカスする。欄は 読み仮名・タグ・メモ
// - 読み仮名・タグ・メモの欄で Enter・「保存」→ 保存して閉じる。日本語入力の変換を確定する Enter では保存しない
// - Esc・「やめる」→ 保存せずに閉じる
// - タグは読点 (、) かカンマ (,) で区切る
// - 「この視聴者を消す…」→ 編集の中に確認 (「取り消せません。」で始まる)。フォーカスは「やめる」。「消す」で消す
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import ViewerEditor from './ViewerEditor.svelte';
import type { ViewerWithCustomInfo } from '$lib/types';

const VIEWER: ViewerWithCustomInfo = {
	id: 7,
	broadcaster_channel_id: 'UC_B',
	channel_id: 'UC_X',
	display_name: '夜ふかし研究会',
	first_seen: '2026-03-02T12:00:00Z',
	last_seen: '2026-10-09T12:00:00Z',
	message_count: 1284,
	total_contribution: 12000,
	membership_level: null,
	tags: ['常連', '攻略'],
	reading: 'よふかし',
	notes: 'ホラーが苦手',
	custom_data: null,
};

function renderEditor(overrides: Record<string, unknown> = {}) {
	const props = {
		viewer: VIEWER,
		onSave: vi.fn(async () => {}),
		onCancel: vi.fn(),
		onDelete: vi.fn(async () => {}),
		...overrides,
	};
	render(ViewerEditor, props);
	return props;
}

const field = (name: string | RegExp) => screen.getByLabelText(name) as HTMLInputElement;

describe('ViewerEditor', () => {
	it('開いたら読み仮名の欄にフォーカスし、登録済みの値を入れる (タグは読点でつなぐ)', async () => {
		renderEditor();
		expect(field('読み仮名').value).toBe('よふかし');
		expect(field(/タグ/).value).toBe('常連、攻略');
		expect(field('メモ').value).toBe('ホラーが苦手');
		await waitFor(() => expect(document.activeElement).toBe(field('読み仮名')));
	});

	it('どの欄で Enter を押しても保存する。タグは読点・カンマで区切る', async () => {
		const props = renderEditor();
		await fireEvent.input(field('読み仮名'), { target: { value: 'よふかしけんきゅうかい' } });
		await fireEvent.input(field(/タグ/), { target: { value: '常連, 攻略、注意' } });
		await fireEvent.input(field('メモ'), { target: { value: '' } });
		await fireEvent.keyDown(field('メモ'), { key: 'Enter' });
		expect(props.onSave).toHaveBeenCalledWith({ reading: 'よふかしけんきゅうかい', notes: '', tags: ['常連', '攻略', '注意'] });

		await fireEvent.keyDown(field(/タグ/), { key: 'Enter' });
		await fireEvent.keyDown(field('読み仮名'), { key: 'Enter' });
		expect(props.onSave).toHaveBeenCalledTimes(3);
	});

	it('日本語入力の変換を確定する Enter では保存しない', async () => {
		const props = renderEditor();
		await fireEvent.keyDown(field('読み仮名'), { key: 'Enter', isComposing: true });
		await fireEvent.keyDown(field('メモ'), { key: 'Enter', keyCode: 229 });
		expect(props.onSave).not.toHaveBeenCalled();
	});

	it('「保存」でも保存する', async () => {
		const props = renderEditor();
		await fireEvent.click(screen.getByRole('button', { name: '保存' }));
		expect(props.onSave).toHaveBeenCalledWith({ reading: 'よふかし', notes: 'ホラーが苦手', tags: ['常連', '攻略'] });
	});

	it('保存に失敗したら閉じずに、失敗したことを出す', async () => {
		const props = renderEditor({ onSave: vi.fn(async () => Promise.reject(new Error('db locked'))) });
		await fireEvent.click(screen.getByRole('button', { name: '保存' }));
		await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('保存に失敗しました'));
		expect(props.onCancel).not.toHaveBeenCalled();
	});

	it('Esc・「やめる」は保存せずに閉じる', async () => {
		const props = renderEditor();
		await fireEvent.keyDown(field('読み仮名'), { key: 'Escape' });
		await fireEvent.click(screen.getByRole('button', { name: 'やめる' }));
		expect(props.onCancel).toHaveBeenCalledTimes(2);
		expect(props.onSave).not.toHaveBeenCalled();
	});

	it('「この視聴者を消す…」で確認を出し、フォーカスは「やめる」。「消す」で消す', async () => {
		const props = renderEditor();
		await fireEvent.click(screen.getByRole('button', { name: 'この視聴者を消す…' }));
		const confirm = screen.getByRole('alert');
		expect(confirm.textContent).toMatch(/^取り消せません。夜ふかし研究会 の記録 \(読み仮名・メモ・タグ・コメント数など\) をすべて消します/);
		await waitFor(() => expect(document.activeElement?.textContent?.trim()).toBe('やめる'));

		await fireEvent.click(screen.getByRole('button', { name: '消す' }));
		expect(props.onDelete).toHaveBeenCalledOnce();
	});

	it('確認で「やめる」を押すと、消さずに編集に戻る', async () => {
		const props = renderEditor();
		await fireEvent.click(screen.getByRole('button', { name: 'この視聴者を消す…' }));
		await fireEvent.click(screen.getByRole('button', { name: 'やめる' }));
		expect(props.onDelete).not.toHaveBeenCalled();
		expect(props.onCancel).not.toHaveBeenCalled();
		expect(screen.getByRole('button', { name: '保存' })).toBeTruthy();
	});
});
