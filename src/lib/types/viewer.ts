// Viewer-related type definitions

export interface ViewerProfile {
  channel_id: string;
  display_name: string;
  first_seen: string;
  last_seen: string;
  message_count: number;
  total_contribution: number;
  membership_level: string | null;
  tags: string[];
}

export interface ViewerCustomInfo {
  broadcaster_channel_id: string;
  viewer_channel_id: string;
  reading: string | null;
  notes: string | null;
  custom_data: string | null;
}

export interface ViewerWithCustomInfo {
  channel_id: string;
  display_name: string;
  first_seen: string;
  last_seen: string;
  message_count: number;
  total_contribution: number;
  membership_level: string | null;
  tags: string[];
  reading: string | null;
  notes: string | null;
}

export interface Session {
  id: string;
  start_time: string;
  end_time: string | null;
  stream_url: string | null;
  stream_title: string | null;
  broadcaster_name: string | null;
  total_messages: number;
  total_revenue: number;
}

export interface ContributorStats {
  channel_id: string;
  display_name: string;
  message_count: number;
  total_contribution: number;
}

export interface BroadcasterChannel {
  channel_id: string;
  channel_name: string | null;
  handle: string | null;
  viewer_count: number;
}
