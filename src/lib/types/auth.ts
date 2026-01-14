// Authentication types (01_auth.md)

export type StorageType = 'secure' | 'fallback';

export interface AuthStatus {
  is_authenticated: boolean;
  has_saved_credentials: boolean;
  storage_type: StorageType;
  storage_error: string | null;
}

export interface SessionValidity {
  is_valid: boolean;
  checked_at: string;
  error: string | null;
}

// Auth indicator states for UI
export type AuthIndicatorState =
  | 'unauthenticated'      // グレー / 鍵（閉）
  | 'authenticated_valid'   // 緑 / 鍵（開）
  | 'authenticated_checking' // 黄 / スピナー
  | 'authenticated_invalid'  // 赤 / 警告
  | 'authenticated_error'    // オレンジ / ?
  | 'storage_error';         // 赤 / !
