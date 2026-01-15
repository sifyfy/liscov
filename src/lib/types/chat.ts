// Chat-related type definitions

/** Message run (text or emoji) */
export type MessageRun =
  | { type: 'Text'; content: string }
  | { type: 'Emoji'; emoji_id: string; image_url: string; alt_text: string };

/** Badge information */
export interface BadgeInfo {
  badge_type: string;           // "member", "moderator", "verified", etc.
  label: string;                // Display label
  tooltip: string | null;       // Tooltip text
  image_url: string | null;     // Badge image URL
}

/** SuperChat color scheme from YouTube */
export interface SuperChatColors {
  header_background: string;
  header_text: string;
  body_background: string;
  body_text: string;
}

/** Message metadata */
export interface MessageMetadata {
  amount: string | null;
  milestone_months: number | null;    // Membership milestone months
  gift_count: number | null;          // Membership gift count
  badges: string[];
  badge_info: BadgeInfo[];
  is_moderator: boolean;
  is_verified: boolean;
  superchat_colors: SuperChatColors | null;
}

export interface ChatMessage {
  id: string;
  timestamp: string;
  timestamp_usec: string;
  author: string;
  author_icon_url: string | null;
  channel_id: string;
  content: string;
  runs: MessageRun[];
  message_type: MessageType;
  amount: string | null;
  is_member: boolean;
  comment_count: number | null;
  metadata: MessageMetadata | null;
}

export type MessageType =
  | 'text'
  | 'superchat'
  | 'supersticker'
  | 'membership'
  | 'membership_gift'
  | 'system';

export interface ConnectionResult {
  success: boolean;
  stream_title: string | null;
  broadcaster_channel_id: string | null;
  broadcaster_name: string | null;
  is_replay: boolean;
  error: string | null;
}

export type ChatMode = 'top' | 'all';

export interface ChatFilter {
  showText: boolean;
  showSuperchat: boolean;
  showMembership: boolean;
  searchQuery: string;
}
