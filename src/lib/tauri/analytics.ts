// アナリティクス関連の Tauri コマンドラッパー
import { invoke } from '@tauri-apps/api/core';
import type { RevenueAnalytics, ExportConfig } from '$lib/types';
import { normalizeError } from './errors';

/**
 * 現在のセッションの収益アナリティクスを取得する
 */
export async function getRevenueAnalytics(): Promise<RevenueAnalytics> {
  try {
    return await invoke('get_revenue_analytics');
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * データベースから特定セッションのアナリティクスを取得する
 */
export async function getSessionAnalytics(sessionId: string): Promise<RevenueAnalytics> {
  try {
    return await invoke('get_session_analytics', { sessionId });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * セッションデータをファイルにエクスポートする
 */
export async function exportSessionData(
  sessionId: string,
  filePath: string,
  config: ExportConfig
): Promise<void> {
  try {
    return await invoke('export_session_data', { sessionId, filePath, config });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * 現在のセッションメッセージをファイルにエクスポートする
 */
export async function exportCurrentMessages(
  filePath: string,
  config: ExportConfig
): Promise<void> {
  try {
    return await invoke('export_current_messages', { filePath, config });
  } catch (e) {
    throw normalizeError(e);
  }
}
