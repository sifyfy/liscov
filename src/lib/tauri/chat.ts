// チャット関連の Tauri コマンドラッパー
import { invoke } from '@tauri-apps/api/core';
import type { ChatMessage, ConnectionResult, ChatMode } from '$lib/types';
import { normalizeError } from './errors';

/**
 * YouTube ライブストリームに接続する
 */
export async function connectToStream(
  url: string,
  chatMode?: ChatMode
): Promise<ConnectionResult> {
  try {
    return await invoke('connect_to_stream', {
      url,
      chatMode: chatMode === 'all' ? 'AllChat' : 'TopChat'
    });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * 現在のストリームから切断する
 */
export async function disconnectStream(): Promise<void> {
  try {
    return await invoke('disconnect_stream');
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * 最近のチャットメッセージを取得する
 */
export async function getChatMessages(limit?: number): Promise<ChatMessage[]> {
  try {
    return await invoke('get_chat_messages', { limit });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * チャットモードを設定する（トップ or 全て）
 */
export async function setChatMode(mode: ChatMode): Promise<boolean> {
  try {
    return await invoke('set_chat_mode', {
      mode: mode === 'all' ? 'AllChat' : 'TopChat'
    });
  } catch (e) {
    throw normalizeError(e);
  }
}
