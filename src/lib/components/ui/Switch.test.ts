import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Switch from './Switch.svelte';

// 設定のオン・オフ (読み上げ・自動起動など)。状態は aria-pressed で表す (E2E もこれで確かめる)
describe('Switch', () => {
	it('オンなら aria-pressed="true"、オフなら "false"。名前は label', () => {
		render(Switch, { props: { pressed: true, label: 'アプリ起動時に自動起動', onclick: () => {} } });
		const on = screen.getByRole('button', { name: 'アプリ起動時に自動起動' });
		expect(on.getAttribute('aria-pressed')).toBe('true');
	});

	it('押すと onclick を呼ぶ (反転と保存は呼び出し側)', async () => {
		const onclick = vi.fn();
		render(Switch, { props: { pressed: false, label: '読み上げ', testId: 'tts-toggle', onclick } });
		const off = screen.getByTestId('tts-toggle');
		expect(off.getAttribute('aria-pressed')).toBe('false');
		await fireEvent.click(off);
		expect(onclick).toHaveBeenCalledOnce();
	});
});
