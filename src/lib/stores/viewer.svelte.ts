// Viewer state management using Svelte 5 runes
import type { ViewerWithCustomInfo, Session, BroadcasterChannel } from '$lib/types';
import * as viewerApi from '$lib/tauri/viewer';

// Reactive state
let viewers = $state<ViewerWithCustomInfo[]>([]);
let sessions = $state<Session[]>([]);
let broadcasters = $state<BroadcasterChannel[]>([]);
let selectedViewer = $state<ViewerWithCustomInfo | null>(null);
let selectedBroadcasterId = $state<string | null>(null);
let searchQuery = $state('');
let isLoading = $state(false);
let error = $state<string | null>(null);
let totalViewers = $state(0);
let currentPage = $state(0);
const pageSize = 50;

// Actions
async function loadSessions(): Promise<void> {
  isLoading = true;
  error = null;

  try {
    sessions = await viewerApi.getSessions(20);
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  } finally {
    isLoading = false;
  }
}

async function loadViewers(broadcasterId: string, page = 0): Promise<void> {
  isLoading = true;
  error = null;
  selectedBroadcasterId = broadcasterId;
  currentPage = page;

  try {
    const query = searchQuery.trim() || undefined;
    viewers = await viewerApi.viewerGetList(
      broadcasterId,
      query,
      pageSize,
      page * pageSize
    );
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    viewers = [];
  } finally {
    isLoading = false;
  }
}

async function searchViewersAction(broadcasterId: string, query: string): Promise<void> {
  searchQuery = query;
  await loadViewers(broadcasterId, 0);
}

async function selectViewer(viewer: ViewerWithCustomInfo): Promise<void> {
  selectedViewer = viewer;
}

function clearSelection(): void {
  selectedViewer = null;
}

/**
 * Update viewer custom info using viewer_profile_id
 */
async function updateViewerCustomInfo(
  viewerProfileId: number,
  reading: string | null,
  notes: string | null,
  customData?: string | null
): Promise<void> {
  try {
    await viewerApi.viewerUpsertCustomInfo(viewerProfileId, reading, notes, customData);

    // Update local state
    if (selectedViewer && selectedViewer.id === viewerProfileId) {
      selectedViewer = { ...selectedViewer, reading, notes, custom_data: customData ?? selectedViewer.custom_data };
    }

    // Refresh viewers list
    if (selectedBroadcasterId) {
      await loadViewers(selectedBroadcasterId, currentPage);
    }
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    throw e;
  }
}

/**
 * Update viewer info (custom info + tags) using viewer_profile_id
 */
async function updateViewerInfo(
  viewerProfileId: number,
  reading: string | null,
  notes: string | null,
  customData: string | null,
  tags: string[] | null
): Promise<void> {
  try {
    await viewerApi.viewerUpdateInfo(viewerProfileId, reading, notes, customData, tags);

    // Update local state
    if (selectedViewer && selectedViewer.id === viewerProfileId) {
      selectedViewer = {
        ...selectedViewer,
        reading,
        notes,
        custom_data: customData,
        tags: tags || []
      };
    }

    // Refresh viewers list
    if (selectedBroadcasterId) {
      await loadViewers(selectedBroadcasterId, currentPage);
    }
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    throw e;
  }
}

function nextPage(): void {
  if (selectedBroadcasterId) {
    loadViewers(selectedBroadcasterId, currentPage + 1);
  }
}

function prevPage(): void {
  if (selectedBroadcasterId && currentPage > 0) {
    loadViewers(selectedBroadcasterId, currentPage - 1);
  }
}

async function loadBroadcasters(): Promise<void> {
  console.log('[viewerStore] loadBroadcasters called');
  isLoading = true;
  error = null;

  try {
    const result = await viewerApi.broadcasterGetList();
    console.log('[viewerStore] broadcasterGetList result:', JSON.stringify(result));
    broadcasters = result;
  } catch (e) {
    console.error('[viewerStore] loadBroadcasters error:', e);
    error = e instanceof Error ? e.message : String(e);
    broadcasters = [];
  } finally {
    isLoading = false;
    console.log('[viewerStore] loadBroadcasters finished, count:', broadcasters.length);
  }
}

/**
 * Delete viewer profile by viewer_profile_id
 */
async function deleteViewer(viewerProfileId: number): Promise<boolean> {
  try {
    const result = await viewerApi.viewerDelete(viewerProfileId);

    // Clear selection if deleted viewer was selected
    if (selectedViewer && selectedViewer.id === viewerProfileId) {
      selectedViewer = null;
    }

    // Refresh viewers list
    if (selectedBroadcasterId) {
      await loadViewers(selectedBroadcasterId, currentPage);
    }

    return result;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    throw e;
  }
}

async function deleteBroadcaster(broadcasterId: string): Promise<[boolean, number]> {
  try {
    const result = await viewerApi.broadcasterDelete(broadcasterId);

    // Clear selection if deleted broadcaster was selected
    if (selectedBroadcasterId === broadcasterId) {
      selectedBroadcasterId = null;
      viewers = [];
      selectedViewer = null;
    }

    // Refresh broadcasters list
    await loadBroadcasters();

    return result;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    throw e;
  }
}

// Export store interface
export const viewerStore = {
  // Getters (reactive)
  get viewers() {
    return viewers;
  },
  get sessions() {
    return sessions;
  },
  get broadcasters() {
    return broadcasters;
  },
  get selectedViewer() {
    return selectedViewer;
  },
  get selectedBroadcasterId() {
    return selectedBroadcasterId;
  },
  get searchQuery() {
    return searchQuery;
  },
  get isLoading() {
    return isLoading;
  },
  get error() {
    return error;
  },
  get currentPage() {
    return currentPage;
  },
  get pageSize() {
    return pageSize;
  },

  // Actions
  loadSessions,
  loadViewers,
  loadBroadcasters,
  searchViewers: searchViewersAction,
  selectViewer,
  clearSelection,
  updateViewerCustomInfo,
  updateViewerInfo,
  deleteViewer,
  deleteBroadcaster,
  nextPage,
  prevPage
};
