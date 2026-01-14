// Chat state management using Svelte 5 runes
import { listen } from '@tauri-apps/api/event';
import type { ChatMessage, ConnectionResult, ChatMode, ChatFilter } from '$lib/types';
import * as chatApi from '$lib/tauri/chat';

const MAX_MESSAGES = 500;

// Reactive state
let messages = $state<ChatMessage[]>([]);
let isConnected = $state(false);
let streamTitle = $state<string | null>(null);
let broadcasterName = $state<string | null>(null);
let broadcasterChannelId = $state<string | null>(null);
let isReplay = $state(false);
let chatMode = $state<ChatMode>('top');
let isConnecting = $state(false);
let error = $state<string | null>(null);
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
  isConnecting = true;
  error = null;

  try {
    const result = await chatApi.connectToStream(url, mode);

    if (result.success) {
      isConnected = true;
      streamTitle = result.stream_title;
      broadcasterName = result.broadcaster_name;
      broadcasterChannelId = result.broadcaster_channel_id;
      isReplay = result.is_replay;
      messages = [];
    } else {
      error = result.error;
    }

    return result;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    return {
      success: false,
      stream_title: null,
      broadcaster_channel_id: null,
      broadcaster_name: null,
      is_replay: false,
      error: error
    };
  } finally {
    isConnecting = false;
  }
}

async function disconnect(): Promise<void> {
  try {
    await chatApi.disconnectStream();
  } finally {
    isConnected = false;
    streamTitle = null;
    broadcasterName = null;
    broadcasterChannelId = null;
    isReplay = false;
  }
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

function addMessage(message: ChatMessage): void {
  messages = [...messages.slice(-(MAX_MESSAGES - 1)), message];
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

// Event listener setup
let unlisten: (() => void) | null = null;

async function setupEventListeners(): Promise<void> {
  // Listen for new chat messages
  const unlistenMessage = await listen<ChatMessage>('chat:message', (event) => {
    addMessage(event.payload);
  });

  // Listen for connection status changes
  const unlistenConnection = await listen<ConnectionResult>('chat:connection', (event) => {
    const result = event.payload;
    isConnected = result.success;
    streamTitle = result.stream_title;
    broadcasterName = result.broadcaster_name;
    broadcasterChannelId = result.broadcaster_channel_id;
    isReplay = result.is_replay;

    if (!result.success) {
      error = result.error;
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

  // Actions
  connect,
  disconnect,
  setChatMode: setChatModeAction,
  setFilter,
  clearMessages,
  setFontSize,
  increaseFontSize,
  decreaseFontSize,
  setShowTimestamps,
  setupEventListeners,
  cleanup
};
