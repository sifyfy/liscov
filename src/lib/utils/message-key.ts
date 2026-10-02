import type { ChatMessage } from '$lib/types';

/** メッセージを一意に識別するキー（02_chat.md 制約: `connection_id:message_id` の複合キー） */
export type MessageKey = string & { readonly __brand: 'MessageKey' };

/**
 * メッセージの複合キーを返す
 *
 * YouTube の message_id は接続をまたぐと一意でない（同じ配信に再接続すると直近のコメントが同じ id で届く）。
 * 重複排除・一覧のキー・選択の判定はすべてこのキーで行う。
 */
export function messageKey(message: Pick<ChatMessage, 'connection_id' | 'id'>): MessageKey {
  return `${message.connection_id}:${message.id}` as MessageKey;
}
