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
export type { GuiReactionUpdate } from './generated/GuiReactionUpdate';
export type { ReactionSummary } from './generated/ReactionSummary';
export type { ReactionUpdate } from './generated/ReactionUpdate';

import type { ReactionMeterState } from '$lib/utils/reactions';

// メッセージタイプ（フロントエンド固有 - Rust側はstringとして送信）
export type MessageType =
  | 'text'
  | 'superchat'
  | 'supersticker'
  | 'membership'
  | 'membership_gift'
  | 'gift'
  | 'system';

// チャットモード（connect_to_stream・set_chat_mode に渡す値。設定に残す値と同じ）
export type { ChatModeSetting as ChatMode } from './generated/ChatModeSetting';

// チャットフィルター（フロントエンド固有）
export interface ChatFilter {
  showText: boolean;
  showSuperchat: boolean; // スーパーチャット/ステッカー/ギフト（有料系）
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
  /** この接続で jewel_count の無いギフトを受けたら true（接続一覧に注記を出す。フロントエンドのみの状態） */
  jewelCountUnavailable: boolean;
  /** リアクションメーターの状態（02_chat.md「リアクションメーター」） */
  reactions: ReactionMeterState;
}
