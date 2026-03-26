// chat.svelte.ts の接続関連ロジックのテスト
// spec: docs/specs/02_chat.md — 接続管理フロー
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import * as chatApi from '$lib/tauri/chat';
import type { ConnectionResult, ConnectionInfo } from '$lib/types';

// chatApiをモック（setupファイルより前に宣言することでホイスティングを確保）
vi.mock('$lib/tauri/chat', () => ({
	connectToStream: vi.fn(),
	disconnectStream: vi.fn(),
	disconnectAllStreams: vi.fn(),
	setChatMode: vi.fn(),
	getConnections: vi.fn(),
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

// 成功時のConnectionResultファクトリ
function makeSuccessResult(overrides: Partial<ConnectionResult> = {}): ConnectionResult {
	return {
		success: true,
		connection_id: BigInt(1),
		stream_title: 'Test Stream',
		broadcaster_name: 'Alice',
		broadcaster_channel_id: 'UC_alice',
		is_replay: false,
		error: null,
		session_id: 'sess_1',
		...overrides,
	};
}

// ConnectionInfoファクトリ
function makeConnectionInfo(overrides: Partial<ConnectionInfo> = {}): ConnectionInfo {
	return {
		id: BigInt(1),
		platform: 'youtube',
		stream_url: 'https://example.com',
		stream_title: 'Test Stream',
		broadcaster_name: 'Alice',
		broadcaster_channel_id: 'UC_alice',
		is_monitoring: true,
		is_cancelling: false,
		...overrides,
	};
}

describe('chatStore 接続管理', () => {
	let chatStore: typeof import('./chat.svelte').chatStore;

	beforeEach(async () => {
		vi.clearAllMocks();
		vi.mocked(listen).mockImplementation(async () => () => {});

		vi.resetModules();
		const mod = await import('./chat.svelte');
		chatStore = mod.chatStore;
	});

	afterEach(() => {
		chatStore.cleanup();
	});

	// =====================================================================
	// connect()
	// =====================================================================
	describe('connect', () => {
		// spec: 成功時に connections へ新エントリが追加される
		it('成功時に connections へ新エントリが追加される', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());

			const result = await chatStore.connect('https://example.com');

			// spec: 戻り値の success が true
			expect(result.success).toBe(true);
			// spec: connections に1件追加される
			expect(chatStore.connections.size).toBe(1);
			// spec: isConnected が true になる
			expect(chatStore.isConnected).toBe(true);

			const conn = chatStore.connections.get(1);
			// spec: connectionState が 'connected'
			expect(conn?.connectionState).toBe('connected');
			// spec: streamTitle が API レスポンス通り
			expect(conn?.streamTitle).toBe('Test Stream');
			// spec: broadcasterName が API レスポンス通り
			expect(conn?.broadcasterName).toBe('Alice');
			// spec: broadcasterChannelId が API レスポンス通り
			expect(conn?.broadcasterChannelId).toBe('UC_alice');
		});

		// spec: 失敗時に error がセットされ connections は変化しない
		it('失敗時に error がセットされ connections は変化しない', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue({
				success: false,
				connection_id: BigInt(0),
				stream_title: null,
				broadcaster_name: null,
				broadcaster_channel_id: null,
				is_replay: false,
				error: 'stream not found',
				session_id: null,
			});

			await chatStore.connect('https://example.com');

			// spec: 接続失敗時は connections に追加されない
			expect(chatStore.connections.size).toBe(0);
			// spec: error に API のエラーメッセージがセットされる
			expect(chatStore.error).toBe('stream not found');
		});

		// spec: 例外発生時に error がセットされ success=false が返る
		it('例外発生時に error がセットされ success=false が返る', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockRejectedValue(new Error('network error'));

			const result = await chatStore.connect('https://example.com');

			// spec: 戻り値の success が false
			expect(result.success).toBe(false);
			// spec: error に例外メッセージがセットされる
			expect(chatStore.error).toBe('network error');
		});

		// spec: connect() 呼び出し前に error がクリアされる
		it('connect() 呼び出し時に前回の error がクリアされる', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			// 1回目: 失敗
			vi.mocked(connectToStream).mockResolvedValue({
				success: false,
				connection_id: BigInt(0),
				stream_title: null,
				broadcaster_name: null,
				broadcaster_channel_id: null,
				is_replay: false,
				error: 'first error',
				session_id: null,
			});
			await chatStore.connect('https://example.com');
			expect(chatStore.error).toBe('first error');

			// 2回目: 成功
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
			await chatStore.connect('https://example.com');

			// spec: 成功後は error が null になる
			expect(chatStore.error).toBeNull();
		});

		// spec: 複数接続を同時に保持できる
		it('複数URLを接続すると connections に複数エントリが追加される', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream)
				.mockResolvedValueOnce(makeSuccessResult({ connection_id: BigInt(1) }))
				.mockResolvedValueOnce(makeSuccessResult({ connection_id: BigInt(2), broadcaster_channel_id: 'UC_bob', broadcaster_name: 'Bob' }));

			await chatStore.connect('https://example1.com');
			await chatStore.connect('https://example2.com');

			expect(chatStore.connections.size).toBe(2);
			expect(chatStore.connections.has(1)).toBe(true);
			expect(chatStore.connections.has(2)).toBe(true);
		});
	});

	// =====================================================================
	// disconnect()
	// =====================================================================
	describe('disconnect', () => {
		// spec: disconnect() 完了後に connections から該当エントリが削除される
		it('disconnect() 完了後に connections から削除され isConnected が false になる', async () => {
			const { connectToStream, disconnectStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
			vi.mocked(disconnectStream).mockResolvedValue(undefined);

			await chatStore.connect('https://example.com');
			expect(chatStore.connections.size).toBe(1);

			await chatStore.disconnect(1);

			// spec: 切断後は connections から削除される
			expect(chatStore.connections.has(1)).toBe(false);
			// spec: 全接続がなくなると isConnected が false
			expect(chatStore.isConnected).toBe(false);
		});

		// spec: disconnectStream が正しい connectionId で呼ばれる
		it('disconnectStream が正しい connectionId で呼ばれる', async () => {
			const { connectToStream, disconnectStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({ connection_id: BigInt(42) }));
			vi.mocked(disconnectStream).mockResolvedValue(undefined);

			await chatStore.connect('https://example.com');
			await chatStore.disconnect(42);

			expect(disconnectStream).toHaveBeenCalledWith(42);
		});

		// spec: disconnect() は API エラーが発生しても connections から削除する
		it('disconnectStream が例外を投げても connections から削除される', async () => {
			const { connectToStream, disconnectStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
			vi.mocked(disconnectStream).mockRejectedValue(new Error('disconnect failed'));

			await chatStore.connect('https://example.com');

			// エラーが投げられても削除されること（finallyブロックの動作）
			await expect(chatStore.disconnect(1)).rejects.toThrow('disconnect failed');
			expect(chatStore.connections.has(1)).toBe(false);
		});
	});

	// =====================================================================
	// disconnectAll()
	// =====================================================================
	describe('disconnectAll', () => {
		// spec: disconnectAll() で全接続が削除される
		it('disconnectAll() で全接続が削除され isConnected が false になる', async () => {
			const { connectToStream, disconnectAllStreams } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({ connection_id: BigInt(1) }));
			vi.mocked(disconnectAllStreams).mockResolvedValue(undefined);

			await chatStore.connect('https://example.com');
			expect(chatStore.connections.size).toBe(1);

			await chatStore.disconnectAll();

			// spec: 全切断後は connections が空
			expect(chatStore.connections.size).toBe(0);
			// spec: isConnected が false
			expect(chatStore.isConnected).toBe(false);
		});

		// spec: disconnectAllStreams が呼ばれる
		it('disconnectAllStreams が呼ばれる', async () => {
			const { disconnectAllStreams } = await import('$lib/tauri/chat');
			vi.mocked(disconnectAllStreams).mockResolvedValue(undefined);

			await chatStore.disconnectAll();

			// beforeEach で resetModules するためモジュールインスタンスが各テストで異なる。
			// "1回以上呼ばれた" ことを確認する（初回呼び出しのみ）
			expect(disconnectAllStreams).toHaveBeenCalled();
		});

		// spec: disconnectAllStreams が例外を投げても connections はクリアされる
		it('disconnectAllStreams が例外を投げても connections がクリアされる', async () => {
			const { connectToStream, disconnectAllStreams } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
			vi.mocked(disconnectAllStreams).mockRejectedValue(new Error('backend error'));

			await chatStore.connect('https://example.com');
			expect(chatStore.connections.size).toBe(1);

			// finallyブロックで connections = new Map() が実行される
			await expect(chatStore.disconnectAll()).rejects.toThrow('backend error');
			expect(chatStore.connections.size).toBe(0);
		});
	});

	// =====================================================================
	// connectionState getter
	// =====================================================================
	describe('connectionState getter', () => {
		// spec: 接続0件のとき 'idle'
		it('接続0件で connectionState が idle', () => {
			expect(chatStore.connectionState).toBe('idle');
		});

		// spec: connected 接続がある場合 'connected'
		it('connected接続がある場合 connectionState が connected', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());

			await chatStore.connect('https://example.com');

			expect(chatStore.connectionState).toBe('connected');
		});

		// spec: some vs every 変異対策 — 1つがdisconnecting、1つがconnectedのとき 'connected' を返す (ID:279,293)
		it('connected のみの場合 connected を返す（some → every 変異対策）', async () => {
			// 2つの接続を追加して両方 connected にする
			const { connectToStream, disconnectStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream)
				.mockResolvedValueOnce(makeSuccessResult({ connection_id: BigInt(1) }))
				.mockResolvedValueOnce(makeSuccessResult({ connection_id: BigInt(2) }));
			await chatStore.connect('https://example.com/1');
			await chatStore.connect('https://example.com/2');

			// disconnecting を1つ混ぜる（永久にpending）
			vi.mocked(disconnectStream).mockImplementation(
				() => new Promise<void>(() => {}) // 永久にpending
			);
			const disconnectPromise = chatStore.disconnect(1);

			// 1つ connected + 1つ disconnecting
			// some(s === 'connected') = true, every(s === 'connected') = false
			expect(chatStore.connectionState).toBe('connected');

			// cleanup: 後続テストに影響しないようにdisconnectStreamを解決する
			vi.mocked(disconnectStream).mockResolvedValue(undefined);
			// disconnectPromise は永久pendingのため、cleanup() で対処
			void disconnectPromise;
		});
	});

	// =====================================================================
	// 後方互換 getter (streamTitle, broadcasterName, broadcasterChannelId)
	// =====================================================================
	describe('後方互換 getter', () => {
		// spec: 接続0件のとき null
		it('接続0件のとき streamTitle / broadcasterName / broadcasterChannelId が null', () => {
			expect(chatStore.streamTitle).toBeNull();
			expect(chatStore.broadcasterName).toBeNull();
			expect(chatStore.broadcasterChannelId).toBeNull();
		});

		// spec: 接続ありのとき最初の接続の値を返す
		it('接続ありのとき最初の接続の streamTitle / broadcasterName / broadcasterChannelId を返す', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({
				stream_title: 'Test Stream',
				broadcaster_name: 'Alice',
				broadcaster_channel_id: 'UC_alice',
			}));

			await chatStore.connect('https://example.com');

			expect(chatStore.streamTitle).toBe('Test Stream');
			expect(chatStore.broadcasterName).toBe('Alice');
			expect(chatStore.broadcasterChannelId).toBe('UC_alice');
		});

		// spec: streamTitle が空文字の場合 null を返す
		it('streamTitle が空文字の接続では null を返す', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({
				stream_title: null,
				broadcaster_name: null,
				broadcaster_channel_id: null,
			}));

			await chatStore.connect('https://example.com');

			// '' || null → null
			expect(chatStore.streamTitle).toBeNull();
			expect(chatStore.broadcasterName).toBeNull();
			expect(chatStore.broadcasterChannelId).toBeNull();
		});
	});

	// =====================================================================
	// chat:connection イベント
	// =====================================================================
	describe('chat:connection イベント', () => {
		let emitConnection: (result: ConnectionResult) => void;

		beforeEach(async () => {
			vi.mocked(listen).mockReset();
			vi.mocked(listen).mockImplementation(async (event: string, handler: unknown) => {
				if (event === 'chat:connection') {
					emitConnection = (result: ConnectionResult) =>
						(handler as (e: { payload: ConnectionResult }) => void)({ payload: result });
				}
				return () => {};
			});

			vi.resetModules();
			vi.mock('$lib/tauri/chat', () => ({
				connectToStream: vi.fn(),
				disconnectStream: vi.fn(),
				disconnectAllStreams: vi.fn(),
				setChatMode: vi.fn(),
				getConnections: vi.fn(),
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

			const mod = await import('./chat.svelte');
			chatStore = mod.chatStore;
			await chatStore.setupEventListeners();
		});

		// spec: success=true のとき接続情報が更新される
		it('success=true で streamTitle と broadcasterName が更新される', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({
				stream_title: 'Old Title',
				broadcaster_name: 'Alice',
			}));
			await chatStore.connect('https://example.com');

			// chat:connection イベントで情報更新
			emitConnection!({
				success: true,
				connection_id: BigInt(1),
				stream_title: 'New Title',
				broadcaster_name: 'NewAlice',
				broadcaster_channel_id: 'UC_alice',
				is_replay: false,
				error: null,
				session_id: 'sess_1',
			});

			const conn = chatStore.connections.get(1);
			// spec: streamTitle が新しい値に更新される
			expect(conn?.streamTitle).toBe('New Title');
			// spec: broadcasterName が新しい値に更新される
			expect(conn?.broadcasterName).toBe('NewAlice');
		});

		// spec: success=true のとき connectionState が 'connected' になる
		it('success=true で connectionState が connected になる', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
			await chatStore.connect('https://example.com');

			emitConnection!({
				success: true,
				connection_id: BigInt(1),
				stream_title: 'Title',
				broadcaster_name: 'Alice',
				broadcaster_channel_id: 'UC_alice',
				is_replay: false,
				error: null,
				session_id: 'sess_1',
			});

			const conn = chatStore.connections.get(1);
			expect(conn?.connectionState).toBe('connected');
		});

		// spec: success=false かつ connected 状態のとき接続が削除されて error がセットされる
		it('success=false + connected 状態で接続が削除されエラーがセットされる', async () => {
			const { connectToStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
			await chatStore.connect('https://example.com');
			expect(chatStore.connections.size).toBe(1);

			// 異常終了を通知
			emitConnection!({
				success: false,
				connection_id: BigInt(1),
				stream_title: null,
				broadcaster_name: null,
				broadcaster_channel_id: null,
				is_replay: false,
				error: 'watch task died',
				session_id: null,
			});

			// spec: 接続が削除される
			expect(chatStore.connections.has(1)).toBe(false);
			// spec: error がセットされる
			expect(chatStore.error).toBe('watch task died');
		});

		// spec: 存在しない connection_id のイベントは無視される
		it('存在しない connection_id のイベントは無視される', async () => {
			emitConnection!({
				success: true,
				connection_id: BigInt(999),
				stream_title: 'Ghost',
				broadcaster_name: 'Ghost',
				broadcaster_channel_id: 'UC_ghost',
				is_replay: false,
				error: null,
				session_id: 'sess_999',
			});

			// spec: connections は変化しない
			expect(chatStore.connections.size).toBe(0);
		});

		// spec: success=false かつ disconnecting 状態のとき削除とerrorセットは行われない
		it('success=false + disconnecting 状態のとき削除は disconnect() の finally に委ねられる', async () => {
			const { connectToStream, disconnectStream } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
			// disconnectStream は解決しないまま pending にする（イベントが先に来るシナリオ）
			let resolveDisconnect!: () => void;
			vi.mocked(disconnectStream).mockImplementation(
				() => new Promise<void>((resolve) => { resolveDisconnect = resolve; })
			);

			await chatStore.connect('https://example.com');
			// disconnect() を開始（pending）
			const disconnectPromise = chatStore.disconnect(1);

			// disconnecting 状態のまま success=false イベントが来る
			emitConnection!({
				success: false,
				connection_id: BigInt(1),
				stream_title: null,
				broadcaster_name: null,
				broadcaster_channel_id: null,
				is_replay: false,
				error: 'intentional disconnect',
				session_id: null,
			});

			// spec: disconnecting 中は error をセットしない
			expect(chatStore.error).toBeNull();

			// disconnect を完了させる
			resolveDisconnect();
			await disconnectPromise;
		});
	});

	// =====================================================================
	// initialize()
	// =====================================================================
	describe('initialize', () => {
		// spec: initialize() で全てクリアして idle 状態に戻る
		it('全てクリアして idle 状態に戻る', async () => {
			const { connectToStream, disconnectAllStreams } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
			vi.mocked(disconnectAllStreams).mockResolvedValue(undefined);

			await chatStore.connect('https://example.com');
			// メッセージはsetupEventListeners経由で追加しないため接続のみ確認

			await chatStore.initialize();

			// spec: connections が空になる
			expect(chatStore.connections.size).toBe(0);
			// spec: messages が空になる
			expect(chatStore.messages).toHaveLength(0);
			// spec: error が null になる
			expect(chatStore.error).toBeNull();
			// spec: connectionState が idle
			expect(chatStore.connectionState).toBe('idle');
		});

		// spec: initialize() 後に再接続できる
		it('initialize() 後に再接続できる', async () => {
			const { connectToStream, disconnectAllStreams } = await import('$lib/tauri/chat');
			vi.mocked(disconnectAllStreams).mockResolvedValue(undefined);
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());

			await chatStore.connect('https://example.com');
			await chatStore.initialize();

			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({ connection_id: BigInt(2) }));
			const result = await chatStore.connect('https://example2.com');

			expect(result.success).toBe(true);
			expect(chatStore.connections.size).toBe(1);
		});
	});

	// =====================================================================
	// restoreConnections()
	// =====================================================================
	describe('restoreConnections', () => {
		// spec: バックエンドの接続をフロントエンドに復元する
		it('バックエンドの接続を connections に追加する', async () => {
			const { getConnections } = await import('$lib/tauri/chat');
			vi.mocked(getConnections).mockResolvedValue([makeConnectionInfo({
				id: BigInt(1),
				stream_title: 'Restored Stream',
				broadcaster_name: 'Bob',
				broadcaster_channel_id: 'UC_bob',
				is_monitoring: true,
			})]);

			await chatStore.restoreConnections();

			// spec: connections に1件追加される
			expect(chatStore.connections.size).toBe(1);
			const conn = chatStore.connections.get(1);
			// spec: streamTitle が復元される
			expect(conn?.streamTitle).toBe('Restored Stream');
			// spec: connectionState が connected（is_monitoring=true）
			expect(conn?.connectionState).toBe('connected');
		});

		// spec: is_monitoring=false のとき connectionState が disconnecting
		it('is_monitoring=false の接続は connectionState が disconnecting で復元される', async () => {
			const { getConnections } = await import('$lib/tauri/chat');
			vi.mocked(getConnections).mockResolvedValue([makeConnectionInfo({
				is_monitoring: false,
			})]);

			await chatStore.restoreConnections();

			const conn = chatStore.connections.get(1);
			expect(conn?.connectionState).toBe('disconnecting');
		});

		// spec: 空の接続リストでは connections に変化なし
		it('空の接続リストでは connections が空のまま', async () => {
			const { getConnections } = await import('$lib/tauri/chat');
			vi.mocked(getConnections).mockResolvedValue([]);

			await chatStore.restoreConnections();

			expect(chatStore.connections.size).toBe(0);
		});

		// spec: 既にフロントエンドに存在する接続はスキップされる
		it('既存の接続はスキップされる', async () => {
			const { connectToStream, getConnections } = await import('$lib/tauri/chat');
			vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({ connection_id: BigInt(1) }));
			await chatStore.connect('https://example.com');

			vi.mocked(getConnections).mockResolvedValue([makeConnectionInfo({
				id: BigInt(1),
				stream_title: 'Should Not Override',
			})]);
			await chatStore.restoreConnections();

			// spec: 既存の接続は上書きされない
			expect(chatStore.connections.size).toBe(1);
			expect(chatStore.connections.get(1)?.streamTitle).toBe('Test Stream');
		});

		// spec: API エラーのときは console.warn を出して正常続行
		it('API エラーのとき console.warn を出して接続は空のまま正常続行する', async () => {
			const { getConnections } = await import('$lib/tauri/chat');
			vi.mocked(getConnections).mockRejectedValue(new Error('API error'));
			const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

			await chatStore.restoreConnections();

			// spec: console.warn に復元失敗メッセージが出力される
			expect(warnSpy).toHaveBeenCalledWith(
				'接続状態の復元に失敗:',
				expect.any(Error),
			);
			// spec: connections は空のまま
			expect(chatStore.connections.size).toBe(0);
			warnSpy.mockRestore();
		});
	});

	describe('connecting 中間状態', () => {
		it('connect() 中に isConnecting が true になる', async () => {
			// connectToStream を保留状態にする
			let resolveConnect!: (result: ConnectionResult) => void;
			vi.mocked(chatApi.connectToStream).mockImplementation(
				() => new Promise<ConnectionResult>(resolve => { resolveConnect = resolve; })
			);

			const connectPromise = chatStore.connect('https://example.com');

			// API応答前: connecting 状態のエントリが存在する
			expect(chatStore.isConnecting).toBe(true);
			expect(chatStore.connectionState).toBe('connecting');
			expect(chatStore.connections.size).toBe(1);

			// API応答を返す
			resolveConnect(makeSuccessResult());
			await connectPromise;

			// API応答後: connected に遷移
			expect(chatStore.isConnecting).toBe(false);
			expect(chatStore.connectionState).toBe('connected');
		});

		it('connect() 失敗時に connecting エントリが削除される', async () => {
			vi.mocked(chatApi.connectToStream).mockResolvedValue(
				makeSuccessResult({ success: false, error: 'stream not found', connection_id: BigInt(0) })
			);

			await chatStore.connect('https://example.com');

			expect(chatStore.isConnecting).toBe(false);
			expect(chatStore.connections.size).toBe(0);
			expect(chatStore.error).toBe('stream not found');
		});

		it('connect() 例外時に connecting エントリが削除される', async () => {
			vi.mocked(chatApi.connectToStream).mockRejectedValue(new Error('network error'));

			await chatStore.connect('https://example.com');

			expect(chatStore.isConnecting).toBe(false);
			expect(chatStore.connections.size).toBe(0);
		});

		it('connectionState は connecting > connected の優先度で判定する', async () => {
			// 1つ目の接続を成功させる
			vi.mocked(chatApi.connectToStream).mockResolvedValueOnce(makeSuccessResult());
			await chatStore.connect('https://example.com/1');
			expect(chatStore.connectionState).toBe('connected');

			// 2つ目の接続を保留にする
			let resolveConnect!: (result: ConnectionResult) => void;
			vi.mocked(chatApi.connectToStream).mockImplementation(
				() => new Promise<ConnectionResult>(resolve => { resolveConnect = resolve; })
			);

			const connectPromise = chatStore.connect('https://example.com/2');

			// connected + connecting の混在 → connecting が優先
			expect(chatStore.connectionState).toBe('connecting');

			resolveConnect(makeSuccessResult({ connection_id: BigInt(2) }));
			await connectPromise;

			expect(chatStore.connectionState).toBe('connected');
		});

		// spec: connecting中の仮エントリのフィールドが正しく設定される (ID:114,116,118,119,120,121)
		it('connecting中のエントリはplatform=youtubeを持つ', async () => {
			let resolveConnect!: (result: ConnectionResult) => void;
			vi.mocked(chatApi.connectToStream).mockImplementation(
				() => new Promise<ConnectionResult>(resolve => { resolveConnect = resolve; })
			);

			const connectPromise = chatStore.connect('https://example.com');

			// connecting中のエントリを取得
			const entries = [...chatStore.connections.values()];
			expect(entries).toHaveLength(1);
			expect(entries[0].platform).toBe('youtube');
			expect(entries[0].streamTitle).toBe('');
			expect(entries[0].broadcasterName).toBe('');
			expect(entries[0].broadcasterChannelId).toBe('');

			resolveConnect(makeSuccessResult());
			await connectPromise;
		});

		// spec: 仮エントリのIDは負数である (ID:114,119)
		it('仮エントリのIDは負数である', async () => {
			let resolveConnect!: (result: ConnectionResult) => void;
			vi.mocked(chatApi.connectToStream).mockImplementation(
				() => new Promise<ConnectionResult>(resolve => { resolveConnect = resolve; })
			);

			const connectPromise = chatStore.connect('https://example.com');

			const entries = [...chatStore.connections.values()];
			expect(entries[0].id).toBeLessThan(0);

			resolveConnect(makeSuccessResult());
			await connectPromise;
		});

		// spec: 複数の同時connect()で仮エントリIDが一意である (ID:121)
		it('複数の同時connect()で仮エントリIDが一意である', async () => {
			const resolvers: ((result: ConnectionResult) => void)[] = [];
			vi.mocked(chatApi.connectToStream).mockImplementation(
				() => new Promise<ConnectionResult>(resolve => { resolvers.push(resolve); })
			);

			const p1 = chatStore.connect('https://example.com/1');
			const p2 = chatStore.connect('https://example.com/2');

			const entries = [...chatStore.connections.values()];
			expect(entries).toHaveLength(2);
			// IDが異なることを確認
			expect(entries[0].id).not.toBe(entries[1].id);
			// 両方とも負数
			expect(entries[0].id).toBeLessThan(0);
			expect(entries[1].id).toBeLessThan(0);

			resolvers[0](makeSuccessResult({ connection_id: BigInt(1) }));
			resolvers[1](makeSuccessResult({ connection_id: BigInt(2) }));
			await p1;
			await p2;
		});
	});

	// =====================================================================
	// restoreConnections 詳細 (ID:213,214,216,217,221,229,230,231)
	// =====================================================================
	describe('restoreConnections 詳細', () => {
		// spec: 復元された接続の全フィールドが正しく設定される
		it('復元された接続のフィールドが正しく設定される', async () => {
			const { getConnectionColor } = await import('$lib/utils/connection-colors');
			vi.mocked(chatApi.getConnections).mockResolvedValue([makeConnectionInfo({
				id: BigInt(5),
				platform: 'youtube',
				stream_url: 'https://example.com/5',
				stream_title: 'Restored Title',
				broadcaster_name: 'Bob',
				broadcaster_channel_id: 'UC_bob',
				is_monitoring: true,
			})]);

			await chatStore.restoreConnections();

			const conn = chatStore.connections.get(5);
			expect(conn).toBeDefined();
			expect(conn!.streamTitle).toBe('Restored Title');
			expect(conn!.broadcasterName).toBe('Bob');
			expect(conn!.broadcasterChannelId).toBe('UC_bob');
			expect(conn!.platform).toBe('youtube');
			expect(conn!.connectionState).toBe('connected');
			// spec: color は broadcaster_channel_id ベースで計算される
			expect(conn!.color).toBe(getConnectionColor('UC_bob'));
		});

		// spec: is_monitoring=false の接続は disconnecting 状態で復元される
		it('is_monitoring=false の接続は disconnecting 状態で復元される', async () => {
			vi.mocked(chatApi.getConnections).mockResolvedValue([makeConnectionInfo({
				id: BigInt(3),
				is_monitoring: false,
			})]);

			await chatStore.restoreConnections();

			const conn = chatStore.connections.get(3);
			expect(conn!.connectionState).toBe('disconnecting');
		});

		// spec: 既にフロントエンドに存在する接続はスキップされる（上書きされない）
		it('既にフロントエンドに存在する接続はスキップされる', async () => {
			// 先にconnect()で接続を追加
			vi.mocked(chatApi.connectToStream).mockResolvedValue(makeSuccessResult({
				connection_id: BigInt(1),
				stream_title: 'Original',
			}));
			await chatStore.connect('https://example.com');

			// 同じID=1でrestoreConnections
			vi.mocked(chatApi.getConnections).mockResolvedValue([makeConnectionInfo({
				id: BigInt(1),
				stream_title: 'Restored',
			})]);

			await chatStore.restoreConnections();

			// 元のタイトルが保持される（上書きされない）
			const conn = chatStore.connections.get(1);
			expect(conn!.streamTitle).toBe('Original');
		});

		// spec: broadcaster_channel_id が空文字の場合 String(connId) でcolor計算される
		it('broadcaster_channel_id が空文字の場合 color が String(connId) ベースで計算される', async () => {
			const { getConnectionColor } = await import('$lib/utils/connection-colors');
			vi.mocked(chatApi.getConnections).mockResolvedValue([makeConnectionInfo({
				id: BigInt(7),
				broadcaster_channel_id: '',
			})]);

			await chatStore.restoreConnections();

			const conn = chatStore.connections.get(7);
			// spec: 空文字はfalsyなので String(connId) がcolor計算に使われる
			expect(conn!.color).toBe(getConnectionColor('7'));
		});

		// spec: 複数の接続が一度に復元される
		it('複数の接続が一度に復元される', async () => {
			vi.mocked(chatApi.getConnections).mockResolvedValue([
				makeConnectionInfo({ id: BigInt(10), broadcaster_name: 'Alice' }),
				makeConnectionInfo({ id: BigInt(20), broadcaster_name: 'Bob' }),
			]);

			await chatStore.restoreConnections();

			expect(chatStore.connections.size).toBe(2);
			expect(chatStore.connections.get(10)!.broadcasterName).toBe('Alice');
			expect(chatStore.connections.get(20)!.broadcasterName).toBe('Bob');
		});
	});

	// =====================================================================
	// setChatMode 詳細 (ID:160,161)
	// =====================================================================
	describe('setChatMode 詳細', () => {
		// spec: 接続が複数ある場合、全ての接続に対してAPIが呼ばれる
		it('接続が複数ある場合、全ての接続に対してAPIが呼ばれる', async () => {
			// 2つの接続を追加
			vi.mocked(chatApi.connectToStream)
				.mockResolvedValueOnce(makeSuccessResult({ connection_id: BigInt(1) }))
				.mockResolvedValueOnce(makeSuccessResult({ connection_id: BigInt(2) }));
			await chatStore.connect('https://example.com/1');
			await chatStore.connect('https://example.com/2');

			vi.mocked(chatApi.setChatMode).mockResolvedValue(true);
			await chatStore.setChatMode('all');

			// 両方の接続IDに対して呼ばれる
			expect(chatApi.setChatMode).toHaveBeenCalledWith(1, 'all');
			expect(chatApi.setChatMode).toHaveBeenCalledWith(2, 'all');
		});

		// spec: 一部の接続でAPI失敗しても残りは処理され chatMode は更新される
		it('一部の接続でAPI失敗しても残りは処理される', async () => {
			vi.mocked(chatApi.connectToStream)
				.mockResolvedValueOnce(makeSuccessResult({ connection_id: BigInt(1) }))
				.mockResolvedValueOnce(makeSuccessResult({ connection_id: BigInt(2) }));
			await chatStore.connect('https://example.com/1');
			await chatStore.connect('https://example.com/2');

			// 前テストの呼び出し履歴をクリア
			vi.mocked(chatApi.setChatMode).mockClear();

			const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
			vi.mocked(chatApi.setChatMode)
				.mockRejectedValueOnce(new Error('fail'))
				.mockResolvedValueOnce(true);
			await chatStore.setChatMode('all');

			// 1つ失敗しても2つ目は呼ばれる
			expect(chatApi.setChatMode).toHaveBeenCalledTimes(2);
			// spec: console.warn にconnection IDを含むメッセージが出力される
			expect(warnSpy).toHaveBeenCalledWith(
				expect.stringContaining('チャットモード変更失敗'),
				expect.any(Error),
			);
			// chatMode は更新される
			expect(chatStore.chatMode).toBe('all');
			warnSpy.mockRestore();
		});
	});

});

