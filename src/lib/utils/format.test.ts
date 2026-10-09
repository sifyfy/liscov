import { describe, it, expect } from 'vitest';
import { formatContribution, formatDay, formatTimestamp } from './format';

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

// spec: docs/specs/06_viewer.md「視聴者のタブ」1 人の行の最終確認の日付
describe('formatDay', () => {
  const now = new Date(2026, 9, 9, 21, 0);

  it('今年の日付は 月/日', () => {
    expect(formatDay(new Date(2026, 9, 9, 12, 0).toISOString(), now)).toBe('10/9');
    expect(formatDay(new Date(2026, 0, 3, 12, 0).toISOString(), now)).toBe('1/3');
  });

  it('去年より前は 年/月/日', () => {
    expect(formatDay(new Date(2025, 9, 1, 12, 0).toISOString(), now)).toBe('2025/10/1');
  });

  it('解釈できない文字列はそのまま', () => {
    expect(formatDay('不明', now)).toBe('不明');
  });
});
