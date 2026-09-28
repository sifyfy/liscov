// リアクションメーターの状態計算（02_chat.md「リアクションメーター」）
import type { ReactionSummary } from '$lib/types/generated/ReactionSummary';
import type { ReactionUpdate } from '$lib/types/generated/ReactionUpdate';

/** 勢い（/分）を数える窓: 直近60秒 */
export const REACTION_WINDOW_USEC = 60 * 1_000_000;

/** 1接続ぶんのリアクションメーターの状態 */
export interface ReactionMeterState {
  /** 配信（video_id）単位の絵文字別累計 */
  readonly totals: Readonly<Record<string, number>>;
  /** 窓の中の更新（古い順） */
  readonly recent: readonly ReactionUpdate[];
  /** 表示を始めてからの勢いの最大値（バーの満タン） */
  readonly peakPerMinute: number;
}

export const emptyReactionMeter: ReactionMeterState = { totals: {}, recent: [], peakPerMinute: 0 };

/** 窓の中（nowUsec から60秒以内）の更新か */
function isInWindow(update: ReactionUpdate, nowUsec: number): boolean {
  return update.update_time_usec >= nowUsec - REACTION_WINDOW_USEC;
}

/** 直近60秒の件数（/分）。60秒の判定は update_time_usec と現在時刻で行う */
export function reactionsPerMinute(state: ReactionMeterState, nowUsec: number): number {
  return state.recent
    .filter((u) => isInWindow(u, nowUsec))
    .reduce((sum, u) => sum + u.total, 0);
}

/** 更新を1件加える（窓の外になった更新は recent から落とす） */
export function applyReactionUpdate(
  state: ReactionMeterState,
  update: ReactionUpdate,
  nowUsec: number
): ReactionMeterState {
  const totals = { ...state.totals };
  for (const [emoji, count] of Object.entries(update.counts)) {
    totals[emoji] = (totals[emoji] ?? 0) + count;
  }
  const recent = [...state.recent, update].filter((u) => isInWindow(u, nowUsec));
  const next = { totals, recent, peakPerMinute: state.peakPerMinute };
  return { ...next, peakPerMinute: Math.max(state.peakPerMinute, reactionsPerMinute(next, nowUsec)) };
}

/** F5 リロード後に get_connection_reactions の結果から復元する */
export function restoreReactionMeter(summary: ReactionSummary, nowUsec: number): ReactionMeterState {
  const restored = { totals: { ...summary.totals }, recent: [...summary.recent], peakPerMinute: 0 };
  return { ...restored, peakPerMinute: reactionsPerMinute(restored, nowUsec) };
}

/** 累計を多い順に並べる */
export function sortedReactionTotals(state: ReactionMeterState): readonly (readonly [string, number])[] {
  return Object.entries(state.totals).sort(([, a], [, b]) => b - a);
}

/** 1件以上のリアクションがあるか（メーターの行を出すか） */
export function hasReactions(state: ReactionMeterState): boolean {
  return Object.values(state.totals).some((count) => count > 0);
}
