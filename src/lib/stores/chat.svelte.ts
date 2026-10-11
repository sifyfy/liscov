// Chat state management using Svelte 5 runes
import { listen } from '@tauri-apps/api/event';
import { normalizeError } from '$lib/tauri/errors';
import type {
  ChatMessage,
  ConnectionResult,
  ChatMode,
  FrontendConnectionState,
  GuiReactionUpdate,
  ViewerInfo
} from '$lib/types';
import { SvelteMap } from 'svelte/reactivity';
import type { MessageOrigin } from '$lib/design/priority';
import { newcomersOf, supportersOf } from '$lib/design/flow';
import * as chatApi from '$lib/tauri/chat';
import { applyReactionUpdate, emptyReactions, restoreReactions } from '$lib/utils/reactions';
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

  // パネル・視聴者のタブで保存した読み仮名とメモ（キーは `配信者 channel_id:視聴者 channel_id`）。
  // 届いた後に登録し直したものを、画面にある行にもすぐ出すため（02_chat.md「配信中の画面」1 行の形）
  const infoOverrides = new SvelteMap<string, ViewerInfo>();
  const infoKey = (broadcasterChannelId: string, channelId: string) => `${broadcasterChannelId}:${channelId}`;

  // 主の配信の接続（最初の接続。Map は接続した順。最初を切断すれば次に古いものが繰り上がる）
  let primaryConnection = $derived(connections.values().next().value);

  // そのメッセージが主の配信のものか相手の配信のものか（02_chat.md「主の配信と相手の配信」）
  function originOf(message: ChatMessage): MessageOrigin {
    return primaryConnection === undefined || Number(message.connection_id) === primaryConnection.id ? 'primary' : 'partner';
  }

  let supporters = $derived(supportersOf(list.messages, originOf));
  let newcomers = $derived(newcomersOf(list.messages, originOf));

  // 多接続ベースの派生状態
  let isConnected = $derived(connections.size > 0);
  let isConnecting = $derived([...connections.values()].some(c => c.connectionState === 'connecting'));

  // チャット表示設定
  const MIN_FONT_SIZE = 10;
  const MAX_FONT_SIZE = 24;
  const DEFAULT_FONT_SIZE = 13;
  let messageFontSize = $state(DEFAULT_FONT_SIZE);
  let showTimestamps = $state(true);

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

  // リアクション更新をその接続の累計に加える（02_chat.md「状態の行のリアクション」）
  function addReaction(update: GuiReactionUpdate): void {
    const connId = Number(update.connection_id);
    const conn = connections.get(connId);
    if (!conn) return;
    const next = new SvelteMap(connections);
    next.set(connId, { ...conn, reactions: applyReactionUpdate(conn.reactions, update) });
    connections = next;
  }

  // 配信（video_id）単位の累計を DB から読み込む（再接続・F5 リロード後）
  // 失敗したらその接続の累計は 0 から始める
  async function loadReactions(connId: number): Promise<void> {
    try {
      const summary = await chatApi.getConnectionReactions(connId);
      const conn = connections.get(connId);
      if (!conn) return;
      const next = new SvelteMap(connections);
      next.set(connId, { ...conn, reactions: restoreReactions(summary) });
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
      jewelCountUnavailable: false,
      reactions: emptyReactions
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
          jewelCountUnavailable: false,
          reactions: emptyReactions
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
          jewelCountUnavailable: false,
          reactions: emptyReactions
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
    // 主の配信（最初の接続）の配信者チャンネル ID（視聴者情報パネル・視聴者タブの既定の配信者）
    get broadcasterChannelId() {
      return primaryConnection?.broadcasterChannelId || null;
    },
    // 主の配信の接続（最初の接続。切断したら残ったうちいちばん先のもの）。状態の行のリアクションに使う
    get primaryConnection() {
      return primaryConnection;
    },
    originOf,
    // スパチャ等の一覧 (詳細・概要) の候補と、流れた新規さんの候補（02_chat.md「流れたもの」「配信の最後のお礼」）。
    // フィルタ・表示件数によらず、一覧にあるものすべてから取る（クリアすると消える）。届いた順
    get supporters() {
      return supporters;
    },
    get newcomers() {
      return newcomers;
    },
    // パネル・視聴者のタブで保存した読み仮名とメモ。保存していなければ undefined（届いたときの author_reading・author_notes を使う）
    infoOf(message: ChatMessage): ViewerInfo | undefined {
      const broadcasterChannelId = connections.get(Number(message.connection_id))?.broadcasterChannelId;
      return broadcasterChannelId ? infoOverrides.get(infoKey(broadcasterChannelId, message.channel_id)) : undefined;
    },
    setViewerInfo(broadcasterChannelId: string, channelId: string, info: ViewerInfo) {
      infoOverrides.set(infoKey(broadcasterChannelId, channelId), info);
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
    // 状態の行のエラーの「閉じる」（02_chat.md「配信中の画面」の「状態」）
    dismissError() {
      error = null;
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
    get displayLimit() {
      return list.displayLimit;
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
