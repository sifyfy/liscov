// Viewer-related Tauri commands
import { invoke } from '@tauri-apps/api/core';
import type {
  ViewerProfile,
  ViewerWithCustomInfo,
  ViewerCustomInfo,
  Session,
  ContributorStats,
  BroadcasterChannel
} from '$lib/types';

/**
 * Get viewer profile by channel ID
 */
export async function getViewerProfile(channelId: string): Promise<ViewerProfile | null> {
  return invoke('get_viewer_profile', { channelId });
}

/**
 * Get viewer with custom info
 */
export async function getViewerWithCustomInfo(
  broadcasterId: string,
  viewerId: string
): Promise<ViewerWithCustomInfo | null> {
  return invoke('get_viewer_with_custom_info', { broadcasterId, viewerId });
}

/**
 * Search viewers
 */
export async function searchViewers(
  broadcasterId: string,
  query: string,
  limit?: number
): Promise<ViewerWithCustomInfo[]> {
  return invoke('search_viewers', { broadcasterId, query, limit });
}

/**
 * Get top contributors for a session
 */
export async function getTopContributors(
  sessionId: string,
  limit?: number
): Promise<ContributorStats[]> {
  return invoke('get_top_contributors', { sessionId, limit });
}

/**
 * Get sessions list
 */
export async function getSessions(limit?: number): Promise<Session[]> {
  return invoke('get_sessions', { limit });
}

/**
 * Get viewers for a broadcaster
 */
export async function getViewersForBroadcaster(
  broadcasterId: string,
  searchQuery?: string,
  limit?: number,
  offset?: number
): Promise<ViewerWithCustomInfo[]> {
  return invoke('get_viewers_for_broadcaster', {
    broadcasterId,
    searchQuery,
    limit,
    offset
  });
}

/**
 * Upsert viewer custom info
 */
export async function upsertViewerCustomInfo(info: {
  broadcaster_channel_id: string;
  viewer_channel_id: string;
  reading?: string | null;
  notes?: string | null;
  custom_data?: string | null;
}): Promise<number> {
  return invoke('upsert_viewer_custom_info', { info });
}

/**
 * Get broadcaster list
 */
export async function getBroadcasterList(): Promise<BroadcasterChannel[]> {
  return invoke('broadcaster_get_list');
}

/**
 * Delete viewer custom info (keeps viewer_profiles)
 */
export async function deleteViewerCustomInfo(
  broadcasterId: string,
  viewerId: string
): Promise<boolean> {
  return invoke('viewer_delete', { broadcasterId, viewerId });
}

/**
 * Delete broadcaster and all associated viewer custom info
 * Returns [broadcaster_deleted, viewers_deleted_count]
 */
export async function deleteBroadcaster(
  broadcasterId: string
): Promise<[boolean, number]> {
  return invoke('broadcaster_delete', { broadcasterId });
}

/**
 * Update viewer info (custom info + tags)
 */
export async function updateViewerInfo(
  broadcasterId: string,
  viewerId: string,
  reading: string | null,
  notes: string | null,
  tags: string[] | null
): Promise<boolean> {
  return invoke('viewer_update_info', {
    broadcasterId,
    viewerId,
    reading,
    notes,
    tags
  });
}
