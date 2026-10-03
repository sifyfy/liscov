// spec: docs/FEATURE_SPECIFICATION.md「画面の描画エラー」— 描画中の例外はその領域だけを置き換え、ほかは動き続ける
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import ErrorBoundaryHarness from '$lib/test/ErrorBoundaryHarness.svelte';

describe('ErrorBoundary', () => {
	it('キーが重複した each で例外が出ても、その領域だけ置き換わり、外側は更新され続ける', async () => {
		const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
		render(ErrorBoundaryHarness);
		expect(screen.getByText('item-a')).toBeTruthy();

		await fireEvent.click(screen.getByText('重複させる'));
		flushSync();

		expect(screen.getByText('表示中にエラーが発生しました')).toBeTruthy();
		expect(consoleError).toHaveBeenCalledWith(expect.stringContaining('[テスト領域]'), expect.anything());

		await fireEvent.click(screen.getByText('外側を数える'));
		expect(screen.getByText('外側: 1')).toBeTruthy();
		consoleError.mockRestore();
	});

	it('「再表示」で領域を作り直す', async () => {
		const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
		render(ErrorBoundaryHarness);
		await fireEvent.click(screen.getByText('重複させる'));
		await fireEvent.click(screen.getByText('直す'));
		await fireEvent.click(screen.getByText('再表示'));

		expect(screen.queryByText('表示中にエラーが発生しました')).toBeNull();
		expect(screen.getByText('item-b')).toBeTruthy();
		consoleError.mockRestore();
	});
});
