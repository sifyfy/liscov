// spec: docs/specs/04_tts.md「TtsSettings.svelte」— 自動保存の最中もフォームはそのまま（入力中の欄を失わない）
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import * as ttsApi from '$lib/tauri/tts';
import { defaultTtsConfig } from '$lib/types';
import TtsSettings from './TtsSettings.svelte';

vi.mock('$lib/tauri/tts', () => ({
	ttsGetConfig: vi.fn(),
	ttsUpdateConfig: vi.fn(),
	ttsGetStatus: vi.fn(),
	ttsGetLaunchStatus: vi.fn(),
	ttsStart: vi.fn(),
	ttsStop: vi.fn(),
	ttsTestConnection: vi.fn(),
	ttsSpeakDirect: vi.fn(),
	ttsDiscoverExe: vi.fn(),
	ttsSelectExe: vi.fn(),
	ttsLaunchBackend: vi.fn(),
	ttsKillBackend: vi.fn(),
	ttsClearQueue: vi.fn(),
	ttsSpeak: vi.fn(),
}));

describe('TtsSettings', () => {
	it('自動保存の最中もフォームを作り直さず、入力中の欄が残る', async () => {
		vi.mocked(ttsApi.ttsGetConfig).mockResolvedValue({ ...defaultTtsConfig, backend: 'bouyomichan' });
		vi.mocked(ttsApi.ttsGetLaunchStatus).mockResolvedValue({
			bouyomichan_launched: false,
			voicevox_launched: false,
		});
		vi.mocked(ttsApi.ttsGetStatus).mockResolvedValue({
			is_processing: false,
			queue_size: 0,
			backend_name: null,
		});
		// 保存は終わらないままにして「保存の最中」を作る
		vi.mocked(ttsApi.ttsUpdateConfig).mockReturnValue(new Promise(() => {}));

		render(TtsSettings);
		const host = (await screen.findByLabelText('ホスト')) as HTMLInputElement;
		const port = screen.getByLabelText('ポート') as HTMLInputElement;

		await fireEvent.input(host, { target: { value: '192.168.0.2' } });
		await fireEvent.change(host);
		await waitFor(() => expect(ttsApi.ttsUpdateConfig).toHaveBeenCalled(), { timeout: 1000 });

		expect(screen.queryByText('読み込み中...')).toBeNull();
		expect(port.isConnected).toBe(true);
		expect(host.value).toBe('192.168.0.2');
	});
});
