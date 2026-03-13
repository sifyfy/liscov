// Chat state management using Svelte 5 runes
import { listen } from '@tauri-apps/api/event';
import type { ChatMessage, ConnectionResult, ConnectionInfo, ChatMode, ChatFilter, FrontendConnectionState } from '$lib/types';
import * as chatApi from '$lib/tauri/chat';
import { getConnectionColor } from '$lib/utils/connection-colors';
import { configStore } from './config.svelte';

// ファクトリ関数：テスト時に独立したストアインスタンスを生成できる
function createChatStore() {
  // リアクティブ状態
  let messages = $state<ChatMessage[]>([]);
  // 多接続状態マップ（キー: connection_id as number）
  let connections = $state<Map<number, FrontendConnectionState>>(new Map());
  let chatMode = $state<ChatMode>('top');
  let error = $state<string | null>(null);

  // 多接続ベースの派生状態
  let isConnected = $derived(connections.size > 0);
  let isConnecting = $derived([...connections.values()].some(c => c.connectionState === 'connecting'));
  // 多接続ではglobalなpauseはない（常にfalse）
  let isPaused = $derived(false);
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

  // O(1)検索のための重複チェック用セット（複合キー: connection_id:message_id）
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
      // 複合キー（connection_id:message_id）で重複排除
      const key = `${msg.connection_id}:${msg.id}`;
      messageIds.add(key);
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
    // 複合キー（connection_id:message_id）でO(1)重複チェック
    const key = `${message.connection_id}:${message.id}`;
    if (messageIds.has(key) || pendingMessages.some((m) => `${m.connection_id}:${m.id}` === key)) {
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
    error = null;

    try {
      const result = await chatApi.connectToStream(url, mode);

      if (result.success) {
        const connId = Number(result.connection_id);
        const newConn: FrontendConnectionState = {
          id: connId,
          platform: 'youtube', // TODO: Rustから返ってきたときに更新
          streamUrl: url,
          streamTitle: result.stream_title ?? '',
          broadcasterName: result.broadcaster_name ?? '',
          broadcasterChannelId: result.broadcaster_channel_id ?? '',
          connectionState: 'connected',
          color: getConnectionColor(result.broadcaster_channel_id ?? String(connId))
        };
        // イミュータブルに新しいMapを作成して置き換え
        const next = new Map(connections);
        next.set(connId, newConn);
        connections = next;
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
        error: error,
        session_id: null,
        connection_id: BigInt(0)
      };
    }
  }

  // 特定の接続を切断
  async function disconnect(connectionId: number): Promise<void> {
    // 切断中状態に更新
    const conn = connections.get(connectionId);
    if (conn) {
      const next = new Map(connections);
      next.set(connectionId, { ...conn, connectionState: 'disconnecting' });
      connections = next;
    }

    try {
      await chatApi.disconnectStream(connectionId);
    } finally {
      // 接続マップから削除
      const next = new Map(connections);
      next.delete(connectionId);
      connections = next;
    }
  }

  // 全接続を切断
  async function disconnectAll(): Promise<void> {
    try {
      await chatApi.disconnectAllStreams();
    } finally {
      connections = new Map();
    }
  }

  // pause は多接続では非推奨 → disconnectAllのエイリアス
  async function pause(): Promise<void> {
    await disconnectAll();
  }

  // resume は多接続では廃止（ユーザーがURLを再入力して接続）
  // 後方互換のため空実装を残す
  async function resume(): Promise<ConnectionResult> {
    return {
      success: false,
      stream_title: null,
      broadcaster_channel_id: null,
      broadcaster_name: null,
      is_replay: false,
      error: 'resume() is not supported in multi-stream mode',
      session_id: null,
      connection_id: BigInt(0)
    };
  }

  // 初期化（全てクリアしてidle状態に戻る）
  async function initialize(): Promise<void> {
    try {
      await disconnectAll();
    } catch {
      // クリーンアップ中のエラーは無視
    } finally {
      connections = new Map();
      messages = [];
      messageIds.clear();
      messagesByChannel.clear();
      pendingMessages = [];
      error = null;
    }
  }

  async function setChatModeAction(mode: ChatMode): Promise<void> {
    chatMode = mode;
    // チャットモード動的切り替えは未実装（Phase 2）
    // エラーはログに出力し、ユーザーには chatMode の UI 状態のみ更新する
    for (const [connId] of connections) {
      try {
        await chatApi.setChatMode(connId, mode);
      } catch {
        // 未実装エラーは想定内 — 切断・再接続で反映される
      }
    }
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
      const connId = Number(result.connection_id);
      const conn = connections.get(connId);

      // 対象接続が存在しない場合は無視
      if (!conn) {
        return;
      }

      if (result.success) {
        // 接続情報を更新
        const next = new Map(connections);
        next.set(connId, {
          ...conn,
          connectionState: 'connected',
          streamTitle: result.stream_title ?? conn.streamTitle,
          broadcasterName: result.broadcaster_name ?? conn.broadcasterName,
          broadcasterChannelId: result.broadcaster_channel_id ?? conn.broadcasterChannelId
        });
        connections = next;
      } else if (conn.connectionState === 'disconnecting') {
        // 意図的切断 — disconnect() の finally で処理されるため何もしない
      } else {
        // 監視タスクの異常終了等 — 接続を削除してエラーを表示
        const next = new Map(connections);
        next.delete(connId);
        connections = next;
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
    get connections() {
      return connections;
    },
    get isConnected() {
      return isConnected;
    },
    // 後方互換のため残す（最初の接続のstreamTitle）
    get streamTitle() {
      if (connections.size === 0) return null;
      return [...connections.values()][0].streamTitle || null;
    },
    // 後方互換のため残す（最初の接続のbroadcasterName）
    get broadcasterName() {
      if (connections.size === 0) return null;
      return [...connections.values()][0].broadcasterName || null;
    },
    // 後方互換のため残す（最初の接続のbroadcasterChannelId）
    get broadcasterChannelId() {
      if (connections.size === 0) return null;
      return [...connections.values()][0].broadcasterChannelId || null;
    },
    // 後方互換のため残す（常にfalse）
    get isReplay() {
      return false;
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
    // 後方互換のため残す（多接続では常に'idle'か'connected'相当）
    get connectionState() {
      if (connections.size === 0) return 'idle' as const;
      const states = [...connections.values()].map(c => c.connectionState);
      if (states.some(s => s === 'connecting')) return 'connecting' as const;
      if (states.some(s => s === 'connected')) return 'connected' as const;
      return 'idle' as const;
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
    disconnectAll,
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
