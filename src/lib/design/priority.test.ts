/**
 * 読み取る情報の優先順位 (docs/design/brief.md §2) の分類
 *
 * テストの例はブリーフ §2 から取る:
 * 1. スパチャ・スーパーステッカー・ギフト・メンバーシップ (ギフトを含む)・マイルストーン
 *    (マイルストーンは message_type が membership で milestone_months を持つ。docs/specs/02_chat.md)
 * 2. 新規さん (そのチャンネルで初めてコメントした人) の初回コメント
 * 3. 新規さんの、その配信での 2 回目以降のコメント
 * 4. 常連の、その配信での初回コメント
 * 5. 常連の、その配信での 2 回目以降のコメント (強調の標準)
 *
 * 2 つの配信を同時に見ているとき (§2):
 * - 相手の配信に来たコメント・新規さんは、自分の配信のものと同じ強さ
 * - 相手の配信に来たスパチャ等は、通常のコメントとほぼ同じ強さ (5)
 */
import { describe, expect, it } from 'vitest';
import { messagePriority } from './priority';

const base = { message_type: 'text', is_first_time_viewer: false, in_stream_comment_count: 5 } as const;

describe('messagePriority', () => {
  it.each(['superchat', 'supersticker', 'gift', 'membership', 'membership_gift'])(
    '%s は優先順位 1',
    (message_type) => {
      expect(messagePriority({ ...base, message_type })).toBe(1);
    },
  );

  it('有料のものは、新規さんの初回でも優先順位 1', () => {
    expect(messagePriority({ message_type: 'superchat', is_first_time_viewer: true, in_stream_comment_count: 1 })).toBe(1);
  });

  it('新規さんの初回コメントは優先順位 2', () => {
    expect(messagePriority({ ...base, is_first_time_viewer: true, in_stream_comment_count: 1 })).toBe(2);
  });

  it('新規さんの 2 回目以降のコメントは優先順位 3', () => {
    expect(messagePriority({ ...base, is_first_time_viewer: true, in_stream_comment_count: 2 })).toBe(3);
  });

  it('常連の配信内初回コメントは優先順位 4', () => {
    expect(messagePriority({ ...base, in_stream_comment_count: 1 })).toBe(4);
  });

  it('常連の 2 回目以降のコメントは優先順位 5', () => {
    expect(messagePriority({ ...base, in_stream_comment_count: 2 })).toBe(5);
  });

  it('配信内コメント回数が取れない (送り主が分からない) コメントは優先順位 5', () => {
    expect(messagePriority({ ...base, in_stream_comment_count: null })).toBe(5);
  });

  describe('相手の配信 (コラボ) のもの', () => {
    it('スパチャは優先順位 5', () => {
      expect(messagePriority({ ...base, message_type: 'superchat' }, 'partner')).toBe(5);
    });

    it('メンバーシップは優先順位 5', () => {
      expect(messagePriority({ ...base, message_type: 'membership' }, 'partner')).toBe(5);
    });

    it('新規さんの初回コメントは、自分の配信と同じ優先順位 2', () => {
      expect(messagePriority({ ...base, is_first_time_viewer: true, in_stream_comment_count: 1 }, 'partner')).toBe(2);
    });

    it('常連の配信内初回コメントは、自分の配信と同じ優先順位 4', () => {
      expect(messagePriority({ ...base, in_stream_comment_count: 1 }, 'partner')).toBe(4);
    });
  });
});
