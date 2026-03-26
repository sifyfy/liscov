// WebSocket state management using Svelte 5 runes
// WebSocketサーバーはアプリ起動時に自動起動するため、手動での開始・停止は不要
import * as wsApi from '$lib/tauri/websocket';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ファクトリ関数：テスト時に独立したストアインスタンスを生成できる
function createWebsocketStore() {
  // リアクティブ状態
  let isRunning = $state(false);
  let actualPort = $state<number | null>(null);
  let connectedClients = $state(0);
  let error = $state<string | null>(null);
  let initialized = $state(false);

  // イベントリスナーの参照
  let unlistenConnect: UnlistenFn | null = null;
  let unlistenDisconnect: UnlistenFn | null = null;

  async function setupEventListeners(): Promise<void> {
    // 既存リスナーをクリーンアップ
    await cleanupEventListeners();

    // クライアント接続イベントを購読
    unlistenConnect = await listen<{ client_id: number }>('websocket-client-connected', () => {
      connectedClients++;
    });

    // クライアント切断イベントを購読
    unlistenDisconnect = await listen<{ client_id: number }>('websocket-client-disconnected', () => {
      connectedClients = Math.max(0, connectedClients - 1);
    });
  }

  async function cleanupEventListeners(): Promise<void> {
    if (unlistenConnect) {
      unlistenConnect();
      unlistenConnect = null;
    }
    if (unlistenDisconnect) {
      unlistenDisconnect();
      unlistenDisconnect = null;
    }
  }

  // ストア初期化 - 初回使用時に一度だけ呼ぶ
  // WebSocketサーバーが準備完了になるまでポーリング（アプリ起動時に自動起動）
  async function init(): Promise<void> {
    if (initialized) return;

    try {
      // クライアント接続・切断のイベントリスナーを設定
      await setupEventListeners();

      // WebSocketサーバーが準備完了になるまでポーリング（最大10秒待機）
      const maxWait = 10000;
      const pollInterval = 500;
      const start = Date.now();

      while (Date.now() - start < maxWait) {
        await refreshStatus();
        if (isRunning) {
          break;
        }
        await new Promise(resolve => setTimeout(resolve, pollInterval));
      }

      initialized = true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  // バックエンドからステータスを更新
  async function refreshStatus(): Promise<void> {
    try {
      const status = await wsApi.websocketGetStatus();
      isRunning = status.is_running;
      actualPort = status.actual_port;
      connectedClients = status.connected_clients;
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  return {
    // Getters (リアクティブ)
    get isRunning() {
      return isRunning;
    },
    get actualPort() {
      return actualPort;
    },
    get connectedClients() {
      return connectedClients;
    },
    get error() {
      return error;
    },
    get initialized() {
      return initialized;
    },

    // アクション
    init,
    refreshStatus
  };
}

// アプリ全体で使うシングルトンインスタンス
export const websocketStore = createWebsocketStore();
