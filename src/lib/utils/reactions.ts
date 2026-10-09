// リアクションの累計（02_chat.md「ライブリアクション」の「状態の行のリアクション」）
import type { ReactionSummary } from '$lib/types/generated/ReactionSummary';
import type { ReactionUpdate } from '$lib/types/generated/ReactionUpdate';

/** 1接続ぶんのリアクションの状態 */
export interface ReactionState {
  /** 配信（video_id）単位の絵文字別累計 */
  readonly totals: Readonly<Record<string, number>>;
}

export const emptyReactions: ReactionState = { totals: {} };

/** 状態の行に出す数 */
const STATUS_REACTION_COUNT = 3;

/** 更新を1件加える */
export function applyReactionUpdate(state: ReactionState, update: ReactionUpdate): ReactionState {
  const totals = { ...state.totals };
  for (const [emoji, count] of Object.entries(update.counts)) {
    totals[emoji] = (totals[emoji] ?? 0) + count;
  }
  return { totals };
}

/** F5 リロード後・接続直後に get_connection_reactions の結果から復元する */
export function restoreReactions(summary: ReactionSummary): ReactionState {
  return { totals: { ...summary.totals } };
}

/** 累計を多い順に並べる */
export function sortedReactionTotals(state: ReactionState): readonly (readonly [string, number])[] {
  return Object.entries(state.totals).sort(([, a], [, b]) => b - a);
}

/** 状態の行に出す分（多い順に 3 つまで） */
export function statusReactions(state: ReactionState): readonly (readonly [string, number])[] {
  return sortedReactionTotals(state).slice(0, STATUS_REACTION_COUNT);
}

/** ヒントと読み上げ用の名前（全部） */
export function reactionsLabel(state: ReactionState): string {
  return `リアクション ${sortedReactionTotals(state)
    .map(([emoji, count]) => `${emoji} ${count}`)
    .join('、')}`;
}

/** 1件以上のリアクションがあるか */
export function hasReactions(state: ReactionState): boolean {
  return Object.values(state.totals).some((count) => count > 0);
}
