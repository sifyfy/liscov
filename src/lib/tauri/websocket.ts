// WebSocket-related Tauri commands
import { invoke } from '@tauri-apps/api/core';
import type { WebSocketStatus } from '$lib/types';

/**
 * Get WebSocket server status
 *
 * WebSocket server starts automatically on app launch.
 * No manual start/stop is required.
 */
export async function websocketGetStatus(): Promise<WebSocketStatus> {
  return invoke('websocket_get_status');
}
