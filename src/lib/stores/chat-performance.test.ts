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
		vi.mocked(listen).mockImplementation(async (event: string, handler: (e: { payload: ChatMessage }) => void) => {
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

	describe('フィルタ追加ケース', () => {
		// showText=false のとき text タイプのメッセージが除外される
		it('showText=false で text タイプが filteredMessages から除外される', () => {
			chatStore.setFilter({ showText: false });
			addAndFlush([createMessage('1', { message_type: 'text' })]);

			expect(chatStore.filteredMessages).toHaveLength(0);
		});

		// showMembership=false のとき membership タイプが除外される
		it('showMembership=false で membership タイプが filteredMessages から除外される', () => {
			chatStore.setFilter({ showMembership: false });
			addAndFlush([createMessage('1', { message_type: 'membership' })]);

			expect(chatStore.filteredMessages).toHaveLength(0);
		});

		// showMembership=false のとき membership_gift タイプが除外される
		it('showMembership=false で membership_gift タイプが filteredMessages から除外される', () => {
			chatStore.setFilter({ showMembership: false });
			addAndFlush([createMessage('1', { message_type: 'membership_gift' })]);

			expect(chatStore.filteredMessages).toHaveLength(0);
		});

		// showSuperchat=false のとき supersticker タイプが除外される
		it('showSuperchat=false で supersticker タイプが filteredMessages から除外される', () => {
			chatStore.setFilter({ showSuperchat: false });
			addAndFlush([createMessage('1', { message_type: 'supersticker' })]);

			expect(chatStore.filteredMessages).toHaveLength(0);
		});
	});

	describe('setFontSize クランプロジック', () => {
		// MIN_FONT_SIZE=10 未満の値はクランプされる
		it('MIN(10)未満の値を渡すと messageFontSize が 10 にクランプされる', () => {
			chatStore.setFontSize(5);

			expect(chatStore.messageFontSize).toBe(10);
		});

		// MAX_FONT_SIZE=24 超の値はクランプされる
		it('MAX(24)超の値を渡すと messageFontSize が 24 にクランプされる', () => {
			chatStore.setFontSize(30);

			expect(chatStore.messageFontSize).toBe(24);
		});

		// 範囲内の値はそのまま設定される
		it('範囲内の値を渡すと messageFontSize がその値になる', () => {
			chatStore.setFontSize(13);

			expect(chatStore.messageFontSize).toBe(13);
		});
	});

	describe('increaseFontSize / decreaseFontSize', () => {
		// increaseFontSize は messageFontSize を 1 増やす
		it('increaseFontSize で messageFontSize が 1 増加する', () => {
			chatStore.setFontSize(13);
			chatStore.increaseFontSize();

			expect(chatStore.messageFontSize).toBe(14);
		});

		// decreaseFontSize は messageFontSize を 1 減らす
		it('decreaseFontSize で messageFontSize が 1 減少する', () => {
			chatStore.setFontSize(13);
			chatStore.decreaseFontSize();

			expect(chatStore.messageFontSize).toBe(12);
		});
	});

	describe('scrollToLatest トリガー', () => {
		// scrollToLatest を呼ぶたびに scrollToLatestTrigger が単調増加する
		it('scrollToLatest を呼ぶたびに scrollToLatestTrigger がインクリメントされる', () => {
			const initial = chatStore.scrollToLatestTrigger;
			chatStore.scrollToLatest();
			expect(chatStore.scrollToLatestTrigger).toBe(initial + 1);

			chatStore.scrollToLatest();
			expect(chatStore.scrollToLatestTrigger).toBe(initial + 2);
		});
	});

	describe('フィルタ初期値', () => {
		// デフォルト状態でフィルタが全タイプ表示・検索クエリなしであることを確認
		it('デフォルト状態の filter が showText=true, showSuperchat=true, showMembership=true, searchQuery="" である', () => {
			expect(chatStore.filter.showText).toBe(true);
			expect(chatStore.filter.showSuperchat).toBe(true);
			expect(chatStore.filter.showMembership).toBe(true);
			expect(chatStore.filter.searchQuery).toBe('');
		});
	});

	// spec: showTimestamps/autoScroll/chatMode の初期値
	describe('初期値', () => {
		it('showTimestamps の初期値は true', () => {
			expect(chatStore.showTimestamps).toBe(true);
		});
		it('autoScroll の初期値は true', () => {
			expect(chatStore.autoScroll).toBe(true);
		});
		it('chatMode の初期値は top', () => {
			expect(chatStore.chatMode).toBe('top');
		});
	});

	// spec: setShowTimestamps/setAutoScroll がストア状態を変更すること
	describe('setShowTimestamps / setAutoScroll', () => {
		it('setShowTimestamps(false) で showTimestamps が false になる', () => {
			chatStore.setShowTimestamps(false);
			expect(chatStore.showTimestamps).toBe(false);
		});
		it('setAutoScroll(false) で autoScroll が false になる', () => {
			chatStore.setAutoScroll(false);
			expect(chatStore.autoScroll).toBe(false);
		});
	});

	// spec: フィルタ論理の境界ケース
	describe('フィルタ境界ケース', () => {
		it('showText=false のとき superchat は filteredMessages に残る', () => {
			chatStore.setFilter({ showText: false });
			addAndFlush([
				createMessage('1', { message_type: 'superchat' }),
				createMessage('2', { message_type: 'text' }),
			]);
			expect(chatStore.filteredMessages).toHaveLength(1);
			expect(chatStore.filteredMessages[0].id).toBe('1');
		});

		it('showMembership=true のとき membership は filteredMessages に含まれる', () => {
			chatStore.setFilter({ showMembership: true });
			addAndFlush([createMessage('1', { message_type: 'membership' })]);
			expect(chatStore.filteredMessages).toHaveLength(1);
		});

		it('showMembership=false のとき text は filteredMessages に残る', () => {
			chatStore.setFilter({ showMembership: false });
			addAndFlush([
				createMessage('1', { message_type: 'text' }),
				createMessage('2', { message_type: 'membership' }),
			]);
			expect(chatStore.filteredMessages).toHaveLength(1);
			expect(chatStore.filteredMessages[0].id).toBe('1');
		});
	});

	// spec: 検索クエリは大文字小文字を区別しない
	describe('検索クエリ大文字小文字非区別', () => {
		it('大文字クエリで小文字コンテンツがヒットする', () => {
			chatStore.setFilter({ searchQuery: 'ABC' });
			addAndFlush([
				createMessage('1', { content: 'abc text' }),
				createMessage('2', { content: 'xyz' }),
			]);
			expect(chatStore.filteredMessages).toHaveLength(1);
			expect(chatStore.filteredMessages[0].id).toBe('1');
		});

		it('小文字クエリで大文字authorがヒットする', () => {
			chatStore.setFilter({ searchQuery: 'alice' });
			addAndFlush([
				createMessage('1', { author: 'ALICE' }),
				createMessage('2', { author: 'Bob' }),
			]);
			expect(chatStore.filteredMessages).toHaveLength(1);
			expect(chatStore.filteredMessages[0].id).toBe('1');
		});
	});

	// spec: 多接続モードの初期状態確認
	describe('多接続モード初期値', () => {
		// isPaused は多接続では常に false（グローバルpauseなし）
		it('isPaused の初期値は false', () => {
			expect(chatStore.isPaused).toBe(false);
		});

		// connections は初期状態で空のMap
		it('connections の初期値は空のMap', () => {
			expect(chatStore.connections).toBeInstanceOf(Map);
			expect(chatStore.connections.size).toBe(0);
		});

		// isConnected は connections.size === 0 なので false
		it('isConnected の初期値は false', () => {
			expect(chatStore.isConnected).toBe(false);
		});

		// isReplay は後方互換のため常に false
		it('isReplay は常に false', () => {
			expect(chatStore.isReplay).toBe(false);
		});

		// cleanup() を複数回呼んでも安全であること
		it('cleanup を複数回呼んでも安全', () => {
			chatStore.cleanup();
			chatStore.cleanup();
			// エラーが発生しなければOK
		});

		// isConnecting は connections が空のため false
		it('isConnecting の初期値は false', () => {
			expect(chatStore.isConnecting).toBe(false);
		});

		// error は初期状態で null
		it('error の初期値は null', () => {
			expect(chatStore.error).toBeNull();
		});

		// connectionState は connections.size === 0 のとき 'idle'
		it('connectionState の初期値は idle', () => {
			expect(chatStore.connectionState).toBe('idle');
		});
	});

	// spec: displayLimit getter が setDisplayLimit の値を反映する
	describe('displayLimit getter', () => {
		it('setDisplayLimit(5) 後に displayLimit が 5 を返す', () => {
			chatStore.setDisplayLimit(5);
			expect(chatStore.displayLimit).toBe(5);
		});

		it('setDisplayLimit(null) 後に displayLimit が null を返す', () => {
			chatStore.setDisplayLimit(null);
			expect(chatStore.displayLimit).toBeNull();
		});
	});
});
