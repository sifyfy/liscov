// spec: docs/specs/02_chat.md「多接続」— 接続が成立したら URL 入力欄を空にする。欄の内容が変わっていたら残す
// spec: docs/specs/02_chat.md「配信中の画面」の「上の行と操作パネル」— 配信に接続したら (接続したら操作パネルを隠す)、「すべてのコメントを取る」
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import * as chatApi from '$lib/tauri/chat';
import { chatStore } from '$lib/stores/chat.svelte';
import ConnectForm from './ConnectForm.svelte';
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

describe('ConnectForm', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		vi.mocked(chatApi.getConnectionReactions).mockResolvedValue({ totals: {}, recent: [] });
	});

	function setup(onConnected = vi.fn()) {
		render(ConnectForm, { props: { onConnected } });
		return { input: screen.getByPlaceholderText(/youtube\.com/) as HTMLInputElement, onConnected };
	}

	async function connect(input: HTMLInputElement, url = 'https://www.youtube.com/watch?v=first') {
		await fireEvent.input(input, { target: { value: url } });
		await fireEvent.click(screen.getByRole('button', { name: '接続' }));
	}

	it('接続が成立したら URL 入力欄を空にし、onConnected を呼ぶ', async () => {
		const { input, onConnected } = setup();
		vi.mocked(chatApi.connectToStream).mockResolvedValue(success(1));

		await connect(input);

		await vi.waitFor(() => expect(input.value).toBe(''));
		expect(onConnected).toHaveBeenCalledOnce();
	});

	it('接続に失敗したら onConnected を呼ばず、URL を残す', async () => {
		const { input, onConnected } = setup();
		vi.mocked(chatApi.connectToStream).mockRejectedValue(new Error('network error'));

		await connect(input);

		await vi.waitFor(() => expect(chatStore.error).toBe('network error'));
		expect(onConnected).not.toHaveBeenCalled();
		expect(input.value).toBe('https://www.youtube.com/watch?v=first');
	});

	it('Enter でも接続する', async () => {
		const { input } = setup();
		vi.mocked(chatApi.connectToStream).mockResolvedValue(success(3));

		await fireEvent.input(input, { target: { value: 'https://www.youtube.com/watch?v=enter' } });
		await fireEvent.keyDown(input, { key: 'Enter' });

		await vi.waitFor(() => expect(chatApi.connectToStream).toHaveBeenCalled());
	});

	it('接続の完了を待つ間に入力し直した URL は消さない', async () => {
		const { input } = setup();
		vi.mocked(chatApi.connectToStream).mockResolvedValue(success(2));
		// 接続の成立後、リアクションの読み込みが終わるまでの間に入力し直す
		let resolveReactions!: (summary: ReactionSummary) => void;
		vi.mocked(chatApi.getConnectionReactions).mockReturnValue(
			new Promise((resolve) => {
				resolveReactions = resolve;
			})
		);

		await connect(input);
		await vi.waitFor(() => expect(chatApi.getConnectionReactions).toHaveBeenCalled());
		await fireEvent.input(input, { target: { value: 'https://www.youtube.com/watch?v=second' } });
		resolveReactions({ totals: {}, recent: [] });

		await new Promise((r) => setTimeout(r, 0));
		expect(input.value).toBe('https://www.youtube.com/watch?v=second');
	});

	it('「すべてのコメントを取る」はチャットモードの「全て」。切り替えるとすべての接続に送る', async () => {
		setup();
		const allChat = screen.getByRole('checkbox', { name: /すべてのコメントを取る/ }) as HTMLInputElement;
		expect(allChat.checked).toBe(chatStore.chatMode === 'all');

		await fireEvent.click(allChat);
		await vi.waitFor(() => expect(chatStore.chatMode).toBe('all'));
		await fireEvent.click(allChat);
		await vi.waitFor(() => expect(chatStore.chatMode).toBe('top'));
	});
});