// =====================================================================
// 追加テスト: survived mutants を殺すための補強テスト群
// =====================================================================
describe('chatStore 追加補強テスト', () => {
	let chatStore: typeof import('./chat.svelte').chatStore;

	beforeEach(async () => {
		vi.clearAllMocks();
		vi.mocked(listen).mockImplementation(async () => () => {});

		vi.resetModules();
		const mod = await import('./chat.svelte');
		chatStore = mod.chatStore;
	});

	afterEach(() => {
		chatStore.cleanup();
	});

	// =====================================================================
	// 1. pause() が disconnectAll を実行すること
	// spec: pause() は disconnectAll のエイリアス — disconnectAllStreams が呼ばれ connections が空になる
	// =====================================================================
	it('pause() 後に connections.size === 0 かつ disconnectAllStreams が呼ばれる', async () => {
		const { connectToStream, disconnectAllStreams } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
		vi.mocked(disconnectAllStreams).mockResolvedValue(undefined);

		await chatStore.connect('https://example.com');
		expect(chatStore.connections.size).toBe(1);

		await chatStore.pause();

		// spec: pause() 後は全接続が削除される
		expect(chatStore.connections.size).toBe(0);
		// spec: disconnectAllStreams が呼ばれる
		expect(disconnectAllStreams).toHaveBeenCalled();
	});

	// =====================================================================
	// 2. resume() の戻り値が仕様通りであること
	// spec: resume() は多接続では廃止 — success=false, is_replay=false, エラーメッセージを返す
	// =====================================================================
	it('resume() は success=false, is_replay=false, error メッセージを返す', async () => {
		const result = await chatStore.resume();

		// spec: success は false
		expect(result.success).toBe(false);
		// spec: is_replay は false
		expect(result.is_replay).toBe(false);
		// spec: error に規定のメッセージ
		expect(result.error).toBe('resume() is not supported in multi-stream mode');
	});

	// =====================================================================
	// 3. initialize() が disconnectAll を呼び、かつ例外時もクリアすること
	// spec: initialize() は例外が起きても connections/messages をクリアする
	// =====================================================================
	it('initialize() 通常ケースで disconnectAllStreams が呼ばれる', async () => {
		const { disconnectAllStreams } = await import('$lib/tauri/chat');
		vi.mocked(disconnectAllStreams).mockResolvedValue(undefined);

		await chatStore.initialize();

		expect(disconnectAllStreams).toHaveBeenCalled();
	});

	it('initialize() で disconnectAllStreams が例外をthrowしても connections/messages がクリアされる', async () => {
		vi.useFakeTimers();

		// メッセージを追加するためにイベントハンドラをセットアップ
		let messageHandler: ((e: { payload: unknown }) => void) | null = null;
		vi.mocked(listen).mockReset();
		vi.mocked(listen).mockImplementation(async (event: string, handler: unknown) => {
			if (event === 'chat:message') {
				messageHandler = handler as (e: { payload: unknown }) => void;
			}
			return vi.fn();
		});

		vi.resetModules();
		const mod = await import('./chat.svelte');
		const store = mod.chatStore;
		await store.setupEventListeners();

		const { connectToStream, disconnectAllStreams } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult());
		vi.mocked(disconnectAllStreams).mockRejectedValue(new Error('backend error'));

		await store.connect('https://example.com');
		expect(store.connections.size).toBe(1);

		// メッセージを追加
		messageHandler!({ payload: {
			id: 'msg_before_init', connection_id: 1, channel_id: 'ch_1',
			author: 'User', content: 'hello', message_type: 'text',
			timestamp: '2024-01-01T00:00:00Z', author_photo: null, amount: null, currency: null,
		}});
		vi.advanceTimersByTime(50);
		expect(store.messages).toHaveLength(1);

		await store.initialize();

		// spec: 例外時も connections がクリアされる
		expect(store.connections.size).toBe(0);
		// spec: 例外時も messages がクリアされる（finallyブロックの役割）
		expect(store.messages).toHaveLength(0);

		store.cleanup();
		vi.useRealTimers();
	});

	// =====================================================================
	// 4. initialize() 後の pendingMessages 初期化
	// spec: initialize() 後は pendingMessages がクリアされ、新規メッセージのみがflushされる
	// =====================================================================
	it('initialize() 後にメッセージを追加すると messages が正確に1件', async () => {
		vi.useFakeTimers();

		let messageHandler: ((e: { payload: unknown }) => void) | null = null;
		vi.mocked(listen).mockReset();
		vi.mocked(listen).mockImplementation(async (event: string, handler: unknown) => {
			if (event === 'chat:message') {
				messageHandler = handler as (e: { payload: unknown }) => void;
			}
			return () => {};
		});

		vi.resetModules();
		vi.mock('$lib/tauri/chat', () => ({
			connectToStream: vi.fn(),
			disconnectStream: vi.fn(),
			disconnectAllStreams: vi.fn(),
			setChatMode: vi.fn(),
			getConnections: vi.fn(),
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

		const mod = await import('./chat.svelte');
		const store = mod.chatStore;

		const { disconnectAllStreams } = await import('$lib/tauri/chat');
		vi.mocked(disconnectAllStreams).mockResolvedValue(undefined);

		await store.setupEventListeners();
		await store.initialize();

		// initialize() 後に1件だけメッセージを追加
		const testMsg = {
			id: 'msg_1',
			connection_id: 1,
			channel_id: 'ch_1',
			author: 'User',
			content: 'hello',
			message_type: 'text' as const,
			timestamp: '2024-01-01T00:00:00Z',
			author_photo: null,
			amount: null,
			currency: null,
		};
		messageHandler!({ payload: testMsg });
		vi.advanceTimersByTime(50);

		// spec: pendingMessages が初期化されているため messages は正確に1件
		expect(store.messages).toHaveLength(1);

		store.cleanup();
		vi.useRealTimers();
	});

	// =====================================================================
	// 5. setChatMode() の動作確認
	// spec: setChatMode() は chatMode を更新し、全接続に chatApi.setChatMode を呼ぶ
	// =====================================================================
	it('setChatMode("all") で chatMode が all になり chatApi.setChatMode が呼ばれる', async () => {
		const { connectToStream, setChatMode } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({ connection_id: BigInt(1) }));
		vi.mocked(setChatMode).mockResolvedValue(true);

		await chatStore.connect('https://example.com');
		await chatStore.setChatMode('all');

		// spec: chatMode が 'all' になる
		expect(chatStore.chatMode).toBe('all');
		// spec: chatApi.setChatMode が呼ばれる
		expect(setChatMode).toHaveBeenCalledWith(1, 'all');
	});

	it('setChatMode() で chatApi.setChatMode が例外をthrowしても console.warn が呼ばれて正常続行', async () => {
		const { connectToStream, setChatMode } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({ connection_id: BigInt(1) }));
		vi.mocked(setChatMode).mockRejectedValue(new Error('mode error'));
		const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

		await chatStore.connect('https://example.com');

		// 例外が発生しても正常続行（throwされない）
		await expect(chatStore.setChatMode('all')).resolves.toBeUndefined();
		// spec: console.warn が呼ばれる
		expect(warnSpy).toHaveBeenCalled();

		warnSpy.mockRestore();
	});

	// =====================================================================
	// 6. connect() 成功時の platform フィールド
	// spec: connect() 成功時の FrontendConnectionState に platform: 'youtube' がセットされる
	// =====================================================================
	it('connect() 成功後に connections.get(connId).platform === "youtube"', async () => {
		const { connectToStream } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({ connection_id: BigInt(1) }));

		await chatStore.connect('https://example.com');

		const conn = chatStore.connections.get(1);
		// spec: platform は 'youtube'
		expect(conn?.platform).toBe('youtube');
	});

	// =====================================================================
	// 7. 存在しない connectionId での disconnect() 安全性
	// spec: disconnect() は存在しない connectionId に対しても disconnectStream を呼ぶ
	// =====================================================================
	it('disconnect(999) で不正なエントリが追加されず disconnectStream が呼ばれる', async () => {
		const { disconnectStream } = await import('$lib/tauri/chat');
		vi.mocked(disconnectStream).mockResolvedValue(undefined);

		await chatStore.disconnect(999);

		// spec: connections に不正なエントリが追加されない
		expect(chatStore.connections.has(999)).toBe(false);
		// spec: disconnectStream が呼ばれる
		expect(disconnectStream).toHaveBeenCalledWith(999);
	});

	// =====================================================================
	// 8. chat:connection イベントで broadcaster_channel_id=null のとき既存値保持
	// spec: broadcaster_channel_id が null のとき既存の broadcasterChannelId を保持する
	// =====================================================================
	it('chat:connection イベントで broadcaster_channel_id=null のとき既存値 "UC_alice" を保持する', async () => {
		let emitConnectionLocal: ((result: ConnectionResult) => void) | undefined;

		vi.mocked(listen).mockReset();
		vi.mocked(listen).mockImplementation(async (event: string, handler: unknown) => {
			if (event === 'chat:connection') {
				emitConnectionLocal = (result: ConnectionResult) =>
					(handler as (e: { payload: ConnectionResult }) => void)({ payload: result });
			}
			return () => {};
		});

		vi.resetModules();
		vi.mock('$lib/tauri/chat', () => ({
			connectToStream: vi.fn(),
			disconnectStream: vi.fn(),
			disconnectAllStreams: vi.fn(),
			setChatMode: vi.fn(),
			getConnections: vi.fn(),
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

		const mod = await import('./chat.svelte');
		const store = mod.chatStore;
		await store.setupEventListeners();

		const { connectToStream } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(
			makeSuccessResult({ connection_id: BigInt(1), broadcaster_channel_id: 'UC_alice' })
		);
		await store.connect('https://example.com');
		expect(store.connections.get(1)?.broadcasterChannelId).toBe('UC_alice');

		// broadcaster_channel_id=null のイベントを送信
		emitConnectionLocal!({
			success: true,
			connection_id: BigInt(1),
			stream_title: 'Some Title',
			broadcaster_name: 'Alice',
			broadcaster_channel_id: null,
			is_replay: false,
			error: null,
			session_id: 'sess_1',
		});

		// spec: null のとき既存の broadcasterChannelId が保持される
		expect(store.connections.get(1)?.broadcasterChannelId).toBe('UC_alice');

		store.cleanup();
	});

	// =====================================================================
	// 9. cleanup() が unlisten 関数を呼ぶこと
	// spec: cleanup() はイベントリスナーを解除する
	// =====================================================================
	it('cleanup() が listen の返すunlisten関数を呼ぶ', async () => {
		const unlistenMessage = vi.fn();
		const unlistenConnection = vi.fn();
		vi.clearAllMocks();
		vi.mocked(listen).mockReset();
		vi.mocked(listen).mockImplementation(async (event: string) => {
			if (event === 'chat:message') return unlistenMessage;
			if (event === 'chat:connection') return unlistenConnection;
			return vi.fn();
		});

		vi.resetModules();
		const mod = await import('./chat.svelte');
		const store = mod.chatStore;
		await store.setupEventListeners();

		// setupEventListeners 後、unlisten関数はまだ呼ばれていない
		expect(unlistenMessage).not.toHaveBeenCalled();
		expect(unlistenConnection).not.toHaveBeenCalled();

		// cleanup() で両方のunlistenが呼ばれる
		store.cleanup();
		expect(unlistenMessage).toHaveBeenCalledTimes(1);
		expect(unlistenConnection).toHaveBeenCalledTimes(1);

		// 二重呼び出しでも安全（unlisten=nullなので再呼び出しされない）
		store.cleanup();
		expect(unlistenMessage).toHaveBeenCalledTimes(1);
		expect(unlistenConnection).toHaveBeenCalledTimes(1);
	});

	// =====================================================================
	// 10. connect() 成功時の color 計算（broadcaster_channel_id=null 時は connection_id を使う）
	// spec: broadcaster_channel_id が null のとき getConnectionColor(String(connId)) が使われる
	// spec: broadcaster_channel_id が非null のとき getConnectionColor(broadcaster_channel_id) が使われる
	// =====================================================================
	it('broadcaster_channel_id=null のとき color が String(connId) ベースで計算される', async () => {
		const { getConnectionColor } = await import('$lib/utils/connection-colors');
		const { connectToStream } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(
			makeSuccessResult({ connection_id: BigInt(5), broadcaster_channel_id: null })
		);

		await chatStore.connect('https://example.com');

		const conn = chatStore.connections.get(5);
		// spec: color が non-null
		expect(conn?.color).not.toBeNull();
		// spec: color が getConnectionColor(String(5)) と一致
		expect(conn?.color).toBe(getConnectionColor('5'));
	});

	it('broadcaster_channel_id が非null のとき color が getConnectionColor(broadcaster_channel_id) になる', async () => {
		const { getConnectionColor } = await import('$lib/utils/connection-colors');
		const { connectToStream } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(
			makeSuccessResult({ connection_id: BigInt(5), broadcaster_channel_id: 'UC_alice' })
		);

		await chatStore.connect('https://example.com');

		const conn = chatStore.connections.get(5);
		// spec: color が getConnectionColor('UC_alice') と一致
		expect(conn?.color).toBe(getConnectionColor('UC_alice'));
	});

	it('broadcaster_channel_id=null の color と非null の color は異なる値になる', async () => {
		const { getConnectionColor } = await import('$lib/utils/connection-colors');
		const colorWithNull = getConnectionColor('5');
		const colorWithId = getConnectionColor('UC_alice');
		// spec: 両ケースで color が異なる（パレットの多様性の確認）
		expect(colorWithNull).not.toBe(colorWithId);
	});

	// =====================================================================
	// 11. connect() 例外時の is_replay フィールド
	// spec: connect() で例外が発生した場合、戻り値の is_replay は false
	// =====================================================================
	it('connect() 例外時の戻り値 is_replay === false', async () => {
		const { connectToStream } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockRejectedValue(new Error('network error'));

		const result = await chatStore.connect('https://example.com');

		// spec: 例外時の戻り値 is_replay は false
		expect(result.is_replay).toBe(false);
	});

	// =====================================================================
	// 12. connectionState — 'disconnecting' のみの接続がある場合
	// spec: disconnecting のみの接続では connectionState は 'idle'（connecting でも connected でもない）
	// =====================================================================
	it('disconnecting のみの接続がある場合 connectionState が idle', async () => {
		const { connectToStream, disconnectStream } = await import('$lib/tauri/chat');
		vi.mocked(connectToStream).mockResolvedValue(makeSuccessResult({ connection_id: BigInt(1) }));

		const pendingResolve: (() => void)[] = [];
		vi.mocked(disconnectStream).mockImplementation(
			() => new Promise<void>((resolve) => { pendingResolve.push(resolve); })
		);

		await chatStore.connect('https://example.com');
		// disconnect() を開始（pending状態 — disconnecting になる）
		const disconnectPromise = chatStore.disconnect(1);

		// この時点で connectionState は disconnecting のみ
		// spec: disconnecting のみなら 'idle'（connecting でも connected でもない）
		expect(chatStore.connectionState).toBe('idle');

		// クリーンアップ
		pendingResolve[0]();
		await disconnectPromise;
	});
});

// =====================================================================
// initDisplaySettings() — vi.doMock で動的モックを使い isLoaded を制御する
// =====================================================================
describe('chatStore initDisplaySettings', () => {
	afterEach(() => {
		vi.resetModules();
	});

	// spec: isLoaded=false の場合は何もしない（デフォルト値を維持）
	it('isLoaded=false のとき displaySettings はデフォルト値のまま', async () => {
		vi.resetModules();
		// vi.doMock はホイスティングされないため、テスト内で順序通りに実行される
		vi.doMock('$lib/tauri/chat', () => ({
			connectToStream: vi.fn(),
			disconnectStream: vi.fn(),
			disconnectAllStreams: vi.fn(),
			setChatMode: vi.fn(),
			getConnections: vi.fn(),
		}));
		vi.doMock('./config.svelte', () => ({
			configStore: {
				isLoaded: false,
				messageFontSize: 99, // isLoaded=false なので反映されないはず
				showTimestamps: false,
				autoScrollEnabled: false,
				setMessageFontSize: vi.fn(),
			},
		}));

		const mod = await import('./chat.svelte');
		const store = mod.chatStore;

		store.initDisplaySettings();

		// spec: isLoaded=false のため configStore の値は反映されず、デフォルト値のまま
		expect(store.messageFontSize).toBe(13);
		expect(store.showTimestamps).toBe(true);
		expect(store.autoScroll).toBe(true);

		store.cleanup();
		vi.doUnmock('$lib/tauri/chat');
		vi.doUnmock('./config.svelte');
	});

	// spec: isLoaded=true の場合に configStore の値が反映される
	it('isLoaded=true のとき configStore の値が messageFontSize/showTimestamps/autoScroll に反映される', async () => {
		vi.resetModules();
		vi.doMock('$lib/tauri/chat', () => ({
			connectToStream: vi.fn(),
			disconnectStream: vi.fn(),
			disconnectAllStreams: vi.fn(),
			setChatMode: vi.fn(),
			getConnections: vi.fn(),
		}));
		vi.doMock('./config.svelte', () => ({
			configStore: {
				isLoaded: true,
				messageFontSize: 20,
				showTimestamps: false,
				autoScrollEnabled: false,
				setMessageFontSize: vi.fn(),
			},
		}));

		const mod = await import('./chat.svelte');
		const store = mod.chatStore;

		store.initDisplaySettings();

		// spec: messageFontSize が configStore の値になる
		expect(store.messageFontSize).toBe(20);
		// spec: showTimestamps が configStore の値になる
		expect(store.showTimestamps).toBe(false);
		// spec: autoScroll が configStore.autoScrollEnabled になる
		expect(store.autoScroll).toBe(false);

		store.cleanup();
		vi.doUnmock('$lib/tauri/chat');
		vi.doUnmock('./config.svelte');
	});
});
