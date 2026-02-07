// Analytics types

// SuperChat Tier (based on header background color from YouTube API)
export type SuperChatTier = 'Blue' | 'Cyan' | 'Green' | 'Yellow' | 'Orange' | 'Magenta' | 'Red';

// Tier統計（spec: 07_revenue.md）
export interface SuperChatTierStats {
  tier_red: number;
  tier_magenta: number;
  tier_orange: number;
  tier_yellow: number;
  tier_green: number;
  tier_cyan: number;
  tier_blue: number;
}

// 収益分析（Tierベース）
export interface RevenueAnalytics {
  super_chat_count: number;
  super_chat_by_tier: SuperChatTierStats;
  super_sticker_count: number;
  membership_gains: number;
  hourly_stats: HourlyStats[];
  top_contributors: ContributorInfo[];
}

// 貢献者情報（Tierベース）
export interface ContributorInfo {
  channel_id: string;
  display_name: string;
  super_chat_count: number;
  highest_tier: SuperChatTier | null;
}

// 時間帯別統計
export interface HourlyStats {
  hour: string;
  super_chat_count: number;
  membership_count: number;
  message_count: number;
}

export interface ExportConfig {
  format: 'csv' | 'json';
  include_metadata: boolean;
  include_system_messages: boolean;
  max_records?: number;
}

export interface SessionExportData {
  metadata: SessionMetadata;
  messages: ExportMessage[];
  statistics: SessionStatistics;
}

export interface SessionMetadata {
  session_id: string;
  stream_title: string | null;
  stream_url: string | null;
  broadcaster_name: string | null;
  broadcaster_channel_id: string | null;
  start_time: string;
  end_time: string | null;
  export_time: string;
}

export interface ExportMessage {
  id: string;
  timestamp: string;
  author: string;
  author_id: string;
  content: string;
  message_type: string;
  amount: number | null;
  is_member: boolean;
}

export interface SessionStatistics {
  total_messages: number;
  unique_viewers: number;
  super_chat_total: number;
  super_chat_count: number;
  membership_count: number;
}
