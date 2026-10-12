// spec: docs/specs/02_chat.md「プッシュ通知 > 設定の画面」、docs/specs/09_config.md「chat_fetch セクション」
import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as configApi from '$lib/tauri/config';
import { createConfigStore } from './config.svelte';

vi.mock('$lib/tauri/config', () => ({
	configLoad: vi.fn(),
	configSave: vi.fn(),
	configSetValue: vi.fn()
}));

describe('configStore のプッシュ通知の設定', () => {
	beforeEach(() => {
		vi.mocked(configApi.configSetValue).mockReset().mockResolvedValue(undefined);
	});

	it('既定は有効', () => {
		expect(createConfigStore().pushNotification).toBe(true);
	});

	it('切り替えるとすぐ保存する', async () => {
		const store = createConfigStore();
		await store.setPushNotification(false);
		expect(configApi.configSetValue).toHaveBeenCalledWith('chat_fetch', 'push_notification', false);
		expect(store.pushNotification).toBe(false);
		expect(store.error).toBeNull();
	});

	it('保存に失敗したら、エラーを出してチェックを保存されている値に戻す', async () => {
		vi.mocked(configApi.configSetValue).mockRejectedValue(new Error('disk full'));
		const store = createConfigStore();
		await store.setPushNotification(false);
		expect(store.pushNotification).toBe(true);
		expect(store.error).toContain('disk full');
	});
});
