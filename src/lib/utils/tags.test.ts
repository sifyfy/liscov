/**
 * 視聴者のタグの入力 (docs/specs/06_viewer.md「視聴者のタブ」: 読点 (、) かカンマ (,) で区切る。前後の空白と空のタグは除く)
 */
import { describe, expect, it } from 'vitest';
import { formatTags, parseTags } from './tags';

describe('parseTags', () => {
  it('読点とカンマのどちらでも区切る', () => {
    expect(parseTags('常連、攻略,注意')).toEqual(['常連', '攻略', '注意']);
  });

  it('前後の空白と空のタグは除く', () => {
    expect(parseTags(' 常連 、、 攻略 ,, ')).toEqual(['常連', '攻略']);
  });

  it('空の入力はタグなし', () => {
    expect(parseTags('')).toEqual([]);
    expect(parseTags('  ')).toEqual([]);
  });
});

describe('formatTags', () => {
  it('入力欄には読点でつないで出す (読み直すと同じタグになる)', () => {
    expect(formatTags(['常連', '攻略'])).toBe('常連、攻略');
    expect(parseTags(formatTags(['常連', '攻略']))).toEqual(['常連', '攻略']);
  });
});
