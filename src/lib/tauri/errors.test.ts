import { describe, it, expect } from 'vitest';
import { normalizeError } from './errors';

describe('normalizeError', () => {
  it('CommandError JSON をパースする', () => {
    const error = { kind: 'ConnectionFailed', message: '接続に失敗' };
    const result = normalizeError(JSON.stringify(error));
    expect(result.code).toBe('ConnectionFailed');
    expect(result.message).toBe('接続に失敗');
    expect(result.recoverable).toBe(true);
  });

  it('プレーン文字列エラーを Internal として扱う', () => {
    const result = normalizeError('something went wrong');
    expect(result.code).toBe('Internal');
    expect(result.message).toBe('something went wrong');
  });

  it('AuthRequired は recoverable', () => {
    const error = { kind: 'AuthRequired', message: '認証が必要です' };
    const result = normalizeError(JSON.stringify(error));
    expect(result.recoverable).toBe(true);
  });

  it('Internal は not recoverable', () => {
    const error = { kind: 'Internal', message: 'panic' };
    const result = normalizeError(JSON.stringify(error));
    expect(result.recoverable).toBe(false);
  });

  it('Error オブジェクトを処理する', () => {
    const result = normalizeError(new Error('test error'));
    expect(result.code).toBe('Internal');
    expect(result.message).toBe('test error');
  });

  it('null/undefined を処理する', () => {
    const result = normalizeError(null);
    expect(result.code).toBe('Internal');
  });
});
