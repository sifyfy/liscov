import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import type { ChatMessage } from '$lib/types';

// Mock Tauri API before importing store
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(() => Promise.resolve(() => {})),
	emit: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
}));

vi.mock('./config.svelte', () => ({
	configStore: {
		isLoaded: false,
		messageFontSize: 13,
		showTimestamps: true,
		autoScrollEnabled: true,
		setMessageFontSize: vi.fn(),
	},
}));

function createMessage(id: string, overrides: Partial<ChatMessage> = {}): ChatMessage {
	return {
		id,
		timestamp: '2026-01-27T11:36:06+09:00',
		timestamp_usec: '0',
		author: 'TestUser',
		author_icon_url: null,
		channel_id: 'UC_test',
		content: `メッセージ ${id}`,
		runs: [{ type: 'Text', content: `メッセージ ${id}` }],
		message_type: 'text',
		amount: null,
		is_member: false,
		is_first_time_viewer: false,
		in_stream_comment_count: null,
		metadata: null,
		// 多接続対応で追加されたフィールド
		connection_id: BigInt(1),
		platform: 'youtube',
		broadcaster_name: 'TestBroadcaster',
		...overrides,
	};
}

describe('chatStore パフォーマンス最適化', () => {
	let chatStore: typeof import('./chat.svelte').chatStore;
	let emitMessage: (msg: ChatMessage) => void;

	beforeEach(async () => {
		vi.useFakeTimers();
		vi.mocked(listen).mockReset();

		// listen モックでコールバックをキャプチャ
		vi.mocked(listen).mockImplementation(async (event: string, handler: any) => {
			if (event === 'chat:message') {
				emitMessage = (msg: ChatMessage) => handler({ payload: msg });
			}
			return () => {};
		});

		// モジュールを再インポートしてクリーンな状態を得る
		vi.resetModules();
		const mod = await import('./chat.svelte');
		chatStore = mod.chatStore;

		// イベントリスナーをセットアップしてemitMessageをキャプチャ
		await chatStore.setupEventListeners();
	});

	afterEach(() => {
		chatStore.cleanup();
		vi.useRealTimers();
	});

	/** メッセージを追加してバッチフラッシュを実行 */
	function addAndFlush(messages: ChatMessage[]): void {
		for (const msg of messages) {
			emitMessage(msg);
		}
		vi.advanceTimersByTime(50); // BATCH_DELAY_MS
	}

	describe('displayedMessages (displayLimit適用)', () => {
		it('displayLimit=null の場合、filteredMessages と同一の配列を返す', () => {
			chatStore.setDisplayLimit(null);
			addAndFlush([
				createMessage('1'),
				createMessage('2'),
				createMessage('3'),
			]);

			expect(chatStore.displayedMessages).toEqual(chatStore.filteredMessages);
		});

		it('displayLimit=2 の場合、末尾2件のみ返す', () => {
			chatStore.setDisplayLimit(2);
			addAndFlush([
				createMessage('1'),
				createMessage('2'),
				createMessage('3'),
			]);

			expect(chatStore.displayedMessages).toHaveLength(2);
			expect(chatStore.displayedMessages[0].id).toBe('2');
			expect(chatStore.displayedMessages[1].id).toBe('3');
		});

		it('displayLimit がメッセージ数より大きい場合、全件返す', () => {
			chatStore.setDisplayLimit(100);
			addAndFlush([
				createMessage('1'),
				createMessage('2'),
			]);

			expect(chatStore.displayedMessages).toHaveLength(2);
		});

		it('filteredMessages.length はスライス前の全件数を返す', () => {
			chatStore.setDisplayLimit(1);
			addAndFlush([
				createMessage('1'),
				createMessage('2'),
				createMessage('3'),
			]);

			expect(chatStore.filteredMessages).toHaveLength(3);
			expect(chatStore.displayedMessages).toHaveLength(1);
		});

		it('フィルタ + displayLimit の組み合わせが正しく動作する', () => {
			chatStore.setDisplayLimit(2);
			chatStore.setFilter({ showSuperchat: false });
			addAndFlush([
				createMessage('1', { message_type: 'text' }),
				createMessage('2', { message_type: 'superchat' }),
				createMessage('3', { message_type: 'text' }),
				createMessage('4', { message_type: 'text' }),
			]);

			// superchat除外で3件、うち末尾2件
			expect(chatStore.filteredMessages).toHaveLength(3);
			expect(chatStore.displayedMessages).toHaveLength(2);
			expect(chatStore.displayedMessages[0].id).toBe('3');
			expect(chatStore.displayedMessages[1].id).toBe('4');
		});
	});

	describe('重複チェック', () => {
		it('同一IDのメッセージは追加されない', () => {
			addAndFlush([createMessage('dup_1')]);
			addAndFlush([createMessage('dup_1')]);

			expect(chatStore.messages).toHaveLength(1);
		});

		it('異なるIDのメッセージは全て追加される', () => {
			addAndFlush([
				createMessage('a'),
				createMessage('b'),
				createMessage('c'),
			]);

			expect(chatStore.messages).toHaveLength(3);
		});

		it('clearMessages 後に同一IDのメッセージを再追加できる', () => {
			addAndFlush([createMessage('reuse_1')]);
			expect(chatStore.messages).toHaveLength(1);

			chatStore.clearMessages();
			addAndFlush([createMessage('reuse_1')]);
			expect(chatStore.messages).toHaveLength(1);
			expect(chatStore.messages[0].id).toBe('reuse_1');
		});

		it('同一バッチ内の重複も排除される', () => {
			addAndFlush([
				createMessage('same'),
				createMessage('same'),
			]);

			expect(chatStore.messages).toHaveLength(1);
		});
	});

	describe('フロントエンドメッセージ上限', () => {
		it('上限未満のメッセージは全件保持される', () => {
			const messages = Array.from({ length: 100 }, (_, i) => createMessage(`msg_${i}`));
			addAndFlush(messages);

			expect(chatStore.messages).toHaveLength(100);
		});

		it('clearMessages 後のメッセージ追加が正しく動作する', () => {
			const messages = Array.from({ length: 50 }, (_, i) => createMessage(`cap_${i}`));
			addAndFlush(messages);
			expect(chatStore.messages).toHaveLength(50);

			chatStore.clearMessages();

			const newMessages = Array.from({ length: 50 }, (_, i) => createMessage(`cap_${i}`));
			addAndFlush(newMessages);
			expect(chatStore.messages).toHaveLength(50);
		});
	});

	describe('デフォルトフィルタ最適化 (Phase 2)', () => {
		it('デフォルトフィルタ時にfilteredMessagesがmessagesと同一参照を返す', () => {
			// デフォルト状態: showText=true, showSuperchat=true, showMembership=true, searchQuery=''
			chatStore.setFilter({ showText: true, showSuperchat: true, showMembership: true, searchQuery: '' });
			addAndFlush([
				createMessage('ref_1'),
				createMessage('ref_2'),
				createMessage('ref_3'),
			]);

			// 同一参照（O(1)パス）
			expect(chatStore.filteredMessages).toBe(chatStore.messages);
		});

		it('フィルタ変更時にfilteredMessagesが新しい配列を返す', () => {
			chatStore.setFilter({ showSuperchat: false });
			addAndFlush([
				createMessage('filter_1', { message_type: 'text' }),
				createMessage('filter_2', { message_type: 'superchat' }),
			]);

			// 新配列（O(n)パス）
			expect(chatStore.filteredMessages).not.toBe(chatStore.messages);
			expect(chatStore.filteredMessages).toHaveLength(1);
			expect(chatStore.filteredMessages[0].id).toBe('filter_1');
		});

		it('検索クエリ設定時にO(n)フィルタが走る', () => {
			chatStore.setFilter({ showText: true, showSuperchat: true, showMembership: true, searchQuery: 'ターゲット' });
			addAndFlush([
				createMessage('search_1', { content: 'ターゲットメッセージ' }),
				createMessage('search_2', { content: '無関係' }),
			]);

			expect(chatStore.filteredMessages).not.toBe(chatStore.messages);
			expect(chatStore.filteredMessages).toHaveLength(1);
			expect(chatStore.filteredMessages[0].id).toBe('search_1');
		});
	});

	describe('channelIdインデックス (Phase 2)', () => {
		it('getMessagesForChannelで特定チャンネルのメッセージを取得できる', () => {
			addAndFlush([
				createMessage('ch_1', { channel_id: 'UC_alice' }),
				createMessage('ch_2', { channel_id: 'UC_bob' }),
				createMessage('ch_3', { channel_id: 'UC_alice' }),
			]);

			const aliceMessages = chatStore.getMessagesForChannel('UC_alice');
			expect(aliceMessages).toHaveLength(2);
			expect(aliceMessages[0].id).toBe('ch_1');
			expect(aliceMessages[1].id).toBe('ch_3');

			const bobMessages = chatStore.getMessagesForChannel('UC_bob');
			expect(bobMessages).toHaveLength(1);
			expect(bobMessages[0].id).toBe('ch_2');
		});

		it('存在しないチャンネルIDは空配列を返す', () => {
			addAndFlush([createMessage('ch_exist', { channel_id: 'UC_exist' })]);

			expect(chatStore.getMessagesForChannel('UC_nonexistent')).toEqual([]);
		});

		it('clearMessages後にインデックスもクリアされる', () => {
			addAndFlush([createMessage('ch_clear', { channel_id: 'UC_clear' })]);
			expect(chatStore.getMessagesForChannel('UC_clear')).toHaveLength(1);

			chatStore.clearMessages();
			expect(chatStore.getMessagesForChannel('UC_clear')).toEqual([]);
		});

		it('複数バッチにわたってインデックスが正しく更新される', () => {
			addAndFlush([createMessage('batch_a1', { channel_id: 'UC_a' })]);
			addAndFlush([createMessage('batch_a2', { channel_id: 'UC_a' })]);

			const aMessages = chatStore.getMessagesForChannel('UC_a');
			expect(aMessages).toHaveLength(2);
			expect(aMessages[0].id).toBe('batch_a1');
			expect(aMessages[1].id).toBe('batch_a2');
		});
	});

	describe('バッチフラッシュ', () => {
		it('メッセージが正しい順序で追加される', () => {
			addAndFlush([
				createMessage('flush_1'),
				createMessage('flush_2'),
			]);

			expect(chatStore.messages).toHaveLength(2);
			expect(chatStore.messages[0].id).toBe('flush_1');
			expect(chatStore.messages[1].id).toBe('flush_2');
		});

		it('複数バッチが順序を保って追加される', () => {
			addAndFlush([createMessage('batch1_1')]);
			addAndFlush([createMessage('batch2_1')]);

			expect(chatStore.messages).toHaveLength(2);
			expect(chatStore.messages[0].id).toBe('batch1_1');
			expect(chatStore.messages[1].id).toBe('batch2_1');
		});
	});
});
