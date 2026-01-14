// WebSocket state management using Svelte 5 runes
import type { WebSocketStatus } from '$lib/types';
import * as wsApi from '$lib/tauri/websocket';

// Reactive state
let isRunning = $state(false);
let actualPort = $state<number | null>(null);
let connectedClients = $state(0);
let isStarting = $state(false);
let error = $state<string | null>(null);

// Actions
async function start(port?: number): Promise<void> {
  isStarting = true;
  error = null;

  try {
    const result = await wsApi.websocketStart(port);
    isRunning = true;
    actualPort = result.actual_port;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    isRunning = false;
    actualPort = null;
  } finally {
    isStarting = false;
  }
}

async function stop(): Promise<void> {
  try {
    await wsApi.websocketStop();
  } finally {
    isRunning = false;
    actualPort = null;
    connectedClients = 0;
  }
}

async function refreshStatus(): Promise<void> {
  try {
    const status = await wsApi.websocketGetStatus();
    isRunning = status.is_running;
    actualPort = status.actual_port;
    connectedClients = status.connected_clients;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }
}

// Export store interface
export const websocketStore = {
  // Getters (reactive)
  get isRunning() {
    return isRunning;
  },
  get actualPort() {
    return actualPort;
  },
  get connectedClients() {
    return connectedClients;
  },
  get isStarting() {
    return isStarting;
  },
  get error() {
    return error;
  },

  // Actions
  start,
  stop,
  refreshStatus
};
