// Chat state management using Svelte 5 runes
import { listen } from '@tauri-apps/api/event';
import type { ChatMessage, ConnectionResult, ChatMode, ChatFilter } from '$lib/types';
import * as chatApi from '$lib/tauri/chat';

const MAX_MESSAGES = 500;

// Connection states: 'idle' | 'connecting' | 'connected' | 'paused' | 'error'
type ConnectionState = 'idle' | 'connecting' | 'connected' | 'paused' | 'error';

// Reactive state
let messages = $state<ChatMessage[]>([]);
let connectionState = $state<ConnectionState>('idle');
let streamTitle = $state<string | null>(null);
let broadcasterName = $state<string | null>(null);
let broadcasterChannelId = $state<string | null>(null);
let streamUrl = $state<string | null>(null);
let isReplay = $state(false);
let chatMode = $state<ChatMode>('top');
let error = $state<string | null>(null);

// Derived states for backward compatibility
let isConnected = $derived(connectionState === 'connected');
let isConnecting = $derived(connectionState === 'connecting');
let isPaused = $derived(connectionState === 'paused');
let filter = $state<ChatFilter>({
  showText: true,
  showSuperchat: true,
  showMembership: true,
  searchQuery: ''
});

// Chat display settings
const MIN_FONT_SIZE = 10;
const MAX_FONT_SIZE = 24;
const DEFAULT_FONT_SIZE = 13;
let messageFontSize = $state(DEFAULT_FONT_SIZE);
let showTimestamps = $state(true);
let autoScroll = $state(true);
let displayLimit = $state<number | null>(null);
let scrollToLatestTrigger = $state(0); // Increment to trigger scroll

// Derived state: filtered messages
let filteredMessages = $derived.by(() => {
  return messages.filter((msg) => {
    // Filter by message type
    if (!filter.showText && msg.message_type === 'text') return false;
    if (
      !filter.showSuperchat &&
      (msg.message_type === 'superchat' || msg.message_type === 'supersticker')
    )
      return false;
    if (
      !filter.showMembership &&
      (msg.message_type === 'membership' || msg.message_type === 'membership_gift')
    )
      return false;

    // Filter by search query
    if (filter.searchQuery) {
      const query = filter.searchQuery.toLowerCase();
      return (
        msg.content.toLowerCase().includes(query) || msg.author.toLowerCase().includes(query)
      );
    }

    return true;
  });
});

// Actions
async function connect(url: string, mode?: ChatMode): Promise<ConnectionResult> {
  connectionState = 'connecting';
  error = null;

  try {
    const result = await chatApi.connectToStream(url, mode);

    if (result.success) {
      connectionState = 'connected';
      streamTitle = result.stream_title;
      broadcasterName = result.broadcaster_name;
      broadcasterChannelId = result.broadcaster_channel_id;
      streamUrl = url;
      isReplay = result.is_replay;
      messages = [];
    } else {
      connectionState = 'error';
      error = result.error;
    }

    return result;
  } catch (e) {
    connectionState = 'error';
    error = e instanceof Error ? e.message : String(e);
    return {
      success: false,
      stream_title: null,
      broadcaster_channel_id: null,
      broadcaster_name: null,
      is_replay: false,
      error: error
    };
  }
}

// Pause monitoring (preserves messages and stream info)
async function pause(): Promise<void> {
  try {
    await chatApi.disconnectStream();
  } finally {
    connectionState = 'paused';
    // Keep streamTitle, broadcasterName, messages, etc.
  }
}

// Resume monitoring (reconnect to the same stream)
async function resume(): Promise<ConnectionResult> {
  console.log('[chat.svelte.ts] resume() called, streamUrl:', streamUrl, 'chatMode:', chatMode);

  if (!streamUrl) {
    console.log('[chat.svelte.ts] resume() - no streamUrl, returning error');
    return {
      success: false,
      stream_title: null,
      broadcaster_channel_id: null,
      broadcaster_name: null,
      is_replay: false,
      error: 'No stream URL to resume'
    };
  }

  console.log('[chat.svelte.ts] resume() - setting connectionState to connecting');
  connectionState = 'connecting';
  error = null;

  try {
    console.log('[chat.svelte.ts] resume() - calling chatApi.connectToStream...');
    const result = await chatApi.connectToStream(streamUrl, chatMode);
    console.log('[chat.svelte.ts] resume() - connectToStream returned:', JSON.stringify(result));

    if (result.success) {
      console.log('[chat.svelte.ts] resume() - success, updating state...');
      connectionState = 'connected';
      streamTitle = result.stream_title;
      broadcasterName = result.broadcaster_name;
      broadcasterChannelId = result.broadcaster_channel_id;
      isReplay = result.is_replay;
      console.log('[chat.svelte.ts] resume() - state updated, connectionState:', connectionState);
      // Don't clear messages on resume
    } else {
      console.log('[chat.svelte.ts] resume() - failed:', result.error);
      connectionState = 'error';
      error = result.error;
    }

    return result;
  } catch (e) {
    console.error('[chat.svelte.ts] resume() - exception:', e);
    connectionState = 'error';
    error = e instanceof Error ? e.message : String(e);
    return {
      success: false,
      stream_title: null,
      broadcaster_channel_id: null,
      broadcaster_name: null,
      is_replay: false,
      error: error
    };
  }
}

