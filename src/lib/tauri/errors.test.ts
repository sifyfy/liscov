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

  // プレーン文字列の recoverable: false を明示的にアサート
  it('プレーン文字列エラーは recoverable=false', () => {
    const result = normalizeError('plain string');
    expect(result.recoverable).toBe(false);
  });

  // オブジェクト型エラーパス
  it('オブジェクト { kind: ConnectionFailed } は recoverable=true', () => {
    const result = normalizeError({ kind: 'ConnectionFailed', message: 'test' });
    expect(result.code).toBe('ConnectionFailed');
    expect(result.recoverable).toBe(true);
  });

  it('オブジェクト { kind: Internal } は recoverable=false', () => {
    const result = normalizeError({ kind: 'Internal', message: 'test' });
    expect(result.code).toBe('Internal');
    expect(result.recoverable).toBe(false);
  });

  it('オブジェクト kind なし・message あり → Internal でメッセージ保持', () => {
    const result = normalizeError({ message: 'no kind field' });
    expect(result.code).toBe('Internal');
    expect(result.message).toBe('no kind field');
  });

  it('空オブジェクトは Internal', () => {
    const result = normalizeError({});
    expect(result.code).toBe('Internal');
  });

  it('RECOVERABLE_CODES に含まれないコード (StorageError) は recoverable=false', () => {
    const result = normalizeError({ kind: 'StorageError', message: 'db error' });
    expect(result.recoverable).toBe(false);
  });

  // RECOVERABLE_CODES の個別テスト
  it('AuthFailed は recoverable=true', () => {
    const result = normalizeError(JSON.stringify({ kind: 'AuthFailed', message: 'auth failed' }));
    expect(result.recoverable).toBe(true);
  });

  it('NotConnected は recoverable=true', () => {
    const result = normalizeError(JSON.stringify({ kind: 'NotConnected', message: 'not connected' }));
    expect(result.recoverable).toBe(true);
  });

  it('TtsError は recoverable=true', () => {
    const result = normalizeError(JSON.stringify({ kind: 'TtsError', message: 'tts error' }));
    expect(result.recoverable).toBe(true);
  });

  it('ApiError は recoverable=true', () => {
    const result = normalizeError(JSON.stringify({ kind: 'ApiError', message: 'api error' }));
    expect(result.recoverable).toBe(true);
  });

  // Error.message が JSON 形式の場合に再帰パースされること
  it('Error.messageがJSON形式の場合に再帰パースされる', () => {
    const error = new Error('{"kind":"ConnectionFailed","message":"接続失敗"}');
    const result = normalizeError(error);
    expect(result.code).toBe('ConnectionFailed');
    expect(result.message).toBe('接続失敗');
    expect(result.recoverable).toBe(true);
  });

  // JSON の kind のみ存在（message なし）は Internal になること
  it('JSON にkindのみでmessageがない場合はInternalとして扱う', () => {
    const result = normalizeError(JSON.stringify({ kind: 'ConnectionFailed' }));
    expect(result.code).toBe('Internal');
  });

  // オブジェクトの message が非文字列の場合に Internal になること
  it('オブジェクトのmessageが文字列でない場合はInternalとして扱う', () => {
    const result = normalizeError({ kind: 'StorageError', message: 42 });
    expect(result.code).toBe('Internal');
  });

  // kindなし・messageありオブジェクトの recoverable=false を確認
  it('kindなし・messageありオブジェクトのrecoverableはfalse', () => {
    const result = normalizeError({ message: 'some error' });
    expect(result.recoverable).toBe(false);
  });

  // messageが非文字列のオブジェクトも Internal として扱う
  it('messageが数値のオブジェクトはInternalとして扱う', () => {
    const result = normalizeError({ message: 42 });
    expect(result.code).toBe('Internal');
  });

  // null/undefined/その他の recoverable=false を確認
  it('null の recoverable は false', () => {
    const result = normalizeError(null);
    expect(result.recoverable).toBe(false);
  });

  it('undefined の recoverable は false', () => {
    const result = normalizeError(undefined);
    expect(result.recoverable).toBe(false);
  });

  it('数値の recoverable は false', () => {
    const result = normalizeError(42);
    expect(result.code).toBe('Internal');
    expect(result.recoverable).toBe(false);
  });

  // JSON文字列 '{}' は kind/message が揃わないため Internal
  it('JSON文字列 {} はInternalとして扱う', () => {
    const result = normalizeError('{}');
    expect(result.code).toBe('Internal');
  });

  // JSON の kind が非文字列の場合は Internal（文字列パスのフォールスルー）
  it('JSONのkindが文字列でない場合はInternal', () => {
    const result = normalizeError(JSON.stringify({ kind: 123, message: 'test' }));
    expect(result.code).toBe('Internal');
  });

  // message が非文字列オブジェクトはオブジェクトパスをフォールスルーし String(error) をメッセージとして返す
  it('messageが数値のオブジェクトはString(error)をメッセージとして返す', () => {
    const result = normalizeError({ message: 42 });
    expect(result.code).toBe('Internal');
    expect(result.message).toBe('[object Object]');
  });
});
