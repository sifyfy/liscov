// spec: docs/specs/02_chat.md「視聴者情報パネル」読み仮名機能 — 読み込み中に別の視聴者を開いたら、前の視聴者の結果は捨てる
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import ViewerInfoPanel from './ViewerInfoPanel.svelte';
import type { ChatMessage } from '$lib/types';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

function viewer(channelId: string, displayName: string) {
	return { channelId, displayName, message: { id: `${channelId}-msg` } as ChatMessage };
}

describe('ViewerInfoPanel', () => {
	it('A を開いてすぐ B を開き、A の応答があとから届いても、読み仮名と保存先は B のまま', async () => {
		let resolveProfileA: (profile: { id: number }) => void = () => {};
		vi.mocked(invoke).mockImplementation(async (command, args) => {
			const a = args as Record<string, unknown>;
			switch (command) {
				case 'viewer_get_profile':
					return a.channelId === 'UC_A'
						? new Promise((resolve) => (resolveProfileA = resolve))
						: { id: 2 };
				case 'viewer_get_custom_info':
					return { reading: a.viewerProfileId === 1 ? 'えー' : 'びー', notes: null };
				default:
					return null;
			}
		});

		const props = { broadcasterChannelId: 'UC_B0', onClose: () => {} };
		const { rerender } = render(ViewerInfoPanel, { ...props, viewer: viewer('UC_A', 'A') });
		await rerender({ ...props, viewer: viewer('UC_B', 'B') });
		const readingInput = screen.getByLabelText(/読み仮名/) as HTMLInputElement;
		await waitFor(() => expect(readingInput.value).toBe('びー'));

		resolveProfileA({ id: 1 });
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(readingInput.value).toBe('びー');

		await fireEvent.click(screen.getByRole('button', { name: /保存/ }));
		expect(invoke).toHaveBeenCalledWith(
			'viewer_upsert_custom_info',
			expect.objectContaining({ viewerProfileId: 2, reading: 'びー' })
		);
	});

	it('読み込みが終わるまで読み仮名・メモ・保存は押せない', async () => {
		let resolveProfile: (profile: { id: number }) => void = () => {};
		vi.mocked(invoke).mockImplementation(async (command) => {
			switch (command) {
				case 'viewer_get_profile':
					return new Promise((resolve) => (resolveProfile = resolve));
				case 'viewer_get_custom_info':
					return null;
				default:
					return null;
			}
		});

		render(ViewerInfoPanel, {
			broadcasterChannelId: 'UC_B0',
			onClose: () => {},
			viewer: viewer('UC_A', 'A'),
		});
		const readingInput = screen.getByLabelText(/読み仮名/) as HTMLInputElement;
		const notesInput = screen.getByLabelText(/メモ/) as HTMLTextAreaElement;
		const saveButton = screen.getByRole('button', { name: /保存/ }) as HTMLButtonElement;
		expect([readingInput.disabled, notesInput.disabled, saveButton.disabled]).toEqual([true, true, true]);

		resolveProfile({ id: 1 });
		await waitFor(() => expect(readingInput.disabled).toBe(false));
		expect(notesInput.disabled).toBe(false);
		expect(saveButton.disabled).toBe(false);
	});
});
