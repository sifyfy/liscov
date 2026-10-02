// spec: docs/specs/02_chat.md「接続状態遷移」— 接続中（connecting）のエントリの切断ボタンは無効
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import * as chatApi from '$lib/tauri/chat';
import { chatStore } from '$lib/stores/chat.svelte';
import ConnectionList from './ConnectionList.svelte';
import type { ConnectionResult } from '$lib/types';

vi.mock('$lib/tauri/chat', () => ({
	connectToStream: vi.fn(),
	disconnectStream: vi.fn(),
	disconnectAllStreams: vi.fn(),
	getConnectionReactions: vi.fn(),
	setChatMode: vi.fn(),
	getConnections: vi.fn(),
}));

describe('ConnectionList', () => {
	it('接続中のエントリの切断ボタンは押せない', () => {
		vi.mocked(chatApi.connectToStream).mockReturnValue(new Promise<ConnectionResult>(() => {}));

		void chatStore.connect('https://www.youtube.com/watch?v=abc');
		render(ConnectionList);

		expect((screen.getByTitle('切断') as HTMLButtonElement).disabled).toBe(true);
	});
});
