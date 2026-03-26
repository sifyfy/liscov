import { describe, it, expect } from 'vitest';
import { getConnectionColor, CONNECTION_COLORS } from './connection-colors';

describe('getConnectionColor', () => {
  it('同じbroadcasterChannelIdには常に同じ色を返す', () => {
    const color1 = getConnectionColor('UC12345');
    const color2 = getConnectionColor('UC12345');
    expect(color1).toBe(color2);
  });

  it('異なるbroadcasterChannelIdには異なる色を返しやすい', () => {
    const color1 = getConnectionColor('UC12345');
    const color2 = getConnectionColor('UC67890');
    expect(color1).not.toBe(color2);
  });

  it('パレット内の色を返す', () => {
    const color = getConnectionColor('UC12345');
    expect(CONNECTION_COLORS).toContain(color);
  });

  it('空文字列でもクラッシュしない', () => {
    const color = getConnectionColor('');
    expect(CONNECTION_COLORS).toContain(color);
  });

  it('既知の入力に対して安定した色を返す（ハッシュ関数のピン留め）', () => {
    expect(getConnectionColor('UC12345')).toBe('#ff6d01');
    expect(getConnectionColor('UC67890')).toBe('#fbbc04');
    expect(getConnectionColor('UCabcdef')).toBe('#ab47bc');
  });
});
