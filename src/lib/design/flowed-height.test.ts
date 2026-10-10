/**
 * 流れたものの欄の高さ (docs/specs/02_chat.md「配信中の画面」の「流れたもの」)
 *
 * テストの例は仕様の表と例から取る:
 * - 例: 流れの部分が 800px、中身が 300px → 既定の上限は 100px (1/8)。境目を 60px 上へ引くと 160px (0.2)。
 *   さらに 200px 上へ引いても 266px (1/3) で止まる。中身が 120px しかなければ 120px (0.15) で止まる。
 *   ダブルクリックで 100px (0.125) に戻る
 * - 範囲は 0.05〜1/3。範囲外の値 (古い・手で書いた config.toml) は表示のときに範囲に収める
 * - ↑ / ↓ は高さの 1/50 ずつ
 */
import { describe, expect, it } from 'vitest';
import { FLOWED_HEIGHT, clampFlowedRatio, resizedFlowedRatio } from './flowed-height';

describe('clampFlowedRatio', () => {
  it('範囲の中の値はそのまま', () => {
    expect(clampFlowedRatio(0.2)).toBe(0.2);
  });

  it('範囲外は範囲に収める', () => {
    expect(clampFlowedRatio(0.01)).toBe(0.05);
    expect(clampFlowedRatio(0.9)).toBe(1 / 3);
  });

  it('数でない値は既定 (1/8)', () => {
    expect(clampFlowedRatio(Number.NaN)).toBe(0.125);
    expect(FLOWED_HEIGHT.default).toBe(0.125);
  });
});

describe('resizedFlowedRatio (例: 流れの部分 800px、中身 300px)', () => {
  const base = { parentPx: 800, contentPx: 300 };

  it('既定の 100px から 60px 上へ引くと 0.2 (160px)', () => {
    expect(resizedFlowedRatio({ ...base, startPx: 100, dragUpPx: 60 })).toBeCloseTo(0.2);
  });

  it('さらに 200px 上へ引いても 1/3 (266px) で止まる', () => {
    expect(resizedFlowedRatio({ ...base, startPx: 160, dragUpPx: 200 })).toBe(1 / 3);
  });

  it('中身が 120px しかなければ 0.15 (120px) で止まる', () => {
    expect(resizedFlowedRatio({ ...base, contentPx: 120, startPx: 100, dragUpPx: 60 })).toBeCloseTo(0.15);
  });

  it('下へ引くと低くなり、下限 0.05 で止まる', () => {
    expect(resizedFlowedRatio({ ...base, startPx: 100, dragUpPx: -20 })).toBeCloseTo(0.1);
    expect(resizedFlowedRatio({ ...base, startPx: 100, dragUpPx: -500 })).toBe(0.05);
  });

  it('↑ は高さの 1/50 (16px) ずつ高くする', () => {
    expect(resizedFlowedRatio({ ...base, startPx: 100, dragUpPx: FLOWED_HEIGHT.step * 800 })).toBeCloseTo(0.145);
    expect(FLOWED_HEIGHT.step).toBe(1 / 50);
  });

  it('流れの部分の高さが測れない (0) ときは既定', () => {
    expect(resizedFlowedRatio({ parentPx: 0, contentPx: 0, startPx: 0, dragUpPx: 10 })).toBe(0.125);
  });
});
