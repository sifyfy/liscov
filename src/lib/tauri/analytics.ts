// Analytics Tauri commands
import { invoke } from '@tauri-apps/api/core';
import type { RevenueAnalytics, ExportConfig } from '$lib/types';

/**
 * Get revenue analytics for current session
 */
export async function getRevenueAnalytics(): Promise<RevenueAnalytics> {
  return invoke('get_revenue_analytics');
}

/**
 * Get analytics for a specific session from database
 */
export async function getSessionAnalytics(sessionId: string): Promise<RevenueAnalytics> {
  return invoke('get_session_analytics', { sessionId });
}

/**
 * Export session data to file
 */
export async function exportSessionData(
  sessionId: string,
  filePath: string,
  config: ExportConfig
): Promise<void> {
  return invoke('export_session_data', { sessionId, filePath, config });
}

/**
 * Export current session messages to file
 */
export async function exportCurrentMessages(
  filePath: string,
  config: ExportConfig
): Promise<void> {
  return invoke('export_current_messages', { filePath, config });
}
