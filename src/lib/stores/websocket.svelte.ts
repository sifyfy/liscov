// WebSocket state management using Svelte 5 runes
import type { WebSocketStatus } from '$lib/types';
import * as wsApi from '$lib/tauri/websocket';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// Reactive state
let isRunning = $state(false);
let actualPort = $state<number | null>(null);
let connectedClients = $state(0);
let isStarting = $state(false);
let error = $state<string | null>(null);

// Event listeners
let unlistenConnect: UnlistenFn | null = null;
let unlistenDisconnect: UnlistenFn | null = null;

async function setupEventListeners(): Promise<void> {
  // Clean up existing listeners
  await cleanupEventListeners();

  // Listen for client connections
  unlistenConnect = await listen<{ client_id: number }>('websocket-client-connected', () => {
    connectedClients++;
  });

  // Listen for client disconnections
  unlistenDisconnect = await listen<{ client_id: number }>('websocket-client-disconnected', () => {
    connectedClients = Math.max(0, connectedClients - 1);
  });
}

async function cleanupEventListeners(): Promise<void> {
  if (unlistenConnect) {
    unlistenConnect();
    unlistenConnect = null;
  }
  if (unlistenDisconnect) {
    unlistenDisconnect();
    unlistenDisconnect = null;
  }
}

// Actions
async function start(port?: number): Promise<void> {
  isStarting = true;
  error = null;

  try {
    const result = await wsApi.websocketStart(port);
    isRunning = true;
    actualPort = result.actual_port;
    connectedClients = 0;
    // Setup event listeners for client connect/disconnect
    await setupEventListeners();
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
    // Cleanup event listeners
    await cleanupEventListeners();
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
