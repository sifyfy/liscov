// spec: docs/specs/02_chat.md「接続設定の統計（InputSection）」「視聴者情報パネル」
// 受信のたびにストアが差分で数えた値と、視聴者ごとの一覧が画面に届くことを確かめる
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import type { ChatMessage } from '$lib/types';
import { chatStore } from './chat.svelte';

vi.mock('./config.svelte', () => ({
	configStore: {
		isLoaded: false,
		messageFontSize: 13,
		showTimestamps: true,
		autoScrollEnabled: true,
		setMessageFontSize: vi.fn(),
	},
}));

let seq = 0;
function message(channelId: string, overrides: Partial<ChatMessage> = {}): ChatMessage {
	seq += 1;
	return {
		id: `m${seq}`,
		timestamp: '2026-10-03T12:00:00+00:00',
		timestamp_usec: '0',
		author: `@${channelId}`,
		author_icon_url: null,
		channel_id: channelId,
		content: 'こんにちは',
		runs: [],
		message_type: 'text',
		amount: null,
		is_member: false,
		is_first_time_viewer: false,
		in_stream_comment_count: null,
		metadata: null,
		connection_id: BigInt(1),
		platform: 'youtube',
		broadcaster_name: 'B',
		...overrides,
	};
}

describe('chatStore の統計と視聴者ごとの一覧', () => {
	let emit: (msg: ChatMessage) => void = () => {};

	beforeEach(async () => {
		vi.useFakeTimers();
		vi.mocked(listen).mockImplementation(async (event, handler) => {
			if (event === 'chat:message') {
				const typed = handler as (e: { payload: ChatMessage }) => void;
				emit = (msg) => typed({ payload: msg });
			}
			return () => {};
		});
		chatStore.clearMessages();
		await chatStore.setupEventListeners();
	});

	afterEach(() => {
		chatStore.cleanup();
		vi.useRealTimers();
	});

	function receive(...messages: ChatMessage[]) {
		for (const msg of messages) emit(msg);
		vi.advanceTimersByTime(100);
		flushSync();
	}

	it('UCa が 3 件、UCb が 1 件、視聴者を特定できないギフトが 1 件 → 2 人', () => {
		receive(
			message('UCa'),
			message('UCa'),
			message('UCb'),
			message('UCa'),
			message('', { message_type: 'gift' })
		);
		expect(chatStore.viewerCount).toBe(2);
		expect(chatStore.messages.length).toBe(5);

		chatStore.clearMessages();
		expect(chatStore.viewerCount).toBe(0);
	});

	it('件数・人数は、新しいメッセージが届くと画面に反映される', () => {
		let observed = { count: 0, viewers: 0 };
		const stop = $effect.root(() => {
			$effect(() => {
				observed = { count: chatStore.messages.length, viewers: chatStore.viewerCount };
			});
		});

		receive(message('UCa'));
		receive(message('UCa'), message('UCb'));
		expect(observed).toEqual({ count: 3, viewers: 2 });
		stop();
	});

	// 視聴者情報パネルはその視聴者の一覧だけを見ている。開いたまま 2 件目が届いても反映される
	it('既にいる視聴者の新しいメッセージが、その視聴者の一覧に反映される', () => {
		receive(message('UCa'));
		let ofUCa = 0;
		const stop = $effect.root(() => {
			const list = $derived(chatStore.getMessagesForChannel('UCa'));
			$effect(() => {
				ofUCa = list.length;
			});
		});
		flushSync();
		expect(ofUCa).toBe(1);

		receive(message('UCa'));
		expect(ofUCa).toBe(2);
		stop();
	});
});
