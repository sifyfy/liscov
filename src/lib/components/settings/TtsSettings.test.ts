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

// spec: docs/specs/04_tts.md「TtsSettings.svelte」バックエンド固有設定 — 自動起動トグル・実行ファイルパス
// （参照・自動検出）・終了時に自動停止トグル・手動起動/停止ボタン + 起動状態表示（両バックエンドで同じ）
describe('TtsSettings 自動起動設定', () => {
	const BOUYOMI_EXE = 'C:\\BouyomiChan\\BouyomiChan.exe';

	async function renderWith(
		config: Partial<typeof defaultTtsConfig>,
		launched = { bouyomichan_launched: false, voicevox_launched: false }
	) {
		vi.mocked(ttsApi.ttsGetConfig).mockResolvedValue({ ...defaultTtsConfig, ...config });
		vi.mocked(ttsApi.ttsGetLaunchStatus).mockResolvedValue(launched);
		vi.mocked(ttsApi.ttsGetStatus).mockResolvedValue({ is_processing: false, queue_size: 0, backend_name: null });
		vi.mocked(ttsApi.ttsUpdateConfig).mockReset().mockResolvedValue(undefined);
		render(TtsSettings);
		await screen.findByText('自動起動設定');
	}

	it('自動検出で見つかったパスを欄に入れて保存する', async () => {
		vi.mocked(ttsApi.ttsDiscoverExe).mockResolvedValue(BOUYOMI_EXE);
		await renderWith({ backend: 'bouyomichan' });

		await fireEvent.click(screen.getByTitle('自動検出'));

		expect(ttsApi.ttsDiscoverExe).toHaveBeenCalledWith('bouyomichan');
		await waitFor(() =>
			expect(ttsApi.ttsUpdateConfig).toHaveBeenCalledWith(
				expect.objectContaining({ bouyomichan_exe_path: BOUYOMI_EXE })
			), { timeout: 1000 }
		);
		expect((screen.getByPlaceholderText('自動検出または参照で指定') as HTMLInputElement).value).toBe(BOUYOMI_EXE);
	});

	it('参照で選んだパスを欄に入れて保存する', async () => {
		vi.mocked(ttsApi.ttsSelectExe).mockResolvedValue('D:\\voicevox\\VOICEVOX.exe');
		await renderWith({ backend: 'voicevox' });

		await fireEvent.click(screen.getByTitle('ファイル参照'));

		await waitFor(() =>
			expect(ttsApi.ttsUpdateConfig).toHaveBeenCalledWith(
				expect.objectContaining({ voicevox_exe_path: 'D:\\voicevox\\VOICEVOX.exe' })
			), { timeout: 1000 }
		);
	});

	it('自動起動トグルを押すと反転してすぐ保存する', async () => {
		await renderWith({ backend: 'bouyomichan', bouyomichan_auto_launch: false });
		const toggle = screen.getByTestId('bouyomichan-auto-launch-toggle');

		await fireEvent.click(toggle);

		expect(toggle.getAttribute('aria-pressed')).toBe('true');
		expect(ttsApi.ttsUpdateConfig).toHaveBeenCalledWith(
			expect.objectContaining({ bouyomichan_auto_launch: true })
		);
	});

	it('終了時に自動停止トグルを押すと反転してすぐ保存する', async () => {
		await renderWith({ backend: 'voicevox', voicevox_auto_close: true });
		const toggle = screen.getByTestId('voicevox-auto-close-toggle');

		await fireEvent.click(toggle);

		expect(toggle.getAttribute('aria-pressed')).toBe('false');
		expect(ttsApi.ttsUpdateConfig).toHaveBeenCalledWith(
			expect.objectContaining({ voicevox_auto_close: false })
		);
	});

	it('停止中なら「起動」で、欄のパスを渡して起動する', async () => {
		vi.mocked(ttsApi.ttsLaunchBackend).mockResolvedValue(1234);
		await renderWith({ backend: 'bouyomichan', bouyomichan_exe_path: BOUYOMI_EXE });
		const button = screen.getByTestId('bouyomichan-launch-button');
		expect(button.textContent?.trim()).toBe('起動');
		expect(screen.getByText('停止中')).toBeTruthy();

		await fireEvent.click(button);

		await waitFor(() => expect(ttsApi.ttsLaunchBackend).toHaveBeenCalledWith('bouyomichan', BOUYOMI_EXE));
	});

	it('起動中なら「停止」で止める', async () => {
		vi.mocked(ttsApi.ttsKillBackend).mockResolvedValue(undefined);
		await renderWith({ backend: 'voicevox' }, { bouyomichan_launched: false, voicevox_launched: true });
		const button = screen.getByTestId('voicevox-launch-button');
		await waitFor(() => expect(button.textContent?.trim()).toBe('停止'));
		expect(screen.getByText('起動中')).toBeTruthy();

		await fireEvent.click(button);

		await waitFor(() => expect(ttsApi.ttsKillBackend).toHaveBeenCalledWith('voicevox'));
	});
});
