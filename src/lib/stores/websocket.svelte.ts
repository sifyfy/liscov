// WebSocket state management using Svelte 5 runes
// WebSocket server auto-starts on app launch - no manual start/stop needed
import type { WebSocketStatus } from '$lib/types';
import * as wsApi from '$lib/tauri/websocket';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// Reactive state
let isRunning = $state(false);
let actualPort = $state<number | null>(null);
let connectedClients = $state(0);
let error = $state<string | null>(null);
let initialized = $state(false);

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

// Initialize store - called once on first use
// Polls for WebSocket server to be ready (auto-starts on app launch)
async function init(): Promise<void> {
  if (initialized) return;

  try {
    // Set up event listeners for client connect/disconnect
    await setupEventListeners();

    // Poll for WebSocket server to be ready (it auto-starts on app launch)
    // Give it up to 10 seconds to start
    const maxWait = 10000;
    const pollInterval = 500;
    const start = Date.now();

    while (Date.now() - start < maxWait) {
      await refreshStatus();
      if (isRunning) {
        break;
      }
      await new Promise(resolve => setTimeout(resolve, pollInterval));
    }

    initialized = true;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }
}

// Refresh status from backend
async function refreshStatus(): Promise<void> {
  try {
    const status = await wsApi.websocketGetStatus();
    isRunning = status.is_running;
    actualPort = status.actual_port;
    connectedClients = status.connected_clients;
    error = null;
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
  get error() {
    return error;
  },
  get initialized() {
    return initialized;
  },

  // Actions
  init,
  refreshStatus
};
