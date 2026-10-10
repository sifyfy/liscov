// 状態の行に出す異常（02_chat.md「配信中の画面」の「状態 (異常のときだけ目立つ)」）
import type { AuthIndicatorState } from '$lib/types';

/** 操作パネルを閉じていても状態の行に出す異常 */
export type StatusAlert = 'session_expired' | 'storage_error' | 'websocket_stopped';

export const STATUS_ALERT_LABELS = {
	session_expired: 'ログイン: セッション切れ',
	storage_error: 'ストレージエラー',
	websocket_stopped: 'WebSocket: 止まっています',
} as const satisfies Record<StatusAlert, string>;

/** WebSocket サーバーの状態 (websocketStore の一部) */
export interface WebSocketHealth {
	/** 起動を待ち終えたか */
	readonly initialized: boolean;
	readonly isRunning: boolean;
	/** 状態を取れなかったときの理由 */
	readonly error: string | null;
}

/**
 * いま出す異常 (ログイン、WebSocket の順)。
 * ログインの検証失敗 (ネットワークエラー)・未ログインは異常と決まっていないので出さない。
 * WebSocket は起動を待っている間は出さない
 */
export function statusAlerts(auth: AuthIndicatorState, websocket: WebSocketHealth): readonly StatusAlert[] {
	const loginAlert: StatusAlert | null =
		auth === 'authenticated_invalid' ? 'session_expired' : auth === 'storage_error' ? 'storage_error' : null;
	const websocketStopped = websocket.error !== null || (websocket.initialized && !websocket.isRunning);
	return [...(loginAlert ? [loginAlert] : []), ...(websocketStopped ? (['websocket_stopped'] as const) : [])];
}
