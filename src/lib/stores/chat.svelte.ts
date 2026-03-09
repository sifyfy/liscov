// Chat state management using Svelte 5 runes
import { listen } from '@tauri-apps/api/event';
import type { ChatMessage, ConnectionResult, ChatMode, ChatFilter } from '$lib/types';
import * as chatApi from '$lib/tauri/chat';
import { configStore } from './config.svelte';

// 接続状態の型定義
type ConnectionState = 'idle' | 'connecting' | 'connected' | 'paused' | 'error';

// ファクトリ関数：テスト時に独立したストアインスタンスを生成できる
function createChatStore() {
  // リアクティブ状態
  let messages = $state<ChatMessage[]>([]);
  let connectionState = $state<ConnectionState>('idle');
  let streamTitle = $state<string | null>(null);
  let broadcasterName = $state<string | null>(null);
  let broadcasterChannelId = $state<string | null>(null);
  let streamUrl = $state<string | null>(null);
  let isReplay = $state(false);
  let chatMode = $state<ChatMode>('top');
  let error = $state<string | null>(null);

  // 後方互換のための派生状態
  let isConnected = $derived(connectionState === 'connected');
  let isConnecting = $derived(connectionState === 'connecting');
  let isPaused = $derived(connectionState === 'paused');
  let filter = $state<ChatFilter>({
    showText: true,
    showSuperchat: true,
    showMembership: true,
    searchQuery: ''
  });

  // チャット表示設定
  const MIN_FONT_SIZE = 10;
  const MAX_FONT_SIZE = 24;
  const DEFAULT_FONT_SIZE = 13;
  let messageFontSize = $state(DEFAULT_FONT_SIZE);
  let showTimestamps = $state(true);
  let autoScroll = $state(true);
  let displayLimit = $state<number | null>(null);
  let scrollToLatestTrigger = $state(0); // インクリメントでスクロールをトリガー

  // O(1)検索のための重複チェック用セット
  let messageIds = new Set<string>();

  // O(1)ビューワーメッセージ検索のためのチャンネルIDインデックス
  let messagesByChannel = new Map<string, ChatMessage[]>();

  // フィルターがデフォルト状態かどうか（全タイプ表示かつ検索クエリなし）
  let isDefaultFilter = $derived(
    filter.showText && filter.showSuperchat && filter.showMembership && !filter.searchQuery
  );

  // 派生状態：フィルタ済みメッセージ（カウント表示用）
  let filteredMessages = $derived.by(() => {
    if (isDefaultFilter) {
      return messages; // O(1)：参照をそのまま返す
    }
    return messages.filter((msg) => {
      // メッセージタイプでフィルタ
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

      // 検索クエリでフィルタ
      if (filter.searchQuery) {
        const query = filter.searchQuery.toLowerCase();
        return (
          msg.content.toLowerCase().includes(query) || msg.author.toLowerCase().includes(query)
        );
      }

      return true;
    });
  });

  // 派生状態：表示メッセージ（displayLimit適用済み、レンダリング用）
  let displayedMessages = $derived.by(() => {
    if (displayLimit !== null) {
      return filteredMessages.slice(-displayLimit);
    }
    return filteredMessages;
  });

  // メッセージバッチング（高ボリームストリーム用）
  let pendingMessages: ChatMessage[] = [];
  let batchTimeout: ReturnType<typeof setTimeout> | null = null;
  const BATCH_DELAY_MS = 50; // 50ms以内のメッセージをバッチ処理

  function flushPendingMessages(): void {
    if (pendingMessages.length === 0) return;

    for (const msg of pendingMessages) {
      messageIds.add(msg.id);
      // チャンネルインデックスを更新
      const arr = messagesByChannel.get(msg.channel_id);
      if (arr) arr.push(msg);
      else messagesByChannel.set(msg.channel_id, [msg]);
    }
    messages.push(...pendingMessages);
    pendingMessages = [];
    batchTimeout = null;
  }

  function addMessage(message: ChatMessage): void {
    // O(1)重複チェック（セットとペンディングリスト両方を確認）
    if (messageIds.has(message.id) || pendingMessages.some((m) => m.id === message.id)) {
      return;
    }

    pendingMessages.push(message);

    // バッチフラッシュをスケジュール（未スケジュールの場合のみ）
    if (!batchTimeout) {
      batchTimeout = setTimeout(flushPendingMessages, BATCH_DELAY_MS);
    }
  }

  // アクション
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
        messageIds.clear();
        messagesByChannel.clear();
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
        error: error,
        session_id: null
      };
    }
  }

  // モニタリングを一時停止（メッセージとストリーム情報は保持）
  async function pause(): Promise<void> {
    try {
      await chatApi.disconnectStream();
    } finally {
      connectionState = 'paused';
      // streamTitle、broadcasterName、messages等はそのまま維持
    }
  }

  // モニタリングを再開（同じストリームに再接続）
  async function resume(): Promise<ConnectionResult> {
    if (!streamUrl) {
      return {
        success: false,
        stream_title: null,
        broadcaster_channel_id: null,
        broadcaster_name: null,
        is_replay: false,
        error: 'No stream URL to resume',
        session_id: null
      };
    }

    connectionState = 'connecting';
    error = null;

    try {
      const result = await chatApi.connectToStream(streamUrl, chatMode);

      if (result.success) {
        connectionState = 'connected';
        streamTitle = result.stream_title;
        broadcasterName = result.broadcaster_name;
        broadcasterChannelId = result.broadcaster_channel_id;
        isReplay = result.is_replay;
        // 再開時はメッセージをクリアしない
      } else {
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
        error: error,
        session_id: null
      };
    }
  }

  // 初期化（全てクリアしてidle状態に戻る）
  async function initialize(): Promise<void> {
    try {
      await chatApi.disconnectStream();
    } catch {
      // クリーンアップ中のエラーは無視
    } finally {
      connectionState = 'idle';
      streamTitle = null;
      broadcasterName = null;
      broadcasterChannelId = null;
      streamUrl = null;
      isReplay = false;
      messages = [];
      messageIds.clear();
      messagesByChannel.clear();
      error = null;
    }
  }

  // 後方互換のためのdisconnect（pauseのエイリアス）
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
    messageIds.clear();
    messagesByChannel.clear();
    pendingMessages = [];
  }

  function setFontSize(size: number): void {
    const clampedSize = Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, size));
    messageFontSize = clampedSize;
    // 永続化 (spec: 09_config.md)
    configStore.setMessageFontSize(clampedSize);
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

  function getMessagesForChannel(channelId: string): ChatMessage[] {
    return messagesByChannel.get(channelId) || [];
  }

  // イベントリスナーのクリーンアップ関数
  let unlisten: (() => void) | null = null;

  async function setupEventListeners(): Promise<void> {
    // 新規チャットメッセージイベントを購読
    const unlistenMessage = await listen<ChatMessage>('chat:message', (event) => {
      addMessage(event.payload);
    });

    // 接続状態変更イベントを購読
    const unlistenConnection = await listen<ConnectionResult>('chat:connection', (event) => {
      const result = event.payload;

      // idle状態（未接続）の場合は無視
      if (connectionState === 'idle') {
        return;
      }

      if (result.success) {
        connectionState = 'connected';
        streamTitle = result.stream_title;
        broadcasterName = result.broadcaster_name;
        broadcasterChannelId = result.broadcaster_channel_id;
        isReplay = result.is_replay;
      } else {
        connectionState = 'error';
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

  // コンフィグからディスプレイ設定を初期化 (spec: 09_config.md)
  function initDisplaySettings(): void {
    if (configStore.isLoaded) {
      messageFontSize = configStore.messageFontSize;
      showTimestamps = configStore.showTimestamps;
      autoScroll = configStore.autoScrollEnabled;
    }
  }

  return {
    // Getters (リアクティブ)
    get messages() {
      return messages;
    },
    get filteredMessages() {
      return filteredMessages;
    },
    get displayedMessages() {
      return displayedMessages;
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

    // アクション
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
    getMessagesForChannel,
    setupEventListeners,
    cleanup,
    initDisplaySettings
  };
}

// アプリ全体で使うシングルトンインスタンス
export const chatStore = createChatStore();
