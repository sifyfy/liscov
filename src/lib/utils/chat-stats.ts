// 状態の行の統計（02_chat.md「状態の行の統計（StatusLine）」）
import { formatCount } from './format';

/** /分 の窓（直近 60 秒） */
const RECENT_WINDOW_MS = 60_000;
/** 末尾から辿るのをやめる古さ。一覧は到着順で、接続をまたいだ前後は数十秒に収まる前提 */
const SCAN_LIMIT_MS = 90_000;

/**
 * 直近 60 秒に投稿されたメッセージの数を数える。
 * 一覧の末尾から辿り、投稿時刻が 90 秒より前のものに当たったら止める（全件は数えない）
 */
export function countRecentMessages(
	messages: readonly { readonly timestamp_usec: string }[],
	nowMs: number
): number {
	let count = 0;
	for (let i = messages.length - 1; i >= 0; i--) {
		const postedMs = Number(messages[i].timestamp_usec) / 1000;
		if (Number.isNaN(postedMs)) continue;
		if (postedMs < nowMs - SCAN_LIMIT_MS) break;
		if (postedMs > nowMs - RECENT_WINDOW_MS) count++;
	}
	return count;
}

/**
 * フィルタ・表示件数で一部を隠しているときの「表示中 N / 全 M 件」。隠していなければ null
 */
export function shownCountLabel(shown: number, total: number): string | null {
	return shown < total ? `表示中 ${formatCount(shown)} / 全 ${formatCount(total)} 件` : null;
}
