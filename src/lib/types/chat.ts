// チャット関連の型定義
// Rust型は generated/ から re-export、フロントエンド固有型はここで定義

export type { ConnectionResult } from './generated/ConnectionResult';
export type { ConnectionInfo } from './generated/ConnectionInfo';
export type { Platform } from './generated/Platform';
export type { MessageRun } from './generated/MessageRun';
export type { BadgeInfo } from './generated/BadgeInfo';
export type { SuperChatColors } from './generated/SuperChatColors';
// GuiMessageMetadata を MessageMetadata として re-export（フロントエンドの命名慣習に合わせる）
export type { GuiMessageMetadata as MessageMetadata } from './generated/GuiMessageMetadata';
// GuiChatMessage を ChatMessage として re-export
export type { GuiChatMessage as ChatMessage } from './generated/GuiChatMessage';

// メッセージタイプ（フロントエンド固有 - Rust側はstringとして送信）
export type MessageType =
  | 'text'
  | 'superchat'
  | 'supersticker'
  | 'membership'
  | 'membership_gift'
  | 'system';

// チャットモード（フロントエンド固有）
export type ChatMode = 'top' | 'all';

// チャットフィルター（フロントエンド固有）
export interface ChatFilter {
  showText: boolean;
  showSuperchat: boolean;
  showMembership: boolean;
  searchQuery: string;
}

/** フロントエンド側の接続状態（色情報等を含む） */
export interface FrontendConnectionState {
  id: number;
  platform: string;
  streamUrl: string;
  streamTitle: string;
  broadcasterName: string;
  broadcasterChannelId: string;
  connectionState: 'connecting' | 'connected' | 'paused' | 'disconnecting' | 'error';
  color: string;
}
