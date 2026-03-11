// WebSocket 関連の Tauri コマンドラッパー
import { invoke } from '@tauri-apps/api/core';
import type { WebSocketStatus } from '$lib/types';
import { normalizeError } from './errors';

/**
 * WebSocket サーバーの状態を取得する
 *
 * WebSocket サーバーはアプリ起動時に自動的に開始される。
 * 手動での起動・停止は不要。
 */
export async function websocketGetStatus(): Promise<WebSocketStatus> {
  try {
    return await invoke('websocket_get_status');
  } catch (e) {
    throw normalizeError(e);
  }
}