// Initialize (clear everything and go back to idle)
async function initialize(): Promise<void> {
  try {
    await chatApi.disconnectStream();
  } catch {
    // Ignore errors during cleanup
  } finally {
    connectionState = 'idle';
    streamTitle = null;
    broadcasterName = null;
    broadcasterChannelId = null;
    streamUrl = null;
    isReplay = false;
    messages = [];
    error = null;
  }
}

// Legacy disconnect (alias for pause for backward compatibility)
async function disconnect(): Promise<void> {
  await pause();
}

async function setChatModeAction(mode: ChatMode): Promise<void> {
  chatMode = mode;
  await chatApi.setChatMode(mode);
}

function setFilter(newFilter: Partial<ChatFilter>): void {
  filter = { ...filter, ...newFilter };
}

function clearMessages(): void {
  messages = [];
}

// Message batching for high-volume streams
let pendingMessages: ChatMessage[] = [];
let batchTimeout: ReturnType<typeof setTimeout> | null = null;
const BATCH_DELAY_MS = 50; // Batch messages within 50ms window

function flushPendingMessages(): void {
  if (pendingMessages.length === 0) return;

  // Combine pending messages with existing, respecting MAX_MESSAGES limit
  const newMessages = [...messages, ...pendingMessages];
  messages = newMessages.slice(-MAX_MESSAGES);
  pendingMessages = [];
  batchTimeout = null;
}

function addMessage(message: ChatMessage): void {
  // Skip duplicate IDs (can happen when reconnecting to YouTube)
  const isDuplicate =
    messages.some((m) => m.id === message.id) ||
    pendingMessages.some((m) => m.id === message.id);
  if (isDuplicate) {
    return;
  }

  pendingMessages.push(message);

  // Schedule a batch flush if not already scheduled
  if (!batchTimeout) {
    batchTimeout = setTimeout(flushPendingMessages, BATCH_DELAY_MS);
  }
}

function setFontSize(size: number): void {
  messageFontSize = Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, size));
}

function increaseFontSize(): void {
  setFontSize(messageFontSize + 1);
}

function decreaseFontSize(): void {
  setFontSize(messageFontSize - 1);
}

function setShowTimestamps(show: boolean): void {
  showTimestamps = show;
}

function setAutoScroll(enabled: boolean): void {
  autoScroll = enabled;
}

function scrollToLatest(): void {
  scrollToLatestTrigger++;
}

function setDisplayLimit(limit: number | null): void {
  displayLimit = limit;
}

// Event listener setup
let unlisten: (() => void) | null = null;
let messageCountSinceResume = 0;
let lastMessageLogTime = 0;

async function setupEventListeners(): Promise<void> {
  console.log('[chat.svelte.ts] setupEventListeners() called');

  // Listen for new chat messages
  const unlistenMessage = await listen<ChatMessage>('chat:message', (event) => {
    messageCountSinceResume++;
    const now = Date.now();
    // Log every 5 seconds at most
    if (now - lastMessageLogTime > 5000) {
      console.log('[chat.svelte.ts] chat:message received, count since last log:', messageCountSinceResume, 'connectionState:', connectionState);
      messageCountSinceResume = 0;
      lastMessageLogTime = now;
    }
    addMessage(event.payload);
  });

  // Listen for connection status changes
  const unlistenConnection = await listen<ConnectionResult>('chat:connection', (event) => {
    const result = event.payload;
    console.log('[chat.svelte.ts] chat:connection received:', JSON.stringify(result), 'current connectionState:', connectionState);

    // Don't update state if idle (not connected yet)
    if (connectionState === 'idle') {
      console.log('[chat.svelte.ts] chat:connection - ignoring because connectionState is idle');
      return;
    }

    if (result.success) {
      connectionState = 'connected';
      streamTitle = result.stream_title;
      broadcasterName = result.broadcaster_name;
      broadcasterChannelId = result.broadcaster_channel_id;
      isReplay = result.is_replay;
      console.log('[chat.svelte.ts] chat:connection - updated to connected');
    } else {
      connectionState = 'error';
      error = result.error;
      console.log('[chat.svelte.ts] chat:connection - updated to error:', error);
    }
  });

  unlisten = () => {
    unlistenMessage();
    unlistenConnection();
  };
}

function cleanup(): void {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

// Export store interface
export const chatStore = {
  // Getters (reactive)
  get messages() {
    return messages;
  },
  get filteredMessages() {
    return filteredMessages;
  },
  get isConnected() {
    return isConnected;
  },
  get streamTitle() {
    return streamTitle;
  },
  get broadcasterName() {
    return broadcasterName;
  },
  get broadcasterChannelId() {
    return broadcasterChannelId;
  },
  get isReplay() {
    return isReplay;
  },
  get chatMode() {
    return chatMode;
  },
  get isConnecting() {
    return isConnecting;
  },
  get error() {
    return error;
  },
  get filter() {
    return filter;
  },
  get messageFontSize() {
    return messageFontSize;
  },
  get showTimestamps() {
    return showTimestamps;
  },
  get isPaused() {
    return isPaused;
  },
  get connectionState() {
    return connectionState;
  },
  get autoScroll() {
    return autoScroll;
  },
  get displayLimit() {
    return displayLimit;
  },
  get scrollToLatestTrigger() {
    return scrollToLatestTrigger;
  },

  // Actions
  connect,
  disconnect,
  pause,
  resume,
  initialize,
  setChatMode: setChatModeAction,
  setFilter,
  clearMessages,
  setFontSize,
  increaseFontSize,
  decreaseFontSize,
  setShowTimestamps,
  setAutoScroll,
  scrollToLatest,
  setDisplayLimit,
  setupEventListeners,
  cleanup
};
