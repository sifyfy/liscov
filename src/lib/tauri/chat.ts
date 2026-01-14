// Chat-related Tauri commands
import { invoke } from '@tauri-apps/api/core';
import type { ChatMessage, ConnectionResult, ChatMode } from '$lib/types';

/**
 * Connect to a YouTube live stream
 */
export async function connectToStream(
  url: string,
  chatMode?: ChatMode
): Promise<ConnectionResult> {
  return invoke('connect_to_stream', {
    url,
    chatMode: chatMode === 'all' ? 'AllChat' : 'TopChat'
  });
}

/**
 * Disconnect from the current stream
 */
export async function disconnectStream(): Promise<void> {
  return invoke('disconnect_stream');
}

/**
 * Get recent chat messages
 */
export async function getChatMessages(limit?: number): Promise<ChatMessage[]> {
  return invoke('get_chat_messages', { limit });
}

/**
 * Set chat mode (top or all)
 */
export async function setChatMode(mode: ChatMode): Promise<boolean> {
  return invoke('set_chat_mode', {
    mode: mode === 'all' ? 'AllChat' : 'TopChat'
  });
}
