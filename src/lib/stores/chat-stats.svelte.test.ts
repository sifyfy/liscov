// spec: docs/specs/02_chat.md「接続設定の統計（InputSection）」「視聴者情報パネル」「メンバーシップギフトの受け取り」
// 受信のたびにストアが差分で数えた値と、視聴者ごとの一覧が画面に届くこと、受け取りを贈った人の行にまとめることを確かめる
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
		hidePanelOnConnect: true,
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
		author_reading: null,
		author_notes: null,
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

	// 02_chat.md「メンバーシップギフトの受け取り」の「まとめ方」
	describe('メンバーシップギフトの受け取りのまとめ', () => {
		const giftBy = (gifter: string, overrides: Partial<ChatMessage> = {}) =>
			message(`UC${gifter}`, { author: gifter, message_type: 'membership_gift', ...overrides });
		const redemption = (channelId: string, gifter: string | null, overrides: Partial<ChatMessage> = {}) =>
			message(channelId, {
				message_type: 'membership_gift_redemption',
				content: gifter ? `received a gift membership by ${gifter}` : 'received a gift membership',
				is_member: true,
				metadata: {
					amount: null,
					milestone_months: null,
					membership_level: null,
					gift_count: null,
					gifter,
					gift_name: null,
					gift_image_url: null,
					jewel_count: null,
					badges: [],
					badge_info: [],
					is_moderator: false,
					is_verified: false,
					superchat_colors: null,
				},
				...overrides,
			});
		const authors = (msgs: readonly ChatMessage[]) => msgs.map((m) => m.author);

		it('贈った人のギフトの後に届いた受け取りは、そのギフトの行にまとめ、流れの行は増えない', () => {
			const gift = giftBy('@g');
			receive(gift, redemption('UCa', '@g'), redemption('UCb', '@g'));
			receive(redemption('UCc', '@g'));

			expect(chatStore.messages).toEqual([gift]);
			expect(authors(chatStore.recipientsOf(gift))).toEqual(['@UCa', '@UCb', '@UCc']);
		});

		it('UCa のメンバーシップギフト 1 件と、それにまとめた UCb・UCc の受け取り → 件は 1、人は 3', () => {
			receive(message('UCa', { message_type: 'membership_gift', author: '@g' }), redemption('UCb', '@g'), redemption('UCc', '@g'));

			expect(chatStore.messages.length).toBe(1);
			expect(chatStore.viewerCount).toBe(3);
		});

		it('まとめ先が無ければ受け取りの行を出し、続く同じ贈った人の受け取りをそこにまとめる', () => {
			const first = redemption('UCa', '@g');
			receive(first, redemption('UCb', '@g'));
			receive(redemption('UCc', '@g'));

			expect(chatStore.messages).toEqual([first]);
			expect(authors(chatStore.recipientsOf(first))).toEqual(['@UCa', '@UCb', '@UCc']);
		});

		it('贈った人が分からない受け取り同士もまとめる', () => {
			const first = redemption('UCa', null);
			receive(first, redemption('UCb', null));

			expect(chatStore.messages).toEqual([first]);
			expect(authors(chatStore.recipientsOf(first))).toEqual(['@UCa', '@UCb']);
		});

		it('2 回贈ったら、受け取りは届いた時点で最後のギフトの行に入る', () => {
			const gift1 = giftBy('@g');
			const gift2 = giftBy('@g');
			receive(gift1, redemption('UCa', '@g'), gift2, redemption('UCb', '@g'));

			expect(authors(chatStore.recipientsOf(gift1))).toEqual(['@UCa']);
			expect(authors(chatStore.recipientsOf(gift2))).toEqual(['@UCb']);
		});

		it('別の接続のギフトにはまとめない', () => {
			const gift = giftBy('@g', { connection_id: BigInt(1) });
			const other = redemption('UCa', '@g', { connection_id: BigInt(2) });
			receive(gift, other);

			expect(chatStore.messages).toEqual([gift, other]);
			expect(chatStore.recipientsOf(gift)).toEqual([]);
		});

		it('コメントを消した後に届いた受け取りは、新しい受け取りの行になる', () => {
			receive(giftBy('@g'));
			chatStore.clearMessages();
			const after = redemption('UCa', '@g');
			receive(after);

			expect(chatStore.messages).toEqual([after]);
		});

		it('受け取りの行はお礼の一覧にも流れた新規さんにも入らない', () => {
			const gift = giftBy('@g');
			const row = redemption('UCa', '@h');
			receive(gift, redemption('UCb', '@g'), row);

			expect(chatStore.supporters).toEqual([gift]);
			expect(chatStore.newcomers).toEqual([]);
		});

		it('メンバーシップのフィルタで受け取りの行も隠れる', () => {
			const row = redemption('UCa', '@g');
			const text = message('UCb');
			receive(row, text);
			chatStore.setFilter({ showMembership: false });

			expect(chatStore.filteredMessages).toEqual([text]);
			chatStore.setFilter({ showMembership: true });
		});
	});
});
