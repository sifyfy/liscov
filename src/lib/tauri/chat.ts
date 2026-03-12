// チャット関連の Tauri コマンドラッパー
import { invoke } from '@tauri-apps/api/core';
import type { ChatMessage, ConnectionResult, ConnectionInfo, ChatMode } from '$lib/types';
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
 * 特定の接続IDのストリームから切断する
 */
export async function disconnectStream(connectionId: number): Promise<void> {
  try {
    return await invoke('disconnect_stream', { connectionId });
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * 全ての接続を切断する
 */
export async function disconnectAllStreams(): Promise<void> {
  try {
    return await invoke('disconnect_all_streams');
  } catch (e) {
    throw normalizeError(e);
  }
}

/**
 * 現在の全接続情報を取得する
 */
export async function getConnections(): Promise<ConnectionInfo[]> {
  try {
    return await invoke('get_connections');
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
 * 特定の接続のチャットモードを設定する（トップ or 全て）
 */
export async function setChatMode(connectionId: number, mode: ChatMode): Promise<boolean> {
  try {
    const chatMode = mode === 'all' ? 'AllChat' : 'TopChat';
    return await invoke('set_chat_mode', { connectionId, mode: chatMode });
  } catch (e) {
    throw normalizeError(e);
  }
}
