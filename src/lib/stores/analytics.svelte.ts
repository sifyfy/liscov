// Analytics state management using Svelte 5 runes
import type { RevenueAnalytics, ExportConfig } from '$lib/types';
import * as analyticsApi from '$lib/tauri/analytics';

// ファクトリ関数：テスト時に独立したストアインスタンスを生成できる
function createAnalyticsStore() {
  // リアクティブ状態
  let analytics = $state<RevenueAnalytics | null>(null);
  let isLoading = $state(false);
  let error = $state<string | null>(null);
  let lastUpdate = $state<Date | null>(null);

  // アクション
  async function loadAnalytics(): Promise<void> {
    isLoading = true;
    error = null;

    try {
      analytics = await analyticsApi.getRevenueAnalytics();
      lastUpdate = new Date();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      isLoading = false;
    }
  }

  async function loadSessionAnalytics(sessionId: string): Promise<void> {
    isLoading = true;
    error = null;

    try {
      analytics = await analyticsApi.getSessionAnalytics(sessionId);
      lastUpdate = new Date();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      isLoading = false;
    }
  }

  async function exportSession(
    sessionId: string,
    filePath: string,
    config: ExportConfig
  ): Promise<void> {
    try {
      await analyticsApi.exportSessionData(sessionId, filePath, config);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  async function exportCurrent(filePath: string, config: ExportConfig): Promise<void> {
    try {
      await analyticsApi.exportCurrentMessages(filePath, config);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  function clearError(): void {
    error = null;
  }

  return {
    // Getters (リアクティブ)
    get analytics() {
      return analytics;
    },
    get isLoading() {
      return isLoading;
    },
    get error() {
      return error;
    },
    get lastUpdate() {
      return lastUpdate;
    },

    // 算出値
    get totalPaidCount() {
      if (!analytics) return 0;
      return analytics.super_chat_count + analytics.super_sticker_count;
    },
    get totalTierCount() {
      if (!analytics) return 0;
      const t = analytics.super_chat_by_tier;
      return t.tier_red + t.tier_magenta + t.tier_orange + t.tier_yellow + t.tier_green + t.tier_cyan + t.tier_blue;
    },

    // アクション
    loadAnalytics,
    loadSessionAnalytics,
    exportSession,
    exportCurrent,
    clearError
  };
}

// アプリ全体で使うシングルトンインスタンス
export const analyticsStore = createAnalyticsStore();
