// spec: docs/specs/02_chat.md「状態の行の統計（StatusLine）」— /分 は直近 60 秒に投稿されたコメントの数
import { describe, it, expect } from 'vitest';
import { countRecentMessages, shownCountLabel } from './chat-stats';

// いま 12:00:00（UTC）
const NOW_MS = Date.UTC(2026, 9, 3, 12, 0, 0);
const at = (h: number, m: number, s: number) => ({
	timestamp_usec: String(Date.UTC(2026, 9, 3, h, m, s) * 1000),
});

describe('countRecentMessages', () => {
	it('11:58:00・11:59:10・11:59:50 に投稿されたコメント → 2', () => {
		expect(countRecentMessages([at(11, 58, 0), at(11, 59, 10), at(11, 59, 50)], NOW_MS)).toBe(2);
	});

	it('最後のコメントが 11:59:50 で、そのあと 60 秒コメントが無い → 12:01:00 までに 0', () => {
		const messages = [at(11, 59, 10), at(11, 59, 50)];
		expect(countRecentMessages(messages, Date.UTC(2026, 9, 3, 12, 0, 51))).toBe(0);
	});

	it('アーカイブ（投稿時刻が過去）のコメントだけ → 0', () => {
		expect(countRecentMessages([at(3, 0, 0), at(3, 0, 5)], NOW_MS)).toBe(0);
	});

	it('メッセージが無い → 0', () => {
		expect(countRecentMessages([], NOW_MS)).toBe(0);
	});
});

// spec: docs/specs/02_chat.md「状態の行の統計（StatusLine）」の「表示中の件数」— フィルタ・表示件数で一部を隠しているときだけ出す
describe('shownCountLabel', () => {
	it('フィルタも表示件数も使っていない → 出さない', () => {
		expect(shownCountLabel(30, 30)).toBeNull();
	});

	it('全 80 件で、表示件数を 50 にした → 表示中 50 / 全 80 件', () => {
		expect(shownCountLabel(50, 80)).toBe('表示中 50 / 全 80 件');
	});

	it('全 80 件で、検索で 12 件に絞った → 表示中 12 / 全 80 件', () => {
		expect(shownCountLabel(12, 80)).toBe('表示中 12 / 全 80 件');
	});

	it('大きい数は 3 桁ごとに区切る', () => {
		expect(shownCountLabel(500, 1820)).toBe('表示中 500 / 全 1,820 件');
	});
});
