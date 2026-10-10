/**
 * 流れたものの欄の高さ (docs/specs/02_chat.md「配信中の画面」の「流れたもの」)
 *
 * 欄の高さの上限は、コメントの流れの部分 (ChatDisplay) の高さに対する割合で覚える (ウィンドウの大きさを変えても割合を保つ)。
 * 境目のドラッグと ↑↓ はどちらも「見えている高さから何 px 動かすか」で計算する (中身の高さで止め、見えている高さを覚える)
 */

/** 割合の既定・範囲と、↑↓ 1 回で動かす量 (09_config.md の flowed_*_height_ratio) */
export const FLOWED_HEIGHT = {
  default: 1 / 8,
  min: 0.05,
  max: 1 / 3,
  step: 1 / 50,
} as const satisfies Readonly<Record<string, number>>;

/** 範囲外を範囲に収める。数でない値は既定 */
export function clampFlowedRatio(ratio: number): number {
  if (Number.isNaN(ratio)) return FLOWED_HEIGHT.default;
  return Math.min(FLOWED_HEIGHT.max, Math.max(FLOWED_HEIGHT.min, ratio));
}

export interface FlowedResize {
  /** 動かし始めたときに見えていた欄の高さ */
  readonly startPx: number;
  /** 上へ動かした量 (下へは負) */
  readonly dragUpPx: number;
  /** コメントの流れの部分の高さ */
  readonly parentPx: number;
  /** 欄の中身の高さ (これより高くはしない) */
  readonly contentPx: number;
}

/** 境目を動かしたあとの割合 */
export function resizedFlowedRatio({ startPx, dragUpPx, parentPx, contentPx }: FlowedResize): number {
  if (parentPx <= 0) return FLOWED_HEIGHT.default;
  return clampFlowedRatio(Math.min(startPx + dragUpPx, contentPx) / parentPx);
}
