/**
 * 流れたものの欄 (スパチャ等の一覧の概要と詳細を含む)の中身 (docs/specs/02_chat.md「配信中の画面」の「流れたもの」「配信の最後のお礼」)
 *
 * テストの例は仕様の表と例から取る:
 * - 自分の配信のスパチャ等が流れの画面の外 (上) に出た → 「この配信のスパチャ等の一覧」の概要の先頭
 * - 新規さんの初回コメント (相手の配信のものを含む) が画面の外に出た → 「流れた新規さん」の先頭
 * - 欄の中は新しいものが上。手でスクロールして戻り、また見えたら欄から外れる
 * - 相手の配信のスパチャ等はどちらの欄にも入らない (お礼の対象ではない)
 * - スパチャ等の一覧の詳細は自分の配信の優先順位 1 を時刻順 (古いものが上)
 * - 例 (640×1080): ¥10,000 の赤スパのあとにコメントが 40 件来て流れの画面から外れた → 「この配信のスパチャ等の一覧」の概要の先頭に出る
 * - 最新から離れている間に、流れに出る優先順位 1・2 が届いた → 「最新へ」に件数を添える (「自動スクロール」)
 */
import { describe, expect, it } from 'vitest';
import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';
import type { MessageOrigin } from './priority';
import { arrivedSince, flowedAbove, newcomersOf, supportersOf } from './flow';

type Message = Pick<GuiChatMessage, 'id' | 'message_type' | 'is_first_time_viewer' | 'in_stream_comment_count' | 'connection_id'>;

const PARTNER = 2n;
const originOf = (m: Message): MessageOrigin => (m.connection_id === PARTNER ? 'partner' : 'primary');

const text = (id: string, count = 5, firstTime = false, connection_id = 1n): Message => ({
  id,
  message_type: 'text',
  is_first_time_viewer: firstTime,
  in_stream_comment_count: count,
  connection_id,
});
const paid = (id: string, message_type = 'superchat', connection_id = 1n): Message => ({
  id,
  message_type,
  is_first_time_viewer: false,
  in_stream_comment_count: 1,
  connection_id,
});
const ids = (messages: readonly Message[]) => messages.map((m) => m.id);

describe('supportersOf (スパチャ等の一覧の詳細・概要の候補)', () => {
  it('自分の配信のスパチャ・ステッカー・ギフト・メンバーシップ系を時刻順 (古いものが先) に集める', () => {
    const messages = [
      paid('sc', 'superchat'),
      text('t1'),
      paid('st', 'supersticker'),
      paid('gift', 'gift'),
      paid('mem', 'membership'),
      paid('mg', 'membership_gift'),
    ];
    expect(ids(supportersOf(messages, originOf))).toEqual(['sc', 'st', 'gift', 'mem', 'mg']);
  });

  it('相手の配信のスパチャ等は入れない', () => {
    expect(ids(supportersOf([paid('mine'), paid('theirs', 'superchat', PARTNER)], originOf))).toEqual(['mine']);
  });
});

describe('newcomersOf (流れた新規さんの候補)', () => {
  it('新規さんの初回コメントを、相手の配信のものも含めて集める', () => {
    const messages = [text('new1', 1, true), text('new1-2', 2, true), text('regular', 1), text('theirs', 1, true, PARTNER)];
    expect(ids(newcomersOf(messages, originOf))).toEqual(['new1', 'theirs']);
  });
});

describe('flowedAbove (流れの画面の外 (上) に出たもの)', () => {
  it('例: ¥10,000 のスパチャのあとに 40 件来て画面から外れた → 欄の先頭に出る', () => {
    const red = paid('red');
    const after = Array.from({ length: 40 }, (_, i) => text(`c${i}`));
    const all = [red, ...after];
    // 画面のいちばん上に見えているのは 10 件目のコメント
    expect(ids(flowedAbove([red], all, after[9]))).toEqual(['red']);
  });

  it('新しいものが先に並ぶ', () => {
    const [a, b] = [paid('a'), paid('b')];
    const all = [a, text('x'), b, text('y'), text('z')];
    expect(ids(flowedAbove([a, b], all, all[4]))).toEqual(['b', 'a']);
  });

  it('画面に見えている (またはそれより下の) ものは入れない。手で戻ってまた見えたら外れる', () => {
    const [a, b] = [paid('a'), paid('b')];
    const all = [text('x'), a, text('y'), b, text('z')];
    expect(ids(flowedAbove([a, b], all, all[2]))).toEqual(['a']);
    // 上に戻って a がいちばん上に見えた
    expect(ids(flowedAbove([a, b], all, a))).toEqual([]);
  });

  it('何も見えていない (一覧が空・クリアした) ときは空', () => {
    expect(flowedAbove([], [], undefined)).toEqual([]);
    const a = paid('a');
    expect(flowedAbove([a], [a], undefined)).toEqual([]);
  });
});

describe('arrivedSince (離れている間に届いた優先順位 1・2 の件数、「最新へ」に添える)', () => {
  it('離れたときの最後のコメントより後に届いた、自分の配信のスパチャ等と新規さんの初回を数える', () => {
    const left = text('left');
    const messages = [paid('before'), text('new-before', 1, true), left, paid('sc1'), text('t'), paid('sc2', 'membership'), text('new1', 1, true)];
    expect(arrivedSince(messages, left, originOf)).toEqual({ supporters: 2, newcomers: 1 });
  });

  it('相手の配信のスパチャ等は数えない。相手の配信の新規さんの初回は数える (流れた新規さんと同じ)', () => {
    const left = text('left');
    const messages = [left, paid('theirs', 'superchat', PARTNER), text('their-new', 1, true, PARTNER), text('new-2nd', 2, true)];
    expect(arrivedSince(messages, left, originOf)).toEqual({ supporters: 0, newcomers: 1 });
  });

  it('離れたときの最後のコメントが表示件数で一覧から外れたら、一覧のすべてが離れた後に届いたもの', () => {
    const messages = [paid('sc1'), text('t'), text('new1', 1, true)];
    expect(arrivedSince(messages, text('gone'), originOf)).toEqual({ supporters: 1, newcomers: 1 });
  });

  it('何も届いていなければ 0 件', () => {
    const left = text('left');
    expect(arrivedSince([paid('before'), left], left, originOf)).toEqual({ supporters: 0, newcomers: 0 });
  });
});
