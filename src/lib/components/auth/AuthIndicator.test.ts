import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import AuthIndicator from './AuthIndicator.svelte';
import type { AuthIndicatorState } from '$lib/types';

// spec: docs/specs/01_auth.md「AuthIndicator.svelte」
// 状態ごとの点の色と文字。文字は全文を出し、ヒントに頼らない
const cases: readonly (readonly [AuthIndicatorState, string, string, boolean])[] = [
	['unauthenticated', 'bg-fg-3', '未ログイン', false],
	['authenticated_valid', 'bg-success', 'ログイン中', false],
	['authenticated_checking', 'bg-fg-3', 'ログイン中: 検証中', false],
	['authenticated_invalid', 'bg-error', 'ログイン: セッション切れ。再ログインが必要です', true],
	['authenticated_error', 'bg-warning', 'ログイン中: 検証失敗 (ネットワークエラー)', false],
	['storage_error', 'bg-error', 'ストレージエラー。設定を確認してください', true],
];

describe('AuthIndicator', () => {
	it.each(cases)('%s: 点は %s、文字は「%s」', (state, dot, label, errorText) => {
		render(AuthIndicator, { props: { state, onclick: () => {} } });
		const button = screen.getByTestId('auth-indicator');
		expect(button.textContent?.trim()).toBe(label);
		expect(button.querySelector('[data-testid="auth-dot"]')?.className).toContain(dot);
		expect(button.className.includes('text-error')).toBe(errorText);
		// 文字に出しているので、ヒントで補わない
		expect(button.getAttribute('title')).toBeNull();
	});

	it('押すと設定のログインを開く', async () => {
		const onclick = vi.fn();
		render(AuthIndicator, { props: { state: 'authenticated_valid', onclick } });
		await fireEvent.click(screen.getByTestId('auth-indicator'));
		expect(onclick).toHaveBeenCalledOnce();
	});
});
