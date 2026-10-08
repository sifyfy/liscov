// Chat state management using Svelte 5 runes
import { listen } from '@tauri-apps/api/event';
import { normalizeError } from '$lib/tauri/errors';
import type {
  ChatMessage,
  ConnectionResult,
  ChatMode,
  FrontendConnectionState,
  GuiReactionUpdate
} from '$lib/types';
import { SvelteMap } from 'svelte/reactivity';
import * as chatApi from '$lib/tauri/chat';
import { getConnectionColor } from '$lib/utils/connection-colors';
import { applyReactionUpdate, emptyReactionMeter, restoreReactionMeter } from '$lib/utils/reactions';
import { configStore } from './config.svelte';
import { createMessageList } from './chat-messages.svelte';

// ファクトリ関数：テスト時に独立したストアインスタンスを生成できる
function createChatStore() {
  // チャット一覧のメッセージ（バッチ・重複チェック・索引・フィルタ）
  const list = createMessageList();

  // 多接続状態マップ（キー: connection_id as number）
  // eslint-disable-next-line svelte/no-unnecessary-state-wrap -- 再代入パターン (connections = new SvelteMap(...)) でリアクティビティをトリガーするため$state必須
  let connections = $state<SvelteMap<number, FrontendConnectionState>>(new SvelteMap());
  let chatMode = $state<ChatMode>('top');
  let error = $state<string | null>(null);

  // 多接続ベースの派生状態
  let isConnected = $derived(connections.size > 0);
  let isConnecting = $derived([...connections.values()].some(c => c.connectionState === 'connecting'));

  // チャット表示設定
  const MIN_FONT_SIZE = 10;
  const MAX_FONT_SIZE = 24;
  const DEFAULT_FONT_SIZE = 13;
  let messageFontSize = $state(DEFAULT_FONT_SIZE);
  let showTimestamps = $state(true);
  let autoScroll = $state(true);
  let scrollToLatestTrigger = $state(0); // インクリメントでスクロールをトリガー

  function addMessage(message: ChatMessage): void {
    if (list.add(message)) {
      markJewelCountUnavailable(message);
    }
  }

  // ジュエル数の無いギフトを受けた接続に注記フラグを立てる（02_chat.md）
  function markJewelCountUnavailable(message: ChatMessage): void {
    if (message.message_type !== 'gift' || message.metadata?.jewel_count != null) return;
    const connId = Number(message.connection_id);
    const conn = connections.get(connId);
    if (!conn || conn.jewelCountUnavailable) return;
    const next = new SvelteMap(connections);
    next.set(connId, { ...conn, jewelCountUnavailable: true });
    connections = next;
  }

  // 現在時刻（マイクロ秒）。リアクションの60秒窓の判定に使う
  function nowUsec(): number {
    return Date.now() * 1000;
  }

  // リアクション更新をその接続のメーターに加える（02_chat.md「リアクションメーター」）
  function addReaction(update: GuiReactionUpdate): void {
    const connId = Number(update.connection_id);
    const conn = connections.get(connId);
    if (!conn) return;
    const next = new SvelteMap(connections);
    next.set(connId, { ...conn, reactions: applyReactionUpdate(conn.reactions, update, nowUsec()) });
    connections = next;
  }

  // 配信（video_id）単位の累計と直近60秒を DB から読み込む（再接続・F5 リロード後）
  // 失敗したらそのメーターは空から始める
  async function loadReactions(connId: number): Promise<void> {
    try {
      const summary = await chatApi.getConnectionReactions(connId);
      const conn = connections.get(connId);
      if (!conn) return;
      const next = new SvelteMap(connections);
      next.set(connId, { ...conn, reactions: restoreReactionMeter(summary, nowUsec()) });
      connections = next;
    } catch (e) {
      console.warn(`リアクションの読み込みに失敗 (connection ${connId}):`, e);
    }
  }

  // アクション
  // 接続中エントリの仮IDカウンタ（API応答前に一意なキーが必要）
  let nextTempConnId = -1;

  async function connect(url: string, mode?: ChatMode): Promise<ConnectionResult> {
    error = null;

    // connecting 中間状態をセット（UI: 開始ボタン無効化 + 「接続中...」表示）
    const tempId = nextTempConnId--;
    const connectingConn: FrontendConnectionState = {
      id: tempId,
      platform: 'youtube',
      streamUrl: url,
      streamTitle: '',
      broadcasterName: '',
      broadcasterChannelId: '',
      connectionState: 'connecting',
      color: getConnectionColor(String(tempId)),
      jewelCountUnavailable: false,
      reactions: emptyReactionMeter
    };
    const beforeConnect = new SvelteMap(connections);
    beforeConnect.set(tempId, connectingConn);
    connections = beforeConnect;

    try {
      const result = await chatApi.connectToStream(url, mode);

      // 仮エントリを削除
      const next = new SvelteMap(connections);
      next.delete(tempId);

      if (result.success) {
        const connId = Number(result.connection_id);
        next.set(connId, {
          id: connId,
          platform: 'youtube', // TODO: Rustから返ってきたときに更新
          streamUrl: url,
          streamTitle: result.stream_title ?? '',
          broadcasterName: result.broadcaster_name ?? '',
          broadcasterChannelId: result.broadcaster_channel_id ?? '',
          connectionState: 'connected',
          color: getConnectionColor(result.broadcaster_channel_id ?? String(connId)),
          jewelCountUnavailable: false,
          reactions: emptyReactionMeter
        });
      } else {
        error = result.error;
      }

      connections = next;
      if (result.success) {
        await loadReactions(Number(result.connection_id));
      }
      return result;
    } catch (e) {
      // 仮エントリを削除
      const next = new SvelteMap(connections);
      next.delete(tempId);
      connections = next;

      error = normalizeError(e).message;
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
      const next = new SvelteMap(connections);
      next.set(connectionId, { ...conn, connectionState: 'disconnecting' });
      connections = next;
    }

    try {
      await chatApi.disconnectStream(connectionId);
    } finally {
      // 接続マップから削除
      const next = new SvelteMap(connections);
      next.delete(connectionId);
      connections = next;
    }
  }

  // 全接続を切断（接続中のものは残す。成立したら connect() が一覧に入れる）
  async function disconnectAll(): Promise<void> {
    try {
      await chatApi.disconnectAllStreams();
    } finally {
      connections = new SvelteMap(
        [...connections].filter(([, conn]) => conn.connectionState === 'connecting')
      );
    }
  }

  // 初期化（全てクリアしてidle状態に戻る）
  async function initialize(): Promise<void> {
    try {
      await disconnectAll();
    } catch {
      // クリーンアップ中のエラーは無視
    } finally {
      connections = new SvelteMap();
      list.clear();
      error = null;
    }
  }

  async function setChatModeAction(mode: ChatMode): Promise<void> {
    chatMode = mode;
    // 永続化 (spec: 02_chat.md チャットモード、09_config.md)
    configStore.setChatMode(mode);
    // 全接続にチャットモード変更要求を送信（watch チャネル経由で次回ポーリング時に適用）
    for (const [connId] of connections) {
      try {
        await chatApi.setChatMode(connId, mode);
      } catch (e) {
        console.warn(`チャットモード変更失敗 (connection ${connId}):`, e);
      }
    }
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
        const next = new SvelteMap(connections);
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
        const next = new SvelteMap(connections);
        next.delete(connId);
        connections = next;
        error = result.error;
      }
    });

    // ライブリアクションの更新を購読
    const unlistenReaction = await listen<GuiReactionUpdate>('chat:reaction', (event) => {
      addReaction(event.payload);
    });

    unlisten = () => {
      unlistenMessage();
      unlistenConnection();
      unlistenReaction();
    };
  }

  function cleanup(): void {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
  }

  // バックエンドのアクティブ接続をフロントエンドに復元（F5リロード対応）
  async function restoreConnections(): Promise<void> {
    try {
      const backendConnections = await chatApi.getConnections();
      if (backendConnections.length === 0) return;

      const next = new SvelteMap(connections);
      for (const info of backendConnections) {
        const connId = Number(info.id);
        // 既にフロントエンドに存在する接続はスキップ
        if (next.has(connId)) continue;

        next.set(connId, {
          id: connId,
          platform: info.platform.toLowerCase() as FrontendConnectionState['platform'],
          streamUrl: info.stream_url,
          streamTitle: info.stream_title,
          broadcasterName: info.broadcaster_name,
          broadcasterChannelId: info.broadcaster_channel_id,
          connectionState: info.is_monitoring ? 'connected' : 'disconnecting',
          color: getConnectionColor(info.broadcaster_channel_id || String(connId)),
          jewelCountUnavailable: false,
          reactions: emptyReactionMeter
        });
      }
      connections = next;
      await Promise.all(backendConnections.map((info) => loadReactions(Number(info.id))));
    } catch (e) {
      console.warn('接続状態の復元に失敗:', e);
    }
  }

  // コンフィグからディスプレイ設定を初期化 (spec: 09_config.md)
  function initDisplaySettings(): void {
    if (configStore.isLoaded) {
      messageFontSize = configStore.messageFontSize;
      showTimestamps = configStore.showTimestamps;
      autoScroll = configStore.autoScrollEnabled;
      chatMode = configStore.chatMode;
    }
  }

  return {
    // Getters (リアクティブ)
    get viewerCount() {
      return list.viewerCount;
    },
    get messages() {
      return list.messages;
    },
    get filteredMessages() {
      return list.filteredMessages;
    },
    get displayedMessages() {
      return list.displayedMessages;
    },
    get connections() {
      return connections;
    },
    get isConnected() {
      return isConnected;
    },
    // 最初の接続の配信者チャンネル ID（視聴者情報パネル・視聴者タブの既定の配信者）
    get broadcasterChannelId() {
      if (connections.size === 0) return null;
      return [...connections.values()][0].broadcasterChannelId || null;
    },
    // そのメッセージが届いた接続の配信者チャンネル ID（視聴者情報パネル用。TTS が読み仮名を引く配信者と同じ）
    // 接続が一覧に無い（切断済み）か配信者が分からなければ null
    broadcasterChannelIdOf(message: ChatMessage): string | null {
      return connections.get(Number(message.connection_id))?.broadcasterChannelId || null;
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
      return list.filter;
    },
    get messageFontSize() {
      return messageFontSize;
    },
    get showTimestamps() {
      return showTimestamps;
    },
    get autoScroll() {
      return autoScroll;
    },
    get displayLimit() {
      return list.displayLimit;
    },
    get scrollToLatestTrigger() {
      return scrollToLatestTrigger;
    },

    // アクション
    connect,
    disconnect,
    disconnectAll,
    initialize,
    setChatMode: setChatModeAction,
    setFilter: list.setFilter,
    clearMessages: list.clear,
    setFontSize,
    increaseFontSize,
    decreaseFontSize,
    setShowTimestamps,
    setAutoScroll,
    scrollToLatest,
    setDisplayLimit: list.setDisplayLimit,
    getMessagesForChannel: list.getMessagesForChannel,
    setupEventListeners,
    cleanup,
    initDisplaySettings,
    restoreConnections
  };
}

// アプリ全体で使うシングルトンインスタンス
export const chatStore = createChatStore();
