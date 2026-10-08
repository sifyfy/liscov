/**
 * デザイントークン (docs/design/tokens.md) の検査
 *
 * テストの例は tokens.md の規則から取る:
 * - 「意味」の層の色は、ダークとライトの両方に定義する
 * - 文字・操作・状態・ハイライトの色は、すべての面の上で WCAG 2 AA (4.5:1) を満たす。
 *   ただし fg-3 は surface-highest (入力欄) の上では使わないので検査しない
 * - 優先順位 2 の行 (面に highlight を 20% 重ねた背景) の上では、fg-1 と fg-2 が 4.5:1 を満たす。
 *   fg-3 はこの背景の上では使わない
 */
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { describe, expect, it } from 'vitest';

// コメントはセレクタの読み取りの邪魔になるので先に取り除く
const css = readFileSync(resolve(__dirname, '../../app.css'), 'utf-8').replace(/\/\*[\s\S]*?\*\//g, '');

type Theme = 'dark' | 'light';
const THEME_SELECTORS = { dark: ':root', light: "[data-theme='light']" } as const satisfies Record<Theme, string>;

/** セレクタがちょうど一致するブロックから、指定した層 (sys / ref) の宣言を集める */
function tokensOf(selector: string, layer: 'sys' | 'ref'): ReadonlyMap<string, string> {
  const tokens = new Map<string, string>();
  for (const block of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    // 直前の @import などの文が混ざらないよう、最後の ; より後ろをセレクタとみなす
    if (block[1].split(';').at(-1)?.trim() !== selector) continue;
    for (const decl of block[2].matchAll(new RegExp(`--${layer}-([\\w-]+)\\s*:\\s*([^;]+);`, 'g'))) {
      tokens.set(decl[1], decl[2].trim());
    }
  }
  return tokens;
}

// 値の層はテーマで変わらないので :root だけに置く
const REF = tokensOf(':root', 'ref');

/** WCAG 2 の相対輝度 */
function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const lin = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/** 面 (base) の上に色 (top) を alpha の不透明度で重ねた色 */
function blend(top: string, base: string, alpha: number): string {
  const channel = (hex: string, i: number) => parseInt(hex.slice(i, i + 2), 16);
  return `#${[1, 3, 5]
    .map((i) => Math.round(channel(top, i) * alpha + channel(base, i) * (1 - alpha)))
    .map((c) => c.toString(16).padStart(2, '0'))
    .join('')}`;
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const SURFACES = ['surface', 'surface-low', 'surface-mid', 'surface-high', 'surface-highest'] as const;
const FOREGROUNDS = ['fg-1', 'fg-2', 'accent', 'success', 'warning', 'error', 'highlight', 'member-text'] as const;
const REQUIRED = [...SURFACES, ...FOREGROUNDS, 'fg-3', 'outline', 'outline-variant'] as const;

describe.each(['dark', 'light'] as const)('デザイントークン (%s)', (theme) => {
  const tokens = tokensOf(THEME_SELECTORS[theme], 'sys');
  const color = (name: string) => {
    // 意味の層は、値を直接書くか、値の層を var(--ref-*) で参照する
    const raw = tokens.get(name);
    const value = raw?.match(/^var\(--ref-([\w-]+)\)$/) ? REF.get(raw.slice(10, -1)) : raw;
    if (!value || !/^#[0-9a-f]{6}$/i.test(value)) throw new Error(`--sys-${name} が 6 桁の色で定義されていない: ${value}`);
    return value;
  };

  it.each(REQUIRED)('--sys-%s が定義されている', (name) => {
    expect(() => color(name)).not.toThrow();
  });

  it.each(FOREGROUNDS.flatMap((fg) => SURFACES.map((s) => [fg, s] as const)))(
    '%s は %s の上で 4.5:1 以上',
    (fg, surface) => {
      expect(contrast(color(fg), color(surface))).toBeGreaterThanOrEqual(4.5);
    },
  );

  it.each(SURFACES.filter((s) => s !== 'surface-highest'))('fg-3 は %s の上で 4.5:1 以上', (surface) => {
    expect(contrast(color('fg-3'), color(surface))).toBeGreaterThanOrEqual(4.5);
  });

  it.each((['fg-1', 'fg-2'] as const).flatMap((fg) => SURFACES.map((s) => [fg, s] as const)))(
    '%s は %s に highlight を 20% 重ねた背景 (優先順位 2 の行) の上で 4.5:1 以上',
    (fg, surface) => {
      expect(contrast(color(fg), blend(color('highlight'), color(surface), 0.2))).toBeGreaterThanOrEqual(4.5);
    },
  );
});
