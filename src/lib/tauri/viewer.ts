// ビューワー関連の Tauri コマンドラッパー
import { invoke } from '@tauri-apps/api/core';
import type {
  ViewerProfile,
  ViewerWithCustomInfo,
  Session,
  ContributorStats,
  BroadcasterChannel
} from '$lib/types';
import { normalizeError } from './errors';

/**
 * ブロードキャスター ID とチャンネル ID でビューワープロファイルを取得する
 */
export async function viewerGetProfile(
  broadcasterId: string,
  channelId: string
): Promise<ViewerProfile | null> {
  try {
    return await invoke('viewer_get_profile', { broadcasterId, channelId });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * ブロードキャスターのビューワーリストを取得する（検索・ページネーション対応）
 */
export async function viewerGetList(
  broadcasterId: string,
  searchQuery?: string,
  limit?: number,
  offset?: number
): Promise<ViewerWithCustomInfo[]> {
  try {
    return await invoke('viewer_get_list', {
      broadcasterId,
      searchQuery,
      limit,
      offset
    });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * ビューワーを検索する
 */
export async function viewerSearch(
  broadcasterId: string,
  query: string,
  limit?: number
): Promise<ViewerWithCustomInfo[]> {
  try {
    return await invoke('viewer_search', { broadcasterId, query, limit });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * viewer_profile_id でビューワーカスタム情報を upsert する
 */
export async function viewerUpsertCustomInfo(
  viewerProfileId: number,
  reading?: string | null,
  notes?: string | null,
  customData?: string | null
): Promise<void> {
  try {
    return await invoke('viewer_upsert_custom_info', {
      viewerProfileId,
      reading,
      notes,
      customData
    });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * viewer_profile_id でビューワープロファイルを削除する
 */
export async function viewerDelete(viewerProfileId: number): Promise<boolean> {
  try {
    return await invoke('viewer_delete', { viewerProfileId });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * ビューワー数付きのブロードキャスターリストを取得する
 */
export async function broadcasterGetList(): Promise<BroadcasterChannel[]> {
  try {
    return await invoke('broadcaster_get_list');
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * ブロードキャスターと関連するビューワープロファイルを全て削除する
 * 戻り値: [broadcaster_deleted, viewers_deleted_count]
 */
export async function broadcasterDelete(
  broadcasterId: string
): Promise<[boolean, number]> {
  try {
    return await invoke('broadcaster_delete', { broadcasterId });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * セッションのトップコントリビューターを取得する
 */
export async function getTopContributors(
  sessionId: string,
  limit?: number
): Promise<ContributorStats[]> {
  try {
    return await invoke('get_top_contributors', { sessionId, limit });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * viewer_profile_id でビューワー情報（カスタム情報 + タグ）を更新する
 */
export async function viewerUpdateInfo(
  viewerProfileId: number,
  reading: string | null,
  notes: string | null,
  customData: string | null,
  tags: string[] | null
): Promise<boolean> {
  try {
    return await invoke('viewer_update_info', {
      viewerProfileId,
      reading,
      notes,
      customData,
      tags
    });
  } catch (e) {
    throw normalizeError(e);
  }
}

// ============================================================================
// セッションヘルパー
// ============================================================================

/**
 * セッション一覧を取得する
 */
export async function getSessions(limit?: number): Promise<Session[]> {
  try {
    return await invoke('get_sessions', { limit });
  } catch (e) {
    throw normalizeError(e);
  }
}
