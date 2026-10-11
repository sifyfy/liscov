import { describe, it, expect } from 'vitest';
import { statusAlerts, STATUS_ALERT_LABELS } from './status-alerts';

// spec: docs/specs/02_chat.md「配信中の画面」の「状態 (異常のときだけ目立つ)」
const wsRunning = { initialized: true, isRunning: true, error: null };

describe('statusAlerts', () => {
	it('ログインのセッションが切れた → 「ログイン: セッション切れ」', () => {
		const alerts = statusAlerts('authenticated_invalid', wsRunning);
		expect(alerts).toEqual(['session_expired']);
		expect(STATUS_ALERT_LABELS.session_expired).toBe('ログイン: セッション切れ');
	});

	it('ストレージエラー → 「ストレージエラー」', () => {
		expect(statusAlerts('storage_error', wsRunning)).toEqual(['storage_error']);
		expect(STATUS_ALERT_LABELS.storage_error).toBe('ストレージエラー');
	});

	it('WebSocket サーバーが止まっている (起動を待ち終えても動いていない) → 「WebSocket: 止まっています」', () => {
		expect(statusAlerts('authenticated_valid', { initialized: true, isRunning: false, error: null })).toEqual([
			'websocket_stopped',
		]);
		expect(STATUS_ALERT_LABELS.websocket_stopped).toBe('WebSocket: 止まっています');
	});

	it('WebSocket の状態を取れなかった → 止まっているものとして出す', () => {
		expect(
			statusAlerts('authenticated_valid', { initialized: false, isRunning: false, error: '取得に失敗' }),
		).toEqual(['websocket_stopped']);
	});

	it('WebSocket サーバーの起動を待っている間は出さない', () => {
		expect(statusAlerts('authenticated_valid', { initialized: false, isRunning: false, error: null })).toEqual([]);
	});

	it('ログインの検証に失敗した (ネットワークエラー)・未ログイン・検証中・有効 → 出さない', () => {
		for (const state of ['authenticated_error', 'unauthenticated', 'authenticated_checking', 'authenticated_valid'] as const) {
			expect(statusAlerts(state, wsRunning)).toEqual([]);
		}
	});

	it('ログインと WebSocket が両方おかしい → ログイン、WebSocket の順に両方', () => {
		expect(statusAlerts('authenticated_invalid', { initialized: true, isRunning: false, error: null })).toEqual([
			'session_expired',
			'websocket_stopped',
		]);
	});
});
