import { describe, it, expect } from 'vitest';
import type { ReactionUpdate } from '$lib/types/generated/ReactionUpdate';
import {
  applyReactionUpdate,
  emptyReactions,
  hasReactions,
  reactionsLabel,
  restoreReactions,
  sortedReactionTotals,
  statusReactions
} from './reactions';

const SEC = 1_000_000;
const T0 = 1_790_422_357 * SEC;

function update(time: number, counts: Record<string, number>, duration = 1): ReactionUpdate {
  const total = Object.values(counts).reduce((a, b) => a + b, 0);
  return { update_time_usec: time, duration_seconds: duration, counts, total };
}

// spec: 02_chat.md「ライブリアクション」の「状態の行のリアクション」
describe('リアクションの累計', () => {
  // 仕様例: 主の配信に 🎉4 → ❤3 → `🎉 4  ❤ 3`
  it('更新を足して累計を数え、多い順に並べる', () => {
    const afterFirst = applyReactionUpdate(emptyReactions, update(T0, { '🎉': 4 }, 2));
    const state = applyReactionUpdate(afterFirst, update(T0 + 30 * SEC, { '❤': 3 }));

    expect(state.totals).toEqual({ '❤': 3, '🎉': 4 });
    expect(statusReactions(state)).toEqual([
      ['🎉', 4],
      ['❤', 3]
    ]);
  });

  // 仕様例: ❤430・🎉418・😄94・💯56 → `❤ 430  🎉 418  😄 94`。💯 56 はヒントにだけ出る
  it('状態の行には多い順に 3 つまで。ヒントには全部', () => {
    const state = applyReactionUpdate(emptyReactions, update(T0, { '💯': 56, '❤': 430, '😄': 94, '🎉': 418 }));

    expect(statusReactions(state)).toEqual([
      ['❤', 430],
      ['🎉', 418],
      ['😄', 94]
    ]);
    expect(reactionsLabel(state)).toBe('リアクション ❤ 430、🎉 418、😄 94、💯 56');
  });

  it('累計を多い順に並べる', () => {
    const state = applyReactionUpdate(emptyReactions, update(T0, { '😳': 1, '❤': 430, '🎉': 418 }));
    expect(sortedReactionTotals(state)).toEqual([
      ['❤', 430],
      ['🎉', 418],
      ['😳', 1]
    ]);
  });

  it('元の状態を書き換えない', () => {
    const before = applyReactionUpdate(emptyReactions, update(T0, { '❤': 1 }));
    applyReactionUpdate(before, update(T0 + SEC, { '❤': 1 }));

    expect(before.totals).toEqual({ '❤': 1 });
    expect(emptyReactions.totals).toEqual({});
  });

  // 仕様例: F5リロード → 累計を get_connection_reactions で読み直す
  it('get_connection_reactions の結果から累計を復元する', () => {
    const state = restoreReactions({ totals: { '❤': 430, '🎉': 5 }, recent: [update(T0 - 10 * SEC, { '🎉': 5 })] });
    expect(state.totals).toEqual({ '❤': 430, '🎉': 5 });
  });

  // 仕様: 表示条件は主の配信に1件以上のリアクションがあるとき
  it('リアクションが無ければ出さない', () => {
    expect(hasReactions(emptyReactions)).toBe(false);
    expect(hasReactions(applyReactionUpdate(emptyReactions, update(T0, { '❤': 1 })))).toBe(true);
    expect(hasReactions(restoreReactions({ totals: { '❤': 1 }, recent: [] }))).toBe(true);
  });
});
