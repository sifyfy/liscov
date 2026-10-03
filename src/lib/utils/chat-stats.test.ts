// spec: docs/specs/02_chat.md「接続設定の統計（InputSection）」— /分 は直近 60 秒に投稿されたコメントの数
import { describe, it, expect } from 'vitest';
import { countRecentMessages } from './chat-stats';

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
