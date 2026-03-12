<script lang="ts">
  import { chatStore, websocketStore, authStore } from '$lib/stores';
  import { AuthIndicator, StorageErrorDialog } from '$lib/components/auth';
  import { ViewerManagement } from '$lib/components/viewer';
  import { ChatTab, AnalyticsTab, SettingsTab } from '$lib/components/tabs';
  import Icon from '$lib/components/ui/Icon.svelte';

  type Tab = 'chat' | 'viewers' | 'analytics' | 'settings';
  type SettingsSubTab = 'auth' | 'tts' | 'raw' | 'theme';

  // アクティブタブと設定サブタブの状態
  let activeTab = $state<Tab>('chat');
  let activeSettingsSubTab = $state<SettingsSubTab>('auth');
  let showStorageErrorDialog = $state(false);

  // タブごとのアイコン・ラベル定義
  const tabInfo: Record<Tab, { icon: 'chat' | 'users' | 'chart' | 'settings'; label: string; shortLabel: string }> = {
    chat: { icon: 'chat', label: 'Chat Monitor', shortLabel: 'Chat' },
    viewers: { icon: 'users', label: 'Viewers', shortLabel: 'Viewers' },
    analytics: { icon: 'chart', label: 'Analytics', shortLabel: 'Analytics' },
    settings: { icon: 'settings', label: 'Settings', shortLabel: 'Settings' }
  };

  // 最初の接続のbroadcasterChannelIdを使用（ViewerInfoPanel用）
  let broadcasterId = $derived(
    chatStore.connections.size > 0
      ? [...chatStore.connections.values()][0].broadcasterChannelId
      : ''
  );

  // ストレージエラーが発生した場合にダイアログを表示
  let hasStorageError = $derived(authStore.storageError !== null);
  $effect(() => {
    if (hasStorageError) {
      showStorageErrorDialog = true;
    }
  });

  // 認証設定画面を開く（AuthIndicator・StorageErrorDialog からのコールバック用）
  function openAuthSettings() {
    activeTab = 'settings';
    activeSettingsSubTab = 'auth';
  }
</script>

<div class="h-screen flex flex-col overflow-hidden bg-[var(--bg-base)]">
  <!-- ヘッダー -->
  <header class="flex items-center justify-between px-4 py-2 flex-shrink-0 bg-[var(--bg-surface-1)] border-b" style="border-color: var(--border-default);">
    <!-- 左: タブ情報 + ステータスインジケーター -->
    <div class="flex items-center gap-3 min-w-0">
      <div class="flex items-center gap-2">
        <Icon name={tabInfo[activeTab].icon} size={18} class="text-[var(--accent)]" />
        <h1 class="text-sm font-semibold text-[var(--text-primary)]" style="font-family: var(--font-heading);">
          {tabInfo[activeTab].label}
        </h1>
      </div>

      <!-- ステータスインジケーター -->
      <div class="flex items-center gap-2 ml-2">
        <AuthIndicator onclick={openAuthSettings} />
        <!-- チャット接続状態 -->
        <div class="flex items-center gap-1.5 px-2 py-1 rounded-md bg-[var(--bg-surface-2)]">
          <div
            class="w-1.5 h-1.5 rounded-full"
            style="background: {chatStore.isConnected ? 'var(--success)' : 'var(--text-muted)'};"
          ></div>
          <span class="text-xs text-[var(--text-secondary)]">
            {#if chatStore.connections.size === 0}
              未接続
            {:else if chatStore.connections.size === 1}
              {@const first = [...chatStore.connections.values()][0]}
              {first.streamTitle || first.broadcasterName || '接続中'}
            {:else}
              {chatStore.connections.size}配信接続中
            {/if}
          </span>
        </div>
        <!-- WebSocket状態（実行中の場合のみ表示） -->
        {#if websocketStore.isRunning}
          <div class="flex items-center gap-1.5 px-2 py-1 rounded-md bg-[var(--bg-surface-2)]">
            <div class="w-1.5 h-1.5 rounded-full" style="background: var(--info);"></div>
            <span class="text-xs text-[var(--text-secondary)]" style="font-family: var(--font-mono);">
              WS:{websocketStore.actualPort}({websocketStore.connectedClients})
            </span>
          </div>
        {/if}
      </div>
    </div>

    <!-- 右: タブナビゲーション -->
    <nav class="flex gap-0.5 flex-shrink-0 p-0.5 rounded-lg bg-[var(--bg-surface-2)]">
      {#each (['chat', 'viewers', 'analytics', 'settings'] as const) as tab}
        <button
          onclick={() => (activeTab = tab)}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all"
          style={activeTab === tab
            ? 'background: var(--accent-subtle); color: var(--accent);'
            : 'color: var(--text-muted);'}
        >
          <Icon name={tabInfo[tab].icon} size={14} />
          <span>{tabInfo[tab].shortLabel}</span>
        </button>
      {/each}
    </nav>
  </header>

  <!-- メインコンテンツエリア -->
  <main class="flex-1 flex flex-col overflow-hidden">
    <!-- Chatタブ: VList の再マウントコストを避けるため display:none で維持 -->
    <div style:display={activeTab === 'chat' ? 'flex' : 'none'} class="flex-1 flex flex-col overflow-hidden">
      <ChatTab />
    </div>
    {#if activeTab === 'viewers'}
      <div class="flex-1 p-4 bg-[var(--bg-base)] overflow-y-auto">
        <ViewerManagement broadcasterId={broadcasterId || undefined} />
      </div>
    {:else if activeTab === 'analytics'}
      <AnalyticsTab />
    {:else if activeTab === 'settings'}
      <SettingsTab initialTab={activeSettingsSubTab} />
    {/if}
  </main>
</div>

<!-- ストレージエラーダイアログ -->
<StorageErrorDialog
  open={showStorageErrorDialog}
  onClose={() => (showStorageErrorDialog = false)}
  onOpenSettings={openAuthSettings}
/>
