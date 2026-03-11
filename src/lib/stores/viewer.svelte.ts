// Viewer state management using Svelte 5 runes
import type { ViewerWithCustomInfo, Session, BroadcasterChannel } from '$lib/types';
import * as viewerApi from '$lib/tauri/viewer';

// ファクトリ関数：テスト時に独立したストアインスタンスを生成できる
function createViewerStore() {
  // リアクティブ状態
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

  // アクション
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
   * viewer_profile_id でビューワーのカスタム情報を更新する
   */
  async function updateViewerCustomInfo(
    viewerProfileId: number,
    reading: string | null,
    notes: string | null,
    customData?: string | null
  ): Promise<void> {
    try {
      await viewerApi.viewerUpsertCustomInfo(viewerProfileId, reading, notes, customData);

      // ローカル状態を更新
      if (selectedViewer && selectedViewer.id === viewerProfileId) {
        selectedViewer = { ...selectedViewer, reading, notes, custom_data: customData ?? selectedViewer.custom_data };
      }

      // ビューワー一覧を再取得
      if (selectedBroadcasterId) {
        await loadViewers(selectedBroadcasterId, currentPage);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  /**
   * viewer_profile_id でビューワー情報（カスタム情報 + タグ）を更新する
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

      // ローカル状態を更新
      if (selectedViewer && selectedViewer.id === viewerProfileId) {
        selectedViewer = {
          ...selectedViewer,
          reading,
          notes,
          custom_data: customData,
          tags: tags || []
        };
      }

      // ビューワー一覧を再取得
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
    isLoading = true;
    error = null;

    try {
      broadcasters = await viewerApi.broadcasterGetList();
    } catch (e) {
      console.error('[viewerStore] loadBroadcasters error:', e);
      error = e instanceof Error ? e.message : String(e);
      broadcasters = [];
    } finally {
      isLoading = false;
    }
  }

  /**
   * viewer_profile_id でビューワープロファイルを削除する
   */
  async function deleteViewer(viewerProfileId: number): Promise<boolean> {
    try {
      const result = await viewerApi.viewerDelete(viewerProfileId);

      // 削除したビューワーが選択中だった場合は選択解除
      if (selectedViewer && selectedViewer.id === viewerProfileId) {
        selectedViewer = null;
      }

      // ビューワー一覧を再取得
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

      // 削除した配信者が選択中だった場合は選択解除
      if (selectedBroadcasterId === broadcasterId) {
        selectedBroadcasterId = null;
        viewers = [];
        selectedViewer = null;
      }

      // 配信者一覧を再取得
      await loadBroadcasters();

      return result;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  return {
    // Getters (リアクティブ)
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

    // アクション
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
}

// アプリ全体で使うシングルトンインスタンス
export const viewerStore = createViewerStore();
