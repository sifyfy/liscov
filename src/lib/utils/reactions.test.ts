import { describe, it, expect } from 'vitest';
import type { ReactionUpdate } from '$lib/types/generated/ReactionUpdate';
import {
  applyReactionUpdate,
  emptyReactionMeter,
  hasReactions,
  reactionsPerMinute,
  restoreReactionMeter,
  sortedReactionTotals
} from './reactions';

const SEC = 1_000_000;
const T0 = 1_790_422_357 * SEC;

function update(time: number, counts: Record<string, number>, duration = 1): ReactionUpdate {
  const total = Object.values(counts).reduce((a, b) => a + b, 0);
  return { update_time_usec: time, duration_seconds: duration, counts, total };
}

describe('リアクションメーター (02_chat.md)', () => {
  // 仕様例: 🎉4（2秒）→ 30秒後に ❤3 → 累計 ❤3・🎉4、勢い 7/分
  it('累計と直近60秒の勢いを数える', () => {
    const afterFirst = applyReactionUpdate(emptyReactionMeter, update(T0, { '🎉': 4 }, 2), T0);
    const state = applyReactionUpdate(afterFirst, update(T0 + 30 * SEC, { '❤': 3 }), T0 + 30 * SEC);

    expect(state.totals).toEqual({ '❤': 3, '🎉': 4 });
    expect(reactionsPerMinute(state, T0 + 30 * SEC)).toBe(7);
  });

  // 仕様例: 最後の更新から60秒を超えた → 累計はそのまま、勢い 0/分
  it('60秒を超えた更新は勢いに数えない', () => {
    const state = applyReactionUpdate(emptyReactionMeter, update(T0, { '❤': 3 }), T0);

    expect(reactionsPerMinute(state, T0 + 60 * SEC)).toBe(3);
    expect(reactionsPerMinute(state, T0 + 61 * SEC)).toBe(0);
    expect(state.totals).toEqual({ '❤': 3 });
  });

  // 仕様: バーは表示を始めてからの勢いの最大値を満タンとする
  it('勢いの最大値を覚えておく', () => {
    let state = applyReactionUpdate(emptyReactionMeter, update(T0, { '❤': 10 }), T0);
    state = applyReactionUpdate(state, update(T0 + 90 * SEC, { '❤': 2 }), T0 + 90 * SEC);

    expect(state.peakPerMinute).toBe(10);
    expect(reactionsPerMinute(state, T0 + 90 * SEC)).toBe(2);
  });

  it('窓の外になった更新は recent から落とす', () => {
    let state = applyReactionUpdate(emptyReactionMeter, update(T0, { '❤': 1 }), T0);
    state = applyReactionUpdate(state, update(T0 + 61 * SEC, { '😄': 1 }), T0 + 61 * SEC);

    expect(state.recent.map((u) => u.update_time_usec)).toEqual([T0 + 61 * SEC]);
  });

  it('元の状態を書き換えない', () => {
    const before = applyReactionUpdate(emptyReactionMeter, update(T0, { '❤': 1 }), T0);
    applyReactionUpdate(before, update(T0 + SEC, { '❤': 1 }), T0 + SEC);

    expect(before.totals).toEqual({ '❤': 1 });
    expect(before.recent).toHaveLength(1);
    expect(emptyReactionMeter.totals).toEqual({});
  });

  // 仕様: 累計は多い順に並べる
  it('累計を多い順に並べる', () => {
    const state = applyReactionUpdate(
      emptyReactionMeter,
      update(T0, { '😳': 1, '❤': 430, '🎉': 418 }),
      T0
    );
    expect(sortedReactionTotals(state)).toEqual([
      ['❤', 430],
      ['🎉', 418],
      ['😳', 1]
    ]);
  });

  // 仕様: F5リロード → 累計と直近60秒の件数を読み直す。バーの最大値は読み直した勢いから数え直す
  it('get_connection_reactions の結果から復元する', () => {
    const state = restoreReactionMeter(
      {
        totals: { '❤': 430, '🎉': 5 },
        recent: [update(T0 - 10 * SEC, { '🎉': 5 }), update(T0 - 5 * SEC, { '❤': 2 })]
      },
      T0
    );

    expect(state.totals).toEqual({ '❤': 430, '🎉': 5 });
    expect(reactionsPerMinute(state, T0)).toBe(7);
    expect(state.peakPerMinute).toBe(7);
  });

  // 仕様: 表示条件は1件以上のリアクションがあるとき
  it('リアクションが無ければ行を出さない', () => {
    expect(hasReactions(emptyReactionMeter)).toBe(false);
    expect(hasReactions(applyReactionUpdate(emptyReactionMeter, update(T0, { '❤': 1 }), T0))).toBe(true);
    expect(hasReactions(restoreReactionMeter({ totals: { '❤': 1 }, recent: [] }, T0))).toBe(true);
  });
});
