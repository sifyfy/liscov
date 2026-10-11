// spec: docs/specs/03_websocket.md「画面の表示」— 起動を待ち終えた後、30 秒ごとに状態を取り直す (途中で止まったことに気づく)
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as wsApi from '$lib/tauri/websocket';
import { createWebsocketStore, STATUS_POLL_MS } from './websocket.svelte';
import type { WebSocketStatus } from '$lib/types';

vi.mock('$lib/tauri/websocket', () => ({ websocketGetStatus: vi.fn() }));

const running: WebSocketStatus = { is_running: true, actual_port: 8765, connected_clients: 2 };
const stopped: WebSocketStatus = { is_running: false, actual_port: null, connected_clients: 0 };

describe('websocketStore の定期の確認', () => {
	beforeEach(() => {
		vi.useFakeTimers();
		vi.mocked(wsApi.websocketGetStatus).mockReset().mockResolvedValue(running);
	});
	afterEach(() => vi.useRealTimers());

	it('間隔は 30 秒', () => {
		expect(STATUS_POLL_MS).toBe(30_000);
	});

	it('起動を待ち終えた後、30 秒ごとに状態を取り直す', async () => {
		const store = createWebsocketStore();
		await store.init();
		expect(store.initialized).toBe(true);
		const calls = vi.mocked(wsApi.websocketGetStatus).mock.calls.length;

		await vi.advanceTimersByTimeAsync(STATUS_POLL_MS - 1);
		expect(wsApi.websocketGetStatus).toHaveBeenCalledTimes(calls);
		await vi.advanceTimersByTimeAsync(1);
		expect(wsApi.websocketGetStatus).toHaveBeenCalledTimes(calls + 1);
		await vi.advanceTimersByTimeAsync(STATUS_POLL_MS);
		expect(wsApi.websocketGetStatus).toHaveBeenCalledTimes(calls + 2);
	});

	it('途中で止まったら、次の確認で止まっていると分かる', async () => {
		const store = createWebsocketStore();
		await store.init();
		expect(store.isRunning).toBe(true);

		vi.mocked(wsApi.websocketGetStatus).mockResolvedValue(stopped);
		await vi.advanceTimersByTimeAsync(STATUS_POLL_MS);
		expect(store.isRunning).toBe(false);
	});

	it('起動に失敗した (待ち終えても動いていない) ときも、その後の確認は続ける', async () => {
		vi.mocked(wsApi.websocketGetStatus).mockResolvedValue(stopped);
		const store = createWebsocketStore();
		const init = store.init();
		await vi.advanceTimersByTimeAsync(10_000);
		await init;
		expect(store.initialized).toBe(true);
		expect(store.isRunning).toBe(false);

		const calls = vi.mocked(wsApi.websocketGetStatus).mock.calls.length;
		await vi.advanceTimersByTimeAsync(STATUS_POLL_MS);
		expect(wsApi.websocketGetStatus).toHaveBeenCalledTimes(calls + 1);
	});
});
