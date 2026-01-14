// WebSocket-related Tauri commands
import { invoke } from '@tauri-apps/api/core';
import type { WebSocketStatus, WebSocketStartResult } from '$lib/types';

/**
 * Start WebSocket server for external app integration
 */
export async function websocketStart(port?: number): Promise<WebSocketStartResult> {
  return invoke('websocket_start', { port });
}

/**
 * Stop WebSocket server
 */
export async function websocketStop(): Promise<void> {
  return invoke('websocket_stop');
}

/**
 * Get WebSocket server status
 */
export async function websocketGetStatus(): Promise<WebSocketStatus> {
  return invoke('websocket_get_status');
}
