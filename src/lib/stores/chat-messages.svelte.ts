// チャット一覧のメッセージ: 受信のバッチ・重複チェック・視聴者ごとの索引・フィルタ・表示件数
// （02_chat.md「メッセージ表示」「フィルタ機能」「接続設定の統計」）。接続の管理は chat.svelte.ts
import type { ChatMessage, ChatFilter } from '$lib/types';
import { SvelteMap } from 'svelte/reactivity';
import { messageKey, type MessageKey } from '$lib/utils/message-key';

// 50ms 以内に届いたメッセージをまとめて一覧に足す（高ボリュームの配信で描画を減らす）
const BATCH_DELAY_MS = 50;

// フィルタに合うか（02_chat.md「フィルタ機能」）
function matchesFilter(msg: ChatMessage, filter: ChatFilter): boolean {
  if (!filter.showText && msg.message_type === 'text') return false;
  if (
    !filter.showSuperchat &&
    (msg.message_type === 'superchat' ||
      msg.message_type === 'supersticker' ||
      msg.message_type === 'gift')
  )
    return false;
  if (
    !filter.showMembership &&
    (msg.message_type === 'membership' || msg.message_type === 'membership_gift')
  )
    return false;

  if (filter.searchQuery) {
    const query = filter.searchQuery.toLowerCase();
    return msg.content.toLowerCase().includes(query) || msg.author.toLowerCase().includes(query);
  }
  return true;
}

export function createMessageList() {
  // メッセージは受け取ったまま変えないので深いリアクティブにしない。足すときは配列ごと差し替える
  let messages = $state.raw<ChatMessage[]>([]);

  // 重複チェック用セット（複合キー: connection_id:message_id）。受け取った時点（バッチ待ちを含む）で入れる
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- 画面から読まない。受信ごとの通知を起こさないため素の Set
  const messageIds = new Set<MessageKey>();

  // 視聴者ごとのメッセージ（視聴者情報パネル用）。届いたら配列ごと差し替えて、開いているパネルにも反映する
  const messagesByChannel = new SvelteMap<string, ChatMessage[]>();

  // 一覧にいる視聴者の数（視聴者を特定できないギフトの channel_id '' は数えない。02_chat.md「接続設定の統計」）
  let viewerCount = $derived(messagesByChannel.size - (messagesByChannel.has('') ? 1 : 0));

  let filter = $state<ChatFilter>({
    showText: true,
    showSuperchat: true,
    showMembership: true,
    searchQuery: ''
  });
  let displayLimit = $state<number | null>(null);

  // フィルターがデフォルト状態かどうか（全タイプ表示かつ検索クエリなし）
  let isDefaultFilter = $derived(
    filter.showText && filter.showSuperchat && filter.showMembership && !filter.searchQuery
  );

  // 前回のフィルタ結果。messages は追記とクリアでしか変わらないので、
  // 同じフィルタ（setFilter は filter を差し替える）で前回の messages が今回の先頭なら、増えた分だけ判定する
  let lastFiltered: { source: ChatMessage[]; filter: ChatFilter; result: ChatMessage[] } | null = null;

  // フィルタ済みメッセージ（カウント表示用）
  let filteredMessages = $derived.by(() => {
    if (isDefaultFilter) {
      lastFiltered = null;
      return messages; // O(1)：参照をそのまま返す
    }
    const prev = lastFiltered;
    const extendsPrev =
      prev !== null &&
      prev.filter === filter &&
      messages.length >= prev.source.length &&
      messages[prev.source.length - 1] === prev.source[prev.source.length - 1];
    const result = extendsPrev
      ? [...prev.result, ...messages.slice(prev.source.length).filter((m) => matchesFilter(m, filter))]
      : messages.filter((m) => matchesFilter(m, filter));
    lastFiltered = { source: messages, filter, result };
    return result;
  });

  // 表示メッセージ（displayLimit適用済み、レンダリング用）
  let displayedMessages = $derived.by(() => {
    if (displayLimit !== null) {
      return filteredMessages.slice(-displayLimit);
    }
    return filteredMessages;
  });

  let pendingMessages: ChatMessage[] = [];
  let batchTimeout: ReturnType<typeof setTimeout> | null = null;

  function flushPendingMessages(): void {
    batchTimeout = null;
    if (pendingMessages.length === 0) return;

    // チャンネルインデックスを更新（視聴者ごとに 1 回だけ差し替える）
    const byChannel = Map.groupBy(pendingMessages, (msg) => msg.channel_id);
    for (const [channelId, added] of byChannel) {
      messagesByChannel.set(channelId, [...(messagesByChannel.get(channelId) ?? []), ...added]);
    }
    messages = [...messages, ...pendingMessages];
    pendingMessages = [];
  }

  /** メッセージを受け取る。同じ接続の同じ message_id を既に受け取っていれば false（足さない） */
  function add(message: ChatMessage): boolean {
    const key = messageKey(message);
    if (messageIds.has(key)) {
      return false;
    }

    messageIds.add(key);
    pendingMessages.push(message);

    // バッチフラッシュをスケジュール（未スケジュールの場合のみ）
    if (!batchTimeout) {
      batchTimeout = setTimeout(flushPendingMessages, BATCH_DELAY_MS);
    }
    return true;
  }

  function clear(): void {
    messages = [];
    messageIds.clear();
    messagesByChannel.clear();
    pendingMessages = [];
  }

  return {
    get messages() {
      return messages;
    },
    get filteredMessages() {
      return filteredMessages;
    },
    get displayedMessages() {
      return displayedMessages;
    },
    get viewerCount() {
      return viewerCount;
    },
    get filter() {
      return filter;
    },
    get displayLimit() {
      return displayLimit;
    },
    add,
    clear,
    setFilter(newFilter: Partial<ChatFilter>): void {
      filter = { ...filter, ...newFilter };
    },
    setDisplayLimit(limit: number | null): void {
      displayLimit = limit;
    },
    getMessagesForChannel(channelId: string): ChatMessage[] {
      return messagesByChannel.get(channelId) ?? [];
    }
  };
}
