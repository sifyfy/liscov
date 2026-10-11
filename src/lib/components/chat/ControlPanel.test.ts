import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { chatStore } from '$lib/stores';
import ControlPanel from './ControlPanel.svelte';

// spec: docs/specs/02_chat.md「表示件数制限」
describe('ControlPanel の表示件数', () => {
	beforeEach(() => chatStore.setDisplayLimit(null));

	function select() {
		render(ControlPanel, { props: { showConnect: false, onConnected: () => {}, onOpenAuthSettings: () => {} } });
		return screen.getByRole('combobox', { name: '表示件数' }) as HTMLSelectElement;
	}

	it('選択肢は「無制限」と「500件」の 2 つ。起動したら無制限', () => {
		const limit = select();
		expect([...limit.options].map((o) => o.text)).toEqual(['無制限', '500件']);
		expect(limit.selectedOptions[0].text).toBe('無制限');
	});

	it('500件を選ぶと新しい 500 件に絞る', async () => {
		const limit = select();
		await fireEvent.change(limit, { target: { value: '500' } });
		expect(chatStore.displayLimit).toBe(500);
	});
});
