// spec: docs/specs/02_chat.md「多接続」— 接続が成立したら URL 入力欄を空にする。欄の内容が変わっていたら残す
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import * as chatApi from '$lib/tauri/chat';
import InputSection from './InputSection.svelte';
import type { ConnectionResult } from '$lib/types';
import type { ReactionSummary } from '$lib/types/generated/ReactionSummary';

vi.mock('$lib/tauri/chat', () => ({
	connectToStream: vi.fn(),
	disconnectStream: vi.fn(),
	disconnectAllStreams: vi.fn(),
	getConnectionReactions: vi.fn(),
	setChatMode: vi.fn(),
	getConnections: vi.fn(),
}));

const success = (id: number): ConnectionResult => ({
	success: true,
	connection_id: BigInt(id),
	stream_title: 'Test Stream',
	broadcaster_name: 'Alice',
	broadcaster_channel_id: 'UC_alice',
	is_replay: false,
	error: null,
	session_id: 'sess_1',
});

describe('InputSection', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	function setup() {
		render(InputSection);
		return screen.getByPlaceholderText(/youtube\.com/) as HTMLInputElement;
	}

	it('接続が成立したら URL 入力欄を空にする', async () => {
		const input = setup();
		vi.mocked(chatApi.connectToStream).mockResolvedValue(success(1));
		vi.mocked(chatApi.getConnectionReactions).mockResolvedValue({ totals: {}, recent: [] });

		await fireEvent.input(input, { target: { value: 'https://www.youtube.com/watch?v=first' } });
		await fireEvent.click(screen.getByRole('button', { name: '開始' }));

		await vi.waitFor(() => expect(input.value).toBe(''));
	});

	it('接続の完了を待つ間に入力し直した URL は消さない', async () => {
		const input = setup();
		vi.mocked(chatApi.connectToStream).mockResolvedValue(success(2));
		// 接続の成立後、リアクションの読み込みが終わるまでの間に入力し直す
		let resolveReactions!: (summary: ReactionSummary) => void;
		vi.mocked(chatApi.getConnectionReactions).mockReturnValue(
			new Promise((resolve) => {
				resolveReactions = resolve;
			})
		);

		await fireEvent.input(input, { target: { value: 'https://www.youtube.com/watch?v=first' } });
		await fireEvent.click(screen.getByRole('button', { name: '開始' }));
		await vi.waitFor(() => expect(chatApi.getConnectionReactions).toHaveBeenCalled());
		await fireEvent.input(input, { target: { value: 'https://www.youtube.com/watch?v=second' } });
		resolveReactions({ totals: {}, recent: [] });

		await new Promise((r) => setTimeout(r, 0));
		expect(input.value).toBe('https://www.youtube.com/watch?v=second');
	});
});
