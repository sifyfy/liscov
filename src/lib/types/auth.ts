// 認証関連の型定義
// Rust型は generated/ から re-export、フロントエンド固有型はここで定義

export type { AuthStatus } from './generated/AuthStatus';
export type { SessionValidity } from './generated/SessionValidity';
export type { StorageType } from './generated/StorageType';

// Auth indicator states for UI（フロントエンド固有）
export type AuthIndicatorState =
  | 'unauthenticated'         // グレー / 鍵（閉）
  | 'authenticated_valid'     // 緑 / 鍵（開）
  | 'authenticated_checking'  // 黄 / スピナー
  | 'authenticated_invalid'   // 赤 / 警告
  | 'authenticated_error'     // オレンジ / ?
  | 'storage_error';          // 赤 / !
