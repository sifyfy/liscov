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
// Backward compatibility aliases (deprecated)
// ============================================================================

/**
 * @deprecated Use viewerGetProfile instead
 */
export async function getViewerProfile(
  broadcasterId: string,
  channelId: string
): Promise<ViewerProfile | null> {
  return viewerGetProfile(broadcasterId, channelId);
}

/**
 * @deprecated Use viewerSearch instead
 */
export async function searchViewers(
  broadcasterId: string,
  query: string,
  limit?: number
): Promise<ViewerWithCustomInfo[]> {
  return viewerSearch(broadcasterId, query, limit);
}

/**
 * @deprecated Use viewerGetList instead
 */
export async function getViewersForBroadcaster(
  broadcasterId: string,
  searchQuery?: string,
  limit?: number,
  offset?: number
): Promise<ViewerWithCustomInfo[]> {
  return viewerGetList(broadcasterId, searchQuery, limit, offset);
}

/**
 * @deprecated Use broadcasterGetList instead
 */
export async function getBroadcasterList(): Promise<BroadcasterChannel[]> {
  return broadcasterGetList();
}

/**
 * @deprecated Use broadcasterDelete instead
 */
export async function deleteBroadcaster(
  broadcasterId: string
): Promise<[boolean, number]> {
  return broadcasterDelete(broadcasterId);
}

// ============================================================================
// Session helpers (unchanged)
// ============================================================================

/**
 * Get sessions list
 */
export async function getSessions(limit?: number): Promise<Session[]> {
  return invoke('get_sessions', { limit });
}
