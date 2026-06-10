import { describe, it, expect } from 'vitest';
import { formatContribution, formatTimestamp } from './format';

describe('formatContribution', () => {
  // 仕様: 視聴者の貢献額は日本円通貨表記（小数点なし）で表示する
  it('正の金額を日本円通貨表記にする', () => {
    expect(formatContribution(1000)).toBe('￥1,000');
  });

  it('桁区切りを付ける', () => {
    expect(formatContribution(1234567)).toBe('￥1,234,567');
  });

  // 仕様: 貢献額0は「貢献なし」を意味し '-' で表示する
  it('0 のときは "-" を返す', () => {
    expect(formatContribution(0)).toBe('-');
  });
});

describe('formatTimestamp', () => {
  // 仕様: タイムスタンプはローカルタイムゾーンの HH:MM:SS（24時間・2桁ゼロ埋め）で表示する
  it('ISO文字列を HH:MM:SS に変換する', () => {
    // タイムゾーン指定なしのISO文字列はローカル時刻として解釈される
    expect(formatTimestamp('2026-06-11T09:05:03')).toBe('09:05:03');
  });

  it('各要素を2桁ゼロ埋めする', () => {
    expect(formatTimestamp('2026-06-11T01:02:03')).toBe('01:02:03');
  });

  // 仕様: 空文字列は空のまま表示する
  it('空文字列のときは空文字列を返す', () => {
    expect(formatTimestamp('')).toBe('');
  });

  // 仕様: 解釈できないタイムスタンプは原文をそのまま表示する
  it('不正な日付文字列のときは原文を返す', () => {
    expect(formatTimestamp('not-a-date')).toBe('not-a-date');
  });
});
