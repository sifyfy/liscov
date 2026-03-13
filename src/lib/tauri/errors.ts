// フロントエンド側のエラー正規化層
// Rust CommandError の JSON をパースし、構造化エラーとして扱う

/** Rust CommandError の各バリアントに対応するエラーコード */
export type ErrorCode =
  | 'AuthRequired'
  | 'AuthFailed'
  | 'StorageError'
  | 'ConnectionFailed'
  | 'NotConnected'
  | 'DatabaseError'
  | 'NotFound'
  | 'ApiError'
  | 'TtsError'
  | 'InvalidInput'
  | 'IoError'
  | 'Internal';

/** フロントエンドで扱う構造化エラー */
export interface AppError {
  code: ErrorCode;
  message: string;
  /** true の場合、ユーザーがリトライ可能なエラー */
  recoverable: boolean;
}

/**
 * リトライ可能なエラーコードのセット
 * ネットワーク・認証系は recoverable、内部エラーは not recoverable
 */
const RECOVERABLE_CODES: Set<ErrorCode> = new Set([
  'AuthRequired',
  'AuthFailed',
  'ConnectionFailed',
  'NotConnected',
  'TtsError',
  'ApiError',
]);

/**
 * Tauri コマンドの catch ブロックで受け取った任意の値を AppError に正規化する
 *
 * - string: JSON 形式なら CommandError としてパース、それ以外は Internal
 * - Error: message を Internal として扱う
 * - null/undefined/その他: Internal として扱う
 */
export function normalizeError(error: unknown): AppError {
  if (typeof error === 'string') {
    try {
      const parsed = JSON.parse(error);
      if (parsed && typeof parsed.kind === 'string' && typeof parsed.message === 'string') {
        const code = parsed.kind as ErrorCode;
        return {
          code,
          message: parsed.message,
          recoverable: RECOVERABLE_CODES.has(code),
        };
      }
    } catch {
      // JSON パース失敗 = プレーン文字列エラー → Internal として扱う
    }
    return { code: 'Internal', message: error, recoverable: false };
  }

  if (error instanceof Error) {
    return normalizeError(error.message);
  }

  // オブジェクト型エラー（Tauri v2がデシリアライズ済みオブジェクトを返す場合）
  if (error && typeof error === 'object') {
    const record = error as Record<string, unknown>;
    if (typeof record.kind === 'string' && typeof record.message === 'string') {
      const code = record.kind as ErrorCode;
      return {
        code,
        message: record.message,
        recoverable: RECOVERABLE_CODES.has(code),
      };
    }
    // kind がなくても message フィールドがあればそれを使う
    if (typeof record.message === 'string') {
      return { code: 'Internal', message: record.message, recoverable: false };
    }
  }

  // null / undefined / その他の型
  return { code: 'Internal', message: String(error), recoverable: false };
}
