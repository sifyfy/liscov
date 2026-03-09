// Viewer-related Tauri commands
import { invoke } from '@tauri-apps/api/core';
import type {
  ViewerProfile,
  ViewerWithCustomInfo,
  Session,
  ContributorStats,
  BroadcasterChannel
} from '$lib/types';

/**
 * Get viewer profile by broadcaster ID and channel ID
 */
export async function viewerGetProfile(
  broadcasterId: string,
  channelId: string
): Promise<ViewerProfile | null> {
  return invoke('viewer_get_profile', { broadcasterId, channelId });
}

/**
 * Get viewer list for a broadcaster with optional search and pagination
 */
export async function viewerGetList(
  broadcasterId: string,
  searchQuery?: string,
  limit?: number,
  offset?: number
): Promise<ViewerWithCustomInfo[]> {
  return invoke('viewer_get_list', {
    broadcasterId,
    searchQuery,
    limit,
    offset
  });
}

/**
 * Search viewers
 */
export async function viewerSearch(
  broadcasterId: string,
  query: string,
  limit?: number
): Promise<ViewerWithCustomInfo[]> {
  return invoke('viewer_search', { broadcasterId, query, limit });
}

/**
 * Upsert viewer custom info by viewer_profile_id
 */
export async function viewerUpsertCustomInfo(
  viewerProfileId: number,
  reading?: string | null,
  notes?: string | null,
  customData?: string | null
): Promise<void> {
  return invoke('viewer_upsert_custom_info', {
    viewerProfileId,
    reading,
    notes,
    customData
  });
}

/**
 * Delete viewer profile by viewer_profile_id
 */
export async function viewerDelete(viewerProfileId: number): Promise<boolean> {
  return invoke('viewer_delete', { viewerProfileId });
}

/**
 * Get broadcaster list with viewer counts
 */
export async function broadcasterGetList(): Promise<BroadcasterChannel[]> {
  return invoke('broadcaster_get_list');
}

/**
 * Delete broadcaster and all associated viewer profiles
 * Returns [broadcaster_deleted, viewers_deleted_count]
 */
export async function broadcasterDelete(
  broadcasterId: string
): Promise<[boolean, number]> {
  return invoke('broadcaster_delete', { broadcasterId });
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
 * Update viewer info (custom info + tags) by viewer_profile_id
 */
export async function viewerUpdateInfo(
  viewerProfileId: number,
  reading: string | null,
  notes: string | null,
  customData: string | null,
  tags: string[] | null
): Promise<boolean> {
  return invoke('viewer_update_info', {
    viewerProfileId,
    reading,
    notes,
    customData,
    tags
  });
}

// ============================================================================
// Session helpers
// ============================================================================

/**
 * Get sessions list
 */
export async function getSessions(limit?: number): Promise<Session[]> {
  return invoke('get_sessions', { limit });
}
