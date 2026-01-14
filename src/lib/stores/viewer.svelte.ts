// Viewer state management using Svelte 5 runes
import type { ViewerWithCustomInfo, Session } from '$lib/types';
import * as viewerApi from '$lib/tauri/viewer';

// Reactive state
let viewers = $state<ViewerWithCustomInfo[]>([]);
let sessions = $state<Session[]>([]);
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
    viewers = await viewerApi.getViewersForBroadcaster(
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

async function updateViewerCustomInfo(
  broadcasterId: string,
  viewerId: string,
  reading: string | null,
  notes: string | null
): Promise<void> {
  try {
    await viewerApi.upsertViewerCustomInfo({
      broadcaster_channel_id: broadcasterId,
      viewer_channel_id: viewerId,
      reading,
      notes
    });

    // Update local state
    if (selectedViewer && selectedViewer.channel_id === viewerId) {
      selectedViewer = { ...selectedViewer, reading, notes };
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

// Export store interface
export const viewerStore = {
  // Getters (reactive)
  get viewers() {
    return viewers;
  },
  get sessions() {
    return sessions;
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
  searchViewers: searchViewersAction,
  selectViewer,
  clearSelection,
  updateViewerCustomInfo,
  nextPage,
  prevPage
};
